use std::ops::Range;

use crate::lines::LineIndex;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Source {
    Original,
    Add,
}

#[derive(Clone, Copy, Debug)]
struct Piece {
    src: Source,
    start: usize,
    len: usize,
}

impl Piece {
    fn end(&self) -> usize {
        self.start + self.len
    }
}

#[derive(Clone, Debug)]
pub struct TextBuffer {
    original: String,
    add: String,
    pieces: Vec<Piece>,
    lines: LineIndex,
    chars: usize,
}

impl Default for TextBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl TextBuffer {
    pub fn new() -> Self {
        Self {
            original: String::new(),
            add: String::new(),
            pieces: Vec::new(),
            lines: LineIndex::from_str(""),
            chars: 0,
        }
    }

    pub fn from(text: impl Into<String>) -> Self {
        let original = text.into();
        let chars = original.chars().count();
        let lines = LineIndex::from_str(&original);
        let pieces = if original.is_empty() {
            Vec::new()
        } else {
            vec![Piece {
                src: Source::Original,
                start: 0,
                len: original.len(),
            }]
        };
        Self {
            original,
            add: String::new(),
            pieces,
            lines,
            chars,
        }
    }

    pub fn len_bytes(&self) -> usize {
        self.pieces.iter().map(|p| p.len).sum()
    }

    pub fn len_chars(&self) -> usize {
        self.chars
    }

    pub fn is_empty(&self) -> bool {
        self.pieces.is_empty()
    }

    pub fn lines(&self) -> &LineIndex {
        &self.lines
    }

    pub fn to_string(&self) -> String {
        let mut out = String::with_capacity(self.len_bytes());
        for chunk in self.chunks() {
            out.push_str(chunk);
        }
        out
    }

    pub fn chunks(&self) -> impl Iterator<Item = &str> {
        let original = self.original.as_str();
        let add = self.add.as_str();
        self.pieces.iter().map(move |p| match p.src {
            Source::Original => &original[p.start..p.end()],
            Source::Add => &add[p.start..p.end()],
        })
    }

    pub fn chunk_offsets(&self) -> Vec<(usize, &str)> {
        let mut out = Vec::with_capacity(self.pieces.len());
        let mut cursor = 0usize;
        for chunk in self.chunks() {
            out.push((cursor, chunk));
            cursor += chunk.len();
        }
        out
    }

    pub fn chars(&self) -> impl Iterator<Item = char> + '_ {
        self.chunks().flat_map(str::chars)
    }

    pub fn substring(&self, range: Range<usize>) -> String {
        let len = self.len_bytes();
        let mut start = range.start.min(len);
        let mut end = range.end.min(len).max(start);
        start = self.snap(start);
        end = self.snap(end).max(start);
        let mut out = String::new();
        let mut cursor = 0usize;
        for chunk in self.chunks() {
            let chunk_end = cursor + chunk.len();
            if chunk_end > start && cursor < end {
                let from = start.saturating_sub(cursor);
                let to = (end - cursor).min(chunk.len());
                if from < to {
                    out.push_str(&chunk[from..to]);
                }
            }
            cursor = chunk_end;
            if cursor >= end {
                break;
            }
        }
        out
    }

    pub fn snap(&self, offset: usize) -> usize {
        let offset = offset.min(self.len_bytes());
        let mut cursor = 0usize;
        for chunk in self.chunks() {
            let chunk_end = cursor + chunk.len();
            if offset < chunk_end {
                let mut b = offset - cursor;
                while b > 0 && !chunk.is_char_boundary(b) {
                    b -= 1;
                }
                return cursor + b;
            }
            cursor = chunk_end;
        }
        offset
    }

    pub fn insert(&mut self, offset: usize, text: &str) {
        if text.is_empty() {
            return;
        }
        let offset = self.snap(offset);
        let at = self.split_at(offset);
        let added_chars = text.chars().count();

        if at > 0
            && let Some(prev) = self.pieces.get_mut(at - 1)
            && prev.src == Source::Add
            && prev.end() == self.add.len()
        {
            self.add.push_str(text);
            prev.len += text.len();
            self.finish_edit(offset, 0, text, added_chars);
            return;
        }
        let base = self.add.len();
        self.add.push_str(text);
        self.pieces.insert(
            at,
            Piece {
                src: Source::Add,
                start: base,
                len: text.len(),
            },
        );
        self.normalize(at);
        self.finish_edit(offset, 0, text, added_chars);
    }

    pub fn delete(&mut self, range: Range<usize>) -> String {
        let len = self.len_bytes();
        let mut start = range.start.min(len);
        let mut end = range.end.min(len).max(start);
        start = self.snap(start);
        end = self.snap(end).max(start);
        if start == end {
            return String::new();
        }
        let removed = self.substring(start..end);
        let removed_chars = removed.chars().count();
        let lo = self.split_at(start);
        let hi = self.split_at(end);
        self.pieces.drain(lo..hi);
        self.normalize(lo.saturating_sub(1));
        self.lines.apply_edit(start, end - start, "");
        self.chars -= removed_chars;
        removed
    }

    pub fn replace(&mut self, range: Range<usize>, text: &str) -> String {
        let len = self.len_bytes();
        let start = range.start.min(len);
        let end = range.end.min(len).max(start);
        if text.is_empty() {
            return self.delete(start..end);
        }
        let removed = self.delete(start..end);
        self.insert(start, text);
        removed
    }

    fn split_at(&mut self, offset: usize) -> usize {
        let mut cursor = 0usize;
        for i in 0..self.pieces.len() {
            let len = self.pieces[i].len;
            let end = cursor + len;
            if offset < end {
                if offset > cursor {
                    let second = Piece {
                        src: self.pieces[i].src,
                        start: self.pieces[i].start + (offset - cursor),
                        len: end - offset,
                    };
                    self.pieces[i].len = offset - cursor;
                    self.pieces.insert(i + 1, second);
                    return i + 1;
                }
                return i;
            }
            cursor = end;
        }
        self.pieces.len()
    }

    fn normalize(&mut self, mut at: usize) {
        at = at.min(self.pieces.len().saturating_sub(1));

        while at > 0 {
            let (prev, cur) = (self.pieces[at - 1], self.pieces[at]);
            if prev.src == cur.src && prev.end() == cur.start {
                self.pieces[at - 1].len += cur.len;
                self.pieces.remove(at);
                at -= 1;
            } else {
                break;
            }
        }

        if at + 1 < self.pieces.len() {
            let (cur, next) = (self.pieces[at], self.pieces[at + 1]);
            if cur.src == next.src && cur.end() == next.start {
                self.pieces[at].len += next.len;
                self.pieces.remove(at + 1);
            }
        }
        self.pieces.retain(|p| p.len > 0);
    }

    fn finish_edit(
        &mut self,
        start: usize,
        deleted_len: usize,
        inserted: &str,
        added_chars: usize,
    ) {
        self.lines.apply_edit(start, deleted_len, inserted);
        self.chars += added_chars;
    }

    #[cfg(test)]
    fn piece_count(&self) -> usize {
        self.pieces.len()
    }

    #[cfg(test)]
    fn check_invariants(&self) {
        let mut bytes = 0usize;
        let mut chars = 0usize;
        for chunk in self.chunks() {
            bytes += chunk.len();
            chars += chunk.chars().count();
        }
        assert_eq!(bytes, self.len_bytes());
        assert_eq!(chars, self.chars);
        assert_eq!(self.lines.len(), bytes);
        let fresh = LineIndex::from_str(&self.to_string());
        assert_eq!(&self.lines, &fresh);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_roundtrip() {
        let buf = TextBuffer::new();
        assert!(buf.is_empty());
        assert_eq!(buf.to_string(), "");
        assert_eq!(buf.len_bytes(), 0);
        assert_eq!(buf.len_chars(), 0);
        assert_eq!(buf.lines().line_count(), 1);
    }

    #[test]
    fn insert_delete_roundtrip() {
        let mut buf = TextBuffer::from("hello world");
        buf.insert(5, ",");
        assert_eq!(buf.to_string(), "hello, world");
        let removed = buf.delete(5..6);
        assert_eq!(removed, ",");
        assert_eq!(buf.to_string(), "hello world");
        buf.check_invariants();
    }

    #[test]
    fn offsets_snap_to_char_boundaries() {
        let mut buf = TextBuffer::from("turn yellow");

        buf.insert(3, "X");
        assert_eq!(buf.to_string(), "zaXyellow");
        let removed = buf.delete(2..3);
        assert_eq!(removed, "X");
        assert_eq!(buf.to_string(), "turn yellow");
        buf.check_invariants();
    }

    #[test]
    fn multiline_edit_keeps_line_index() {
        let mut buf = TextBuffer::from("a\nb\nc");
        buf.insert(2, "X\nY");
        assert_eq!(buf.to_string(), "a\nX\nYb\nc");
        assert_eq!(buf.lines().starts(), &[0, 2, 4, 7]);
        buf.delete(2..5);
        assert_eq!(buf.to_string(), "a\nb\nc");
        assert_eq!(buf.lines().starts(), &[0, 2, 4]);
        buf.check_invariants();
    }

    #[test]
    fn replace_returns_removed() {
        let mut buf = TextBuffer::from("foo bar");
        let removed = buf.replace(4..7, "qux");
        assert_eq!(removed, "bar");
        assert_eq!(buf.to_string(), "foo qux");
        buf.check_invariants();
    }

    #[test]
    #[allow(clippy::reversed_empty_ranges)]
    fn substring_clamps() {
        let buf = TextBuffer::from("hello");
        assert_eq!(buf.substring(1..4), "ell");
        assert_eq!(buf.substring(3..99), "lo");
        assert_eq!(buf.substring(99..100), "");
        assert_eq!(buf.substring(4..1), "");
    }

    #[test]
    fn adjacent_inserts_coalesce() {
        let mut buf = TextBuffer::new();
        buf.insert(0, "a");
        buf.insert(1, "b");
        buf.insert(2, "c");
        assert_eq!(buf.to_string(), "abc");
        assert_eq!(buf.piece_count(), 1);
        buf.check_invariants();
    }

    #[test]
    fn crlf_and_emoji_survive_edits() {
        let mut buf = TextBuffer::from("a\r\nb");
        buf.insert(1, "👨‍👩‍👧");

        assert_eq!(buf.len_chars(), 4 + 5);
        assert_eq!(buf.substring(0..1), "a");
        buf.delete(0..1);
        assert_eq!(buf.to_string(), "👨‍👩‍👧\r\nb");
        buf.check_invariants();
    }

    #[test]
    fn stress_script_matches_string() {
        let mut buf = TextBuffer::from("The quick brown fox\njumps over\nthe lazy dog");
        let mut reference = buf.to_string();
        let ops: Vec<(usize, usize, &str)> = vec![
            (4, 9, "slow"),
            (0, 0, ">>> "),
            (10, 10, "\nnew line\n"),
            (0, 4, ""),
            (20, 30, "🦀🦀"),
        ];
        for (start, end, ins) in ops {
            let b = start.min(buf.len_bytes());
            let e = end.min(buf.len_bytes()).max(b);
            reference.replace_range(b..e, ins);
            buf.replace(b..e, ins);
            assert_eq!(buf.to_string(), reference);
            buf.check_invariants();
        }
    }
}
