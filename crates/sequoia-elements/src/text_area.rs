use std::cell::RefCell;
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

use sequoia_buffer::{SpanAttr, StyledSpan};
use sequoia_core::{Color, Edges, Length, Rect, Size, Vec2};
use sequoia_input::{
    CursorIcon, Event, EventPhase, ImeEvent, Key, KeyEvent, Modifiers, PointerButton, PointerEvent,
};
use sequoia_layout::{LayoutCtx, LayoutStyle};
use sequoia_reactive::Signal;
use sequoia_style::{TextAreaStyle, Theme, VisualState};
use sequoia_view::{AnyView, EventCtx, FontWeight, PaintCtx, Unit, View};

use crate::buffer::{BufferEngine, StyledDoc};
use crate::clipboard::SharedClipboard;

#[derive(Clone)]
pub(crate) enum DocSource {
    Plain(Signal<String>),
    Styled(Signal<StyledDoc>),
}

impl DocSource {
    fn ensure(&self, engine: &mut BufferEngine) -> bool {
        match self {
            DocSource::Plain(s) => engine.ensure(s),
            DocSource::Styled(s) => engine.ensure_styled(s),
        }
    }

    fn commit(&self, engine: &mut BufferEngine) {
        match self {
            DocSource::Plain(s) => engine.commit(s),
            DocSource::Styled(s) => engine.commit_styled(s),
        }
    }

    fn get_text(&self) -> String {
        match self {
            DocSource::Plain(s) => s.get_untracked(),
            DocSource::Styled(s) => s.get_untracked().text,
        }
    }

    fn set_text(&self, text: String) {
        match self {
            DocSource::Plain(s) => s.set(text),

            DocSource::Styled(s) => s.set(StyledDoc {
                text,
                spans: Vec::new(),
            }),
        }
    }

    fn track(&self) {
        match self {
            DocSource::Plain(s) => {
                let _ = s.get();
            }
            DocSource::Styled(s) => {
                let _ = s.get();
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Caret {
    offset: usize,
    anchor: Option<usize>,
}

impl Caret {
    fn collapsed(pos: usize) -> Self {
        Self {
            offset: pos,
            anchor: None,
        }
    }

    fn ends(&self) -> (usize, usize) {
        match self.anchor {
            Some(a) => (a.min(self.offset), a.max(self.offset)),
            None => (self.offset, self.offset),
        }
    }

    fn selected(&self) -> Option<(usize, usize)> {
        let (a, b) = self.ends();
        if a < b { Some((a, b)) } else { None }
    }

    fn move_to(&mut self, pos: usize, extend: bool) {
        if extend {
            let anchor = self.anchor.unwrap_or(self.offset);
            self.anchor = Some(anchor);
            self.offset = pos;
            if self.anchor == Some(self.offset) {
                self.anchor = None;
            }
        } else {
            self.offset = pos;
            self.anchor = None;
        }
    }
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

pub struct TextArea {
    source: DocSource,
    placeholder: String,
    max_length: Option<usize>,
    max_lines: Option<usize>,
    min_lines: usize,
    font_size: Option<f32>,
    weight: Option<FontWeight>,
    text_color: Option<Color>,
    placeholder_color: Option<Color>,
    background: Option<Color>,
    border_color: Option<Color>,
    border_width: Option<f32>,
    radius: Option<f32>,
    line_height_factor: Option<f32>,
    current_line: Option<Color>,
    highlight_current_line: bool,
    pad_x: f32,
    pad_y: f32,
    indent: String,
    auto_indent: bool,
    editable: bool,
    accept_tab: bool,
    appearance: Option<TextAreaStyle>,
    style: LayoutStyle,
    margin: Edges,

    revision: Signal<u32>,

    state: Rc<RefCell<AreaState>>,

    on_edit_start: Option<Rc<dyn Fn()>>,
    on_edit_end: Option<Rc<dyn Fn(String)>>,
    on_scroll: Option<Rc<dyn Fn(Vec2)>>,
}

const DEFAULT_PAD_X: f32 = 6.0;
const DEFAULT_PAD_Y: f32 = 6.0;
const DEFAULT_LINE_HEIGHT: f32 = 1.35;
const DEFAULT_INDENT: &str = "    ";
const DEFAULT_MIN_LINES: usize = 3;
const CARET_W: f32 = 1.5;
const SCROLL_MARGIN: f32 = 2.0;

#[derive(Clone)]
struct AreaState {
    carets: Vec<Caret>,

    goal_x: Option<f32>,
    scroll: Vec2,
    viewport: Size,

    content: Size,
    preedit: String,
    was_focused: bool,
    session_text: Option<String>,

    escalate: u8,
    escalate_base: usize,

    last_click_ms: Option<u128>,
    last_click_pos: usize,
    click_count: u32,

    alt_drag: Option<usize>,

    engine: BufferEngine,
}

impl Default for AreaState {
    fn default() -> Self {
        Self {
            carets: vec![Caret::default()],
            goal_x: None,
            scroll: Vec2::default(),
            viewport: Size::default(),
            content: Size::default(),
            preedit: String::new(),
            was_focused: false,
            session_text: None,
            escalate: 0,
            escalate_base: 0,
            last_click_ms: None,
            last_click_pos: 0,
            click_count: 0,
            alt_drag: None,
            engine: BufferEngine::new(),
        }
    }
}

fn merge_byte_ranges(ranges: &[(usize, usize)]) -> (Vec<(usize, usize)>, Vec<usize>) {
    if ranges.is_empty() {
        return (Vec::new(), Vec::new());
    }
    let mut order: Vec<usize> = (0..ranges.len()).collect();
    order.sort_by_key(|&i| (ranges[i].0, ranges[i].1));
    let mut merged: Vec<(usize, usize)> = Vec::new();
    let mut owner = vec![0usize; ranges.len()];
    for i in order {
        let (s, e) = ranges[i];
        if let Some(last) = merged.last_mut()
            && s <= last.1
        {
            last.1 = last.1.max(e);
            owner[i] = merged.len() - 1;
            continue;
        }
        owner[i] = merged.len();
        merged.push((s, e));
    }
    (merged, owner)
}

fn map_through_deletes(unions: &[(usize, usize)], x: usize) -> usize {
    let mut sub = 0usize;
    for (s, e) in unions {
        if *e <= x {
            sub += e - s;
        } else {
            break;
        }
    }
    x.saturating_sub(sub)
}

fn dedupe_carets(carets: &mut Vec<Caret>) {
    let mut seen = std::collections::HashSet::new();
    carets.retain(|c| seen.insert(c.offset));
    if carets.is_empty() {
        carets.push(Caret::default());
    }
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub(crate) enum StyleFlag {
    Bold,
    Italic,
    Underline,
    Strike,
}

impl StyleFlag {
    fn get(self, a: &SpanAttr) -> bool {
        match self {
            StyleFlag::Bold => a.bold,
            StyleFlag::Italic => a.italic,
            StyleFlag::Underline => a.underline,
            StyleFlag::Strike => a.strike,
        }
    }

    fn set(self, a: &mut SpanAttr, v: bool) {
        match self {
            StyleFlag::Bold => a.bold = v,
            StyleFlag::Italic => a.italic = v,
            StyleFlag::Underline => a.underline = v,
            StyleFlag::Strike => a.strike = v,
        }
    }
}

fn line_starts(chars: &[char]) -> Vec<usize> {
    let mut starts = vec![0usize];
    for (i, &c) in chars.iter().enumerate() {
        if c == '\n' {
            starts.push(i + 1);
        }
    }
    starts
}

fn line_len(chars: &[char], starts: &[usize], line: usize) -> usize {
    let start = starts.get(line).copied().unwrap_or(chars.len());
    match starts.get(line + 1) {
        Some(&next) => next.saturating_sub(start + 1),
        None => chars.len().saturating_sub(start),
    }
}

fn line_col_of(chars: &[char], starts: &[usize], caret: usize) -> (usize, usize) {
    let caret = caret.min(chars.len());
    let line = starts.partition_point(|&s| s <= caret).saturating_sub(1);
    let line = line.min(starts.len().saturating_sub(1));
    (line, caret - starts[line])
}

fn caret_of_line_col(chars: &[char], starts: &[usize], line: usize, col: usize) -> usize {
    let line = line.min(starts.len().saturating_sub(1));
    starts[line] + col.min(line_len(chars, starts, line))
}

fn split_lines(text: &str) -> Vec<&str> {
    text.split('\n').collect()
}

fn leading_indent_len(line: &str) -> usize {
    line.chars().take_while(|&c| c == ' ' || c == '\t').count()
}

fn sanitize_multiline(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\r' => {
                out.push('\n');
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
            }
            '\n' | '\t' => out.push(c),
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    out
}

impl TextArea {
    pub fn new(value: Signal<String>) -> Self {
        Self::with_source(DocSource::Plain(value))
    }

    pub fn new_styled(value: Signal<StyledDoc>) -> Self {
        Self::with_source(DocSource::Styled(value))
    }

    fn with_source(source: DocSource) -> Self {
        Self {
            source,
            placeholder: String::new(),
            max_length: None,
            max_lines: None,
            min_lines: DEFAULT_MIN_LINES,
            font_size: None,
            weight: None,
            text_color: None,
            placeholder_color: None,
            background: None,
            border_color: None,
            border_width: None,
            radius: None,
            line_height_factor: None,
            current_line: None,
            highlight_current_line: false,
            pad_x: DEFAULT_PAD_X,
            pad_y: DEFAULT_PAD_Y,
            indent: DEFAULT_INDENT.to_owned(),
            auto_indent: true,
            editable: true,
            accept_tab: true,
            appearance: None,
            style: LayoutStyle {
                width: Length::Fill(1.0),
                height: Length::Fill(1.0),
                ..LayoutStyle::default()
            },
            margin: Edges::ZERO,
            revision: Signal::new(0),
            state: Rc::new(RefCell::new(AreaState::default())),
            on_edit_start: None,
            on_edit_end: None,
            on_scroll: None,
        }
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn max_length(mut self, max: usize) -> Self {
        self.max_length = Some(max);
        self
    }

    pub fn max_lines(mut self, max: usize) -> Self {
        self.max_lines = Some(max.max(1));
        self
    }

    pub fn min_lines(mut self, min: usize) -> Self {
        self.min_lines = min.max(1);
        self
    }

    pub fn on_edit_start(mut self, f: impl Fn() + 'static) -> Self {
        self.on_edit_start = Some(Rc::new(f));
        self
    }

    pub fn on_edit_end(mut self, f: impl Fn(String) + 'static) -> Self {
        self.on_edit_end = Some(Rc::new(f));
        self
    }

    pub fn on_scroll(mut self, f: impl Fn(Vec2) + 'static) -> Self {
        self.on_scroll = Some(Rc::new(f));
        self
    }

    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = Some(size);
        self
    }

    pub fn weight(mut self, weight: FontWeight) -> Self {
        self.weight = Some(weight);
        self
    }

    pub fn bold(mut self, bold: bool) -> Self {
        self.weight = Some(if bold {
            FontWeight::BOLD
        } else {
            FontWeight::NORMAL
        });
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.text_color = Some(color);
        self
    }

    pub fn placeholder_color(mut self, color: Color) -> Self {
        self.placeholder_color = Some(color);
        self
    }

    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    pub fn border_color(mut self, color: Color) -> Self {
        self.border_color = Some(color);
        self
    }

    pub fn border_width(mut self, width: f32) -> Self {
        self.border_width = Some(width.max(0.0));
        self
    }

    pub fn border(mut self, width: f32, color: Color) -> Self {
        self.border_width = Some(width.max(0.0));
        self.border_color = Some(color);
        self
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = Some(radius.max(0.0));
        self
    }

    pub fn line_height_factor(mut self, factor: f32) -> Self {
        self.line_height_factor = Some(factor);
        self
    }

    pub fn current_line(mut self, color: Color) -> Self {
        self.current_line = Some(color);
        self.highlight_current_line = true;
        self
    }

    pub fn highlight_current_line(mut self, on: bool) -> Self {
        self.highlight_current_line = on;
        self
    }

    pub fn padding(mut self, padding: impl Into<Edges>) -> Self {
        let p = padding.into();
        self.pad_x = (p.left + p.right) * 0.5;
        self.pad_y = (p.top + p.bottom) * 0.5;
        self
    }

    pub fn padding_xy(mut self, x: f32, y: f32) -> Self {
        self.pad_x = x.max(0.0);
        self.pad_y = y.max(0.0);
        self
    }

    pub fn indent(mut self, indent: impl Into<String>) -> Self {
        self.indent = indent.into();
        self
    }

    pub fn auto_indent(mut self, on: bool) -> Self {
        self.auto_indent = on;
        self
    }

    pub fn editable(mut self, editable: bool) -> Self {
        self.editable = editable;
        self
    }

    pub fn accept_tab(mut self, on: bool) -> Self {
        self.accept_tab = on;
        self
    }

    pub fn appearance(mut self, style: TextAreaStyle) -> Self {
        self.appearance = Some(style);
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.style = self.style.width(width.into());
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.style = self.style.height(height.into());
        self
    }

    pub fn min_width(mut self, v: f32) -> Self {
        self.style = self.style.min_width(v);
        self
    }

    pub fn max_width(mut self, v: f32) -> Self {
        self.style = self.style.max_width(v);
        self
    }

    pub fn min_height(mut self, v: f32) -> Self {
        self.style = self.style.min_height(v);
        self
    }

    pub fn max_height(mut self, v: f32) -> Self {
        self.style = self.style.max_height(v);
        self
    }

    pub fn flex(mut self, weight: f32) -> Self {
        self.style = self.style.flex(weight);
        self
    }

    pub fn expand(self) -> Self {
        self.flex(1.0)
    }

    pub fn margin(mut self, margin: impl Into<Edges>) -> Self {
        let m = margin.into();
        self.margin = Edges::only(
            sanitize_margin(m.left),
            sanitize_margin(m.top),
            sanitize_margin(m.right),
            sanitize_margin(m.bottom),
        );
        self
    }

    pub fn margin_xy(self, x: f32, y: f32) -> Self {
        self.margin(Edges::xy(x, y))
    }

    pub fn margin_x(mut self, x: f32) -> Self {
        let x = sanitize_margin(x);
        self.margin.left = x;
        self.margin.right = x;
        self
    }

    pub fn margin_y(mut self, y: f32) -> Self {
        let y = sanitize_margin(y);
        self.margin.top = y;
        self.margin.bottom = y;
        self
    }

    pub fn margin_left(mut self, v: f32) -> Self {
        self.margin.left = sanitize_margin(v);
        self
    }

    pub fn margin_right(mut self, v: f32) -> Self {
        self.margin.right = sanitize_margin(v);
        self
    }

    pub fn margin_top(mut self, v: f32) -> Self {
        self.margin.top = sanitize_margin(v);
        self
    }

    pub fn margin_bottom(mut self, v: f32) -> Self {
        self.margin.bottom = sanitize_margin(v);
        self
    }

    fn with_state<R>(&self, f: impl FnOnce(&mut AreaState) -> R) -> R {
        f(&mut self.state.borrow_mut())
    }

    fn main_mut(s: &mut AreaState) -> &mut Caret {
        if s.carets.is_empty() {
            s.carets.push(Caret::default());
        }
        &mut s.carets[0]
    }

    fn main_of(s: &AreaState) -> Caret {
        s.carets.first().copied().unwrap_or_default()
    }

    fn reset_escalate(s: &mut AreaState) {
        s.escalate = 0;
        s.escalate_base = 0;
    }

    fn chars_via(source: &DocSource) -> Vec<char> {
        source.get_text().chars().collect()
    }

    fn chars(&self) -> Vec<char> {
        Self::chars_via(&self.source)
    }

    fn starts(&self, chars: &[char]) -> Vec<usize> {
        line_starts(chars)
    }

    pub fn caret(&self) -> usize {
        self.with_state(|s| Self::main_of(s).offset)
    }

    pub fn caret_count(&self) -> usize {
        self.with_state(|s| s.carets.len().max(1))
    }

    pub fn set_caret(&self, caret: usize) {
        let source = self.source.clone();
        self.with_state(|s| {
            source.ensure(&mut s.engine);
            let caret = caret.min(s.engine.len_chars());
            s.carets = vec![Caret::collapsed(caret)];
            Self::reset_escalate(s);
            s.goal_x = None;
        });
        self.bump();
    }

    pub fn selection(&self) -> Option<(usize, usize)> {
        self.with_state(|s| {
            let m = Self::main_of(s);
            m.anchor.map(|a| (a.min(m.offset), a.max(m.offset)))
        })
    }

    pub fn selections(&self) -> Vec<(usize, usize)> {
        self.with_state(|s| s.carets.iter().filter_map(|c| c.selected()).collect())
    }

    pub fn caret_line_col(&self) -> (usize, usize) {
        let chars = self.chars();
        let starts = self.starts(&chars);
        let caret = self.caret();
        line_col_of(&chars, &starts, caret)
    }

    pub fn line_count(&self) -> usize {
        let chars = self.chars();
        self.starts(&chars).len()
    }

    pub fn scroll_offset(&self) -> Vec2 {
        self.with_state(|s| s.scroll)
    }

    pub fn scroll_to(&self, offset: Vec2) {
        let (viewport, content) = self.with_state(|s| (s.viewport, s.content));
        let next = clamp_scroll(offset, content.width, content.height, viewport, self.pad());
        let changed = self.with_state(|s| {
            if s.scroll != next {
                s.scroll = next;
                true
            } else {
                false
            }
        });
        if changed {
            if let Some(f) = &self.on_scroll {
                f(next);
            }
            self.bump();
        }
    }

    fn pad(&self) -> (f32, f32) {
        (self.pad_x, self.pad_y)
    }

    fn begin_session(&self) {
        let cur = self.source.get_text();
        self.with_state(|s| {
            s.session_text = Some(cur);
            s.was_focused = true;
        });
        if let Some(f) = &self.on_edit_start {
            f();
        }
    }

    fn note_focus(&self, focused: bool) {
        enum Trans {
            Start,
            End(String),
            None,
        }
        let cur = self.source.get_text();
        let trans = self.with_state(|s| {
            let was = s.was_focused;
            s.was_focused = focused;
            if !was && focused {
                s.session_text = Some(cur.clone());
                Trans::Start
            } else if was && !focused {
                match s.session_text.take() {
                    Some(before) if before != cur => Trans::End(cur.clone()),
                    _ => Trans::None,
                }
            } else {
                Trans::None
            }
        });
        match trans {
            Trans::Start => {
                if let Some(f) = &self.on_edit_start {
                    f();
                }
            }
            Trans::End(text) => {
                if let Some(f) = &self.on_edit_end {
                    f(text);
                }
            }
            Trans::None => {}
        }
    }

    fn set_caret_extend(&self, caret: usize, extend: bool) {
        let source = self.source.clone();
        self.with_state(|s| {
            source.ensure(&mut s.engine);
            let caret = caret.min(s.engine.len_chars());
            let main = Self::main_of(s);
            s.carets = vec![main];
            Self::main_mut(s).move_to(caret, extend);
            Self::reset_escalate(s);
            s.goal_x = None;
        });
    }

    fn bump(&self) {
        self.revision
            .set(self.revision.get_untracked().wrapping_add(1));
    }

    fn font_weight(&self) -> FontWeight {
        self.weight
            .or_else(|| {
                self.appearance
                    .and_then(|s| s.font_weight)
                    .map(FontWeight::new)
            })
            .unwrap_or_default()
    }

    fn resolved_font_size(&self, theme: &Theme) -> f32 {
        self.font_size
            .or_else(|| self.appearance.and_then(|s| s.font_size))
            .unwrap_or(theme.font_size)
    }

    fn line_height(&self) -> f32 {
        let size = self
            .font_size
            .or_else(|| self.appearance.and_then(|s| s.font_size))
            .unwrap_or(14.0);
        size * self
            .line_height_factor
            .or_else(|| self.appearance.and_then(|s| s.line_height))
            .unwrap_or(DEFAULT_LINE_HEIGHT)
            .clamp(1.0, 3.0)
    }

    fn resolved_line_height(&self, theme: &Theme) -> f32 {
        self.resolved_font_size(theme)
            * self
                .line_height_factor
                .or_else(|| self.appearance.and_then(|s| s.line_height))
                .unwrap_or(DEFAULT_LINE_HEIGHT)
                .clamp(1.0, 3.0)
    }

    fn resolved_text_color(&self, theme: &Theme) -> Color {
        self.text_color
            .or_else(|| self.appearance.and_then(|s| s.text))
            .unwrap_or(theme.text)
    }

    fn resolved_placeholder_color(&self, theme: &Theme) -> Color {
        self.placeholder_color
            .or_else(|| self.appearance.and_then(|s| s.placeholder))
            .unwrap_or(theme.text_secondary)
    }

    fn resolved_background(&self, theme: &Theme) -> Color {
        self.background
            .or_else(|| self.appearance.and_then(|s| s.background))
            .unwrap_or(theme.surface)
    }

    fn resolved_border(&self, theme: &Theme) -> Option<(f32, Color)> {
        let width = self
            .border_width
            .or_else(|| self.appearance.and_then(|s| s.border_width))
            .unwrap_or(1.0)
            .max(0.0);
        if width <= 0.0 {
            return None;
        }
        let color = self
            .border_color
            .or_else(|| self.appearance.and_then(|s| s.border))
            .unwrap_or(theme.border);
        if color.a <= 0.0 {
            return None;
        }
        Some((width, color))
    }

    fn resolved_radius(&self, theme: &Theme) -> f32 {
        self.radius
            .or_else(|| self.appearance.and_then(|s| s.radius))
            .unwrap_or(theme.radius)
            .max(0.0)
    }

    fn delete_selection(&self) -> bool {
        let source = self.source.clone();
        let mut deleted = false;
        self.with_state(|s| {
            source.ensure(&mut s.engine);
            let len = s.engine.len_chars();

            if s.carets.len() <= 1 {
                let m = Self::main_of(s);
                let Some((mut a, mut b)) = m.anchor.map(|x| (x.min(m.offset), x.max(m.offset)))
                else {
                    return;
                };
                if a == b {
                    Self::main_mut(s).anchor = None;
                    return;
                }
                (a, b) = (a.min(len), b.min(len));
                let (ab, bb) = (s.engine.to_bytes(a), s.engine.to_bytes(b));
                s.engine.delete(ab..bb);
                source.commit(&mut s.engine);
                let offset = s.engine.to_chars(ab);
                let main = Self::main_mut(s);
                main.offset = offset;
                main.anchor = None;
                s.goal_x = None;
                Self::reset_escalate(s);
                deleted = true;
                return;
            }

            let mut pre: Vec<(usize, usize, usize)> = Vec::new();
            for (i, c) in s.carets.iter().enumerate() {
                if let Some((a, b)) = c.selected() {
                    let (a, b) = (a.min(len), b.min(len));
                    if a < b {
                        pre.push((i, s.engine.to_bytes(a), s.engine.to_bytes(b)));
                    }
                } else if c.anchor.is_some() {
                }
            }
            for c in s.carets.iter_mut() {
                if c.selected().is_none() {
                    c.anchor = None;
                }
            }
            if pre.is_empty() {
                return;
            }
            let raw: Vec<(usize, usize)> = pre.iter().map(|&(_, a, b)| (a, b)).collect();
            let (merged, owner) = merge_byte_ranges(&raw);
            let ranges: Vec<std::ops::Range<usize>> = merged.iter().map(|&(a, b)| a..b).collect();
            s.engine.delete_many(ranges);
            source.commit(&mut s.engine);
            let new_len = s.engine.len_chars();

            let mut new_offsets = vec![0usize; s.carets.len()];
            let mut touched = vec![false; s.carets.len()];
            for (k, &(i, _, _)) in pre.iter().enumerate() {
                let (us, _) = merged[owner[k]];
                let mapped = map_through_deletes(&merged, us);
                new_offsets[i] = s.engine.to_chars(mapped).min(new_len);
                touched[i] = true;
            }
            for (i, c) in s.carets.iter_mut().enumerate() {
                if touched[i] {
                    c.offset = new_offsets[i];
                    c.anchor = None;
                }
            }

            dedupe_carets(&mut s.carets);
            s.goal_x = None;
            Self::reset_escalate(s);
            deleted = true;
        });
        if deleted {
            self.bump();
        }
        deleted
    }

    pub fn selected_text(&self) -> Option<String> {
        let source = self.source.clone();
        self.with_state(|s| {
            source.ensure(&mut s.engine);
            let len = s.engine.len_chars();
            let mut parts = Vec::new();
            for c in &s.carets {
                if let Some((a, b)) = c.selected() {
                    let (a, b) = (a.min(len), b.min(len));
                    if a < b {
                        let text = s
                            .engine
                            .buffer()
                            .substring(s.engine.to_bytes(a)..s.engine.to_bytes(b));
                        if !text.is_empty() {
                            parts.push(text);
                        }
                    }
                }
            }
            if parts.is_empty() {
                None
            } else {
                Some(parts.join("\n"))
            }
        })
    }

    pub fn select_range(&self, a: usize, b: usize) {
        let source = self.source.clone();
        self.with_state(|s| {
            source.ensure(&mut s.engine);
            let len = s.engine.len_chars();
            let (a, b) = (a.min(len), b.min(len));

            s.carets = vec![Caret {
                offset: b,
                anchor: if a == b { None } else { Some(a) },
            }];
            Self::reset_escalate(s);
            s.goal_x = None;
        });
        self.bump();
    }

    pub fn clear_selection(&self) -> bool {
        let had = self.with_state(|s| {
            let had = s.carets.iter().any(|c| c.anchor.is_some());
            for c in s.carets.iter_mut() {
                c.anchor = None;
            }
            Self::reset_escalate(s);
            had
        });
        if had {
            self.bump();
        }
        had
    }

    pub fn select_all(&self) {
        let source = self.source.clone();
        self.with_state(|s| {
            source.ensure(&mut s.engine);
            let len = s.engine.len_chars();
            s.carets = vec![Caret {
                offset: len,
                anchor: Some(0),
            }];
            Self::reset_escalate(s);
            s.goal_x = None;
        });
        self.bump();
    }

    fn insert_str(&self, text: &str) {
        if !self.editable || text.is_empty() {
            return;
        }
        let source = self.source.clone();
        let single = self.with_state(|s| s.carets.len() <= 1);
        if single {
            self.with_state(|st| {
                source.ensure(&mut st.engine);
                let total = st.engine.len_chars();

                let sel_bytes = match Self::main_of(st).anchor.map(|a| {
                    (
                        a.min(Self::main_of(st).offset),
                        a.max(Self::main_of(st).offset),
                    )
                }) {
                    None => None,
                    Some((a, b)) if a == b => {
                        Self::main_mut(st).anchor = None;
                        None
                    }
                    Some((a, b)) => {
                        let (a, b) = (a.min(total), b.min(total));
                        Some((st.engine.to_bytes(a), st.engine.to_bytes(b)))
                    }
                };

                let allowed = match self.max_length {
                    Some(max) => max.saturating_sub(total),
                    None => usize::MAX,
                };
                let ins = BufferEngine::fit_prefix(text, allowed);
                let caret_b = match sel_bytes {
                    Some((ab, bb)) => {
                        st.engine.replace(ab..bb, ins);
                        ab + ins.len()
                    }
                    None => {
                        let caret_b = st.engine.to_bytes(Self::main_of(st).offset.min(total));
                        st.engine.insert(caret_b, ins);
                        caret_b + ins.len()
                    }
                };
                if let Some(max) = self.max_lines {
                    st.engine.enforce_max_lines(max);
                }
                source.commit(&mut st.engine);

                let offset = st.engine.to_chars(caret_b).min(st.engine.len_chars());
                let main = Self::main_mut(st);
                main.offset = offset;
                main.anchor = None;
                st.goal_x = None;
                Self::reset_escalate(st);
            });
            self.bump();
            return;
        }

        self.with_state(|st| {
            source.ensure(&mut st.engine);
            let total = st.engine.len_chars();
            let mut sel_pre: Vec<(usize, usize, usize)> = Vec::new();
            for (i, c) in st.carets.iter().enumerate() {
                if let Some((a, b)) = c.selected() {
                    let (a, b) = (a.min(total), b.min(total));
                    if a < b {
                        sel_pre.push((i, st.engine.to_bytes(a), st.engine.to_bytes(b)));
                    }
                }
            }
            let raw: Vec<(usize, usize)> = sel_pre.iter().map(|&(_, a, b)| (a, b)).collect();
            let (merged, owner) = merge_byte_ranges(&raw);
            if !merged.is_empty() {
                let ranges: Vec<std::ops::Range<usize>> =
                    merged.iter().map(|&(a, b)| a..b).collect();
                st.engine.delete_many(ranges);
            }

            let mut points: Vec<usize> = Vec::with_capacity(st.carets.len());
            let mut caret_union: Vec<Option<usize>> = vec![None; st.carets.len()];
            for (k, &(i, _, _)) in sel_pre.iter().enumerate() {
                caret_union[i] = Some(owner[k]);
            }
            for (i, c) in st.carets.iter().enumerate() {
                let pre_b = match caret_union[i] {
                    Some(u) => merged[u].0,
                    None => st.engine.to_bytes(c.offset.min(total)),
                };

                let mut u_hit = caret_union[i];
                if u_hit.is_none() {
                    for (u, &(us, ue)) in merged.iter().enumerate() {
                        if pre_b >= us && pre_b < ue {
                            u_hit = Some(u);
                            break;
                        }
                    }
                }
                let post_b = match u_hit {
                    Some(u) => map_through_deletes(&merged, merged[u].0),
                    None => map_through_deletes(&merged, pre_b),
                };
                points.push(post_b);
            }

            let allowed = match self.max_length {
                Some(max) => max.saturating_sub(total),
                None => usize::MAX,
            };
            let ins = BufferEngine::fit_prefix(text, allowed);
            if ins.is_empty() {
                source.commit(&mut st.engine);
                let new_len = st.engine.len_chars();
                for (i, c) in st.carets.iter_mut().enumerate() {
                    c.offset = st.engine.to_chars(points[i]).min(new_len);
                    c.anchor = None;
                }
                dedupe_carets(&mut st.carets);
                st.goal_x = None;
                Self::reset_escalate(st);
                return;
            }

            let mut uniq = points.clone();
            uniq.sort_unstable();
            uniq.dedup();

            let mut pcs: Vec<usize> = uniq.iter().map(|&p| st.engine.to_chars(p)).collect();
            pcs.sort_unstable();
            let ins_chars = ins.chars().count();
            let at: Vec<usize> = uniq.clone();
            st.engine.insert_at_many(at, ins);
            if let Some(max) = self.max_lines {
                st.engine.enforce_max_lines(max);
            }
            source.commit(&mut st.engine);
            let new_len = st.engine.len_chars();

            let mut new_offsets = vec![0usize; st.carets.len()];
            for (i, &p) in points.iter().enumerate() {
                let pc = st.engine.to_chars(p);
                let below = pcs.iter().filter(|&&q| q < pc).count();
                new_offsets[i] = (pc + ins_chars * (1 + below)).min(new_len);
            }
            for (i, c) in st.carets.iter_mut().enumerate() {
                c.offset = new_offsets[i];
                c.anchor = None;
            }
            dedupe_carets(&mut st.carets);
            st.goal_x = None;
            Self::reset_escalate(st);
        });
        self.bump();
    }

    fn delete_range_for(
        engine: &BufferEngine,
        caret_b: usize,
        back: bool,
        word: bool,
    ) -> (usize, usize) {
        let buf = engine.buffer();
        if back {
            let from_b = if word {
                sequoia_buffer::word_start_before(buf, caret_b)
            } else {
                sequoia_buffer::prev_grapheme(buf, caret_b)
            };
            (from_b, caret_b)
        } else {
            let to_b = if word {
                sequoia_buffer::word_end_after(buf, caret_b)
            } else {
                sequoia_buffer::next_grapheme(buf, caret_b)
            };
            (caret_b, to_b)
        }
    }

    fn range_styled(engine: &BufferEngine, s: usize, e: usize) -> bool {
        s < e
            && engine
                .styled_buffer()
                .spans()
                .iter()
                .any(|sp| sp.start < e && s < sp.end)
    }

    fn backspace(&self, word: bool) {
        if !self.editable {
            return;
        }
        if self.delete_selection() {
            return;
        }
        let source = self.source.clone();

        if self.with_state(|s| s.carets.len() <= 1) {
            if !word {
                let stripped = self.with_state(|s| {
                    source.ensure(&mut s.engine);
                    let len = s.engine.len_chars();
                    let caret = Self::main_of(s).offset.min(len);
                    let caret_b = s.engine.to_bytes(caret);
                    let total_b = s.engine.to_bytes(len);
                    if caret == 0 {
                        return false;
                    }
                    let from_b = sequoia_buffer::prev_grapheme(s.engine.buffer(), caret_b);
                    if from_b == caret_b || !Self::range_styled(&s.engine, from_b, caret_b) {
                        return false;
                    }
                    let next_styled = if caret_b < total_b {
                        let to_b = sequoia_buffer::next_grapheme(s.engine.buffer(), caret_b);
                        Self::range_styled(&s.engine, caret_b, to_b)
                    } else {
                        false
                    };
                    if next_styled {
                        return false;
                    }
                    s.engine.clear_attr(from_b..caret_b);
                    source.commit(&mut s.engine);

                    Self::main_mut(s).offset = s.engine.to_chars(caret_b).min(s.engine.len_chars());
                    Self::main_mut(s).anchor = None;
                    s.goal_x = None;
                    Self::reset_escalate(s);
                    true
                });
                if stripped {
                    self.bump();
                    return;
                }
            }
            self.with_state(|s| {
                source.ensure(&mut s.engine);
                let len = s.engine.len_chars();
                let caret = Self::main_of(s).offset.min(len);
                if caret == 0 {
                    return;
                }
                let caret_b = s.engine.to_bytes(caret);
                let (from_b, _) = Self::delete_range_for(&s.engine, caret_b, true, word);
                s.engine.delete(from_b..caret_b);
                source.commit(&mut s.engine);
                let offset = s.engine.to_chars(from_b);
                let main = Self::main_mut(s);
                main.offset = offset;
                main.anchor = None;
                s.goal_x = None;
                Self::reset_escalate(s);
            });
            self.bump();
            return;
        }

        self.with_state(|s| {
            source.ensure(&mut s.engine);
            let len = s.engine.len_chars();
            let total_b = s.engine.to_bytes(len);

            let pre_bytes: Vec<usize> = s
                .carets
                .iter()
                .map(|c| s.engine.to_bytes(c.offset.min(len)))
                .collect();
            let mut stripped_bytes: Vec<usize> = Vec::new();
            if !word {
                for c in s.carets.iter() {
                    debug_assert!(c.selected().is_none());
                    let caret = c.offset.min(len);
                    if caret == 0 {
                        continue;
                    }
                    let caret_b = s.engine.to_bytes(caret);
                    let from_b = sequoia_buffer::prev_grapheme(s.engine.buffer(), caret_b);
                    if from_b == caret_b || !Self::range_styled(&s.engine, from_b, caret_b) {
                        continue;
                    }
                    let next_styled = if caret_b < total_b {
                        let to_b = sequoia_buffer::next_grapheme(s.engine.buffer(), caret_b);
                        Self::range_styled(&s.engine, caret_b, to_b)
                    } else {
                        false
                    };
                    if next_styled {
                        continue;
                    }
                    s.engine.clear_attr(from_b..caret_b);
                    stripped_bytes.push(caret_b);
                }
                if !stripped_bytes.is_empty() {
                    source.commit(&mut s.engine);
                }
            }

            let mut ranges: Vec<(usize, usize)> = Vec::new();
            let mut owners: Vec<usize> = Vec::new();
            for (i, c) in s.carets.iter().enumerate() {
                let caret = c.offset.min(len);
                let caret_b = s.engine.to_bytes(caret);
                if stripped_bytes.contains(&caret_b) {
                    continue;
                }
                if caret == 0 {
                    continue;
                }
                let (from_b, to_b) = Self::delete_range_for(&s.engine, caret_b, true, word);
                if from_b < to_b {
                    ranges.push((from_b, to_b));
                    owners.push(i);
                }
            }
            if ranges.is_empty() {
                let new_len = s.engine.len_chars();
                for c in s.carets.iter_mut() {
                    c.offset = c.offset.min(new_len);
                    c.anchor = None;
                }
                Self::reset_escalate(s);
                return;
            }
            let raw = ranges.clone();
            let (merged, owner) = merge_byte_ranges(&raw);
            let dranges: Vec<std::ops::Range<usize>> = merged.iter().map(|&(a, b)| a..b).collect();
            s.engine.delete_many(dranges);
            source.commit(&mut s.engine);
            let new_len = s.engine.len_chars();

            let mut new_offsets = vec![0usize; s.carets.len()];
            let mut touched = vec![false; s.carets.len()];
            for (k, &i) in owners.iter().enumerate() {
                let (us, _) = merged[owner[k]];
                new_offsets[i] = s
                    .engine
                    .to_chars(map_through_deletes(&merged, us))
                    .min(new_len);
                touched[i] = true;
            }
            for (i, c) in s.carets.iter_mut().enumerate() {
                if !touched[i] {
                    let pb = pre_bytes[i];
                    c.offset = s
                        .engine
                        .to_chars(map_through_deletes(&merged, pb))
                        .min(new_len);
                } else {
                    c.offset = new_offsets[i];
                }
                c.anchor = None;
            }
            s.goal_x = None;
            Self::reset_escalate(s);
            dedupe_carets(&mut s.carets);
        });
        self.bump();
    }

    fn delete(&self, word: bool) {
        if !self.editable {
            return;
        }
        if self.delete_selection() {
            return;
        }
        let source = self.source.clone();
        if self.with_state(|s| s.carets.len() <= 1) {
            self.with_state(|s| {
                source.ensure(&mut s.engine);
                let len = s.engine.len_chars();
                let caret = Self::main_of(s).offset.min(len);
                if caret >= len {
                    return;
                }
                let caret_b = s.engine.to_bytes(caret);
                let (_, to_b) = Self::delete_range_for(&s.engine, caret_b, false, word);
                s.engine.delete(caret_b..to_b);
                source.commit(&mut s.engine);
                let offset = s.engine.to_chars(caret_b);
                let main = Self::main_mut(s);
                main.offset = offset;
                main.anchor = None;
                s.goal_x = None;
                Self::reset_escalate(s);
            });
            self.bump();
            return;
        }

        self.with_state(|s| {
            source.ensure(&mut s.engine);
            let len = s.engine.len_chars();
            let pre_bytes: Vec<usize> = s
                .carets
                .iter()
                .map(|c| s.engine.to_bytes(c.offset.min(len)))
                .collect();
            let mut ranges: Vec<(usize, usize)> = Vec::new();
            let mut owners: Vec<usize> = Vec::new();
            for (i, &cb) in pre_bytes.iter().enumerate() {
                let caret = s.carets[i].offset.min(len);
                if caret >= len {
                    continue;
                }
                let (_, to_b) = Self::delete_range_for(&s.engine, cb, false, word);
                if cb < to_b {
                    ranges.push((cb, to_b));
                    owners.push(i);
                }
            }
            if ranges.is_empty() {
                Self::reset_escalate(s);
                return;
            }
            let (merged, owner) = merge_byte_ranges(&ranges);
            let dranges: Vec<std::ops::Range<usize>> = merged.iter().map(|&(a, b)| a..b).collect();
            s.engine.delete_many(dranges);
            source.commit(&mut s.engine);
            let new_len = s.engine.len_chars();
            let mut new_offsets = vec![0usize; s.carets.len()];
            let mut touched = vec![false; s.carets.len()];
            for (k, &i) in owners.iter().enumerate() {
                let (us, _) = merged[owner[k]];
                new_offsets[i] = s
                    .engine
                    .to_chars(map_through_deletes(&merged, us))
                    .min(new_len);
                touched[i] = true;
            }
            for (i, c) in s.carets.iter_mut().enumerate() {
                if touched[i] {
                    c.offset = new_offsets[i];
                } else {
                    c.offset = s
                        .engine
                        .to_chars(map_through_deletes(&merged, pre_bytes[i]))
                        .min(new_len);
                }
                c.anchor = None;
            }
            s.goal_x = None;
            Self::reset_escalate(s);
            dedupe_carets(&mut s.carets);
        });
        self.bump();
    }

    fn move_caret(&self, delta: isize, extend: bool) {
        let source = self.source.clone();
        self.with_state(|s| {
            source.ensure(&mut s.engine);
            let len = s.engine.len_chars();
            for c in s.carets.iter_mut() {
                let mut caret_b = s.engine.to_bytes(c.offset.min(len));
                let steps = delta.unsigned_abs().max(1);
                for _ in 0..steps {
                    let b = if delta < 0 {
                        sequoia_buffer::prev_grapheme(s.engine.buffer(), caret_b)
                    } else {
                        sequoia_buffer::next_grapheme(s.engine.buffer(), caret_b)
                    };
                    if b == caret_b {
                        break;
                    }
                    caret_b = b;
                }
                c.move_to(s.engine.to_chars(caret_b), extend);
            }
            Self::reset_escalate(s);
            s.goal_x = None;
        });
        self.bump();
    }

    fn move_word(&self, dir: isize, extend: bool) {
        let source = self.source.clone();
        self.with_state(|s| {
            source.ensure(&mut s.engine);
            let len = s.engine.len_chars();
            for c in s.carets.iter_mut() {
                let caret_b = s.engine.to_bytes(c.offset.min(len));
                let next_b = if dir < 0 {
                    sequoia_buffer::word_start_before(s.engine.buffer(), caret_b)
                } else {
                    sequoia_buffer::word_end_after(s.engine.buffer(), caret_b)
                };
                c.move_to(s.engine.to_chars(next_b), extend);
            }
            Self::reset_escalate(s);
            s.goal_x = None;
        });
        self.bump();
    }

    fn move_home(&self, extend: bool, doc: bool) {
        let source = self.source.clone();
        self.with_state(|s| {
            source.ensure(&mut s.engine);
            let len = s.engine.len_chars();
            for c in s.carets.iter_mut() {
                let next = if doc {
                    0
                } else {
                    let caret_b = s.engine.to_bytes(c.offset.min(len));
                    let home_b = sequoia_buffer::line_home(s.engine.buffer(), caret_b);
                    s.engine.to_chars(home_b)
                };
                c.move_to(next, extend);
            }
            Self::reset_escalate(s);
            s.goal_x = None;
        });
        self.bump();
    }

    fn move_end(&self, extend: bool, doc: bool) {
        let source = self.source.clone();
        self.with_state(|s| {
            source.ensure(&mut s.engine);
            let total = s.engine.len_chars();
            for c in s.carets.iter_mut() {
                let next = if doc {
                    total
                } else {
                    let caret_b = s.engine.to_bytes(c.offset.min(total));
                    let end_b = sequoia_buffer::line_end(s.engine.buffer(), caret_b);
                    s.engine.to_chars(end_b)
                };
                c.move_to(next, extend);
            }
            Self::reset_escalate(s);
            s.goal_x = None;
        });
        self.bump();
    }

    fn row_at_y(line_h: f32, rows: usize, cy: f32) -> usize {
        if line_h <= 0.0 || rows == 0 {
            return 0;
        }
        ((cy / line_h).floor() as isize).clamp(0, rows.saturating_sub(1) as isize) as usize
    }

    fn move_page(&self, dir: isize, extend: bool, viewport_h: f32, line_h: f32) {
        if viewport_h <= 0.0 || line_h <= 0.0 {
            for _ in 0..3 {
                self.step_vertical(dir, extend);
            }
            self.bump();
            return;
        }

        let step = ((viewport_h / line_h).floor() as usize).max(1);
        for _ in 0..step {
            self.step_vertical(dir, extend);
        }
        self.with_state(|s| {
            let max_y = (s.content.height + self.pad_y * 2.0 - viewport_h).max(0.0);
            s.scroll.y = (s.scroll.y + dir as f32 * viewport_h).clamp(0.0, max_y);
        });
        self.bump();
    }

    fn step_vertical(&self, dir: isize, extend: bool) {
        let source = self.source.clone();
        self.with_state(|s| {
            source.ensure(&mut s.engine);
            let len = s.engine.len_chars();
            for c in s.carets.iter_mut() {
                let caret_b = s.engine.to_bytes(c.offset.min(len));
                let (next_b, _) = sequoia_buffer::move_lines(s.engine.buffer(), caret_b, dir, None);
                if next_b == caret_b {
                    continue;
                }
                c.move_to(s.engine.to_chars(next_b), extend);
            }
            Self::reset_escalate(s);
        });
    }

    fn on_enter(&self) {
        if !self.editable {
            return;
        }
        if self.with_state(|s| s.carets.len() <= 1) {
            self.delete_selection();
            let source = self.source.clone();
            let indent = self.with_state(|s| {
                source.ensure(&mut s.engine);
                if !self.auto_indent {
                    return String::new();
                }
                let caret_b = s
                    .engine
                    .to_bytes(Self::main_of(s).offset.min(s.engine.len_chars()));
                let lines = s.engine.buffer().lines();
                let (ls, le) = lines.line_range(lines.line_of(caret_b));
                let line_text = s.engine.buffer().substring(ls..le);
                line_text
                    .chars()
                    .take(leading_indent_len(&line_text))
                    .collect()
            });
            self.insert_str(&format!("\n{indent}"));
            return;
        }

        self.delete_selection();
        let source = self.source.clone();
        self.with_state(|s| {
            source.ensure(&mut s.engine);
            let total = s.engine.len_chars();
            let mut items: Vec<(usize, String)> = Vec::with_capacity(s.carets.len());
            for c in s.carets.iter() {
                let caret_b = s.engine.to_bytes(c.offset.min(total));
                let indent = if self.auto_indent {
                    let lines = s.engine.buffer().lines();
                    let (ls, le) = lines.line_range(lines.line_of(caret_b));
                    let line_text = s.engine.buffer().substring(ls..le);
                    line_text
                        .chars()
                        .take(leading_indent_len(&line_text))
                        .collect::<String>()
                } else {
                    String::new()
                };
                items.push((caret_b, indent));
            }

            let mut uniq: Vec<usize> = items.iter().map(|(b, _)| *b).collect();
            uniq.sort_unstable();
            uniq.dedup();
            s.engine.insert_at_many(uniq.clone(), "\n");
            if let Some(max) = self.max_lines {
                s.engine.enforce_max_lines(max);
            }

            let post_nl = |pb: usize| -> usize { pb + uniq.iter().filter(|&&q| q <= pb).count() };

            let cur_chars = s.engine.len_chars();
            let indent_total: usize = items.iter().map(|(_, it)| it.chars().count()).sum();
            let allow_indent = match self.max_length {
                Some(max) => cur_chars + indent_total <= max,
                None => true,
            };
            let mut phase2: Vec<(usize, String)> = items
                .iter()
                .map(|(pb, indent)| (post_nl(*pb), indent.clone()))
                .filter(|(_, indent)| !indent.is_empty())
                .collect();
            if !allow_indent {
                phase2.clear();
            }
            phase2.sort_by_key(|b| std::cmp::Reverse(b.0));
            let mut k = 0usize;
            while k < phase2.len() {
                let mut j = k + 1;
                while j < phase2.len() && phase2[j].1 == phase2[k].1 {
                    j += 1;
                }
                let mut pts: Vec<usize> = phase2[k..j].iter().map(|(p, _)| *p).collect();
                pts.sort_unstable();
                pts.dedup();
                let text = phase2[k].1.clone();
                s.engine.insert_at_many(pts, &text);
                k = j;
            }
            if let Some(max) = self.max_lines {
                s.engine.enforce_max_lines(max);
            }
            source.commit(&mut s.engine);

            let new_len = s.engine.len_chars();
            let mut finals: Vec<usize> = Vec::with_capacity(items.len());
            for (pb, indent) in &items {
                let q = post_nl(*pb);
                let own = if allow_indent { indent.len() } else { 0 };
                let below: usize = phase2
                    .iter()
                    .filter(|(p, _)| *p < q)
                    .map(|(_, t)| t.len())
                    .sum();
                finals.push(s.engine.to_chars(q + own + below).min(new_len));
            }
            for (c, f) in s.carets.iter_mut().zip(finals.iter()) {
                c.offset = *f;
                c.anchor = None;
            }
            dedupe_carets(&mut s.carets);
            Self::reset_escalate(s);
            s.goal_x = None;
        });
        self.bump();
    }

    fn on_tab(&self, extend_shift: bool) {
        if !self.editable {
            return;
        }
        if extend_shift {
            self.outdent();
        } else if !self.indent_selection() {
            let indent = self.indent.clone();
            if self.with_state(|s| s.carets.len() <= 1) {
                self.insert_str(&indent);
            } else {
                let source = self.source.clone();
                self.with_state(|s| {
                    source.ensure(&mut s.engine);
                    let total = s.engine.len_chars();
                    let mut pts: Vec<usize> = s
                        .carets
                        .iter()
                        .map(|c| s.engine.to_bytes(c.offset.min(total)))
                        .collect();
                    pts.sort_unstable();
                    pts.dedup();
                    let allowed = match self.max_length {
                        Some(max) => max.saturating_sub(s.engine.len_chars()),
                        None => usize::MAX,
                    };
                    let ins = BufferEngine::fit_prefix(&indent, allowed / s.carets.len().max(1));
                    if ins.is_empty() {
                        return;
                    }

                    let mut pcs: Vec<usize> = pts.iter().map(|&p| s.engine.to_chars(p)).collect();
                    pcs.sort_unstable();
                    let n = ins.chars().count();

                    let pre: Vec<usize> = s.carets.iter().map(|c| c.offset.min(total)).collect();
                    s.engine.insert_at_many(pts, ins);
                    if let Some(max) = self.max_lines {
                        s.engine.enforce_max_lines(max);
                    }
                    source.commit(&mut s.engine);
                    let new_len = s.engine.len_chars();
                    for (c, &old) in s.carets.iter_mut().zip(pre.iter()) {
                        let below = pcs.iter().filter(|&&q| q < old).count();

                        c.offset = (old + n * (1 + below)).min(new_len);
                        c.anchor = None;
                    }
                    dedupe_carets(&mut s.carets);
                    Self::reset_escalate(s);
                    s.goal_x = None;
                });
                self.bump();
            }
        }
    }

    fn indent_selection(&self) -> bool {
        if self.indent.is_empty() {
            return false;
        }
        let source = self.source.clone();
        let indent = self.indent.clone();
        let per = indent.len();

        let take_block = self.with_state(|s| {
            source.ensure(&mut s.engine);
            let len = s.engine.len_chars();
            let buf = s.engine.buffer();
            let lines = buf.lines();
            let total_b = buf.len_bytes();
            let mut touched: std::collections::BTreeSet<usize> = std::collections::BTreeSet::new();
            let mut any_multi = false;
            for c in s.carets.iter() {
                match c.selected() {
                    Some((a, b)) => {
                        let (a, b) = (a.min(len), b.min(len));
                        if a >= b {
                            continue;
                        }
                        let (ab, bb) = (s.engine.to_bytes(a), s.engine.to_bytes(b));
                        let l0 = lines.line_of(ab);
                        let (mut l1, c1) = lines.line_col(bb.min(total_b));
                        if c1 == 0 && l1 > l0 {
                            l1 -= 1;
                        }
                        if l1 > l0 {
                            any_multi = true;
                        }
                        for l in l0..=l1 {
                            touched.insert(l);
                        }
                    }
                    None => {
                        let cb = s.engine.to_bytes(c.offset.min(len));
                        touched.insert(lines.line_of(cb));
                    }
                }
            }
            if touched.is_empty() {
                return false;
            }

            if s.carets.len() <= 1 && !any_multi {
                return false;
            }
            true
        });
        if !take_block {
            return false;
        }

        self.with_state(|s| {
            source.ensure(&mut s.engine);
            let buf = s.engine.buffer();
            let lines = buf.lines();
            let total_b = buf.len_bytes();
            let len = s.engine.len_chars();

            let mut touched: std::collections::BTreeSet<usize> = std::collections::BTreeSet::new();
            for c in s.carets.iter() {
                match c.selected() {
                    Some((a, b)) => {
                        let (a, b) = (a.min(len), b.min(len));
                        if a >= b {
                            continue;
                        }
                        let (ab, bb) = (s.engine.to_bytes(a), s.engine.to_bytes(b));
                        let l0 = lines.line_of(ab);
                        let (mut l1, c1) = lines.line_col(bb.min(total_b));
                        if c1 == 0 && l1 > l0 {
                            l1 -= 1;
                        }
                        for l in l0..=l1 {
                            touched.insert(l);
                        }
                    }
                    None => {
                        let cb = s.engine.to_bytes(c.offset.min(len));
                        touched.insert(lines.line_of(cb));
                    }
                }
            }
            let at: Vec<usize> = touched.iter().map(|&l| lines.line_start(l)).collect();
            let shift = |pos: usize| -> usize {
                let mut k = 0usize;
                for &l in touched.iter() {
                    if lines.line_start(l) <= pos {
                        k += 1;
                    } else {
                        break;
                    }
                }
                pos + per * k
            };

            let ends: Vec<(usize, usize)> = s
                .carets
                .iter()
                .map(|c| {
                    let (a, b) = c.ends();
                    let (a, b) = (a.min(len), b.min(len));
                    (shift(s.engine.to_bytes(a)), shift(s.engine.to_bytes(b)))
                })
                .collect();
            s.engine.insert_at_many(at, &indent);
            if let Some(max) = self.max_lines {
                s.engine.enforce_max_lines(max);
            }
            source.commit(&mut s.engine);
            let new_len = s.engine.len_chars();
            for (c, (na, nb)) in s.carets.iter_mut().zip(ends.iter()) {
                let (na, nb) = (
                    s.engine.to_chars(*na).min(new_len),
                    s.engine.to_chars(*nb).min(new_len),
                );

                let was_fwd = c.offset >= c.anchor.unwrap_or(c.offset);
                if na == nb {
                    c.offset = na;
                    c.anchor = None;
                } else if was_fwd {
                    c.offset = nb;
                    c.anchor = Some(na);
                } else {
                    c.offset = na;
                    c.anchor = Some(nb);
                }
            }
            dedupe_carets(&mut s.carets);
            s.goal_x = None;
            Self::reset_escalate(s);
        });
        self.bump();
        true
    }

    fn outdent(&self) {
        let indent_w = self.indent.chars().count().max(1);
        let source = self.source.clone();

        let take_block = self.with_state(|s| {
            source.ensure(&mut s.engine);
            let len = s.engine.len_chars();
            let buf = s.engine.buffer();
            let lines = buf.lines();
            let total_b = buf.len_bytes();
            let mut touched: std::collections::BTreeSet<usize> = std::collections::BTreeSet::new();
            let mut any_multi = false;
            for c in s.carets.iter() {
                match c.selected() {
                    Some((a, b)) => {
                        let (a, b) = (a.min(len), b.min(len));
                        if a >= b {
                            continue;
                        }
                        let (ab, bb) = (s.engine.to_bytes(a), s.engine.to_bytes(b));
                        let l0 = lines.line_of(ab);
                        let (mut l1, c1) = lines.line_col(bb.min(total_b));
                        if c1 == 0 && l1 > l0 {
                            l1 -= 1;
                        }
                        if l1 > l0 {
                            any_multi = true;
                        }
                        for l in l0..=l1 {
                            touched.insert(l);
                        }
                    }
                    None => {
                        let cb = s.engine.to_bytes(c.offset.min(len));
                        touched.insert(lines.line_of(cb));
                    }
                }
            }
            s.carets.len() > 1 || any_multi || touched.len() > 1
        });
        if take_block {
            self.with_state(|s| {
                source.ensure(&mut s.engine);
                let len = s.engine.len_chars();
                let buf = s.engine.buffer();
                let lines = buf.lines();
                let total_b = buf.len_bytes();
                let mut touched: std::collections::BTreeSet<usize> =
                    std::collections::BTreeSet::new();
                for c in s.carets.iter() {
                    match c.selected() {
                        Some((a, b)) => {
                            let (a, b) = (a.min(len), b.min(len));
                            if a >= b {
                                continue;
                            }
                            let (ab, bb) = (s.engine.to_bytes(a), s.engine.to_bytes(b));
                            let l0 = lines.line_of(ab);
                            let (mut l1, c1) = lines.line_col(bb.min(total_b));
                            if c1 == 0 && l1 > l0 {
                                l1 -= 1;
                            }
                            for l in l0..=l1 {
                                touched.insert(l);
                            }
                        }
                        None => {
                            let cb = s.engine.to_bytes(c.offset.min(len));
                            touched.insert(lines.line_of(cb));
                        }
                    }
                }
                let mut ranges = Vec::new();
                let mut take_of: std::collections::BTreeMap<usize, usize> =
                    std::collections::BTreeMap::new();
                for &line in touched.iter() {
                    let (ls, le) = lines.line_range(line);
                    let head = buf.substring(ls..le.min(ls + indent_w + 1));
                    let take = head
                        .chars()
                        .take_while(|&c| c == ' ' || c == '\t')
                        .take(indent_w)
                        .map(|c| c.len_utf8())
                        .sum::<usize>();
                    take_of.insert(line, take);
                    if take > 0 {
                        ranges.push(ls..ls + take);
                    }
                }

                let map = |pos: usize| -> usize {
                    let (line, _) = lines.line_col(pos.min(total_b));
                    let mut sub = 0usize;
                    for (&l, &take) in take_of.iter() {
                        if l > line {
                            break;
                        }
                        if l < line || lines.line_start(l) + take <= pos {
                            sub += take;
                        }
                    }
                    pos.saturating_sub(sub)
                };

                let ends: Vec<(usize, usize)> = s
                    .carets
                    .iter()
                    .map(|c| {
                        let (a, b) = c.ends();
                        let (a, b) = (a.min(len), b.min(len));
                        (map(s.engine.to_bytes(a)), map(s.engine.to_bytes(b)))
                    })
                    .collect();
                if !ranges.is_empty() {
                    s.engine.delete_many(ranges);
                    source.commit(&mut s.engine);
                }
                let new_len = s.engine.len_chars();
                for (c, (na, nb)) in s.carets.iter_mut().zip(ends.iter()) {
                    let (na, nb) = (
                        s.engine.to_chars(*na).min(new_len),
                        s.engine.to_chars(*nb).min(new_len),
                    );
                    let was_fwd = c.offset >= c.anchor.unwrap_or(c.offset);
                    if na == nb {
                        c.offset = na;
                        c.anchor = None;
                    } else if was_fwd {
                        c.offset = nb;
                        c.anchor = Some(na);
                    } else {
                        c.offset = na;
                        c.anchor = Some(nb);
                    }
                }
                dedupe_carets(&mut s.carets);
                s.goal_x = None;
                Self::reset_escalate(s);
            });
            self.bump();
            return;
        }

        let moved = self.with_state(|s| {
            source.ensure(&mut s.engine);
            let len = s.engine.len_chars();
            let caret = Self::main_of(s).offset.min(len);
            let caret_b = s.engine.to_bytes(caret);
            let lines = s.engine.buffer().lines();
            let start = lines.line_start(lines.line_of(caret_b));
            let prefix = s.engine.buffer().substring(start..caret_b);
            let mut strip = 0usize;
            for c in prefix.chars().rev().take(indent_w) {
                if c == ' ' || c == '\t' {
                    strip += c.len_utf8();
                } else {
                    break;
                }
            }
            if strip == 0 || caret_b == start {
                return None;
            }
            s.engine.delete(caret_b - strip..caret_b);
            source.commit(&mut s.engine);
            Some(s.engine.to_chars(caret_b - strip))
        });
        if let Some(caret) = moved {
            self.with_state(|s| {
                Self::main_mut(s).offset = caret;
                Self::main_mut(s).anchor = None;
                s.goal_x = None;
                Self::reset_escalate(s);
            });
            self.bump();
        }
    }

    pub const LINK_PLACEHOLDER: &'static str = "https://example.com";

    fn word_at(engine: &BufferEngine, text: &str, caret: usize) -> Option<(usize, usize)> {
        let chars: Vec<char> = text.chars().collect();
        let len = chars.len();
        let c = caret.min(len);
        let on_word = |i: usize| chars.get(i).is_some_and(|c| is_word_char(*c));
        if !(on_word(c) || (c > 0 && on_word(c - 1))) {
            return None;
        }
        let buf = engine.buffer();
        let b = engine.to_bytes(c);
        let sb = sequoia_buffer::word_start_before(buf, b);
        let eb = sequoia_buffer::word_end_after(buf, b);
        if sb >= eb {
            return None;
        }
        let (a, z) = (engine.to_chars(sb), engine.to_chars(eb));
        if a < z { Some((a, z)) } else { None }
    }

    fn line_at(engine: &BufferEngine, len_chars: usize, caret: usize) -> (usize, usize) {
        let cb = engine.to_bytes(caret.min(len_chars));
        let lines = engine.buffer().lines();
        let (bs, be) = lines.line_range(lines.line_of(cb));
        let total_b = engine.to_bytes(len_chars);
        let s = engine.to_chars(bs);
        let e = if be < total_b {
            engine.to_chars(be) + 1
        } else {
            engine.to_chars(be)
        };
        (s, e.min(len_chars))
    }

    fn toggle_flag(&self, flag: StyleFlag) {
        if !self.editable {
            return;
        }
        let source = self.source.clone();
        self.with_state(|s| {
            source.ensure(&mut s.engine);
            let len = s.engine.len_chars();
            let snapshot = s.engine.styled_buffer().to_string();

            let mut ranges: Vec<(usize, usize)> = Vec::with_capacity(s.carets.len());
            for c in s.carets.iter() {
                let r = match c.selected() {
                    Some((a, b)) => {
                        let (a, b) = (a.min(len), b.min(len));
                        if a < b { Some((a, b)) } else { None }
                    }
                    None => Self::word_at(&s.engine, &snapshot, c.offset),
                };
                if let Some(r) = r {
                    ranges.push(r);
                }
            }
            if ranges.is_empty() {
                return;
            }
            for (a, b) in ranges {
                let sb = s.engine.to_bytes(a);
                let eb = s.engine.to_bytes(b);
                if sb >= eb {
                    continue;
                }
                let spans: Vec<StyledSpan> = s.engine.styled_buffer().spans().to_vec();

                let mut covered = sb;
                let mut all_set = true;
                for sp in spans.iter() {
                    if sp.end <= covered {
                        continue;
                    }
                    if sp.start > covered {
                        all_set = false;
                        break;
                    }
                    if !flag.get(&sp.attr) {
                        all_set = false;
                        break;
                    }
                    covered = covered.max(sp.end);
                    if covered >= eb {
                        break;
                    }
                }
                if covered < eb {
                    all_set = false;
                }
                if all_set {
                    let pieces: Vec<(usize, usize, SpanAttr)> = spans
                        .iter()
                        .filter(|sp| sp.start < eb && sb < sp.end)
                        .map(|sp| {
                            let ps = sp.start.max(sb);
                            let pe = sp.end.min(eb);
                            let mut attr = sp.attr.clone();
                            flag.set(&mut attr, false);
                            (ps, pe, attr)
                        })
                        .collect();
                    for (ps, pe, attr) in pieces {
                        s.engine.set_attr(ps..pe, attr);
                    }
                } else {
                    let mut bounds = vec![sb, eb];
                    for sp in spans.iter() {
                        if sp.start > sb && sp.start < eb {
                            bounds.push(sp.start);
                        }
                        if sp.end > sb && sp.end < eb {
                            bounds.push(sp.end);
                        }
                    }
                    bounds.sort_unstable();
                    bounds.dedup();
                    for w in bounds.windows(2) {
                        let (ps, pe) = (w[0], w[1]);
                        let mut attr = spans
                            .iter()
                            .find(|sp| sp.start <= ps && pe <= sp.end)
                            .map(|sp| sp.attr.clone())
                            .unwrap_or_else(SpanAttr::plain);
                        flag.set(&mut attr, true);
                        s.engine.set_attr(ps..pe, attr);
                    }
                }
            }
            source.commit(&mut s.engine);

            let new_len = s.engine.len_chars();
            for c in s.carets.iter_mut() {
                c.offset = c.offset.min(new_len);
                if let Some(an) = c.anchor {
                    c.anchor = Some(an.min(new_len));
                    if c.anchor == Some(c.offset) {
                        c.anchor = None;
                    }
                }
            }
            Self::reset_escalate(s);
            s.goal_x = None;
        });
        self.bump();
    }

    pub fn toggle_bold(&self) {
        self.toggle_flag(StyleFlag::Bold);
    }

    pub fn toggle_italic(&self) {
        self.toggle_flag(StyleFlag::Italic);
    }

    pub fn toggle_underline(&self) {
        self.toggle_flag(StyleFlag::Underline);
    }

    pub fn toggle_strike(&self) {
        self.toggle_flag(StyleFlag::Strike);
    }

    pub fn apply_link(&self) {
        self.apply_link_url(Self::LINK_PLACEHOLDER);
    }

    pub fn apply_link_url(&self, url: &str) {
        if !self.editable {
            return;
        }
        let source = self.source.clone();
        let url = url.to_string();
        self.with_state(|s| {
            source.ensure(&mut s.engine);
            let len = s.engine.len_chars();
            let snapshot = s.engine.styled_buffer().to_string();
            let mut ranges: Vec<(usize, usize)> = Vec::new();
            for c in s.carets.iter() {
                let r = match c.selected() {
                    Some((a, b)) => {
                        let (a, b) = (a.min(len), b.min(len));
                        if a < b { Some((a, b)) } else { None }
                    }
                    None => Self::word_at(&s.engine, &snapshot, c.offset),
                };
                if let Some(r) = r {
                    ranges.push(r);
                }
            }
            if ranges.is_empty() {
                return;
            }
            let mut mids: Vec<usize> = Vec::with_capacity(ranges.len());
            for (a, b) in &ranges {
                let sb = s.engine.to_bytes(*a);
                let eb = s.engine.to_bytes(*b);
                if sb < eb {
                    s.engine.set_attr(sb..eb, SpanAttr::link(url.clone()));
                }
                mids.push(a + (b - a) / 2);
            }
            source.commit(&mut s.engine);
            let new_len = s.engine.len_chars();
            for (c, m) in s.carets.iter_mut().zip(mids.iter()) {
                c.offset = (*m).min(new_len);
                c.anchor = None;
            }
            dedupe_carets(&mut s.carets);
            Self::reset_escalate(s);
            s.goal_x = None;
        });
        self.bump();
    }

    fn on_style_shortcut(&self, key: &KeyEvent) -> bool {
        let cmd =
            key.modifiers.contains(Modifiers::CONTROL) || key.modifiers.contains(Modifiers::SUPER);
        if !cmd || key.modifiers.contains(Modifiers::ALT) {
            return false;
        }
        let Some(c) = key.key.char().map(|c| c.to_ascii_lowercase()) else {
            return false;
        };
        match c {
            'b' => {
                if self.editable {
                    self.toggle_bold();
                }
                true
            }
            'i' => {
                if self.editable {
                    self.toggle_italic();
                }
                true
            }
            'u' => {
                if self.editable {
                    self.toggle_underline();
                }
                true
            }
            'k' => {
                if self.editable {
                    self.apply_link();
                }
                true
            }
            _ => false,
        }
    }

    fn try_auto_close(&self, c: char) -> bool {
        let (open, close): (&str, &str) = match c {
            '*' => ("**", "**"),
            '_' => ("_", "_"),
            '`' => ("`", "`"),
            _ => return false,
        };
        if !self.editable {
            return false;
        }
        let source = self.source.clone();
        enum Role {
            Hop(usize),
            Wrap(usize, usize, usize),
            Plain(usize),
        }
        let plan = self.with_state(|s| {
            source.ensure(&mut s.engine);
            let len = s.engine.len_chars();
            let snapshot = s.engine.styled_buffer().to_string();
            let snap_chars: Vec<char> = snapshot.chars().collect();
            let mut roles: Vec<Role> = Vec::with_capacity(s.carets.len());
            for (i, caret) in s.carets.iter().enumerate() {
                if let Some((a, b)) = caret.selected() {
                    let (a, b) = (a.min(len), b.min(len));
                    if a < b {
                        roles.push(Role::Wrap(i, a, b));
                        continue;
                    }
                }
                let p = caret.offset.min(len);

                let tail: String = snap_chars[p..].iter().collect();
                if tail.starts_with(close) {
                    roles.push(Role::Hop(i));
                    continue;
                }
                match Self::word_at(&s.engine, &snapshot, p) {
                    Some((w0, w1)) => roles.push(Role::Wrap(i, w0, w1)),
                    None => roles.push(Role::Plain(i)),
                }
            }
            let _ = open;
            Some(roles)
        });
        let Some(roles) = plan else { return false };
        let has_wrap = roles.iter().any(|r| matches!(r, Role::Wrap(..)));
        let has_hop = roles.iter().any(|r| matches!(r, Role::Hop(_)));
        if !has_wrap && !has_hop {
            return false;
        }
        let single = c.to_string();
        self.with_state(|s| {
            source.ensure(&mut s.engine);
            let len = s.engine.len_chars();
            if !has_wrap {
                let snapshot = s.engine.styled_buffer().to_string();
                let snap_chars: Vec<char> = snapshot.chars().collect();
                for (i, caret) in s.carets.iter_mut().enumerate() {
                    let still_hop = roles.iter().any(|r| matches!(r, Role::Hop(j) if *j == i));
                    if !still_hop {
                        continue;
                    }
                    let p = caret.offset.min(len);
                    let tail: String = snap_chars[p..].iter().collect();
                    let step = if tail.starts_with(close) {
                        close.chars().count()
                    } else {
                        0
                    };
                    caret.offset = (p + step).min(len);
                    caret.anchor = None;
                }
                Self::reset_escalate(s);
                s.goal_x = None;
                return;
            }

            let mut pairs: Vec<(usize, String, usize)> = Vec::new();

            let mut bases: Vec<(usize, usize)> = Vec::with_capacity(roles.len());
            for r in roles.iter() {
                match r {
                    Role::Wrap(i, a, b) => {
                        let ab = s.engine.to_bytes((*a).min(len));
                        let bb = s.engine.to_bytes((*b).min(len));
                        pairs.push((ab, open.to_string(), *a));
                        pairs.push((bb, close.to_string(), *b));
                        bases.push((*i, *b));
                    }
                    Role::Plain(i) => {
                        let c = s.carets[*i].offset.min(len);
                        pairs.push((s.engine.to_bytes(c), single.clone(), c));
                        bases.push((*i, c));
                    }
                    Role::Hop(i) => {
                        bases.push((*i, s.carets[*i].offset.min(len)));
                    }
                }
            }

            let added: usize = pairs.iter().map(|(_, t, _)| t.chars().count()).sum();
            if let Some(max) = self.max_length
                && s.engine.len_chars() + added > max
            {
                return;
            }

            let mut byte_to_char: std::collections::HashMap<usize, usize> =
                std::collections::HashMap::new();
            for (pb, _, pc) in pairs.iter() {
                byte_to_char.entry(*pb).or_insert(*pc);
            }

            let mut ordered = pairs.clone();
            ordered.sort_by_key(|b| std::cmp::Reverse(b.0));

            let mut applied: Vec<(usize, usize)> = Vec::new();
            let mut used: std::collections::HashSet<usize> = std::collections::HashSet::new();
            let mut k = 0usize;
            while k < ordered.len() {
                let mut j = k + 1;
                while j < ordered.len() && ordered[j].1 == ordered[k].1 {
                    j += 1;
                }
                let mut pts: Vec<usize> = ordered[k..j].iter().map(|(p, _, _)| *p).collect();
                pts.sort_unstable();
                pts.dedup();
                let fresh: Vec<usize> = pts.into_iter().filter(|p| used.insert(*p)).collect();
                if !fresh.is_empty() {
                    let text = ordered[k].1.clone();
                    let clen = text.chars().count();
                    for p in fresh.iter() {
                        if let Some(&pc) = byte_to_char.get(p) {
                            applied.push((pc, clen));
                        }
                    }
                    s.engine.insert_at_many(fresh, &text);
                }
                k = j;
            }
            if let Some(max) = self.max_lines {
                s.engine.enforce_max_lines(max);
            }
            source.commit(&mut s.engine);

            applied.sort_unstable();
            let new_len = s.engine.len_chars();
            for (i, base_ch) in bases {
                let add: usize = applied
                    .iter()
                    .filter(|(pc, _)| *pc <= base_ch)
                    .map(|(_, l)| l)
                    .sum();
                s.carets[i].offset = (base_ch + add).min(new_len);
                s.carets[i].anchor = None;
            }
            dedupe_carets(&mut s.carets);
            Self::reset_escalate(s);
            s.goal_x = None;
        });
        self.bump();
        true
    }

    fn undo(&self) {
        if !self.editable {
            return;
        }
        let source = self.source.clone();
        let restored = self.with_state(|s| {
            source.ensure(&mut s.engine);
            let text_before = s.engine.styled_buffer().to_string();
            match s.engine.undo() {
                Some((a, b)) => {
                    source.commit(&mut s.engine);
                    if s.engine.styled_buffer().to_string() == text_before {
                        let len = s.engine.len_chars();
                        for c in s.carets.iter_mut() {
                            c.offset = c.offset.min(len);
                            if let Some(an) = c.anchor {
                                c.anchor = Some(an.min(len));
                                if c.anchor == Some(c.offset) {
                                    c.anchor = None;
                                }
                            }
                        }
                    } else {
                        let at = s.engine.to_chars(a).min(s.engine.len_chars());
                        s.carets = vec![Caret::collapsed(at)];
                        let _ = b;
                    }
                    s.goal_x = None;
                    Self::reset_escalate(s);
                    true
                }
                None => false,
            }
        });
        if restored {
            self.bump();
        }
    }

    fn redo(&self) {
        if !self.editable {
            return;
        }
        let source = self.source.clone();
        let restored = self.with_state(|s| {
            source.ensure(&mut s.engine);
            let text_before = s.engine.styled_buffer().to_string();
            match s.engine.redo() {
                Some((a, b)) => {
                    source.commit(&mut s.engine);
                    if s.engine.styled_buffer().to_string() == text_before {
                        let len = s.engine.len_chars();
                        for c in s.carets.iter_mut() {
                            c.offset = c.offset.min(len);
                            if let Some(an) = c.anchor {
                                c.anchor = Some(an.min(len));
                                if c.anchor == Some(c.offset) {
                                    c.anchor = None;
                                }
                            }
                        }
                        let _ = a;
                    } else {
                        let at = s.engine.to_chars(b).min(s.engine.len_chars());
                        s.carets = vec![Caret::collapsed(at)];
                    }
                    s.goal_x = None;
                    Self::reset_escalate(s);
                    true
                }
                None => false,
            }
        });
        if restored {
            self.bump();
        }
    }

    fn escalate_select(&self) {
        let source = self.source.clone();
        self.with_state(|s| {
            source.ensure(&mut s.engine);
            let len = s.engine.len_chars();
            if s.carets.is_empty() {
                s.carets.push(Caret::default());
            }
            let snapshot = s.engine.styled_buffer().to_string();
            let main = s.carets[0];
            let cur_sel = main.selected().map(|(a, b)| (a.min(len), b.min(len)));
            let (ns, ne, level) = match s.escalate {
                0 => match cur_sel {
                    None => match Self::word_at(&s.engine, &snapshot, main.offset) {
                        Some((w0, w1)) => (w0, w1, 1),

                        None => {
                            let (l0, l1) = Self::line_at(&s.engine, len, main.offset);
                            (l0, l1, 2)
                        }
                    },
                    Some((a, b)) if a == b => match Self::word_at(&s.engine, &snapshot, a) {
                        Some((w0, w1)) => (w0, w1, 1),
                        None => {
                            let (l0, l1) = Self::line_at(&s.engine, len, a);
                            (l0, l1, 2)
                        }
                    },

                    Some(_) => (0, len, 3),
                },
                1 => {
                    let (l0, l1) = Self::line_at(&s.engine, len, s.escalate_base);
                    (l0, l1, 2)
                }
                _ => (0, len, 3),
            };
            let base = if s.escalate == 0 {
                main.offset.min(len)
            } else {
                s.escalate_base
            };
            s.carets = vec![Caret {
                offset: ne,
                anchor: if ns == ne { None } else { Some(ns) },
            }];
            s.escalate = level;
            s.escalate_base = base;
            s.goal_x = None;
        });
        self.bump();
    }

    fn handle_click(&self, caret: usize, extend: bool, alt: bool, now_ms: u128) {
        const TRIPLE_MS: u128 = 500;
        let source = self.source.clone();
        if alt {
            self.with_state(|s| {
                source.ensure(&mut s.engine);
                let caret = caret.min(s.engine.len_chars());
                if let Some(idx) = s.carets.iter().position(|c| c.offset == caret) {
                    if s.carets.len() > 1 {
                        s.carets.remove(idx);
                        if let Some(ad) = s.alt_drag {
                            if ad == idx {
                                s.alt_drag = None;
                            } else if ad > idx {
                                s.alt_drag = Some(ad - 1);
                            }
                        }
                    } else {
                        s.carets[0].move_to(caret, extend);
                    }
                } else {
                    s.carets.push(Caret::collapsed(caret));
                    s.alt_drag = Some(s.carets.len() - 1);
                }
                Self::reset_escalate(s);
                s.goal_x = None;
            });
            self.bump();
            return;
        }
        if extend {
            self.set_caret_extend(caret, true);
            return;
        }
        self.with_state(|s| {
            source.ensure(&mut s.engine);
            let len = s.engine.len_chars();
            let caret = caret.min(len);
            let chained = match s.last_click_ms {
                Some(t) => now_ms.saturating_sub(t) <= TRIPLE_MS,
                None => false,
            };

            s.click_count = if chained { s.click_count + 1 } else { 1 };
            s.last_click_ms = Some(now_ms);
            s.last_click_pos = caret;
            s.alt_drag = None;
            match s.click_count % 3 {
                1 => {
                    s.carets = vec![Caret::collapsed(caret)];
                    Self::reset_escalate(s);
                }
                2 => {
                    let snapshot = s.engine.styled_buffer().to_string();
                    match Self::word_at(&s.engine, &snapshot, caret) {
                        Some((w0, w1)) => {
                            s.carets = vec![Caret {
                                offset: w1,
                                anchor: Some(w0),
                            }];
                            s.escalate = 1;
                            s.escalate_base = caret;
                        }
                        None => {
                            s.carets = vec![Caret::collapsed(caret)];
                            Self::reset_escalate(s);
                        }
                    }
                }
                _ => {
                    let (l0, l1) = Self::line_at(&s.engine, len, caret);
                    s.carets = vec![Caret {
                        offset: l1,
                        anchor: if l0 == l1 { None } else { Some(l0) },
                    }];
                    s.escalate = 2;
                    s.escalate_base = caret;
                }
            }
            s.goal_x = None;
        });
        self.bump();
    }

    fn on_key(&self, key: &KeyEvent, ctx: &mut EventCtx) {
        let extend = key.modifiers.contains(Modifiers::SHIFT);
        let ctrl =
            key.modifiers.contains(Modifiers::CONTROL) || key.modifiers.contains(Modifiers::SUPER);
        let alt = key.modifiers.contains(Modifiers::ALT);
        match key.key {
            Key::BACKSPACE => {
                if !ctrl {
                    self.backspace(false);
                } else {
                    self.backspace(true);
                }
                self.ensure_caret_visible_ctx(ctx);
                ctx.handled();
            }
            Key::DELETE => {
                if !ctrl {
                    self.delete(false);
                } else {
                    self.delete(true);
                }
                self.ensure_caret_visible_ctx(ctx);
                ctx.handled();
            }
            Key::ARROW_LEFT => {
                if ctrl && !alt {
                    self.move_word(-1, extend);
                } else {
                    self.move_caret(-1, extend);
                }
                self.ensure_caret_visible_ctx(ctx);
                ctx.handled();
            }
            Key::ARROW_RIGHT => {
                if ctrl && !alt {
                    self.move_word(1, extend);
                } else {
                    self.move_caret(1, extend);
                }
                self.ensure_caret_visible_ctx(ctx);
                ctx.handled();
            }
            Key::ARROW_UP => {
                self.move_vertical_ctx(-1, extend, ctx);
                self.ensure_caret_visible_ctx(ctx);
                ctx.handled();
            }
            Key::ARROW_DOWN => {
                self.move_vertical_ctx(1, extend, ctx);
                self.ensure_caret_visible_ctx(ctx);
                ctx.handled();
            }
            Key::HOME => {
                self.move_home(extend, ctrl);
                self.ensure_caret_visible_ctx(ctx);
                ctx.handled();
            }
            Key::END => {
                self.move_end(extend, ctrl);
                self.ensure_caret_visible_ctx(ctx);
                ctx.handled();
            }
            Key::PAGE_UP => {
                let vp = self.with_state(|s| s.viewport);
                self.move_page(-1, extend, vp.height, self.line_height());
                self.ensure_caret_visible_ctx(ctx);
                ctx.handled();
            }
            Key::PAGE_DOWN => {
                let vp = self.with_state(|s| s.viewport);
                self.move_page(1, extend, vp.height, self.line_height());
                self.ensure_caret_visible_ctx(ctx);
                ctx.handled();
            }
            Key::ENTER => {
                self.on_enter();
                self.ensure_caret_visible_ctx(ctx);
                ctx.handled();
            }
            Key::TAB => {
                self.on_tab(extend);
                self.ensure_caret_visible_ctx(ctx);
                ctx.handled();
            }
            Key::ESCAPE => {
                let had = self.with_state(|s| {
                    let had = s.carets.iter().any(|c| c.anchor.is_some())
                        || s.carets.len() > 1
                        || !s.preedit.is_empty();
                    s.carets.truncate(1);
                    if s.carets.is_empty() {
                        s.carets.push(Caret::default());
                    }
                    s.carets[0].anchor = None;
                    s.preedit.clear();
                    s.goal_x = None;
                    Self::reset_escalate(s);
                    had
                });
                if had {
                    self.bump();
                }
                ctx.handled();
            }
            Key::SPACE => {
                if extend || ctrl || alt {
                    if extend && !ctrl && !alt {
                        self.move_caret(1, true);
                        self.ensure_caret_visible_ctx(ctx);
                    }
                    ctx.handled();
                } else if self.editable {
                    self.insert_str(" ");
                    self.ensure_caret_visible_ctx(ctx);
                    ctx.handled();
                } else {
                    ctx.handled();
                }
            }
            other => {
                if let Some(c) = other.char()
                    && !c.is_control()
                {
                    if ctrl || alt {
                        ctx.handled();
                        return;
                    }
                    if self.editable {
                        if !self.try_auto_close(c) {
                            let mut s = String::new();
                            s.push(c);
                            self.insert_str(&s);
                        }
                        self.ensure_caret_visible_ctx(ctx);
                    }
                    ctx.handled();
                } else {
                    ctx.handled();
                }
            }
        }
    }

    fn move_vertical_ctx(&self, dir: isize, extend: bool, ctx: &mut EventCtx) {
        let chars = self.chars();
        let starts = self.starts(&chars);

        let mains: Vec<usize> =
            self.with_state(|s| s.carets.iter().map(|c| c.offset.min(chars.len())).collect());
        let rows = starts.len();
        let theme = ctx.environment.get::<Theme>().unwrap_or_else(Theme::dark);
        let size = self.resolved_font_size(&theme);
        let weight = self.font_weight();
        let text = self.source.get_text();
        let lines = split_lines(&text);
        let main_goal = self.with_state(|s| s.goal_x);
        let mut main_goal_next = main_goal;
        let mut nexts: Vec<Option<usize>> = Vec::with_capacity(mains.len());
        for (idx, caret) in mains.iter().enumerate() {
            let (line, col) = line_col_of(&chars, &starts, *caret);
            let target = if dir < 0 {
                line.saturating_sub(dir.unsigned_abs())
            } else {
                (line + dir as usize).min(rows.saturating_sub(1))
            };
            if target == line {
                nexts.push(None);
                continue;
            }
            let bounds_cur =
                ctx.caret_positions(lines.get(line).copied().unwrap_or(""), size, weight);

            let caret_goal = if idx == 0 {
                if let Some(g) = main_goal {
                    g
                } else {
                    let g = bounds_cur
                        .get(col.min(bounds_cur.len().saturating_sub(1)))
                        .copied()
                        .unwrap_or(0.0);
                    main_goal_next = Some(g);
                    g
                }
            } else {
                bounds_cur
                    .get(col.min(bounds_cur.len().saturating_sub(1)))
                    .copied()
                    .unwrap_or(0.0)
            };
            let bounds_target =
                ctx.caret_positions(lines.get(target).copied().unwrap_or(""), size, weight);
            let target_col = closest_index(&bounds_target, caret_goal);
            nexts.push(Some(
                caret_of_line_col(&chars, &starts, target, target_col).min(chars.len()),
            ));
        }
        if nexts.iter().all(|n| n.is_none()) {
            if mains.len() == 1 && !extend {
                self.with_state(|s| s.goal_x = None);
            }
            return;
        }
        self.with_state(|s| {
            for (c, next) in s.carets.iter_mut().zip(nexts.iter()) {
                if let Some(next) = next {
                    c.move_to(*next, extend);
                }
            }
            s.goal_x = main_goal_next;
            Self::reset_escalate(s);
        });
        self.bump();
    }

    fn ensure_caret_visible_ctx(&self, ctx: &mut EventCtx) {
        let theme = ctx.environment.get::<Theme>().unwrap_or_else(Theme::dark);
        let size = self.resolved_font_size(&theme);
        let weight = self.font_weight();
        let line_h = self.resolved_line_height(&theme);
        let text = self.source.get_text();
        let chars: Vec<char> = text.chars().collect();
        let starts = line_starts(&chars);
        let caret = self.caret().min(chars.len());
        let (line, col) = line_col_of(&chars, &starts, caret);
        let lines = split_lines(&text);
        let line_str = lines.get(line).copied().unwrap_or("");
        let bounds = ctx.caret_positions(line_str, size, weight);
        let x = bounds
            .get(col.min(bounds.len().saturating_sub(1)))
            .copied()
            .unwrap_or(0.0);
        self.ensure_visible(x, line, line_h);
    }

    fn ensure_visible(&self, caret_x: f32, line: usize, line_h: f32) {
        let changed = self.with_state(|s| {
            if s.viewport.is_empty() || line_h <= 0.0 {
                return false;
            }
            let before = s.scroll;
            apply_ensure_visible(s, self.pad_x, self.pad_y, caret_x, line, line_h);
            s.scroll != before
        });
        if changed {
            let s = self.with_state(|s| s.scroll);
            if let Some(f) = &self.on_scroll {
                f(s);
            }
            self.bump();
        }
    }

    fn on_ime(&self, ime: &ImeEvent) {
        if !self.editable {
            if ime == &ImeEvent::Disabled {
                let had = self.with_state(|s| {
                    let had = !s.preedit.is_empty();
                    s.preedit.clear();
                    had
                });
                if had {
                    self.bump();
                }
            }
            return;
        }
        match ime {
            ImeEvent::Enabled => {}
            ImeEvent::Preedit { value, .. } => {
                self.with_state(|s| s.preedit = value.clone());
                self.bump();
            }
            ImeEvent::Commit(value) => {
                self.with_state(|s| s.preedit.clear());
                self.insert_str(&sanitize_multiline(value));
            }
            ImeEvent::Disabled => {
                let had = self.with_state(|s| {
                    let had = !s.preedit.is_empty();
                    s.preedit.clear();
                    had
                });
                if had {
                    self.bump();
                }
            }

            _ => {}
        }
    }

    fn on_clipboard_key(&self, key: &KeyEvent, ctx: &mut EventCtx) -> bool {
        let cmd =
            key.modifiers.contains(Modifiers::CONTROL) || key.modifiers.contains(Modifiers::SUPER);
        if !cmd {
            return false;
        }
        let Some(c) = key.key.char().map(|c| c.to_ascii_lowercase()) else {
            return false;
        };

        if c == 'z' && !key.modifiers.contains(Modifiers::SHIFT) {
            self.undo();
            return true;
        }
        if c == 'y' || (c == 'z' && key.modifiers.contains(Modifiers::SHIFT)) {
            self.redo();
            return true;
        }
        if !matches!(c, 'c' | 'x' | 'v' | 'a') {
            return false;
        }
        if c == 'a' {
            self.escalate_select();
            return true;
        }
        let Some(clip) = ctx.environment.get::<SharedClipboard>() else {
            return true;
        };
        match c {
            'c' => {
                if let Some(sel) = self.selected_text() {
                    clip.0.set_text(&sel);
                } else {
                    clip.0.set_text(&self.source.get_text());
                }
            }
            'x' => {
                if !self.editable {
                    return true;
                }
                if let Some(sel) = self.selected_text() {
                    clip.0.set_text(&sel);
                    self.delete_selection();
                } else {
                    clip.0.set_text(&self.source.get_text());
                    self.source.set_text(String::new());
                    self.with_state(|s| {
                        s.carets = vec![Caret::collapsed(0)];
                        s.goal_x = None;
                    });
                    self.bump();
                }
            }
            _ => {
                if !self.editable {
                    return true;
                }
                if let Some(paste) = clip.0.get_text() {
                    self.insert_str(&sanitize_multiline(&paste));
                }
            }
        }
        true
    }

    fn hit_test_caret(&self, bounds: &[f32], x: f32) -> usize {
        if bounds.is_empty() {
            return 0;
        }
        if x >= bounds[bounds.len() - 1] {
            return bounds.len() - 1;
        }
        if x <= 0.0 {
            return 0;
        }
        closest_index(bounds, x)
    }

    fn caret_from_point(
        &self,
        lines: &[&str],
        chars: &[char],
        starts: &[usize],
        line: usize,
        bounds: &[f32],
        cx: f32,
    ) -> usize {
        let _ = lines;
        let col = self.hit_test_caret(bounds, cx);
        caret_of_line_col(chars, starts, line, col)
    }
}

fn closest_index(bounds: &[f32], x: f32) -> usize {
    let mut best = 0usize;
    let mut best_d = f32::MAX;
    for (i, &b) in bounds.iter().enumerate() {
        let d = (b - x).abs();
        if d < best_d {
            best_d = d;
            best = i;
        }
    }
    best
}

fn clamp_scroll(
    scroll: Vec2,
    content_w: f32,
    content_h: f32,
    viewport: Size,
    pad: (f32, f32),
) -> Vec2 {
    let max_x = (content_w + pad.0 * 2.0 - viewport.width).max(0.0);
    let max_y = (content_h + pad.1 * 2.0 - viewport.height).max(0.0);
    Vec2::new(
        if viewport.width <= 0.0 {
            scroll.x.max(0.0)
        } else {
            scroll.x.clamp(0.0, max_x)
        },
        if viewport.height <= 0.0 {
            scroll.y.max(0.0)
        } else {
            scroll.y.clamp(0.0, max_y)
        },
    )
}

fn apply_ensure_visible(
    s: &mut AreaState,
    pad_x: f32,
    pad_y: f32,
    caret_x: f32,
    line: usize,
    line_h: f32,
) {
    let vp = s.viewport;
    if vp.is_empty() {
        return;
    }
    let cx = pad_x + caret_x;
    if cx < s.scroll.x + SCROLL_MARGIN {
        s.scroll.x = (cx - SCROLL_MARGIN).max(0.0);
    } else if cx + CARET_W + SCROLL_MARGIN > s.scroll.x + vp.width {
        s.scroll.x = (cx + CARET_W + SCROLL_MARGIN - vp.width).max(0.0);
    }
    let top = pad_y + line as f32 * line_h;
    if top < s.scroll.y {
        s.scroll.y = top.max(0.0);
    } else if top + line_h > s.scroll.y + vp.height {
        s.scroll.y = (top + line_h - vp.height).max(0.0);
    }
}

impl View for TextArea {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn is_leaf(&self) -> bool {
        true
    }

    fn focusable(&self) -> bool {
        true
    }

    fn accepts_tab(&self) -> bool {
        self.accept_tab
    }

    fn has_visual_state(&self) -> bool {
        true
    }

    fn clip_self(&self) -> bool {
        true
    }

    fn clip_radius(&self) -> f32 {
        self.radius
            .or_else(|| self.appearance.and_then(|s| s.radius))
            .unwrap_or(3.0)
            .max(0.0)
    }

    fn reuse_state(&self, old: &Self) {
        if !Rc::ptr_eq(&self.state, &old.state) {
            *self.state.borrow_mut() = old.state.borrow().clone();
        }
        self.revision.set(old.revision.get_untracked());
    }

    fn keyed_children(&self) -> Option<Vec<(u64, AnyView)>> {
        self.source.track();
        let _ = self.revision.get();

        Some(Vec::new())
    }

    fn cursor_icon(&self, _state: VisualState) -> CursorIcon {
        CursorIcon::Text
    }

    fn layout_style(&self) -> LayoutStyle {
        let mut s = self.style;
        s.margin = self.margin;
        s
    }

    fn intrinsic_size(&self, ctx: &mut LayoutCtx) -> Option<Size> {
        let size = self
            .font_size
            .or_else(|| self.appearance.and_then(|s| s.font_size))
            .unwrap_or(14.0);
        let weight = self.font_weight();
        let line_h = self.line_height();
        let text = self.source.get_text();
        let lines = split_lines(&text);
        let mut w: f32 = 0.0;
        for line in &lines {
            w = w.max(ctx.text_width_weighted(line, size, weight));
        }
        if !self.placeholder.is_empty() && text.is_empty() {
            for line in split_lines(&self.placeholder) {
                w = w.max(ctx.text_width_weighted(line, size, weight));
            }
        }
        let rows = lines.len().max(self.min_lines).max(1) as f32;
        Some(Size::new(
            w + self.pad_x * 2.0,
            rows * line_h + self.pad_y * 2.0,
        ))
    }

    fn on_event(&self, event: &Event, ctx: &mut EventCtx) {
        if ctx.phase != EventPhase::Target {
            if let Event::Pointer(PointerEvent::Wheel {
                delta, modifiers, ..
            }) = event
                && matches!(ctx.phase, EventPhase::Bubble)
            {
                self.on_wheel(*delta, *modifiers, ctx);
            }
            return;
        }
        match event {
            Event::Pointer(PointerEvent::Down {
                button: PointerButton::Left,
                pos,
                modifiers,
                ..
            }) => {
                self.begin_session();
                let theme = ctx.environment.get::<Theme>().unwrap_or_else(Theme::dark);
                let size = self.resolved_font_size(&theme);
                let weight = self.font_weight();
                let line_h = self.resolved_line_height(&theme);
                let text = self.source.get_text();
                let chars: Vec<char> = text.chars().collect();
                let starts = line_starts(&chars);
                let lines = split_lines(&text);
                let scroll = self.with_state(|s| s.scroll);
                let cx = pos.x - ctx.rect.left() - self.pad_x + scroll.x;
                let cy = pos.y - ctx.rect.top() - self.pad_y + scroll.y;
                let line = Self::row_at_y(line_h, lines.len(), cy);
                let bounds =
                    ctx.caret_positions(lines.get(line).copied().unwrap_or(""), size, weight);
                let caret = self.caret_from_point(&lines, &chars, &starts, line, &bounds, cx);
                let extend = modifiers.contains(Modifiers::SHIFT);
                let alt = modifiers.contains(Modifiers::ALT);
                self.handle_click(caret, extend, alt, now_ms());
                self.ensure_caret_visible_ctx(ctx);
                self.bump();
                ctx.request_focus();

                ctx.capture_pointer();
                ctx.handled();
            }
            Event::Pointer(PointerEvent::Move { pos, .. }) => {
                if ctx.state.pressed {
                    let theme = ctx.environment.get::<Theme>().unwrap_or_else(Theme::dark);
                    let size = self.resolved_font_size(&theme);
                    let weight = self.font_weight();
                    let line_h = self.resolved_line_height(&theme);
                    let text = self.source.get_text();
                    let chars: Vec<char> = text.chars().collect();
                    let starts = line_starts(&chars);
                    let lines = split_lines(&text);
                    let scroll = self.with_state(|s| s.scroll);
                    let cx = pos.x - ctx.rect.left() - self.pad_x + scroll.x;
                    let cy = pos.y - ctx.rect.top() - self.pad_y + scroll.y;
                    let line = Self::row_at_y(line_h, lines.len(), cy);
                    let bounds =
                        ctx.caret_positions(lines.get(line).copied().unwrap_or(""), size, weight);
                    let caret = self.caret_from_point(&lines, &chars, &starts, line, &bounds, cx);
                    let alt_drag = self.with_state(|s| s.alt_drag);
                    match alt_drag {
                        Some(idx) => self.with_state(|s| {
                            if let Some(c) = s.carets.get_mut(idx) {
                                c.move_to(caret, true);
                            }
                            s.goal_x = None;
                        }),
                        None => self.set_caret_extend(caret, true),
                    }
                    self.ensure_caret_visible_ctx(ctx);
                    self.bump();
                    ctx.handled();
                }
            }
            Event::Pointer(PointerEvent::Wheel {
                delta, modifiers, ..
            }) => {
                self.on_wheel(*delta, *modifiers, ctx);
            }
            Event::Key(key) => {
                if self.on_style_shortcut(key) {
                    self.ensure_caret_visible_ctx(ctx);
                    ctx.handled();
                } else if !self.on_clipboard_key(key, ctx) {
                    self.on_key(key, ctx);
                } else {
                    self.ensure_caret_visible_ctx(ctx);
                    ctx.handled();
                }
            }
            Event::Ime(ime) => {
                self.on_ime(ime);
                ctx.handled();
            }
            _ => {}
        }
    }

    fn paint_self(&self, ctx: &mut PaintCtx) {
        self.note_focus(ctx.state.focused);
        let rect = ctx.rect;
        if rect.size.width <= 0.0 || rect.size.height <= 0.0 {
            return;
        }
        let theme = ctx.theme();
        let size = self.resolved_font_size(&theme);
        let weight = self.font_weight();
        let line_h = self.resolved_line_height(&theme);
        if line_h <= 0.0 {
            return;
        }
        let text_color = self.resolved_text_color(&theme);
        let placeholder_color = self.resolved_placeholder_color(&theme);
        let background = self.resolved_background(&theme);
        let radius = self.resolved_radius(&theme);

        self.with_state(|s| s.viewport = rect.size);

        let text = self.source.get_text();
        let chars: Vec<char> = text.chars().collect();
        let starts = line_starts(&chars);
        let lines = split_lines(&text);
        let rows = lines.len();

        let mut content_w: f32 = 0.0;
        for line in &lines {
            content_w = content_w.max(ctx.text_width_weighted(line, size, weight));
        }
        let content_h = rows as f32 * line_h;

        let (caret, anchor, preedit) = self.with_state(|s| {
            let main = Self::main_of(s);
            (main.offset, main.anchor, s.preedit.clone())
        });
        let caret = caret.min(chars.len());
        let (caret_line, caret_col) = line_col_of(&chars, &starts, caret);

        let caret_bounds =
            ctx.caret_positions(lines.get(caret_line).copied().unwrap_or(""), size, weight);
        let caret_x = caret_bounds
            .get(caret_col.min(caret_bounds.len().saturating_sub(1)))
            .copied()
            .unwrap_or(0.0);
        self.with_state(|s| {
            s.content = Size::new(content_w, content_h);
            s.scroll = clamp_scroll(
                s.scroll,
                content_w,
                content_h,
                rect.size,
                (self.pad_x, self.pad_y),
            );
            apply_ensure_visible(s, self.pad_x, self.pad_y, caret_x, caret_line, line_h);
            s.scroll = clamp_scroll(
                s.scroll,
                content_w,
                content_h,
                rect.size,
                (self.pad_x, self.pad_y),
            );
        });
        let scroll = self.with_state(|s| s.scroll);

        if background.a > 0.0 {
            match self.resolved_border(&theme) {
                Some((w, color)) => {
                    ctx.bordered_quad(rect, background, radius, w, color);
                }
                None => {
                    ctx.quad(rect, background, radius);
                }
            }
        }

        let ox = rect.left() + self.pad_x - scroll.x;
        let oy = rect.top() + self.pad_y - scroll.y;
        let baseline_at =
            |row: usize| -> f32 { oy + row as f32 * line_h + line_h * 0.5 + size * 0.35 };
        let top_at = |row: usize| -> f32 { oy + row as f32 * line_h };

        let first_row = ((scroll.y - self.pad_y) / line_h).floor().max(0.0) as usize;
        let last_row = (((scroll.y - self.pad_y + rect.size.height) / line_h)
            .ceil()
            .max(0.0) as usize)
            .min(rows.saturating_sub(1));

        if ctx.state.focused && self.highlight_current_line && anchor.is_none() {
            let color = self.current_line.unwrap_or(Color {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 0.05,
            });
            if color.a > 0.0 && caret_line >= first_row && caret_line <= last_row {
                ctx.quad(
                    Rect::from_xywh(rect.left(), top_at(caret_line), rect.size.width, line_h),
                    color,
                    0.0,
                );
            }
        }

        if let Some(a) = anchor {
            let (s0, s1) = (a.min(caret).min(chars.len()), a.max(caret).min(chars.len()));
            if s1 > s0 {
                for row in first_row..=last_row.min(rows.saturating_sub(1)) {
                    let ls = starts[row];
                    let le = ls + line_len(&chars, &starts, row);
                    let line_end_nl = if row + 1 < rows { le + 1 } else { le };
                    if s0 >= line_end_nl || s1 <= ls {
                        continue;
                    }
                    let bounds =
                        ctx.caret_positions(lines.get(row).copied().unwrap_or(""), size, weight);
                    let col0 = s0.saturating_sub(ls).min(bounds.len().saturating_sub(1));
                    let x0 = bounds.get(col0).copied().unwrap_or(0.0);
                    let x1 = if s1 > le {
                        (scroll.x + rect.size.width - self.pad_x).max(x0 + 4.0)
                    } else {
                        let col1 = s1.saturating_sub(ls).min(bounds.len().saturating_sub(1));
                        bounds.get(col1).copied().unwrap_or(x0)
                    };
                    if x1 > x0 {
                        ctx.quad(
                            Rect::from_xywh(ox + x0, top_at(row), x1 - x0, line_h),
                            theme.accent_pressed,
                            0.0,
                        );
                    }
                }
            }
        }

        if chars.is_empty() && preedit.is_empty() && !self.placeholder.is_empty() {
            for (row, line) in split_lines(&self.placeholder).iter().enumerate() {
                if row < first_row || row > last_row {
                    continue;
                }
                ctx.draw_text_weighted(
                    line,
                    (ox, baseline_at(row)),
                    size,
                    weight,
                    placeholder_color,
                );
            }
        } else {
            for row in first_row..=last_row.min(rows.saturating_sub(1)) {
                let line = lines.get(row).copied().unwrap_or("");
                if line.is_empty() {
                    continue;
                }
                if row == caret_line && !preedit.is_empty() && ctx.state.focused {
                    let before: String = line
                        .chars()
                        .take(caret_col.min(line.chars().count()))
                        .collect();
                    let after: String = line
                        .chars()
                        .skip(caret_col.min(line.chars().count()))
                        .collect();
                    let bx = ctx.text_width_weighted(&before, size, weight);
                    ctx.draw_text_weighted(
                        &before,
                        (ox, baseline_at(row)),
                        size,
                        weight,
                        text_color,
                    );
                    ctx.draw_text_weighted(
                        &preedit,
                        (ox + bx, baseline_at(row)),
                        size,
                        weight,
                        text_color,
                    );
                    let pw = ctx.text_width_weighted(&preedit, size, weight);
                    ctx.draw_text_weighted(
                        &after,
                        (ox + bx + pw, baseline_at(row)),
                        size,
                        weight,
                        text_color,
                    );
                    ctx.quad(
                        Rect::from_xywh(ox + bx, top_at(row) + line_h - 2.0, pw.max(1.0), 1.5),
                        text_color,
                        0.0,
                    );
                } else {
                    ctx.draw_text_weighted(line, (ox, baseline_at(row)), size, weight, text_color);
                }
            }
        }

        if ctx.state.focused {
            let mut dx = caret_x;
            if !preedit.is_empty() && anchor.is_none() {
                let line = lines.get(caret_line).copied().unwrap_or("");
                let before: String = line
                    .chars()
                    .take(caret_col.min(line.chars().count()))
                    .collect();
                dx += ctx.text_width_weighted(&preedit, size, weight);
                let _ = before;
            }
            ctx.quad(
                Rect::from_xywh(
                    ox + dx,
                    baseline_at(caret_line) - size * 0.8,
                    CARET_W,
                    size * 1.05,
                ),
                text_color,
                0.0,
            );
        }
    }
}

impl TextArea {
    fn on_wheel(&self, delta: Vec2, modifiers: Modifiers, ctx: &mut EventCtx) {
        let (mut dx, mut dy) = (delta.x, delta.y);
        if modifiers.contains(Modifiers::SHIFT) {
            std::mem::swap(&mut dx, &mut dy);
        }
        let (cur, viewport, content) = self.with_state(|s| (s.scroll, s.viewport, s.content));
        let next = clamp_scroll(
            Vec2::new(cur.x - dx, cur.y - dy),
            content.width,
            content.height,
            viewport,
            self.pad(),
        );
        if next != cur {
            self.with_state(|s| s.scroll = next);
            if let Some(f) = &self.on_scroll {
                f(next);
            }
            self.bump();
            ctx.handled();
        }
    }
}

fn sanitize_margin(v: f32) -> f32 {
    if v.is_finite() { v.max(0.0) } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chars_of(s: &str) -> Vec<char> {
        s.chars().collect()
    }

    #[test]
    fn line_starts_cover_empty_and_trailing_newline() {
        assert_eq!(line_starts(&chars_of("")), vec![0]);
        assert_eq!(line_starts(&chars_of("ab")), vec![0]);
        assert_eq!(line_starts(&chars_of("a\nb")), vec![0, 2]);
        assert_eq!(line_starts(&chars_of("a\n")), vec![0, 2]);
        assert_eq!(line_starts(&chars_of("\n")), vec![0, 1]);
    }

    #[test]
    fn caret_roundtrips_through_line_col() {
        let text = "ab\ncde\n\nf";
        let chars = chars_of(text);
        let starts = line_starts(&chars);
        assert_eq!(starts.len(), 4);
        for caret in 0..=chars.len() {
            let (line, col) = line_col_of(&chars, &starts, caret);
            assert_eq!(caret_of_line_col(&chars, &starts, line, col), caret);
        }
        assert_eq!(line_col_of(&chars, &starts, 0), (0, 0));
        assert_eq!(line_col_of(&chars, &starts, 2), (0, 2));
        assert_eq!(line_col_of(&chars, &starts, 3), (1, 0));
        assert_eq!(line_col_of(&chars, &starts, 7), (2, 0));
        assert_eq!(line_col_of(&chars, &starts, 8), (3, 0));
    }

    #[test]
    fn sanitize_multiline_normalizes_newlines() {
        assert_eq!(sanitize_multiline("a\r\nb\rc"), "a\nb\nc");
        assert_eq!(sanitize_multiline("a\x01b\tc"), "ab\tc");
    }

    #[test]
    fn leading_indent_counts_spaces_and_tabs() {
        assert_eq!(leading_indent_len("    foo"), 4);
        assert_eq!(leading_indent_len("\t\tfoo"), 2);
        assert_eq!(leading_indent_len("foo"), 0);
    }
}
