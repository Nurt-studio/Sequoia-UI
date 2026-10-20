use unicode_segmentation::GraphemeCursor;

use crate::piece::TextBuffer;

#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct Cursor {
    pub offset: usize,

    pub anchor: Option<usize>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Selection {
    pub start: usize,
    pub end: usize,
}

impl Cursor {
    pub fn new(offset: usize) -> Self {
        Self {
            offset,
            anchor: None,
        }
    }

    pub fn selection(&self) -> Option<Selection> {
        self.anchor.map(|a| Selection {
            start: a.min(self.offset),
            end: a.max(self.offset),
        })
    }

    pub fn is_collapsed(&self) -> bool {
        self.anchor.is_none_or(|a| a == self.offset)
    }

    pub fn move_to(&mut self, offset: usize, extend: bool) {
        if extend {
            let anchor = self.anchor.unwrap_or(self.offset);
            self.anchor = Some(anchor);
            self.offset = offset;
            if self.anchor == Some(self.offset) {
                self.anchor = None;
            }
        } else {
            self.offset = offset;
            self.anchor = None;
        }
    }
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

struct Table<'a> {
    chunks: Vec<(usize, &'a str)>,
    len: usize,
}

fn table(buf: &TextBuffer) -> Table<'_> {
    Table {
        chunks: buf.chunk_offsets(),
        len: buf.len_bytes(),
    }
}

fn chunk_at<'a>(t: &Table<'a>, offset: usize) -> Option<(usize, usize, &'a str)> {
    if t.chunks.is_empty() {
        return None;
    }
    let mut idx = t.chunks.partition_point(|&(s, _)| s <= offset);
    if idx == t.chunks.len() || t.chunks[idx].0 > offset {
        idx = idx.saturating_sub(1);
    }
    let (start, chunk) = t.chunks[idx];
    if offset > start + chunk.len() {
        return None;
    }
    Some((idx, start, chunk))
}

fn prev_in_table(t: &Table<'_>, offset: usize) -> usize {
    if offset == 0 || t.len == 0 {
        return 0;
    }
    let mut cursor = GraphemeCursor::new(offset, t.len, true);
    let Some((mut idx, _, _)) = chunk_at(t, offset.saturating_sub(1)) else {
        return 0;
    };

    if offset > 0 {
        while idx + 1 < t.chunks.len() && t.chunks[idx + 1].0 < offset {
            idx += 1;
        }
    }
    loop {
        let (start, chunk) = t.chunks[idx];
        match cursor.prev_boundary(chunk, start) {
            Ok(Some(b)) => return b,
            Ok(None) => return 0,
            Err(unicode_segmentation::GraphemeIncomplete::PrevChunk) => {
                if idx == 0 {
                    return 0;
                }
                idx -= 1;
            }
            Err(_) => return offset,
        }
    }
}

fn next_in_table(t: &Table<'_>, offset: usize) -> usize {
    if offset >= t.len || t.len == 0 {
        return t.len;
    }
    let mut cursor = GraphemeCursor::new(offset, t.len, true);
    let Some((mut idx, _, _)) = chunk_at(t, offset) else {
        return t.len;
    };
    loop {
        let (start, chunk) = t.chunks[idx];
        match cursor.next_boundary(chunk, start) {
            Ok(Some(b)) => return b,
            Ok(None) => return t.len,
            Err(unicode_segmentation::GraphemeIncomplete::NextChunk) => {
                if idx + 1 >= t.chunks.len() {
                    return t.len;
                }
                idx += 1;
            }
            Err(_) => return offset,
        }
    }
}

fn grapheme_first_char(t: &Table<'_>, pos: usize) -> char {
    if pos >= t.len {
        return '\0';
    }
    match chunk_at(t, pos) {
        Some((_, start, chunk)) => chunk[pos - start..].chars().next().unwrap_or('\0'),
        None => '\0',
    }
}

fn is_word_span(t: &Table<'_>, from: usize, to: usize) -> bool {
    from < to && is_word_char(grapheme_first_char(t, from))
}

pub fn prev_grapheme(buf: &TextBuffer, offset: usize) -> usize {
    let t = table(buf);
    prev_in_table(&t, buf.snap(offset))
}

pub fn next_grapheme(buf: &TextBuffer, offset: usize) -> usize {
    let t = table(buf);
    next_in_table(&t, buf.snap(offset))
}

pub fn word_start_before(buf: &TextBuffer, offset: usize) -> usize {
    let t = table(buf);
    let mut i = buf.snap(offset).min(t.len);
    while i > 0 {
        let p = prev_in_table(&t, i);
        if p == i || is_word_span(&t, p, i) {
            break;
        }
        i = p;
    }
    while i > 0 {
        let p = prev_in_table(&t, i);
        if p == i || !is_word_span(&t, p, i) {
            break;
        }
        i = p;
    }
    i
}

pub fn word_end_after(buf: &TextBuffer, offset: usize) -> usize {
    let t = table(buf);
    let mut i = buf.snap(offset).min(t.len);
    while i < t.len {
        let n = next_in_table(&t, i);
        if n == i || is_word_span(&t, i, n) {
            break;
        }
        i = n;
    }
    while i < t.len {
        let n = next_in_table(&t, i);
        if n == i || !is_word_span(&t, i, n) {
            break;
        }
        i = n;
    }
    i
}

pub fn line_home(buf: &TextBuffer, offset: usize) -> usize {
    let lines = buf.lines();
    lines.line_start(lines.line_of(offset.min(buf.len_bytes())))
}

pub fn line_end(buf: &TextBuffer, offset: usize) -> usize {
    let lines = buf.lines();
    let (_, end) = lines.line_range(lines.line_of(offset.min(buf.len_bytes())));
    end
}

pub fn move_lines(
    buf: &TextBuffer,
    offset: usize,
    delta: isize,
    goal_col: Option<usize>,
) -> (usize, usize) {
    let lines = buf.lines();
    let len = buf.len_bytes();
    let offset = buf.snap(offset).min(len);
    let (line, _) = lines.line_col(offset);
    let col = match goal_col {
        Some(c) => c,
        None => {
            let (start, _) = lines.line_range(line);
            buf.substring(start..offset).chars().count()
        }
    };
    let target = (line as isize + delta)
        .max(0)
        .min(lines.line_count().saturating_sub(1) as isize) as usize;
    let (tstart, tend) = lines.line_range(target);
    let mut pos = tstart;
    let mut actual = 0usize;
    for _ in 0..col {
        if pos >= tend {
            break;
        }
        let t = table(buf);
        let n = next_in_table(&t, pos);
        if n == pos || n > tend {
            break;
        }
        pos = n;
        actual += 1;
    }
    (pos, actual)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::piece::TextBuffer;

    #[test]
    fn cursor_selection_semantics() {
        let mut c = Cursor::new(5);
        assert_eq!(c.selection(), None);
        assert!(c.is_collapsed());
        c.move_to(8, true);
        assert_eq!(c.selection(), Some(Selection { start: 5, end: 8 }));

        c.move_to(5, true);
        assert_eq!(c.selection(), None);
        assert!(c.is_collapsed());

        c.move_to(10, false);
        c.move_to(3, true);
        assert_eq!(c.selection(), Some(Selection { start: 3, end: 10 }));
        c.move_to(0, false);
        assert_eq!(c.selection(), None);
    }

    #[test]
    fn graphemes_never_split() {
        let buf = TextBuffer::from("a👨‍👩‍👧🇵🇱é\r\nz");
        let len = buf.len_bytes();

        let mut pos = 0usize;
        let mut count = 0;
        while pos < len {
            let n = next_grapheme(&buf, pos);
            assert!(n > pos, "must advance at {pos}");
            assert!(buf.snap(n) == n, "boundary at {n}");
            pos = n;
            count += 1;
            assert!(count < 100, "no infinite loop");
        }
        assert_eq!(pos, len);

        let mut back = len;
        let mut count = 0;
        while back > 0 {
            back = prev_grapheme(&buf, back);
            count += 1;
            assert!(count < 100, "no infinite loop");
        }
        assert_eq!(back, 0);

        assert_eq!(next_grapheme(&buf, 1), 1 + "👨‍👩‍👧".len());
        assert_eq!(prev_grapheme(&buf, 1 + "👨‍👩‍👧".len()), 1);
    }

    #[test]
    fn words_mirror_widget_behavior() {
        let buf = TextBuffer::from("foo bar_baz2  qux");

        assert_eq!(word_start_before(&buf, 11), 4);
        assert_eq!(word_start_before(&buf, 4), 0);
        assert_eq!(word_start_before(&buf, 3), 0);
        assert_eq!(word_end_after(&buf, 0), 3);
        assert_eq!(word_end_after(&buf, 3), 12);
        assert_eq!(word_end_after(&buf, 11), 12);

        let buf = TextBuffer::from("ab👨‍👩‍👧cd");
        assert_eq!(buf.len_bytes(), 22);
        assert_eq!(word_start_before(&buf, buf.len_bytes()), 20);
    }

    #[test]
    fn line_home_end() {
        let buf = TextBuffer::from("ab\ncde\n\nf");
        assert_eq!(line_home(&buf, 5), 3);
        assert_eq!(line_end(&buf, 5), 6);
        assert_eq!(line_home(&buf, 0), 0);
        assert_eq!(line_end(&buf, 8), 9);
    }

    #[test]
    fn vertical_moves_clamp_to_short_lines() {
        let buf = TextBuffer::from("abcdef\nxy\nabcdef");

        let (pos, col) = move_lines(&buf, 4, 1, None);
        assert_eq!(col, 2);
        assert_eq!(pos, 7 + 2);

        let (pos2, col2) = move_lines(&buf, pos, 1, Some(4));
        assert_eq!((col2, pos2), (4, 10 + 4));

        let (pos3, _) = move_lines(&buf, 2, -5, None);
        assert_eq!(pos3, 2);
    }
}
