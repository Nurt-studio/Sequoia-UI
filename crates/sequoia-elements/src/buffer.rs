use std::ops::Range;
use std::time::Duration;

use sequoia_buffer::{Edit, SpanAttr, StyledBuffer, StyledSpan, TextBuffer, UndoStack};
use sequoia_reactive::Signal;

const UNDO_LIMIT: usize = 100;
const COALESCE_WINDOW: Duration = Duration::from_millis(1500);

#[derive(Clone, Debug, Default, PartialEq)]
pub struct StyledDoc {
    pub text: String,
    pub spans: Vec<StyledSpan>,
}

#[derive(Clone, Debug)]
pub(crate) struct BufferEngine {
    buf: StyledBuffer,
    undo: UndoStack,

    synced: StyledDoc,
}

impl Default for BufferEngine {
    fn default() -> Self {
        Self::new()
    }
}

fn diff_prefix_suffix(old: &str, new: &str) -> (usize, usize) {
    let (ob, nb) = (old.as_bytes(), new.as_bytes());
    let mut prefix = 0usize;
    let max_prefix = ob.len().min(nb.len());
    while prefix < max_prefix && ob[prefix] == nb[prefix] {
        prefix += 1;
    }
    while prefix > 0 && !(old.is_char_boundary(prefix) && new.is_char_boundary(prefix)) {
        prefix -= 1;
    }
    let mut suffix = 0usize;
    while suffix < (ob.len() - prefix).min(nb.len() - prefix)
        && ob[ob.len() - 1 - suffix] == nb[nb.len() - 1 - suffix]
    {
        suffix += 1;
    }
    while suffix > 0
        && !(old.is_char_boundary(ob.len() - suffix) && new.is_char_boundary(nb.len() - suffix))
    {
        suffix -= 1;
    }
    (prefix, suffix)
}

fn sanitize_spans(buf: &StyledBuffer, mut spans: Vec<StyledSpan>) -> Vec<StyledSpan> {
    let len = buf.len_bytes();
    for s in &mut spans {
        s.start = buf.snap(s.start.min(len));
        s.end = buf.snap(s.end.min(len).max(s.start));
    }
    spans.retain(|s| !s.is_empty() && !s.attr.is_plain());
    sequoia_buffer::normalize_spans(&mut spans);
    spans
}

impl BufferEngine {
    pub(crate) fn new() -> Self {
        Self {
            buf: StyledBuffer::new(),
            undo: UndoStack::new(UNDO_LIMIT).coalesce_window(COALESCE_WINDOW),
            synced: StyledDoc::default(),
        }
    }

    pub(crate) fn ensure(&mut self, value: &Signal<String>) -> bool {
        let cur = value.get_untracked();
        if cur == self.synced.text {
            return false;
        }
        let (prefix, suffix) = diff_prefix_suffix(&self.synced.text, &cur);
        let old_len = self.synced.text.len();
        let new_len = cur.len();
        let old_start = prefix;
        let old_end = old_len - suffix;
        let new_text = cur[prefix..new_len - suffix].to_string();
        let before = self.buf.spans().to_vec();
        let old_buf_len = self.buf.len_bytes();
        let mut group: Vec<Edit> = Vec::with_capacity(3);
        let mut deleted_styled = false;
        if old_start < old_end {
            let (removed, deleted_spans) = self.buf.delete(old_start..old_end);
            if !removed.is_empty() {
                group.push(Edit::Delete {
                    pos: old_start,
                    text: removed,
                });
            }
            deleted_styled = !deleted_spans.is_empty();
        }
        if !new_text.is_empty() {
            let at = old_start.min(self.buf.len_bytes());
            self.buf.insert(at, &new_text, Some(SpanAttr::plain()));
            group.push(Edit::Insert {
                pos: at,
                text: new_text,
            });
        }
        if deleted_styled {
            let after = self.buf.spans().to_vec();
            let new_buf_len = self.buf.len_bytes();
            group.push(Edit::AttrChange {
                range: 0..old_buf_len.max(new_buf_len),
                before,
                after,
            });
            self.undo.push_group(group);
        } else if group.len() == 1 {
            match group.pop().unwrap() {
                Edit::Insert { pos, text } => self.undo.push_insert(pos, &text),
                Edit::Delete { pos, text } => self.undo.push_delete(pos, &text),

                other => self.undo.push_group(vec![other]),
            }
        } else if !group.is_empty() {
            self.undo.push_group(group);
        }
        self.synced.text = cur;
        self.synced.spans = self.buf.spans().to_vec();
        true
    }

    pub(crate) fn ensure_styled(&mut self, value: &Signal<StyledDoc>) -> bool {
        let cur = value.get_untracked();
        if cur == self.synced {
            return false;
        }
        let (prefix, suffix) = diff_prefix_suffix(&self.synced.text, &cur.text);
        let old_len = self.synced.text.len();
        let new_len = cur.text.len();
        let old_start = prefix;
        let old_end = old_len - suffix;
        let new_text = cur.text[prefix..new_len - suffix].to_string();
        let before = self.buf.spans().to_vec();
        let old_buf_len = self.buf.len_bytes();
        let mut group: Vec<Edit> = Vec::with_capacity(3);
        if old_start < old_end {
            let at = self.buf.snap(old_start.min(self.buf.len_bytes()));
            let (removed, _) = self.buf.delete(at..old_end.min(self.buf.len_bytes()));
            if !removed.is_empty() {
                group.push(Edit::Delete {
                    pos: at,
                    text: removed,
                });
            }
        }
        if !new_text.is_empty() {
            let at = self.buf.snap(old_start.min(self.buf.len_bytes()));
            self.buf.insert(at, &new_text, Some(SpanAttr::plain()));
            group.push(Edit::Insert {
                pos: at,
                text: new_text,
            });
        }

        let after = sanitize_spans(&self.buf, cur.spans.clone());
        if before != after {
            let new_buf_len = self.buf.len_bytes();
            self.buf.replace_spans(after.clone());
            group.push(Edit::AttrChange {
                range: 0..old_buf_len.max(new_buf_len),
                before,
                after,
            });
        }
        if group.len() == 1 {
            match group.pop().unwrap() {
                Edit::Insert { pos, text } => self.undo.push_insert(pos, &text),
                Edit::Delete { pos, text } => self.undo.push_delete(pos, &text),
                attr @ Edit::AttrChange { .. } => self.undo.push_group(vec![attr]),

                other => self.undo.push_group(vec![other]),
            }
        } else if !group.is_empty() {
            self.undo.push_group(group);
        } else {
            self.synced = cur;
            return false;
        }

        self.synced.text = cur.text;
        self.synced.spans = self.buf.spans().to_vec();
        true
    }

    pub(crate) fn commit(&mut self, value: &Signal<String>) {
        let text = self.buf.to_string();
        if text != self.synced.text {
            self.synced.text = text.clone();
            self.synced.spans = self.buf.spans().to_vec();
            value.set(text);
        } else if self.buf.spans() != self.synced.spans.as_slice() {
            self.synced.spans = self.buf.spans().to_vec();
        }
    }

    pub(crate) fn commit_styled(&mut self, value: &Signal<StyledDoc>) {
        let doc = StyledDoc {
            text: self.buf.to_string(),
            spans: self.buf.spans().to_vec(),
        };
        if doc != self.synced {
            self.synced = doc.clone();
            value.set(doc);
        }
    }

    pub(crate) fn len_chars(&self) -> usize {
        self.buf.len_chars()
    }

    pub(crate) fn to_bytes(&self, char_idx: usize) -> usize {
        let text = self.buf.text();
        let mut seen = 0usize;
        for (start, chunk) in text.chunk_offsets() {
            let n = chunk.chars().count();
            if seen + n >= char_idx {
                let mut b = start;
                for c in chunk.chars().take(char_idx - seen) {
                    b += c.len_utf8();
                }
                return b;
            }
            seen += n;
        }
        text.len_bytes()
    }

    pub(crate) fn to_chars(&self, byte: usize) -> usize {
        let text = self.buf.text();
        let byte = text.snap(byte.min(text.len_bytes()));
        let mut seen_bytes = 0usize;
        let mut seen_chars = 0usize;
        for (_, chunk) in text.chunk_offsets() {
            if seen_bytes + chunk.len() > byte {
                seen_chars += chunk[..byte - seen_bytes].chars().count();
                return seen_chars;
            }
            seen_bytes += chunk.len();
            seen_chars += chunk.chars().count();
        }
        seen_chars
    }

    pub(crate) fn insert(&mut self, offset: usize, s: &str) {
        if s.is_empty() {
            return;
        }
        let offset = self.buf.snap(offset.min(self.buf.len_bytes()));
        self.buf.insert(offset, s, Some(SpanAttr::plain()));
        self.undo.push_insert(offset, s);
    }

    #[allow(dead_code)]
    pub(crate) fn insert_styled(&mut self, offset: usize, s: &str, attr: Option<SpanAttr>) {
        if s.is_empty() {
            return;
        }
        let offset = self.buf.snap(offset.min(self.buf.len_bytes()));
        let before = self.buf.spans().to_vec();
        let old_len = self.buf.len_bytes();
        self.buf.insert(offset, s, attr);
        let after = self.buf.spans().to_vec();
        let new_len = self.buf.len_bytes();
        if before == after {
            self.undo.push_insert(offset, s);
        } else {
            self.undo.push_group(vec![
                Edit::Insert {
                    pos: offset,
                    text: s.to_string(),
                },
                Edit::AttrChange {
                    range: 0..old_len.max(new_len),
                    before,
                    after,
                },
            ]);
        }
    }

    pub(crate) fn set_attr(&mut self, range: Range<usize>, attr: SpanAttr) {
        let len = self.buf.len_bytes();
        let s = self.buf.snap(range.start.min(len));
        let e = self.buf.snap(range.end.min(len).max(s));
        if s == e {
            return;
        }
        let before = self.buf.spans().to_vec();
        self.buf.set_attr(s..e, attr);
        let after = self.buf.spans().to_vec();
        self.undo
            .push_attr_change(0..self.buf.len_bytes(), before, after);
    }

    pub(crate) fn clear_attr(&mut self, range: Range<usize>) {
        let len = self.buf.len_bytes();
        let s = self.buf.snap(range.start.min(len));
        let e = self.buf.snap(range.end.min(len).max(s));
        if s == e {
            return;
        }
        let before = self.buf.spans().to_vec();
        self.buf.clear_attr(s..e);
        let after = self.buf.spans().to_vec();
        self.undo
            .push_attr_change(0..self.buf.len_bytes(), before, after);
    }

    pub(crate) fn fit_prefix(s: &str, allowed: usize) -> &str {
        if s.chars().count() <= allowed {
            return s;
        }
        let mut end = 0usize;
        for c in s.chars().take(allowed) {
            end += c.len_utf8();
        }
        &s[..end]
    }

    pub(crate) fn delete(&mut self, range: Range<usize>) -> String {
        let len = self.buf.len_bytes();
        let start = self.buf.snap(range.start.min(len));
        let end = self.buf.snap(range.end.min(len).max(start));
        if start == end {
            return String::new();
        }
        let before = self.buf.spans().to_vec();
        let old_len = self.buf.len_bytes();
        let (removed, deleted_spans) = self.buf.delete(start..end);
        if removed.is_empty() {
            return String::new();
        }
        if deleted_spans.is_empty() {
            self.undo.push_delete(start, &removed);
        } else {
            let after = self.buf.spans().to_vec();
            let new_len = self.buf.len_bytes();
            self.undo.push_group(vec![
                Edit::Delete {
                    pos: start,
                    text: removed.clone(),
                },
                Edit::AttrChange {
                    range: 0..old_len.max(new_len),
                    before,
                    after,
                },
            ]);
        }
        removed
    }

    pub(crate) fn replace(&mut self, range: Range<usize>, s: &str) -> String {
        let len = self.buf.len_bytes();
        let start = self.buf.snap(range.start.min(len));
        let end = self.buf.snap(range.end.min(len).max(start));
        if s.is_empty() {
            return self.delete(start..end);
        }
        let before = self.buf.spans().to_vec();
        let old_len = self.buf.len_bytes();
        let (removed, deleted_spans) = self.buf.delete(start..end);
        self.buf.insert(start, s, Some(SpanAttr::plain()));
        let after = self.buf.spans().to_vec();
        let new_len = self.buf.len_bytes();
        let mut group = Vec::with_capacity(3);
        if !removed.is_empty() {
            group.push(Edit::Delete {
                pos: start,
                text: removed.clone(),
            });
        }
        group.push(Edit::Insert {
            pos: start,
            text: s.to_string(),
        });
        if !deleted_spans.is_empty() {
            group.push(Edit::AttrChange {
                range: 0..old_len.max(new_len),
                before,
                after,
            });
        }
        self.undo.push_group(group);
        removed
    }

    pub(crate) fn enforce_max_lines(&mut self, max_lines: usize) {
        let count = self.buf.lines().line_count();
        if count <= max_lines {
            return;
        }

        let cut = self.buf.lines().line_start(max_lines).saturating_sub(1);
        let before = self.buf.spans().to_vec();
        let old_len = self.buf.len_bytes();
        let (removed, deleted_spans) = self.buf.delete(cut..self.buf.len_bytes());
        if removed.is_empty() {
            return;
        }
        if deleted_spans.is_empty() {
            self.undo.push_delete(cut, &removed);
        } else {
            let after = self.buf.spans().to_vec();
            self.undo.push_group(vec![
                Edit::Delete {
                    pos: cut,
                    text: removed,
                },
                Edit::AttrChange {
                    range: 0..old_len.max(self.buf.len_bytes()),
                    before,
                    after,
                },
            ]);
        }
    }

    pub(crate) fn insert_at_many(&mut self, mut positions: Vec<usize>, s: &str) -> usize {
        if s.is_empty() || positions.is_empty() {
            return 0;
        }
        positions.sort_unstable_by(|a, b| b.cmp(a));
        let count = positions.len();
        let mut group = Vec::with_capacity(count);
        for pos in positions {
            let pos = pos.min(self.buf.len_bytes());
            let pos = self.buf.snap(pos);
            self.buf.insert(pos, s, Some(SpanAttr::plain()));
            group.push(Edit::Insert {
                pos,
                text: s.to_string(),
            });
        }
        self.undo.push_group(group);
        s.len() * count
    }

    pub(crate) fn delete_many(&mut self, mut ranges: Vec<Range<usize>>) -> usize {
        ranges.retain(|r| r.start < r.end);
        if ranges.is_empty() {
            return 0;
        }
        ranges.sort_by_key(|b| std::cmp::Reverse(b.start));
        let before = self.buf.spans().to_vec();
        let old_len = self.buf.len_bytes();
        let mut group = Vec::with_capacity(ranges.len() + 1);
        let mut removed_total = 0usize;
        let mut styled = false;
        for range in ranges {
            let len = self.buf.len_bytes();
            let start = self.buf.snap(range.start.min(len));
            let end = self.buf.snap(range.end.min(len).max(start));
            if start == end {
                continue;
            }
            let (removed, deleted_spans) = self.buf.delete(start..end);
            removed_total += removed.len();
            styled = styled || !deleted_spans.is_empty();
            group.push(Edit::Delete {
                pos: start,
                text: removed,
            });
        }
        if styled {
            group.push(Edit::AttrChange {
                range: 0..old_len.max(self.buf.len_bytes()),
                before,
                after: self.buf.spans().to_vec(),
            });
        }
        self.undo.push_group(group);
        removed_total
    }

    pub(crate) fn undo(&mut self) -> Option<(usize, usize)> {
        self.undo.undo_styled(&mut self.buf)
    }

    pub(crate) fn redo(&mut self) -> Option<(usize, usize)> {
        self.undo.redo_styled(&mut self.buf)
    }

    pub(crate) fn buffer(&self) -> &TextBuffer {
        self.buf.text()
    }

    pub(crate) fn styled_buffer(&self) -> &StyledBuffer {
        &self.buf
    }

    #[allow(dead_code)]
    pub(crate) fn spans_for_line(&self, line_range: Range<usize>) -> Vec<StyledSpan> {
        self.buf.spans_for_line(line_range)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine_of(text: &str) -> (Signal<String>, BufferEngine) {
        let value = Signal::new(text.to_string());
        let mut eng = BufferEngine::new();
        eng.ensure(&value);
        (value, eng)
    }

    fn styled_engine_of(doc: StyledDoc) -> (Signal<StyledDoc>, BufferEngine) {
        let value = Signal::new(doc);
        let mut eng = BufferEngine::new();
        eng.ensure_styled(&value);
        (value, eng)
    }

    #[test]
    fn ensure_rebases_without_clearing_history() {
        let (value, mut eng) = engine_of("hello");
        eng.insert(5, "!");
        assert_eq!(eng.undo.undo_len(), 1);

        value.set("hello?".to_string());
        assert!(eng.ensure(&value));

        eng.commit(&value);
        assert_eq!(value.get_untracked(), "hello?!");
        assert_eq!(eng.undo.undo_len(), 2);
        eng.undo();
        eng.commit(&value);
        assert_eq!(value.get_untracked(), "hello!");
    }

    #[test]
    fn commit_pushes_only_on_change() {
        let (value, mut eng) = engine_of("hi");
        eng.commit(&value);
        assert_eq!(value.get_untracked(), "hi");
        eng.insert(2, "!");
        eng.commit(&value);
        assert_eq!(value.get_untracked(), "hi!");
    }

    #[test]
    fn char_byte_roundtrip_with_emoji() {
        let (_, eng) = engine_of("a👨‍👩‍👧b");
        assert_eq!(eng.to_bytes(0), 0);
        assert_eq!(eng.to_bytes(1), 1);
        assert_eq!(eng.to_bytes(2), 5);
        assert_eq!(eng.to_bytes(6), 19);
        assert_eq!(eng.to_bytes(7), 20);
        assert_eq!(eng.to_bytes(99), 20);
        assert_eq!(eng.to_chars(0), 0);
        assert_eq!(eng.to_chars(5), 2);
        assert_eq!(eng.to_chars(19), 6);
        assert_eq!(eng.to_chars(99), 7);
    }

    #[test]
    fn replace_is_single_undo_step() {
        let (value, mut eng) = engine_of("hello world");
        eng.replace(6..11, "there");
        eng.commit(&value);
        assert_eq!(value.get_untracked(), "hello there");
        eng.undo();
        eng.commit(&value);
        assert_eq!(value.get_untracked(), "hello world");
    }

    #[test]
    fn max_lines_truncates_tail() {
        let (value, mut eng) = engine_of("a\nb\nc\nd");
        eng.enforce_max_lines(2);
        eng.commit(&value);
        assert_eq!(value.get_untracked(), "a\nb");
    }

    #[test]
    fn styled_roundtrip_and_undo() {
        let doc = StyledDoc {
            text: "hello".to_string(),
            spans: vec![StyledSpan::new(0, 5, SpanAttr::bold())],
        };
        let (value, mut eng) = styled_engine_of(doc);
        assert_eq!(eng.styled_buffer().spans().len(), 1);
        eng.set_attr(0..2, SpanAttr::link("https://x"));
        eng.commit_styled(&value);
        let cur = value.get_untracked();
        assert_eq!(cur.spans.len(), 2);
        eng.undo();
        eng.commit_styled(&value);
        assert_eq!(value.get_untracked().spans.len(), 1);
        eng.redo();
        eng.commit_styled(&value);
        assert_eq!(value.get_untracked().spans.len(), 2);
    }

    #[test]
    fn styled_external_set_merges_history() {
        let (_value, mut eng) = engine_of("hello");
        eng.insert_styled(5, "!", Some(SpanAttr::bold()));
        let n = eng.undo.undo_len();
        assert!(n >= 1);

        let ext = StyledDoc {
            text: "hello?".to_string(),
            spans: vec![StyledSpan::new(0, 5, SpanAttr::bold())],
        };
        let s = Signal::new(ext);
        assert!(eng.ensure_styled(&s));
        assert_eq!(eng.undo.undo_len(), n + 1);
        eng.commit_styled(&s);
        assert_eq!(s.get_untracked().text, "hello?!");
    }

    #[test]
    fn styled_external_noop_spans_push_no_history() {
        let (_value, mut eng) = engine_of("hello");
        let n = eng.undo.undo_len();
        let ext = StyledDoc {
            text: "hello".to_string(),
            spans: vec![StyledSpan::new(0, 2, SpanAttr::plain())],
        };
        let s = Signal::new(ext);
        assert!(!eng.ensure_styled(&s));
        assert_eq!(eng.undo.undo_len(), n);

        eng.commit_styled(&s);
        assert!(s.get_untracked().spans.is_empty());
    }

    #[test]
    fn delete_snaps_mid_char_pos_for_undo() {
        let (value, mut eng) = engine_of("aébc");
        let removed = eng.delete(2..3);
        assert_eq!(removed, "é");
        eng.commit(&value);
        assert_eq!(value.get_untracked(), "abc");
        eng.undo();
        eng.commit(&value);
        assert_eq!(value.get_untracked(), "aébc");
    }
}
