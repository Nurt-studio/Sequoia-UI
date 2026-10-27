use std::ops::Range;

use cosmic_text::CacheKey;
use sequoia_buffer::{SpanAttr, StyledSpan};
use sequoia_core::{Color, Rect, Size};

use crate::{FontStyle, FontWeight, LineHeight};

pub(crate) const RICH_CACHE_MAX: usize = 128;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct SpanKey {
    text: String,
    color: Option<[u32; 4]>,
    background: Option<[u32; 4]>,
    weight: u16,
    style: u8,
    family: String,
    decorations: u8,
    size_delta_q: i32,
    link: Option<String>,
    code: bool,
    mention: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct RichKey {
    spans: Vec<SpanKey>,
    size: u32,
    width: u32,
    wrap: u8,
    spacing: i32,
    align: u8,
    line_height: u32,
}

impl RichKey {
    pub(crate) fn new(
        spans: &[TextSpan],
        size: f32,
        width: f32,
        wrap: u8,
        spacing: f32,
        align: RichAlign,
        line_height: LineHeight,
    ) -> Self {
        fn bits(color: &Color) -> [u32; 4] {
            [
                color.r.to_bits(),
                color.g.to_bits(),
                color.b.to_bits(),
                color.a.to_bits(),
            ]
        }
        Self {
            spans: spans
                .iter()
                .map(|s| SpanKey {
                    text: s.text.clone(),
                    color: s.color.as_ref().map(bits),
                    background: s.background.as_ref().map(bits),
                    weight: s.weight.unwrap_or_default().value(),
                    style: s.style.unwrap_or_default().key(),
                    family: s.family.clone().unwrap_or_default(),
                    decorations: (u8::from(s.underline))
                        | (u8::from(s.strikethrough) << 1)
                        | (u8::from(s.code) << 2),
                    size_delta_q: (s.size_delta * 4.0).round() as i32,
                    link: s.link.clone(),
                    code: s.code,
                    mention: s.mention,
                })
                .collect(),
            size: (size * 4.0).round() as u32,
            width: (width.max(0.0) * 4.0).round() as u32,
            wrap,
            spacing: crate::spacing_key(spacing),
            align: align.key(),
            line_height: line_height.key(),
        }
    }
}

/// Returns a prefix of `spans` containing at most `max_chars` characters.
pub(crate) fn truncate_spans(spans: &[TextSpan], max_chars: usize) -> Vec<TextSpan> {
    let mut out = Vec::new();
    let mut left = max_chars;
    for span in spans {
        if left == 0 {
            break;
        }
        let count = span.text.chars().count();
        if count <= left {
            out.push(span.clone());
            left -= count;
        } else {
            let mut truncated = span.clone();
            truncated.text = span.text.chars().take(left).collect();
            out.push(truncated);
            left = 0;
        }
    }
    out
}

/// Appends an ellipsis to the last span, preserving its style.
pub(crate) fn append_ellipsis(spans: &mut Vec<TextSpan>) {
    if let Some(last) = spans.last_mut() {
        last.text.push(crate::ELLIPSIS);
    } else {
        spans.push(TextSpan::new(crate::ELLIPSIS.to_string()));
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextSpan {
    pub text: String,
    pub color: Option<Color>,
    pub background: Option<Color>,
    pub weight: Option<FontWeight>,
    pub style: Option<FontStyle>,
    pub family: Option<String>,
    pub underline: bool,
    pub strikethrough: bool,
    pub size_delta: f32,
    pub link: Option<String>,
    pub code: bool,
    pub mention: Option<u64>,
}

impl TextSpan {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            ..Self::default()
        }
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    pub fn weight(mut self, weight: FontWeight) -> Self {
        self.weight = Some(weight);
        self
    }

    pub fn bold(mut self) -> Self {
        self.weight = Some(FontWeight::BOLD);
        self
    }

    pub fn semibold(mut self) -> Self {
        self.weight = Some(FontWeight::SEMIBOLD);
        self
    }

    pub fn medium(mut self) -> Self {
        self.weight = Some(FontWeight::MEDIUM);
        self
    }

    pub fn italic(mut self) -> Self {
        self.style = Some(FontStyle::Italic);
        self
    }

    pub fn oblique(mut self) -> Self {
        self.style = Some(FontStyle::Oblique);
        self
    }

    pub fn font_style(mut self, style: FontStyle) -> Self {
        self.style = Some(style);
        self
    }

    pub fn font(mut self, family: impl Into<String>) -> Self {
        let family = family.into();
        self.family = if family.is_empty() {
            None
        } else {
            Some(family)
        };
        self
    }

    pub fn family(self, family: impl Into<String>) -> Self {
        self.font(family)
    }

    pub fn underline(mut self, on: bool) -> Self {
        self.underline = on;
        self
    }

    pub fn strikethrough(mut self, on: bool) -> Self {
        self.strikethrough = on;
        self
    }

    pub fn size_delta(mut self, delta: f32) -> Self {
        self.size_delta = if delta.is_finite() { delta } else { 0.0 };
        self
    }

    pub fn link(mut self, url: impl Into<String>) -> Self {
        let url = url.into();
        self.link = if url.is_empty() { None } else { Some(url) };
        self
    }

    pub fn mention(mut self, id: u64) -> Self {
        self.mention = Some(id);
        self
    }

    pub fn code(mut self, on: bool) -> Self {
        self.code = on;
        self
    }

    pub(crate) fn from_span_attr(text: String, attr: &SpanAttr) -> Self {
        Self {
            text,
            color: attr.color,
            background: attr.bg,
            weight: if attr.bold {
                Some(FontWeight::BOLD)
            } else {
                None
            },
            style: if attr.italic {
                Some(FontStyle::Italic)
            } else {
                None
            },
            family: None,
            underline: attr.underline,
            strikethrough: attr.strike,
            size_delta: if attr.size_delta.is_finite() {
                attr.size_delta
            } else {
                0.0
            },
            link: attr.link.clone(),
            code: attr.code,
            mention: attr.mention,
        }
    }
}

fn floor_char_boundary(s: &str, mut idx: usize) -> usize {
    idx = idx.min(s.len());
    while idx > 0 && !s.is_char_boundary(idx) {
        idx -= 1;
    }
    idx
}

pub fn styled_spans_to_text_spans(
    line_text: &str,
    line_byte_start: usize,
    styled: &[StyledSpan],
) -> Vec<TextSpan> {
    if line_text.is_empty() {
        return vec![TextSpan::new("")];
    }
    let line_len = line_text.len();
    let mut sorted: Vec<&StyledSpan> = styled.iter().filter(|s| !s.is_empty()).collect();
    sorted.sort_by_key(|s| (s.start, s.end));
    let mut out: Vec<TextSpan> = Vec::new();
    let mut cursor_rel = 0usize;
    for s in sorted {
        if s.end <= line_byte_start {
            continue;
        }
        let mut rs = s.start.saturating_sub(line_byte_start).min(line_len);
        let mut re = s.end.saturating_sub(line_byte_start).min(line_len);
        rs = floor_char_boundary(line_text, rs);
        re = floor_char_boundary(line_text, re).max(rs);
        if re <= rs {
            continue;
        }

        if rs < cursor_rel {
            rs = cursor_rel;
            if re <= rs {
                continue;
            }
        }
        if rs > cursor_rel {
            let gap = floor_char_boundary(line_text, rs).max(cursor_rel);
            if gap > cursor_rel
                && let Some(g) = line_text.get(cursor_rel..gap)
            {
                out.push(TextSpan::new(g));
            }
            cursor_rel = gap;
            if rs > cursor_rel {
                rs = cursor_rel;
            }
        }
        if rs < cursor_rel {
            continue;
        }
        match line_text.get(rs..re) {
            Some(slice) => {
                out.push(TextSpan::from_span_attr(slice.to_owned(), &s.attr));
                cursor_rel = re;
            }
            None => {
                let mut end = re;
                while end > rs && line_text.get(rs..end).is_none() {
                    end -= 1;
                }
                let end = floor_char_boundary(line_text, end).max(rs);
                if end > rs
                    && let Some(slice) = line_text.get(rs..end)
                {
                    out.push(TextSpan::from_span_attr(slice.to_owned(), &s.attr));
                    cursor_rel = end;
                }
            }
        }
    }
    if cursor_rel < line_len
        && let Some(g) = line_text.get(cursor_rel..)
    {
        out.push(TextSpan::new(g));
    }
    if out.is_empty() {
        out.push(TextSpan::new(line_text));
    }
    out
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RichAlign {
    #[default]
    Left,
    Center,
    Right,
}

impl RichAlign {
    pub(crate) fn key(self) -> u8 {
        match self {
            Self::Left => 0,
            Self::Center => 1,
            Self::Right => 2,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RichLine {
    pub index: usize,

    pub top: f32,

    pub height: f32,

    pub baseline: f32,

    pub width: f32,

    pub chars: Range<usize>,
}

impl RichLine {
    pub fn bottom(&self) -> f32 {
        self.top + self.height
    }

    pub fn contains_y(&self, y: f32) -> bool {
        y >= self.top && y < self.bottom()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RichHit {
    pub char_index: usize,

    pub span: Option<usize>,

    pub line: usize,
}

impl RichHit {
    pub fn as_tuple(&self) -> (usize, Option<usize>) {
        (self.char_index, self.span)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RichCaret {
    pub x: f32,

    pub line: usize,

    pub top: f32,

    pub height: f32,
}

#[derive(Clone, Debug)]
pub(crate) struct RichGlyph {
    pub x: f32,
    pub y: f32,
    pub key: CacheKey,
    pub span: usize,
    pub color: Option<Color>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RichRect {
    pub rect: Rect,
    pub color: Color,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Squiggle {
    pub range: Range<usize>,
    pub color: Color,
}

#[derive(Clone, Debug)]
pub struct RichParagraph {
    pub(crate) glyphs: Vec<RichGlyph>,
    pub(crate) backgrounds: Vec<RichRect>,
    pub(crate) decorations: Vec<RichRect>,
    pub(crate) lines: Vec<RichLine>,

    pub(crate) carets: Vec<Vec<f32>>,
    pub(crate) span_ranges: Vec<Range<usize>>,
    pub(crate) span_links: Vec<Option<String>>,
    pub(crate) span_mentions: Vec<Option<u64>>,
    pub(crate) span_codes: Vec<bool>,

    pub(crate) squiggles: Vec<Squiggle>,
    pub(crate) squiggle_rects: Vec<RichRect>,
    pub(crate) width: f32,
    pub(crate) height: f32,
    pub(crate) line_height: f32,
    pub(crate) font_size: f32,
    pub(crate) len_chars: usize,
}

impl RichParagraph {
    pub fn is_empty(&self) -> bool {
        self.len_chars == 0
    }

    pub fn len_chars(&self) -> usize {
        self.len_chars
    }

    pub fn size(&self) -> Size {
        Size::new(self.width, self.height)
    }

    pub fn width(&self) -> f32 {
        self.width
    }

    pub fn height(&self) -> f32 {
        self.height
    }

    pub fn line_height(&self) -> f32 {
        self.line_height
    }

    pub fn font_size(&self) -> f32 {
        self.font_size
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    pub fn line(&self, index: usize) -> Option<&RichLine> {
        self.lines.get(index)
    }

    pub fn lines(&self) -> &[RichLine] {
        &self.lines
    }

    pub fn span_count(&self) -> usize {
        self.span_ranges.len()
    }

    pub fn span_range(&self, span: usize) -> Option<Range<usize>> {
        self.span_ranges.get(span).cloned()
    }

    pub fn span_at(&self, char_index: usize) -> Option<usize> {
        for (i, range) in self.span_ranges.iter().enumerate() {
            if range.is_empty() {
                continue;
            }
            if range.start <= char_index && char_index <= range.end {
                return Some(i);
            }
        }
        None
    }

    pub fn line_at(&self, char_index: usize) -> usize {
        for (i, line) in self.lines.iter().enumerate() {
            if char_index <= line.chars.end {
                return i;
            }
        }
        self.lines.len().saturating_sub(1)
    }

    pub fn caret_pos(&self, char_index: usize) -> RichCaret {
        let index = char_index.min(self.len_chars);
        if self.lines.is_empty() {
            return RichCaret {
                x: 0.0,
                line: 0,
                top: 0.0,
                height: self.line_height,
            };
        }
        let li = self.line_at(index);
        let line = &self.lines[li];
        let k = index.saturating_sub(line.chars.start);
        let x = self
            .carets
            .get(li)
            .and_then(|c| c.get(k.min(c.len().saturating_sub(1))))
            .copied()
            .unwrap_or(0.0);
        RichCaret {
            x,
            line: li,
            top: line.top,
            height: line.height,
        }
    }

    pub fn hit_point(&self, x: f32, y: f32) -> RichHit {
        if self.lines.is_empty() {
            return RichHit {
                char_index: 0,
                span: None,
                line: 0,
            };
        }
        let mut li = 0usize;
        for (i, line) in self.lines.iter().enumerate() {
            if y >= line.top {
                li = i;
            } else {
                break;
            }
        }
        let line = &self.lines[li];
        let carets = &self.carets[li];
        let mut best = 0usize;
        let mut best_d = f32::MAX;
        for (k, &cx) in carets.iter().enumerate() {
            let d = (cx - x).abs();
            if d < best_d {
                best_d = d;
                best = k;
            }
        }
        let char_index = (line.chars.start + best).min(self.len_chars);
        RichHit {
            char_index,
            span: self.span_at(char_index),
            line: li,
        }
    }

    pub fn backgrounds(&self) -> &[RichRect] {
        &self.backgrounds
    }

    pub fn decorations(&self) -> &[RichRect] {
        &self.decorations
    }

    pub fn squiggles(&self) -> &[Squiggle] {
        &self.squiggles
    }

    pub fn squiggle_rects(&self) -> &[RichRect] {
        &self.squiggle_rects
    }

    pub fn span_link(&self, span: usize) -> Option<&str> {
        self.span_links.get(span).and_then(|o| o.as_deref())
    }

    pub fn span_mention(&self, span: usize) -> Option<u64> {
        self.span_mentions.get(span).copied().flatten()
    }

    pub fn span_is_code(&self, span: usize) -> bool {
        self.span_codes.get(span).copied().unwrap_or(false)
    }

    pub fn add_squiggle(&mut self, range: Range<usize>, color: Color) {
        let start = range.start.min(self.len_chars);
        let end = range.end.min(self.len_chars);
        if end <= start {
            return;
        }
        self.squiggles.push(Squiggle {
            range: start..end,
            color,
        });

        for (li, line) in self.lines.iter().enumerate() {
            let s0 = start.max(line.chars.start);
            let s1 = end.min(line.chars.end);
            if s1 <= s0 {
                continue;
            }
            let carets = match self.carets.get(li) {
                Some(c) => c,
                None => continue,
            };
            let x_of = |c: usize| -> f32 {
                let k = c.saturating_sub(line.chars.start);
                carets
                    .get(k.min(carets.len().saturating_sub(1)))
                    .copied()
                    .unwrap_or(0.0)
            };
            let x0 = x_of(s0);
            let x1 = x_of(s1).max(x0);
            if x1 <= x0 {
                continue;
            }
            let y_base = (line.baseline + self.font_size * 0.12).round();
            let thick = 1.0f32.max((self.font_size * 0.06).round());

            let mut x = x0;
            let mut up = false;
            while x < x1 {
                let w = (2.0f32).min(x1 - x);
                let dy = if up { -1.0 } else { 1.0 };
                self.squiggle_rects.push(RichRect {
                    rect: Rect::from_xywh(x, y_base + dy, w, thick),
                    color,
                });
                x += w;
                up = !up;
            }
        }
    }

    pub fn glyph_count(&self) -> usize {
        self.glyphs.len()
    }

    pub fn glyph_span(&self, glyph: usize) -> Option<usize> {
        self.glyphs.get(glyph).map(|g| g.span)
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;
    use crate::{TextSystem, WrapMode};

    fn has_fonts() -> bool {
        TextSystem::new().shape("0", 14.0).glyph_count() > 0
    }

    fn spans2() -> Vec<TextSpan> {
        vec![
            TextSpan::new("ab").color(Color::RED).bold(),
            TextSpan::new("cd").color(Color::BLUE),
        ]
    }

    #[test]
    fn glyphs_carry_span_indices() {
        if !has_fonts() {
            return;
        }
        let mut text = TextSystem::new();
        let rich = text.shape_rich(&spans2(), 14.0);
        assert!(rich.glyph_count() > 0);
        assert!(rich.span_count() == 2);
        let mut seen = [false, false];
        for i in 0..rich.glyph_count() {
            let s = rich.glyph_span(i).expect("span");
            assert!(s < 2);
            seen[s] = true;
        }
        assert_eq!(seen, [true, true]);
        assert_eq!(rich.span_range(0), Some(0..2));
        assert_eq!(rich.span_range(1), Some(2..4));
    }

    #[test]
    fn boundaries_belong_to_the_left_span() {
        if !has_fonts() {
            return;
        }
        let mut text = TextSystem::new();
        let rich = text.shape_rich(&spans2(), 14.0);
        assert_eq!(rich.span_at(0), Some(0));
        assert_eq!(rich.span_at(1), Some(0));
        assert_eq!(rich.span_at(2), Some(0));
        assert_eq!(rich.span_at(3), Some(1));
        assert_eq!(rich.span_at(4), Some(1));
    }

    #[test]
    fn multiline_split_and_caret_lines() {
        if !has_fonts() {
            return;
        }
        let mut text = TextSystem::new();
        let rich = text.shape_rich(&[TextSpan::new("a\nb\nc")], 14.0);
        assert_eq!(rich.line_count(), 3);
        assert_eq!(rich.line(0).unwrap().chars, 0..1);
        assert_eq!(rich.line(1).unwrap().chars, 2..3);
        assert_eq!(rich.line(2).unwrap().chars, 4..5);
        assert!((rich.height() - 3.0 * rich.line_height()).abs() < 0.01);
        assert_eq!(rich.caret_pos(0).line, 0);
        assert_eq!(rich.caret_pos(2).line, 1);
        assert_eq!(rich.caret_pos(4).line, 2);

        assert_eq!(rich.caret_pos(1).line, 0);
        assert!(rich.caret_pos(1).x > rich.caret_pos(0).x);
    }

    #[test]
    fn first_line_baseline_preserves_ascent() {
        if !has_fonts() {
            return;
        }
        let mut text = TextSystem::new();
        let rich = text.shape_rich(&[TextSpan::new("a\nb\nc")], 14.0);
        for i in 0..rich.line_count() {
            let line = rich.line(i).unwrap();
            assert!(
                line.baseline > line.top,
                "line {i}: baseline must sit below top (ascent)"
            );
            assert!(
                line.baseline < line.top + line.height,
                "line {i}: baseline inside the box"
            );
        }
        let l0 = rich.line(0).unwrap();
        for g in rich.glyphs.iter() {
            assert!(g.y >= l0.top - 0.5, "glyph above paragraph box: {}", g.y);
        }
        let a0 = l0.baseline - l0.top;
        for i in 1..rich.line_count() {
            let line = rich.line(i).unwrap();
            let a = line.baseline - line.top;
            assert!((a - a0).abs() < 0.5, "line {i}: ascent drift {a} vs {a0}");
        }
    }

    #[test]
    fn trailing_newline_adds_an_empty_line() {
        if !has_fonts() {
            return;
        }
        let mut text = TextSystem::new();
        let rich = text.shape_rich(&[TextSpan::new("a\n")], 14.0);
        assert_eq!(rich.line_count(), 2);
        assert_eq!(rich.line(1).unwrap().chars, 2..2);
        assert_eq!(rich.caret_pos(2).line, 1);
    }

    #[test]
    fn hit_point_roundtrip_near_left_edges() {
        if !has_fonts() {
            return;
        }
        let mut text = TextSystem::new();
        let rich = text.shape_rich(&[TextSpan::new("abcdef")], 16.0);
        let mid_y = rich.line(0).unwrap().top + rich.line_height() * 0.5;
        for c in 0..6 {
            let x0 = rich.caret_pos(c).x;
            let x1 = rich.caret_pos(c + 1).x;
            assert!(x1 > x0, "glyph {c} has no width");
            let hit = rich.hit_point(x0 + (x1 - x0) * 0.25, mid_y);
            assert_eq!(hit.char_index, c);
            assert_eq!(hit.line, 0);
            assert_eq!(hit.span, Some(0));
        }

        assert_eq!(rich.hit_point(1.0, -100.0).line, 0);
        assert_eq!(rich.hit_point(1.0, 100_000.0).line, 0);
    }

    #[test]
    fn background_rect_matches_span_extent() {
        if !has_fonts() {
            return;
        }
        let mut text = TextSystem::new();
        let spans = vec![
            TextSpan::new("ab"),
            TextSpan::new("cd").background(Color::GREEN),
        ];
        let rich = text.shape_rich(&spans, 14.0);
        assert_eq!(rich.backgrounds().len(), 1);
        let bg = rich.backgrounds()[0];
        assert_eq!(bg.color, Color::GREEN);
        assert!((bg.rect.left() - rich.caret_pos(2).x).abs() < 0.01);
        assert!(
            (bg.rect.right() - rich.caret_pos(4).x).abs() < 0.5,
            "bg right {:?} vs caret {:?}",
            bg.rect.right(),
            rich.caret_pos(4).x
        );
        assert!((bg.rect.size.height - rich.line_height()).abs() < 0.01);
    }

    #[test]
    fn wrap_creates_visual_lines_covering_all_chars() {
        if !has_fonts() {
            return;
        }
        let mut text = TextSystem::new();
        let content: String = "abcdefghij".chars().collect();
        let len = content.chars().count();
        let rich =
            text.shape_rich_wrapped(&[TextSpan::new(content)], 16.0, 40.0, WrapMode::Character);
        assert!(rich.line_count() > 1);
        let mut covered = vec![false; len];
        for line in rich.lines() {
            assert!(line.width <= 40.0 + 0.5, "line too wide: {}", line.width);
            for c in line.chars.clone() {
                covered[c] = true;
            }
        }
        assert!(covered.iter().all(|&b| b));

        let second = &rich.lines()[1];
        let hit = rich.hit_point(1.0, second.top + 1.0);
        assert_eq!(hit.line, 1);
        assert!(second.chars.contains(&hit.char_index));
    }

    #[test]
    fn center_align_shifts_short_lines_right() {
        if !has_fonts() {
            return;
        }
        let mut text = TextSystem::new();
        let spans = vec![TextSpan::new("ab")];
        let left = text.shape_rich_aligned(&spans, 16.0, 200.0, WrapMode::Word, RichAlign::Left);
        let center =
            text.shape_rich_aligned(&spans, 16.0, 200.0, WrapMode::Word, RichAlign::Center);
        assert!((left.caret_pos(0).x - 0.0).abs() < 0.01);
        assert!(center.caret_pos(0).x > 10.0);

        let dx = center.caret_pos(0).x - left.caret_pos(0).x;
        assert!((center.caret_pos(2).x - left.caret_pos(2).x - dx).abs() < 1.0);
    }

    #[test]
    fn empty_spans_keep_stable_indices() {
        if !has_fonts() {
            return;
        }
        let mut text = TextSystem::new();
        let rich = text.shape_rich(
            &[TextSpan::new(""), TextSpan::new("x"), TextSpan::new("")],
            14.0,
        );
        assert_eq!(rich.span_count(), 3);
        assert_eq!(rich.span_range(0), Some(0..0));
        assert_eq!(rich.span_range(1), Some(0..1));
        assert_eq!(rich.span_at(0), Some(1));
        for i in 0..rich.glyph_count() {
            assert_eq!(rich.glyph_span(i), Some(1));
        }
    }

    #[test]
    fn cache_returns_shared_paragraphs() {
        if !has_fonts() {
            return;
        }
        let mut text = TextSystem::new();
        let a = text.shape_rich(&spans2(), 14.0);
        let b = text.shape_rich(&spans2(), 14.0);
        assert!(Rc::ptr_eq(&a, &b));
    }

    #[test]
    fn draw_splits_text_by_color_with_background_first() {
        use sequoia_core::{RenderItem, RenderList};

        if !has_fonts() {
            return;
        }
        let mut text = TextSystem::new();
        let spans = vec![
            TextSpan::new("ab").color(Color::RED),
            TextSpan::new("cd")
                .color(Color::BLUE)
                .background(Color::GREEN),
        ];
        let rich = text.shape_rich(&spans, 16.0);
        let mut list = RenderList::new();
        text.draw_rich(&rich, (10.0, 20.0), Color::WHITE, &mut list);
        let mut colors: Vec<Color> = Vec::new();
        let mut quads = 0usize;
        for item in list.items() {
            match item {
                RenderItem::Text(t) => colors.push(t.color),
                RenderItem::Quad(_) => quads += 1,
                _ => {}
            }
        }
        assert_eq!(colors, vec![Color::RED, Color::BLUE]);
        assert_eq!(quads, 1, "one background quad");

        let first_text = list
            .items()
            .iter()
            .position(|i| matches!(i, RenderItem::Text(_)))
            .unwrap();
        let first_quad = list
            .items()
            .iter()
            .position(|i| matches!(i, RenderItem::Quad(_)))
            .unwrap();
        assert!(first_quad < first_text);
    }

    #[test]
    fn decorations_follow_underlined_spans() {
        if !has_fonts() {
            return;
        }
        let mut text = TextSystem::new();
        let rich = text.shape_rich(&[TextSpan::new("ab").underline(true)], 16.0);
        assert_eq!(rich.decorations().len(), 1);
        let d = rich.decorations()[0];
        let line = rich.line(0).unwrap();
        assert!(d.rect.top() > line.baseline);
        assert!((d.rect.size.width - line.width).abs() < 1.0);
    }

    use sequoia_buffer::{SpanAttr, StyledBuffer, StyledSpan};

    #[test]
    fn per_span_size_delta_changes_height_not_just_width() {
        if !has_fonts() {
            return;
        }
        let mut text = TextSystem::new();
        let base = text.shape_rich(&[TextSpan::new("Hello world")], 14.0);
        let big = text.shape_rich(&[TextSpan::new("Hello world").size_delta(10.0)], 14.0);
        assert!(
            big.height() > base.height() + 2.0,
            "larger span = higher line: {} vs {}",
            big.height(),
            base.height()
        );
        assert!(big.width() > base.width());

        let mixed = text.shape_rich(
            &[TextSpan::new("ab"), TextSpan::new("CD").size_delta(12.0)],
            14.0,
        );
        assert!(mixed.height() > base.height());

        let again = text.shape_rich(&[TextSpan::new("Hello world").size_delta(10.0)], 14.0);
        assert!(Rc::ptr_eq(&big, &again));
        assert!(!Rc::ptr_eq(&base, &big));
    }

    #[test]
    fn hit_test_link_and_mention_returns_span_idx() {
        if !has_fonts() {
            return;
        }
        let mut text = TextSystem::new();

        let full = "hello @user world https://x.y";
        let mut buf = StyledBuffer::from(full);
        buf.set_attr(
            6..11,
            SpanAttr {
                bold: true,
                underline: true,
                color: Some(Color::BLUE),
                mention: Some(42),
                ..SpanAttr::plain()
            },
        );
        buf.set_attr(
            11..full.len(),
            SpanAttr {
                underline: true,
                color: Some(Color::BLUE),
                link: Some("https://x.y".to_owned()),
                code: true,
                ..SpanAttr::plain()
            },
        );
        let styled: Vec<StyledSpan> = buf.spans_for_line(0..full.len());
        assert_eq!(styled.len(), 2, "tylko nie-czyste spany z modelu");
        let spans = super::styled_spans_to_text_spans(full, 0, &styled);
        assert_eq!(spans.len(), 3, "luka plain + mention + link");
        let rich = text.shape_rich(&spans, 16.0);
        assert_eq!(rich.span_count(), 3);
        assert_eq!(rich.span_mention(1), Some(42));
        assert_eq!(rich.span_link(2), Some("https://x.y"));
        assert!(rich.span_is_code(2));
        assert_eq!(rich.span_link(0), None);

        let mid_y = rich.line(0).unwrap().top + rich.line_height() * 0.5;
        let x_mid_mention = (rich.caret_pos(7).x + rich.caret_pos(10).x) * 0.5;
        let hit = rich.hit_point(x_mid_mention, mid_y);
        assert_eq!(hit.as_tuple(), (hit.char_index, hit.span));
        assert_eq!(hit.span, Some(1), "mention span_idx");
        assert!((6..=11).contains(&hit.char_index));

        let x_mid_link = (rich.caret_pos(12).x + rich.caret_pos(20).x) * 0.5;
        let hit_link = rich.hit_point(x_mid_link, mid_y);
        assert_eq!(hit_link.span, Some(2), "link span_idx");
        assert_eq!(rich.span_link(hit_link.span.unwrap()), Some("https://x.y"));
    }

    #[test]
    fn squiggle_does_not_change_layout() {
        use sequoia_core::{RenderItem, RenderList};
        if !has_fonts() {
            return;
        }
        let mut text = TextSystem::new();
        let spans = vec![
            TextSpan::new("hello world, "),
            TextSpan::new("this is an error").underline(true),
        ];
        let rich = text.shape_rich(&spans, 16.0);
        let (w0, h0) = (rich.width(), rich.height());
        let lines0: Vec<_> = rich.lines().to_vec();
        let glyphs0 = rich.glyph_count();
        let bg0 = rich.backgrounds().len();
        let deco0 = rich.decorations().len();

        let mut owned = (*rich).clone();

        owned.add_squiggle(5..5, Color::RED);
        assert!(owned.squiggles().is_empty());
        owned.add_squiggle(13..18, Color::RED);
        assert_eq!(owned.squiggles().len(), 1);
        assert!(!owned.squiggle_rects().is_empty());

        assert!((owned.width() - w0).abs() < 1e-4);
        assert!((owned.height() - h0).abs() < 1e-4);
        assert_eq!(owned.lines(), lines0.as_slice());
        assert_eq!(owned.glyph_count(), glyphs0);
        assert_eq!(owned.backgrounds().len(), bg0);
        assert_eq!(
            owned.decorations().len(),
            deco0,
            "squiggle nie w decorations"
        );

        let mut before = RenderList::new();
        text.draw_rich(&rich, (0.0, 0.0), Color::WHITE, &mut before);
        let mut after = RenderList::new();
        text.draw_rich(&owned, (0.0, 0.0), Color::WHITE, &mut after);
        let quads = |l: &RenderList| {
            l.items()
                .iter()
                .filter(|i| matches!(i, RenderItem::Quad(_)))
                .count()
        };
        assert!(quads(&after) > quads(&before), "fala dorzuca quady");

        let texts = |l: &RenderList| {
            l.items()
                .iter()
                .filter(|i| matches!(i, RenderItem::Text(_)))
                .count()
        };
        assert_eq!(texts(&before), texts(&after));
    }

    #[test]
    fn wrap_width_comes_from_call_and_heights_vary() {
        if !has_fonts() {
            return;
        }
        let mut text = TextSystem::new();
        let spans = vec![
            TextSpan::new("ab cd ef gh ij kl mn "),
            TextSpan::new("BIG").size_delta(14.0),
        ];
        let narrow = text.shape_rich_wrapped(&spans, 14.0, 60.0, WrapMode::Word);
        let wide = text.shape_rich_wrapped(&spans, 14.0, 600.0, WrapMode::Word);
        assert!(narrow.line_count() > wide.line_count(), "call wrap");
        assert!(
            !Rc::ptr_eq(&narrow, &wide),
            "different widths = separate entries"
        );

        let plain =
            text.shape_rich_wrapped(&[TextSpan::new("ab cd ef")], 14.0, 60.0, WrapMode::Word);
        assert!(narrow.height() >= plain.height());
        let max_h = narrow
            .lines()
            .iter()
            .map(|l| l.height)
            .fold(0.0f32, f32::max);
        let min_h = narrow
            .lines()
            .iter()
            .map(|l| l.height)
            .fold(f32::MAX, f32::min);
        assert!(max_h >= min_h);
    }

    #[test]
    fn bidi_mixed_text_shapes_and_hits() {
        if !has_fonts() {
            return;
        }
        let mut text = TextSystem::new();
        let spans = vec![TextSpan::new("Hello שלום world مرحبا end")];
        let rich = text.shape_rich(&spans, 16.0);
        assert!(rich.glyph_count() > 0);
        assert!(rich.line_count() >= 1);
        assert!(rich.width() > 0.0);

        let mid_y = rich.line(0).unwrap().top + rich.line(0).unwrap().height * 0.5;
        let hit = rich.hit_point(rich.width() * 0.5, mid_y);
        assert_eq!(hit.span, Some(0));
        assert!(hit.char_index <= rich.len_chars());
    }

    #[test]
    fn styled_mapper_is_utf8_safe_not_ascii() {
        let line = "yellow the goose of the self";
        let oto = |s: &str| s.len();

        let w2_start = oto("turn yellow");
        let w2_end = w2_start + oto("gooseberry");
        let styled = vec![StyledSpan::new(
            w2_start,
            w2_end,
            SpanAttr {
                bold: true,
                mention: Some(99),
                ..SpanAttr::plain()
            },
        )];
        let spans = super::styled_spans_to_text_spans(line, 0, &styled);
        assert_eq!(spans.len(), 3, "przed + styl + po");
        let joined: String = spans.iter().map(|s| s.text.clone()).collect();
        assert_eq!(joined, line, "mapper covers the entire line without loss");
        assert_eq!(spans[1].mention, Some(99));
        assert_eq!(spans[1].weight, Some(crate::FontWeight::BOLD));

        let shifted: Vec<StyledSpan> = styled
            .iter()
            .map(|s| StyledSpan::new(s.start + 1000, s.end + 1000, s.attr.clone()))
            .collect();
        let spans2 = super::styled_spans_to_text_spans(line, 1000, &shifted);
        assert_eq!(spans2, spans, "relatywizacja do line_byte_start");

        let emoji_line = "a👨‍👩‍👧b";
        let b_len = "a".len();
        let e_len = "👨‍👩‍👧".len();
        let estyled = vec![StyledSpan::new(
            b_len,
            b_len + e_len,
            SpanAttr {
                link: Some("x".into()),
                ..SpanAttr::plain()
            },
        )];
        let espans = super::styled_spans_to_text_spans(emoji_line, 0, &estyled);
        let ejoined: String = espans.iter().map(|s| s.text.clone()).collect();
        assert_eq!(ejoined, emoji_line);
        assert_eq!(espans[1].link.as_deref(), Some("x"));
    }

    #[test]
    fn span_attr_hash_sees_every_field() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::Hasher;

        let h = |f: fn(SpanAttr) -> SpanAttr| {
            let mut s = DefaultHasher::new();
            sequoia_buffer::hash_span_attr(&f(SpanAttr::plain()), &mut s);
            s.finish()
        };
        let hashes = [
            h(|a| SpanAttr { bold: true, ..a }),
            h(|a| SpanAttr { italic: true, ..a }),
            h(|a| SpanAttr {
                underline: true,
                ..a
            }),
            h(|a| SpanAttr { strike: true, ..a }),
            h(|a| SpanAttr {
                color: Some(Color::RED),
                ..a
            }),
            h(|a| SpanAttr {
                color: Some(Color::BLUE),
                ..a
            }),
            h(|a| SpanAttr {
                bg: Some(Color::GREEN),
                ..a
            }),
            h(|a| SpanAttr {
                size_delta: 4.0,
                ..a
            }),
            h(|a| SpanAttr {
                link: Some("u".into()),
                ..a
            }),
            h(|a| SpanAttr {
                mention: Some(1),
                ..a
            }),
            h(|a| SpanAttr { code: true, ..a }),
        ];
        for i in 0..hashes.len() {
            for j in (i + 1)..hashes.len() {
                assert_ne!(hashes[i], hashes[j], "pole {i} vs {j}");
            }
        }
    }
}
