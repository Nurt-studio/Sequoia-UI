use sequoia_core::Color;

#[derive(Clone, Debug, PartialEq)]
pub struct SpanAttr {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strike: bool,
    pub color: Option<Color>,
    pub bg: Option<Color>,
    pub size_delta: f32,
    pub link: Option<String>,
    pub code: bool,
    pub mention: Option<u64>,
}

impl Default for SpanAttr {
    fn default() -> Self {
        Self::plain()
    }
}

impl SpanAttr {
    pub fn plain() -> Self {
        Self {
            bold: false,
            italic: false,
            underline: false,
            strike: false,
            color: None,
            bg: None,
            size_delta: 0.0,
            link: None,
            code: false,
            mention: None,
        }
    }

    pub fn is_plain(&self) -> bool {
        !self.bold
            && !self.italic
            && !self.underline
            && !self.strike
            && self.color.is_none()
            && self.bg.is_none()
            && self.size_delta == 0.0
            && self.link.is_none()
            && !self.code
            && self.mention.is_none()
    }

    pub fn bold() -> Self {
        Self {
            bold: true,
            ..Self::plain()
        }
    }

    pub fn link(url: impl Into<String>) -> Self {
        Self {
            link: Some(url.into()),
            ..Self::plain()
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StyledSpan {
    pub start: usize,
    pub end: usize,
    pub attr: SpanAttr,
}

impl StyledSpan {
    pub fn new(start: usize, end: usize, attr: SpanAttr) -> Self {
        Self { start, end, attr }
    }

    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    pub fn is_empty(&self) -> bool {
        self.start >= self.end
    }

    pub fn contains_middle(&self, pos: usize) -> bool {
        self.start < pos && pos < self.end
    }

    pub fn overlaps(&self, range: std::ops::Range<usize>) -> bool {
        self.start < range.end && range.start < self.end
    }

    pub fn clipped(&self, range: std::ops::Range<usize>) -> Option<Self> {
        let s = self.start.max(range.start);
        let e = self.end.min(range.end);
        if s < e {
            Some(Self {
                start: s,
                end: e,
                attr: self.attr.clone(),
            })
        } else {
            None
        }
    }
}

pub fn normalize_spans(spans: &mut Vec<StyledSpan>) {
    spans.retain(|s| s.start < s.end);
    spans.sort_by_key(|s| (s.start, s.end));
    let mut out: Vec<StyledSpan> = Vec::with_capacity(spans.len());
    for mut cur in spans.drain(..) {
        if cur.start >= cur.end {
            continue;
        }
        if let Some(prev) = out.last_mut() {
            if cur.start < prev.end {
                if prev.attr == cur.attr {
                    prev.end = prev.end.max(cur.end);
                    continue;
                }
                cur.start = prev.end;
                if cur.start >= cur.end {
                    continue;
                }
            }
            if prev.end == cur.start && prev.attr == cur.attr {
                prev.end = cur.end;
                continue;
            }
        }
        out.push(cur);
    }
    *spans = out;
}

pub fn hash_span_attr<H: std::hash::Hasher>(a: &SpanAttr, h: &mut H) {
    use std::hash::Hash;
    a.bold.hash(h);
    a.italic.hash(h);
    a.underline.hash(h);
    a.strike.hash(h);
    a.color
        .map(|c| (c.r.to_bits(), c.g.to_bits(), c.b.to_bits(), c.a.to_bits()))
        .hash(h);
    a.bg.map(|c| (c.r.to_bits(), c.g.to_bits(), c.b.to_bits(), c.a.to_bits()))
        .hash(h);
    let q = if a.size_delta.is_finite() {
        (a.size_delta * 4.0).round() as i32
    } else {
        0
    };
    q.hash(h);
    a.link.hash(h);
    a.code.hash(h);
    a.mention.hash(h);
}

pub fn hash_spans_rebased<H: std::hash::Hasher>(spans: &[StyledSpan], base: usize, h: &mut H) {
    use std::hash::Hash;
    spans.len().hash(h);
    for s in spans {
        if s.is_empty() {
            continue;
        }
        s.start.saturating_sub(base).hash(h);
        s.end.saturating_sub(base).hash(h);
        hash_span_attr(&s.attr, h);
    }
}

pub fn spans_hash(spans: &[StyledSpan]) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::Hasher;
    let mut h = DefaultHasher::new();
    hash_spans_rebased(spans, 0, &mut h);
    h.finish()
}

pub fn line_hash(text: &str, spans: &[StyledSpan], width: f32) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    text.hash(&mut h);
    hash_spans_rebased(spans, 0, &mut h);
    ((width.max(0.0) * 4.0).round() as u32).hash(&mut h);
    h.finish()
}

pub fn rebase_spans(spans: &[StyledSpan], base: usize) -> Vec<StyledSpan> {
    spans
        .iter()
        .map(|s| {
            StyledSpan::new(
                s.start.saturating_sub(base),
                s.end.saturating_sub(base),
                s.attr.clone(),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_roundtrip() {
        let p = SpanAttr::plain();
        assert!(p.is_plain());
        assert_eq!(p, SpanAttr::default());
        let b = SpanAttr::bold();
        assert!(!b.is_plain());
        assert!(b.bold);
    }

    #[test]
    fn neighbours_merge() {
        let mut v = vec![
            StyledSpan::new(0, 2, SpanAttr::bold()),
            StyledSpan::new(2, 5, SpanAttr::bold()),
            StyledSpan::new(5, 6, SpanAttr::plain()),
        ];

        normalize_spans(&mut v);
        assert_eq!(v.len(), 2);
        assert_eq!((v[0].start, v[0].end), (0, 5));
    }

    #[test]
    fn overlap_truncates_left_biased() {
        let mut v = vec![
            StyledSpan::new(0, 5, SpanAttr::bold()),
            StyledSpan::new(3, 8, SpanAttr::link("x")),
        ];
        normalize_spans(&mut v);
        assert_eq!(v.len(), 2);
        assert_eq!((v[0].start, v[0].end), (0, 5));
        assert_eq!((v[1].start, v[1].end), (5, 8));
    }

    #[test]
    fn empties_dropped_and_sorted() {
        let mut v = vec![
            StyledSpan::new(8, 9, SpanAttr::bold()),
            StyledSpan::new(4, 4, SpanAttr::bold()),
            StyledSpan::new(0, 2, SpanAttr::bold()),
        ];
        normalize_spans(&mut v);
        assert_eq!(v.len(), 2);
        assert_eq!((v[0].start, v[0].end), (0, 2));
        assert_eq!((v[1].start, v[1].end), (8, 9));
    }

    #[test]
    fn line_hash_contract() {
        let a = vec![StyledSpan::new(0, 2, SpanAttr::bold())];
        let h1 = line_hash("abcdef", &a, 100.0);

        assert_eq!(h1, line_hash("abcdef", &a, 100.0));

        assert_ne!(h1, line_hash("abcdeg", &a, 100.0));
        assert_ne!(
            h1,
            line_hash(
                "abcdef",
                &[StyledSpan::new(0, 2, SpanAttr::link("x"))],
                100.0
            )
        );
        assert_ne!(
            h1,
            line_hash("abcdef", &[StyledSpan::new(1, 3, SpanAttr::bold())], 100.0)
        );
        assert_ne!(h1, line_hash("abcdef", &[], 100.0));
        assert_ne!(h1, line_hash("abcdef", &a, 200.0));

        let abs_a = vec![StyledSpan::new(100, 102, SpanAttr::bold())];
        let abs_b = vec![StyledSpan::new(200, 202, SpanAttr::bold())];
        assert_eq!(
            line_hash("abcdef", &rebase_spans(&abs_a, 100), 0.0),
            line_hash("abcdef", &rebase_spans(&abs_b, 200), 0.0)
        );
    }
}
