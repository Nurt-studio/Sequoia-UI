use std::ops::Range;

use crate::attr::{SpanAttr, StyledSpan, normalize_spans};
use crate::piece::TextBuffer;

#[derive(Clone, Debug, Default)]
pub struct StyledBuffer {
    text: TextBuffer,
    spans: Vec<StyledSpan>,
}

impl StyledBuffer {
    pub fn new() -> Self {
        Self {
            text: TextBuffer::new(),
            spans: Vec::new(),
        }
    }

    pub fn from(text: impl Into<String>) -> Self {
        Self {
            text: TextBuffer::from(text),
            spans: Vec::new(),
        }
    }

    pub fn from_parts(text: TextBuffer, mut spans: Vec<StyledSpan>) -> Self {
        let len = text.len_bytes();
        for s in &mut spans {
            s.start = text.snap(s.start.min(len));
            s.end = text.snap(s.end.min(len).max(s.start));
        }
        spans.retain(|s| !s.is_empty() && !s.attr.is_plain());
        normalize_spans(&mut spans);
        Self { text, spans }
    }

    pub fn text(&self) -> &TextBuffer {
        &self.text
    }

    pub fn text_mut(&mut self) -> &mut TextBuffer {
        &mut self.text
    }

    pub fn spans(&self) -> &[StyledSpan] {
        &self.spans
    }

    pub fn len_bytes(&self) -> usize {
        self.text.len_bytes()
    }

    pub fn len_chars(&self) -> usize {
        self.text.len_chars()
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    pub fn to_string(&self) -> String {
        self.text.to_string()
    }

    pub fn snap(&self, offset: usize) -> usize {
        self.text.snap(offset)
    }

    pub fn lines(&self) -> &crate::LineIndex {
        self.text.lines()
    }

    pub fn substring(&self, range: Range<usize>) -> String {
        self.text.substring(range)
    }

    pub fn insert(&mut self, pos: usize, text: &str, attr: Option<SpanAttr>) {
        if text.is_empty() {
            return;
        }
        let len = self.text.len_bytes();
        let pos = self.text.snap(pos.min(len));
        let ins_len = text.len();

        match attr {
            None => self.insert_inherit(pos, ins_len),
            Some(a) if a.is_plain() => self.shift_for_insert(pos, ins_len, None),
            Some(a) => self.shift_for_insert(pos, ins_len, Some(a)),
        }
        self.text.insert(pos, text);
        normalize_spans(&mut self.spans);
        debug_assert!(self.check_spans_sorted());
    }

    fn shift_for_insert(&mut self, pos: usize, ins_len: usize, middle: Option<SpanAttr>) {
        let host = self.spans.iter().position(|s| s.start < pos && pos < s.end);
        if let Some(idx) = host {
            let (hs, he, hattr) = {
                let h = &self.spans[idx];
                (h.start, h.end, h.attr.clone())
            };

            let mut out = Vec::with_capacity(self.spans.len() + 1);
            for s in self.spans.iter().take(idx) {
                out.push(s.clone());
            }
            out.push(StyledSpan::new(hs, pos, hattr.clone()));
            if let Some(m) = middle {
                if m == hattr {
                    out.pop();
                    out.push(StyledSpan::new(hs, he + ins_len, hattr));
                } else {
                    out.push(StyledSpan::new(pos, pos + ins_len, m));
                    out.push(StyledSpan::new(pos + ins_len, he + ins_len, hattr));
                }
            } else {
                out.push(StyledSpan::new(pos + ins_len, he + ins_len, hattr));
            }
            for s in self.spans.iter().skip(idx + 1) {
                out.push(StyledSpan::new(
                    s.start + ins_len,
                    s.end + ins_len,
                    s.attr.clone(),
                ));
            }
            self.spans = out;
            return;
        }

        let mut out = Vec::with_capacity(self.spans.len() + 1);
        for s in &self.spans {
            if s.start >= pos {
                out.push(StyledSpan::new(
                    s.start + ins_len,
                    s.end + ins_len,
                    s.attr.clone(),
                ));
            } else {
                debug_assert!(s.end <= pos);
                out.push(s.clone());
            }
        }
        if let Some(m) = middle {
            out.push(StyledSpan::new(pos, pos + ins_len, m));
            out.sort_by_key(|s| (s.start, s.end));
        }
        self.spans = out;
    }

    fn insert_inherit(&mut self, pos: usize, ins_len: usize) {
        if let Some(idx) = self.spans.iter().position(|s| s.start < pos && pos < s.end) {
            let (hs, he, hattr) = {
                let h = &self.spans[idx];
                (h.start, h.end, h.attr.clone())
            };
            let mut out = Vec::with_capacity(self.spans.len());
            for s in self.spans.iter().take(idx) {
                out.push(s.clone());
            }
            out.push(StyledSpan::new(hs, he + ins_len, hattr));
            for s in self.spans.iter().skip(idx + 1) {
                out.push(StyledSpan::new(
                    s.start + ins_len,
                    s.end + ins_len,
                    s.attr.clone(),
                ));
            }
            self.spans = out;
            return;
        }

        if let Some(idx) = self.spans.iter().position(|s| s.end == pos) {
            let mut out = self.spans.clone();
            out[idx].end += ins_len;

            for s in &mut out {
                if s.start >= pos {
                    s.start += ins_len;
                    s.end += ins_len;
                }
            }
            self.spans = out;
            return;
        }
        if let Some(idx) = self.spans.iter().position(|s| s.start == pos) {
            let mut out = self.spans.clone();

            for (i, s) in out.iter_mut().enumerate() {
                if i == idx {
                    s.end += ins_len;
                } else if s.start >= pos {
                    s.start += ins_len;
                    s.end += ins_len;
                }
            }
            self.spans = out;
            return;
        }
        self.shift_for_insert(pos, ins_len, None);
    }

    pub fn delete(&mut self, range: Range<usize>) -> (String, Vec<StyledSpan>) {
        let len = self.text.len_bytes();
        let mut s = range.start.min(len);
        let mut e = range.end.min(len).max(s);
        s = self.text.snap(s);
        e = self.text.snap(e).max(s);
        if s == e {
            return (String::new(), Vec::new());
        }
        let d = e - s;
        let mut removed_spans = Vec::new();
        let mut kept = Vec::with_capacity(self.spans.len());
        for sp in &self.spans {
            if sp.end <= s {
                kept.push(sp.clone());
            } else if sp.start >= e {
                kept.push(StyledSpan::new(sp.start - d, sp.end - d, sp.attr.clone()));
            } else if sp.start < s && sp.end > e {
                kept.push(StyledSpan::new(sp.start, sp.end - d, sp.attr.clone()));
                if let Some(c) = sp.clipped(s..e) {
                    removed_spans.push(c);
                }
            } else if sp.start < s {
                debug_assert!(sp.end > s && sp.end <= e);
                kept.push(StyledSpan::new(sp.start, s, sp.attr.clone()));
                if let Some(c) = sp.clipped(s..e) {
                    removed_spans.push(c);
                }
            } else if sp.end > e {
                debug_assert!(sp.start >= s && sp.start < e);
                kept.push(StyledSpan::new(s, sp.end - d, sp.attr.clone()));
                if let Some(c) = sp.clipped(s..e) {
                    removed_spans.push(c);
                }
            } else {
                removed_spans.push(sp.clone());
            }
        }
        let removed_text = self.text.delete(s..e);
        self.spans = kept;
        normalize_spans(&mut self.spans);
        debug_assert!(self.check_spans_sorted());
        (removed_text, removed_spans)
    }

    pub fn replace(
        &mut self,
        range: Range<usize>,
        text: &str,
        attr: Option<SpanAttr>,
    ) -> (String, Vec<StyledSpan>) {
        let len = self.text.len_bytes();
        let s = self.text.snap(range.start.min(len));
        let e = self.text.snap(range.end.min(len).max(s));
        if text.is_empty() {
            return self.delete(s..e);
        }
        let (removed_text, removed_spans) = self.delete(s..e);
        self.insert(s, text, attr);
        (removed_text, removed_spans)
    }

    pub fn set_attr(&mut self, range: Range<usize>, attr: SpanAttr) {
        if attr.is_plain() {
            self.clear_attr(range);
            return;
        }
        let len = self.text.len_bytes();
        let mut s = range.start.min(len);
        let mut e = range.end.min(len).max(s);
        s = self.text.snap(s);
        e = self.text.snap(e).max(s);
        if s == e {
            return;
        }
        let mut out = Vec::with_capacity(self.spans.len() + 1);
        for sp in &self.spans {
            if sp.end <= s || sp.start >= e {
                out.push(sp.clone());
            } else {
                if sp.start < s {
                    out.push(StyledSpan::new(sp.start, s, sp.attr.clone()));
                }
                if sp.end > e {
                    out.push(StyledSpan::new(e, sp.end, sp.attr.clone()));
                }
            }
        }
        out.push(StyledSpan::new(s, e, attr));
        normalize_spans(&mut out);
        self.spans = out;
        debug_assert!(self.check_spans_sorted());
    }

    pub fn clear_attr(&mut self, range: Range<usize>) {
        let len = self.text.len_bytes();
        let mut s = range.start.min(len);
        let mut e = range.end.min(len).max(s);
        s = self.text.snap(s);
        e = self.text.snap(e).max(s);
        if s == e {
            return;
        }
        let mut out = Vec::with_capacity(self.spans.len());
        for sp in &self.spans {
            if sp.end <= s || sp.start >= e {
                out.push(sp.clone());
            } else {
                if sp.start < s {
                    out.push(StyledSpan::new(sp.start, s, sp.attr.clone()));
                }
                if sp.end > e {
                    out.push(StyledSpan::new(e, sp.end, sp.attr.clone()));
                }
            }
        }
        self.spans = out;
        debug_assert!(self.check_spans_sorted());
    }

    pub fn spans_for_line(&self, line_range: Range<usize>) -> Vec<StyledSpan> {
        if line_range.start >= line_range.end {
            return Vec::new();
        }
        self.spans
            .iter()
            .filter(|s| s.start < line_range.end && line_range.start < s.end)
            .filter_map(|s| s.clipped(line_range.clone()))
            .collect()
    }

    pub fn apply_attr_range(&mut self, range: Range<usize>, spans: &[StyledSpan]) {
        let len = self.text.len_bytes();
        let mut s = range.start.min(len);
        let mut e = range.end.min(len).max(s);
        s = self.text.snap(s);
        e = self.text.snap(e).max(s);
        let mut out = Vec::with_capacity(self.spans.len() + spans.len());
        for sp in &self.spans {
            if sp.end <= s || sp.start >= e {
                out.push(sp.clone());
            } else {
                if sp.start < s {
                    out.push(StyledSpan::new(sp.start, s, sp.attr.clone()));
                }
                if sp.end > e {
                    out.push(StyledSpan::new(e, sp.end, sp.attr.clone()));
                }
            }
        }
        for sp in spans {
            if let Some(c) = sp.clipped(s..e)
                && !c.attr.is_plain()
            {
                out.push(c);
            }
        }
        normalize_spans(&mut out);
        self.spans = out;
        debug_assert!(self.check_spans_sorted());
    }

    pub fn replace_spans(&mut self, mut spans: Vec<StyledSpan>) {
        let len = self.text.len_bytes();
        for s in &mut spans {
            s.start = self.text.snap(s.start.min(len));
            s.end = self.text.snap(s.end.min(len).max(s.start));
        }
        spans.retain(|s| !s.is_empty() && !s.attr.is_plain());
        normalize_spans(&mut spans);
        self.spans = spans;
    }

    fn check_spans_sorted(&self) -> bool {
        let len = self.text.len_bytes();
        let mut prev_end = None;
        for s in &self.spans {
            if s.start >= s.end {
                return false;
            }
            if s.end > len {
                return false;
            }

            if self.text.snap(s.start) != s.start || self.text.snap(s.end) != s.end {
                return false;
            }
            if let Some(p) = prev_end {
                if s.start < p {
                    return false;
                }
                if s.start == p && self.spans_first_eq_prev(s) {
                    return false;
                }
            }
            prev_end = Some(s.end);
        }
        true
    }

    fn spans_first_eq_prev(&self, _s: &StyledSpan) -> bool {
        let mut prev: Option<&StyledSpan> = None;
        for cur in &self.spans {
            if let Some(p) = prev
                && p.end == cur.start
                && p.attr == cur.attr
            {
                return true;
            }
            prev = Some(cur);
            if cur.start == _s.start && cur.end == _s.end {
                break;
            }
        }
        false
    }

    #[cfg(test)]
    fn check_invariants(&self) {
        assert!(self.check_spans_sorted(), "spans: {:?}", self.spans);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bold_span(s: usize, e: usize) -> StyledSpan {
        StyledSpan::new(s, e, SpanAttr::bold())
    }

    #[test]
    fn insert_delete_set_attr_roundtrip() {
        let mut b = StyledBuffer::from("hello world");
        b.set_attr(0..5, SpanAttr::bold());
        assert_eq!(b.spans(), &[bold_span(0, 5)]);
        let (t, sp) = b.delete(0..5);
        assert_eq!(t, "hello");
        assert_eq!(sp, vec![bold_span(0, 5)]);
        assert!(b.spans().is_empty());
        assert_eq!(b.to_string(), " world");

        b.insert(0, "hello", Some(SpanAttr::plain()));
        assert_eq!(b.to_string(), "hello world");
        b.apply_attr_range(0..5, &[bold_span(0, 5)]);
        assert_eq!(b.spans(), &[bold_span(0, 5)]);
        b.check_invariants();
    }

    #[test]
    fn neighbours_merge_after_set() {
        let mut b = StyledBuffer::from("abcdef");
        b.set_attr(0..2, SpanAttr::bold());
        b.set_attr(2..4, SpanAttr::bold());
        assert_eq!(b.spans(), &[bold_span(0, 4)]);
        b.check_invariants();
    }

    #[test]
    fn typing_inside_span_inherits() {
        let mut b = StyledBuffer::from("hello");
        b.set_attr(0..5, SpanAttr::bold());
        b.insert(2, "X", None);
        assert_eq!(b.to_string(), "heXllo");
        assert_eq!(b.spans(), &[bold_span(0, 6)]);

        let mut c = StyledBuffer::from("hello");
        c.set_attr(0..5, SpanAttr::bold());
        c.insert(2, "X", Some(SpanAttr::plain()));
        assert_eq!(c.spans(), &[bold_span(0, 2), bold_span(3, 6)]);

        let mut d = StyledBuffer::from("hello");
        d.set_attr(0..5, SpanAttr::bold());
        d.insert(2, "X", Some(SpanAttr::link("u")));
        assert_eq!(d.spans().len(), 3);
        assert_eq!(d.to_string(), "heXllo");
        b.check_invariants();
        c.check_invariants();
        d.check_invariants();
    }

    #[test]
    fn boundary_inserts_left_biased() {
        let mut b = StyledBuffer::from("ab");
        b.set_attr(0..1, SpanAttr::bold());
        b.set_attr(1..2, SpanAttr::link("u"));

        b.insert(1, "X", None);
        assert_eq!(b.to_string(), "aXb");
        assert_eq!(b.spans()[0], bold_span(0, 2));
        b.check_invariants();
    }

    #[test]
    fn grapheme_text_does_not_break_spans() {
        let text = "a👨‍👩‍👧🇵🇱é\r\nz";
        let mut b = StyledBuffer::from(text);
        let len = b.len_bytes();
        b.set_attr(0..len, SpanAttr::bold());
        assert_eq!(b.spans(), &[bold_span(0, len)]);

        let mid = 2usize;
        let snapped = b.snap(mid);
        assert_eq!(b.snap(snapped), snapped);
        b.insert(snapped, "Q", None);
        assert!(b.snap(b.len_bytes()) == b.len_bytes());
        let (t, _) = b.delete(0..1);
        assert_eq!(t, "a");
        b.check_invariants();

        let flag = "🇵🇱";
        let mut f = StyledBuffer::from(flag);
        f.set_attr(0..flag.len(), SpanAttr::bold());
        f.insert(4, "·", Some(SpanAttr::plain()));
        assert_eq!(f.to_string(), "🇵·🇱");
        f.check_invariants();
    }

    #[test]
    fn spans_for_line_clips() {
        let mut b = StyledBuffer::from("ab\ncde\n\nf");
        b.set_attr(0..9, SpanAttr::bold());

        let v = b.spans_for_line(3..6);
        assert_eq!(v, vec![bold_span(3, 6)]);
        assert!(b.spans_for_line(6..6).is_empty());

        let v = b.spans_for_line(5..8);
        assert_eq!(v, vec![bold_span(5, 8)]);
    }

    #[test]
    fn clear_attr_splits() {
        let mut b = StyledBuffer::from("abcdef");
        b.set_attr(0..6, SpanAttr::bold());
        b.clear_attr(2..4);
        assert_eq!(b.spans(), &[bold_span(0, 2), bold_span(4, 6)]);
        b.check_invariants();
    }

    #[test]
    fn delete_covering_joins() {
        let mut b = StyledBuffer::from("hello world");
        b.set_attr(0..11, SpanAttr::bold());
        let (t, sp) = b.delete(5..6);
        assert_eq!(t, " ");
        assert_eq!(sp, vec![bold_span(5, 6)]);
        assert_eq!(b.spans(), &[bold_span(0, 10)]);
        b.check_invariants();
    }
}
