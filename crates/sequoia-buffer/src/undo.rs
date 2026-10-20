use std::ops::Range;
use std::time::{Duration, Instant};

use crate::attr::{SpanAttr, StyledSpan};
use crate::piece::TextBuffer;
use crate::styled::StyledBuffer;

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Edit {
    Insert {
        pos: usize,
        text: String,
    },

    Delete {
        pos: usize,
        text: String,
    },

    AttrChange {
        range: Range<usize>,
        before: Vec<StyledSpan>,
        after: Vec<StyledSpan>,
    },
}

pub type UndoGroup = Vec<Edit>;

#[derive(Clone, Debug)]
pub struct UndoStack {
    undo: Vec<UndoGroup>,
    redo: Vec<UndoGroup>,
    limit: usize,
    coalesce_window: Duration,
    last_push: Option<Instant>,
}

impl UndoStack {
    pub fn new(limit: usize) -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
            limit: limit.max(1),
            coalesce_window: Duration::from_millis(0),
            last_push: None,
        }
    }

    pub fn coalesce_window(mut self, window: Duration) -> Self {
        self.coalesce_window = window;
        self
    }

    pub fn set_limit(&mut self, limit: usize) {
        self.limit = limit.max(1);
        while self.undo.len() > self.limit {
            self.undo.remove(0);
        }
        while self.redo.len() > self.limit {
            self.redo.remove(0);
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo_len(&self) -> usize {
        self.undo.len()
    }

    pub fn redo_len(&self) -> usize {
        self.redo.len()
    }

    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
        self.last_push = None;
    }

    fn push_group_inner(&mut self, group: UndoGroup) {
        if group.is_empty() {
            return;
        }
        self.undo.push(group);
        while self.undo.len() > self.limit {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.last_push = Some(Instant::now());
    }

    pub fn push_insert(&mut self, pos: usize, text: &str) {
        if text.is_empty() {
            return;
        }
        let now = Instant::now();
        if !self.coalesce_window.is_zero()
            && let Some(last) = self.last_push
            && now.duration_since(last) <= self.coalesce_window
            && let Some(group) = self.undo.last_mut()
            && let [
                Edit::Insert {
                    pos: prev_pos,
                    text: prev_text,
                },
            ] = group.as_mut_slice()
            && *prev_pos + prev_text.len() == pos
        {
            prev_text.push_str(text);
            self.last_push = Some(now);
            self.redo.clear();
            return;
        }
        self.push_group_inner(vec![Edit::Insert {
            pos,
            text: text.to_string(),
        }]);
    }

    pub fn push_delete(&mut self, pos: usize, text: &str) {
        if text.is_empty() {
            return;
        }
        let now = Instant::now();
        if !self.coalesce_window.is_zero()
            && let Some(last) = self.last_push
            && now.duration_since(last) <= self.coalesce_window
            && let Some(group) = self.undo.last_mut()
            && let [
                Edit::Delete {
                    pos: prev_pos,
                    text: prev_text,
                },
            ] = group.as_mut_slice()
        {
            if pos + text.len() == *prev_pos {
                let mut merged = text.to_string();
                merged.push_str(prev_text);
                *prev_text = merged;
                *prev_pos = pos;
                self.last_push = Some(now);
                self.redo.clear();
                return;
            }

            if pos == *prev_pos + prev_text.len() {
                prev_text.push_str(text);
                self.last_push = Some(now);
                self.redo.clear();
                return;
            }
        }
        self.push_group_inner(vec![Edit::Delete {
            pos,
            text: text.to_string(),
        }]);
    }

    pub fn push_group(&mut self, group: UndoGroup) {
        self.push_group_inner(group);
    }

    pub fn push_attr_change(
        &mut self,
        range: Range<usize>,
        before: Vec<StyledSpan>,
        after: Vec<StyledSpan>,
    ) {
        if before == after {
            return;
        }
        self.push_group_inner(vec![Edit::AttrChange {
            range,
            before,
            after,
        }]);
    }

    pub fn undo(&mut self, buf: &mut TextBuffer) -> Option<(usize, usize)> {
        let group = self.undo.pop()?;
        let mut lo = usize::MAX;
        let mut hi = 0usize;
        for edit in group.iter().rev() {
            match edit {
                Edit::Insert { pos, text } => {
                    let end = pos + text.len();
                    buf.delete(*pos..end);
                    lo = lo.min(*pos);
                    hi = hi.max(*pos);
                }
                Edit::Delete { pos, text } => {
                    buf.insert(*pos, text);
                    lo = lo.min(*pos);
                    hi = hi.max(pos + text.len());
                }
                Edit::AttrChange { range, .. } => {
                    lo = lo.min(range.start);
                    hi = hi.max(range.end);
                }
            }
        }
        self.redo.push(group);
        while self.redo.len() > self.limit {
            self.redo.remove(0);
        }
        self.last_push = None;
        if lo == usize::MAX {
            return None;
        }
        Some((lo, hi))
    }

    pub fn redo(&mut self, buf: &mut TextBuffer) -> Option<(usize, usize)> {
        let group = self.redo.pop()?;
        let mut lo = usize::MAX;
        let mut hi = 0usize;
        for edit in &group {
            match edit {
                Edit::Insert { pos, text } => {
                    buf.insert(*pos, text);
                    lo = lo.min(*pos);
                    hi = hi.max(pos + text.len());
                }
                Edit::Delete { pos, text } => {
                    let end = pos + text.len();
                    buf.delete(*pos..end);
                    lo = lo.min(*pos);
                    hi = hi.max(*pos);
                }
                Edit::AttrChange { range, .. } => {
                    lo = lo.min(range.start);
                    hi = hi.max(range.end);
                }
            }
        }
        self.undo.push(group);
        while self.undo.len() > self.limit {
            self.undo.remove(0);
        }
        self.last_push = None;
        if lo == usize::MAX {
            return None;
        }
        Some((lo, hi))
    }

    pub fn undo_styled(&mut self, buf: &mut StyledBuffer) -> Option<(usize, usize)> {
        let group = self.undo.pop()?;
        let mut texts: Vec<&Edit> = group
            .iter()
            .filter(|e| matches!(e, Edit::Insert { .. } | Edit::Delete { .. }))
            .collect();
        let mut styles: Vec<&Edit> = group
            .iter()
            .filter(|e| matches!(e, Edit::AttrChange { .. }))
            .collect();
        texts.reverse();
        styles.reverse();
        let mut lo = usize::MAX;
        let mut hi = 0usize;
        for edit in texts {
            match edit {
                Edit::Insert { pos, text } => {
                    let end = pos + text.len();
                    buf.delete(*pos..end);
                    lo = lo.min(*pos);
                    hi = hi.max(*pos);
                }
                Edit::Delete { pos, text } => {
                    buf.insert(*pos, text, Some(SpanAttr::plain()));
                    lo = lo.min(*pos);
                    hi = hi.max(pos + text.len());
                }
                _ => {
                    debug_assert!(false, "future Edit variant in text undo path");
                    continue;
                }
            }
        }
        for edit in styles {
            if let Edit::AttrChange { range, before, .. } = edit {
                buf.apply_attr_range(range.clone(), before);
                lo = lo.min(range.start);
                hi = hi.max(range.end);
            }
        }
        self.redo.push(group);
        while self.redo.len() > self.limit {
            self.redo.remove(0);
        }
        self.last_push = None;
        if lo == usize::MAX {
            return None;
        }
        Some((lo, hi))
    }

    pub fn redo_styled(&mut self, buf: &mut StyledBuffer) -> Option<(usize, usize)> {
        let group = self.redo.pop()?;
        let texts: Vec<&Edit> = group
            .iter()
            .filter(|e| matches!(e, Edit::Insert { .. } | Edit::Delete { .. }))
            .collect();
        let styles: Vec<&Edit> = group
            .iter()
            .filter(|e| matches!(e, Edit::AttrChange { .. }))
            .collect();
        let mut lo = usize::MAX;
        let mut hi = 0usize;
        for edit in texts {
            match edit {
                Edit::Insert { pos, text } => {
                    buf.insert(*pos, text, Some(SpanAttr::plain()));
                    lo = lo.min(*pos);
                    hi = hi.max(pos + text.len());
                }
                Edit::Delete { pos, text } => {
                    let end = pos + text.len();
                    buf.delete(*pos..end);
                    lo = lo.min(*pos);
                    hi = hi.max(*pos);
                }
                _ => {
                    debug_assert!(false, "future Edit variant in text redo path");
                    continue;
                }
            }
        }
        for edit in styles {
            if let Edit::AttrChange { range, after, .. } = edit {
                buf.apply_attr_range(range.clone(), after);
                lo = lo.min(range.start);
                hi = hi.max(range.end);
            }
        }
        self.undo.push(group);
        while self.undo.len() > self.limit {
            self.undo.remove(0);
        }
        self.last_push = None;
        if lo == usize::MAX {
            return None;
        }
        Some((lo, hi))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::piece::TextBuffer;

    #[test]
    fn insert_undo_redo_roundtrip() {
        let mut buf = TextBuffer::from("hello");
        let mut undo = UndoStack::new(100);
        buf.insert(5, " world");
        undo.push_insert(5, " world");
        assert_eq!(buf.to_string(), "hello world");
        undo.undo(&mut buf);
        assert_eq!(buf.to_string(), "hello");
        assert!(undo.can_redo());
        undo.redo(&mut buf);
        assert_eq!(buf.to_string(), "hello world");
    }

    #[test]
    fn delete_undo_restores_text() {
        let mut buf = TextBuffer::from("hello world");
        let removed = buf.delete(5..11);
        let mut undo = UndoStack::new(100);
        undo.push_delete(5, &removed);
        assert_eq!(buf.to_string(), "hello");
        undo.undo(&mut buf);
        assert_eq!(buf.to_string(), "hello world");
    }

    #[test]
    fn typing_coalesces_into_one_step() {
        let mut buf = TextBuffer::new();
        let mut undo = UndoStack::new(100).coalesce_window(Duration::from_secs(60));
        for (i, c) in "abc".chars().enumerate() {
            buf.insert(i, &c.to_string());
            undo.push_insert(i, &c.to_string());
        }
        assert_eq!(undo.undo_len(), 1);
        undo.undo(&mut buf);
        assert_eq!(buf.to_string(), "");
    }

    #[test]
    fn separate_bursts_stay_separate() {
        let mut buf = TextBuffer::new();
        let mut undo = UndoStack::new(100);
        buf.insert(0, "a");
        undo.push_insert(0, "a");
        buf.insert(1, "b");
        undo.push_insert(1, "b");
        assert_eq!(undo.undo_len(), 2);
    }

    #[test]
    fn group_undoes_atomically() {
        let mut buf = TextBuffer::from("a\nb");
        let mut undo = UndoStack::new(100);
        buf.insert(0, "  ");
        buf.insert(4, "  ");
        undo.push_group(vec![
            Edit::Insert {
                pos: 0,
                text: "  ".into(),
            },
            Edit::Insert {
                pos: 4,
                text: "  ".into(),
            },
        ]);
        assert_eq!(buf.to_string(), "  a\n  b");
        undo.undo(&mut buf);
        assert_eq!(buf.to_string(), "a\nb");
        undo.redo(&mut buf);
        assert_eq!(buf.to_string(), "  a\n  b");
    }

    #[test]
    fn limit_drops_oldest_and_new_edit_clears_redo() {
        let mut buf = TextBuffer::from("");
        let mut undo = UndoStack::new(2);
        for c in ['a', 'b', 'c'] {
            let pos = buf.len_bytes();
            buf.insert(pos, &c.to_string());
            undo.push_insert(pos, &c.to_string());
        }
        assert_eq!(undo.undo_len(), 2);
        undo.undo(&mut buf);
        assert!(undo.can_redo());
        let pos = buf.len_bytes();
        buf.insert(pos, "z");
        undo.push_insert(pos, "z");
        assert!(!undo.can_redo());
    }

    #[test]
    fn styled_insert_undo_redo_restores_text_style_caret() {
        use crate::attr::{SpanAttr, StyledSpan};
        use crate::styled::StyledBuffer;
        let mut buf = StyledBuffer::from("hello");
        let mut undo = UndoStack::new(100);

        let before: Vec<StyledSpan> = buf.spans().to_vec();
        buf.insert(2, "X", Some(SpanAttr::bold()));
        let after: Vec<StyledSpan> = buf.spans().to_vec();
        undo.push_group(vec![
            Edit::Insert {
                pos: 2,
                text: "X".into(),
            },
            Edit::AttrChange {
                range: 0..buf.len_bytes(),
                before,
                after: after.clone(),
            },
        ]);
        assert_eq!(buf.to_string(), "heXllo");
        let caret = undo.undo_styled(&mut buf).expect("undo");
        assert_eq!(buf.to_string(), "hello");
        assert!(buf.spans().is_empty());
        assert_eq!(caret.0.min(caret.1), 0);
        let caret = undo.redo_styled(&mut buf).expect("redo");
        assert_eq!(buf.to_string(), "heXllo");
        assert_eq!(buf.spans(), &after);
        assert!(caret.1 >= 3);
    }

    #[test]
    fn styled_delete_undo_restores_bold() {
        use crate::attr::{SpanAttr, StyledSpan};
        use crate::styled::StyledBuffer;
        let mut buf = StyledBuffer::from("hello world");
        buf.set_attr(0..5, SpanAttr::bold());
        let mut undo = UndoStack::new(100);
        let before: Vec<StyledSpan> = buf.spans().to_vec();
        let (removed, _) = buf.delete(0..5);
        let after: Vec<StyledSpan> = buf.spans().to_vec();
        undo.push_group(vec![
            Edit::Delete {
                pos: 0,
                text: removed.clone(),
            },
            Edit::AttrChange {
                range: 0..5,
                before: before.clone(),
                after,
            },
        ]);
        assert_eq!(buf.to_string(), " world");
        assert!(buf.spans().is_empty());
        undo.undo_styled(&mut buf);
        assert_eq!(buf.to_string(), "hello world");
        assert_eq!(buf.spans(), &before);
        undo.redo_styled(&mut buf);
        assert_eq!(buf.to_string(), " world");
        assert!(buf.spans().is_empty());
    }

    #[test]
    fn styled_set_attr_undo_redo() {
        use crate::attr::SpanAttr;
        use crate::styled::StyledBuffer;
        let mut buf = StyledBuffer::from("abcdef");
        let mut undo = UndoStack::new(100);
        buf.set_attr(0..3, SpanAttr::bold());
        undo.push_attr_change(0..3, vec![], buf.spans().to_vec());
        assert_eq!(buf.spans().len(), 1);
        undo.undo_styled(&mut buf);
        assert!(buf.spans().is_empty());
        undo.redo_styled(&mut buf);
        assert_eq!(buf.spans().len(), 1);
    }

    #[test]
    fn styled_plain_typing_still_coalesces() {
        use crate::styled::StyledBuffer;
        let mut buf = StyledBuffer::new();
        let mut undo = UndoStack::new(100).coalesce_window(Duration::from_secs(60));
        for (i, c) in "abc".chars().enumerate() {
            buf.insert(i, &c.to_string(), Some(SpanAttr::plain()));
            undo.push_insert(i, &c.to_string());
        }
        assert_eq!(undo.undo_len(), 1);
        undo.undo_styled(&mut buf);
        assert_eq!(buf.to_string(), "");
    }

    #[test]
    fn styled_graphemes_never_split_bytes() {
        use crate::attr::SpanAttr;
        use crate::styled::StyledBuffer;

        let text = "a👨‍👩‍👧🇵🇱é\r\nz";
        let mut buf = StyledBuffer::from(text);
        let mut undo = UndoStack::new(10);
        let len = buf.len_bytes();
        let before = buf.spans().to_vec();
        buf.set_attr(0..len, SpanAttr::bold());
        undo.push_attr_change(0..len, before, buf.spans().to_vec());

        let p = buf.snap(3);
        buf.insert(p, "Q", None);
        undo.push_insert(p, "Q");
        assert!(buf.to_string().contains('Q'));
        for s in buf.spans() {
            assert_eq!(buf.snap(s.start), s.start);
            assert_eq!(buf.snap(s.end), s.end);
        }
        undo.undo_styled(&mut buf);

        assert!(!buf.to_string().contains('Q'));
        assert_eq!(buf.spans().len(), 1);
    }
}
