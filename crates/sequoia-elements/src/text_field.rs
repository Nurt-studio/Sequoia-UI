use std::cell::RefCell;
use std::rc::Rc;

use sequoia_core::{Color, Edges, Length, Rect, Size};
use sequoia_input::{
    CursorIcon, Event, EventPhase, ImeEvent, Key, KeyEvent, Modifiers, PointerButton, PointerEvent,
};
use sequoia_layout::{LayoutCtx, LayoutStyle};
use sequoia_reactive::Signal;
use sequoia_style::{TextFieldStyle, Theme, VisualState};
use sequoia_view::{AnyView, EventCtx, FontWeight, PaintCtx, Unit, View};

use crate::buffer::BufferEngine;
use crate::clipboard::SharedClipboard;
use crate::underline;

const HEIGHT: f32 = 26.0;

const CARET_W: f32 = 1.5;

#[derive(Clone, Default)]
struct FieldState {
    caret: usize,

    anchor: Option<usize>,

    bounds: Vec<f32>,

    preedit: String,

    was_focused: bool,

    scroll: f32,

    session_text: Option<String>,

    engine: BufferEngine,
}

pub struct TextField {
    value: Signal<String>,
    placeholder: String,
    max_length: Option<usize>,
    font_size: Option<f32>,
    weight: Option<FontWeight>,
    text_color: Option<Color>,
    line_color: Option<Color>,
    line_width: Option<f32>,
    appearance: Option<TextFieldStyle>,
    placeholder_color: Option<Color>,
    background: Option<Color>,
    border_color: Option<Color>,
    border_width: Option<f32>,
    radius: Option<f32>,
    padding: Option<(f32, f32)>,
    style: LayoutStyle,
    margin: Edges,

    revision: Signal<u32>,

    state: Rc<RefCell<FieldState>>,

    on_edit_start: Option<Rc<dyn Fn()>>,

    on_edit_end: Option<Rc<dyn Fn(String)>>,

    on_submit: Option<Rc<dyn Fn(String)>>,
}

impl TextField {
    pub fn new(value: Signal<String>) -> Self {
        Self {
            value,
            placeholder: String::new(),
            max_length: None,
            font_size: None,
            weight: None,
            text_color: None,
            line_color: None,
            line_width: None,
            appearance: None,
            placeholder_color: None,
            background: None,
            border_color: None,
            border_width: None,
            radius: None,
            padding: None,
            style: LayoutStyle {
                width: Length::Fill(1.0),
                height: Length::Px(HEIGHT),
                ..LayoutStyle::default()
            },
            margin: Edges::ZERO,
            revision: Signal::new(0),
            state: Rc::new(RefCell::new(FieldState::default())),
            on_edit_start: None,
            on_edit_end: None,
            on_submit: None,
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

    pub fn on_edit_start(mut self, f: impl Fn() + 'static) -> Self {
        self.on_edit_start = Some(Rc::new(f));
        self
    }

    pub fn on_edit_end(mut self, f: impl Fn(String) + 'static) -> Self {
        self.on_edit_end = Some(Rc::new(f));
        self
    }

    pub fn on_submit(mut self, f: impl Fn(String) + 'static) -> Self {
        self.on_submit = Some(Rc::new(f));
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

    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color = Some(color);
        self
    }

    pub fn line_color(mut self, color: Color) -> Self {
        self.line_color = Some(color);
        self
    }

    pub fn appearance(mut self, style: TextFieldStyle) -> Self {
        self.appearance = Some(style);
        self
    }

    pub fn placeholder_color(mut self, color: Color) -> Self {
        self.placeholder_color = Some(color);
        self
    }

    pub fn line_width(mut self, width: f32) -> Self {
        self.line_width = Some(width.max(0.0));
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

    pub fn padding(mut self, padding: impl Into<Edges>) -> Self {
        let p = padding.into();
        self.padding = Some(((p.left + p.right) * 0.5, (p.top + p.bottom) * 0.5));
        self
    }

    pub fn padding_xy(mut self, x: f32, y: f32) -> Self {
        self.padding = Some((x.max(0.0), y.max(0.0)));
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

    fn with_state<R>(&self, f: impl FnOnce(&mut FieldState) -> R) -> R {
        f(&mut self.state.borrow_mut())
    }

    fn chars(&self) -> Vec<char> {
        self.value.get_untracked().chars().collect()
    }

    pub fn caret(&self) -> usize {
        self.with_state(|s| s.caret)
    }

    pub fn selection(&self) -> Option<(usize, usize)> {
        self.with_state(|s| {
            s.anchor.map(|a| {
                let c = s.caret;
                (a.min(c), a.max(c))
            })
        })
    }

    fn begin_session(&self) {
        let cur = self.value.get_untracked();
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
        let cur = self.value.get_untracked();
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

    fn stored_bounds(&self) -> Vec<f32> {
        self.with_state(|s| s.bounds.clone())
    }

    fn store_bounds(&self, bounds: Vec<f32>) {
        self.with_state(|s| s.bounds = bounds);
    }

    fn set_caret(&self, caret: usize) {
        let value = self.value.clone();
        self.with_state(|s| {
            s.engine.ensure(&value);
            s.caret = caret.min(s.engine.len_chars());
            s.anchor = None;
        });
    }

    fn set_caret_extend(&self, caret: usize, extend: bool) {
        let value = self.value.clone();
        self.with_state(|s| {
            s.engine.ensure(&value);
            let caret = caret.min(s.engine.len_chars());
            if extend {
                let anchor = s.anchor.unwrap_or(s.caret);
                s.anchor = Some(anchor);
                s.caret = caret;
                if s.anchor == Some(s.caret) {
                    s.anchor = None;
                }
            } else {
                s.caret = caret;
                s.anchor = None;
            }
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

    fn resolved_line_width(&self) -> f32 {
        if let Some(w) = self
            .line_width
            .or_else(|| self.appearance.and_then(|s| s.line_width))
        {
            return w;
        }
        let has_line_color =
            self.line_color.is_some() || self.appearance.and_then(|s| s.line).is_some();
        if has_line_color {
            return underline::LINE_W;
        }
        0.0
    }

    fn resolved_text_color(&self) -> Option<Color> {
        self.text_color
            .or_else(|| self.appearance.and_then(|s| s.text))
    }

    fn resolved_line_color(&self) -> Option<Color> {
        self.line_color
            .or_else(|| self.appearance.and_then(|s| s.line))
    }

    fn resolved_placeholder_color(&self, theme: &Theme) -> Color {
        self.placeholder_color
            .or_else(|| self.appearance.and_then(|s| s.placeholder))
            .unwrap_or(theme.text_secondary)
    }

    fn resolved_pad(&self) -> (f32, f32) {
        self.padding
            .or_else(|| self.appearance.and_then(|s| s.padding))
            .unwrap_or((underline::PAD_X, 0.0))
    }

    fn resolved_background(&self) -> Color {
        self.background
            .or_else(|| self.appearance.and_then(|s| s.background))
            .unwrap_or(Color::TRANSPARENT)
    }

    fn resolved_border(&self, theme: &Theme) -> Option<(f32, Color)> {
        let width_opt = self
            .border_width
            .or_else(|| self.appearance.and_then(|s| s.border_width));
        let color_opt = self
            .border_color
            .or_else(|| self.appearance.and_then(|s| s.border));
        match (width_opt, color_opt) {
            (None, None) => None,
            (Some(w), Some(c)) => {
                let w = w.max(0.0);
                if w <= 0.0 || c.a <= 0.0 {
                    None
                } else {
                    Some((w, c))
                }
            }
            (Some(w), None) => {
                let w = w.max(0.0);
                if w <= 0.0 || theme.border.a <= 0.0 {
                    None
                } else {
                    Some((w, theme.border))
                }
            }
            (None, Some(c)) => {
                if c.a <= 0.0 {
                    None
                } else {
                    Some((1.0, c))
                }
            }
        }
    }

    fn resolved_radius(&self, theme: &Theme) -> f32 {
        self.radius
            .or_else(|| self.appearance.and_then(|s| s.radius))
            .unwrap_or(theme.radius)
            .max(0.0)
    }

    fn approx_border_w(&self) -> f32 {
        if let Some(w) = self
            .border_width
            .or_else(|| self.appearance.and_then(|s| s.border_width))
        {
            return w.max(0.0);
        }
        let has_color =
            self.border_color.is_some() || self.appearance.and_then(|s| s.border).is_some();
        if has_color { 1.0 } else { 0.0 }
    }

    fn caret_at(&self, x: f32, len: usize) -> usize {
        let bounds = self.stored_bounds();
        let Some(last) = bounds.last() else {
            return len;
        };
        if x >= *last {
            return len;
        }
        if x <= 0.0 {
            return 0;
        }
        let mut best = 0;
        let mut best_distance = f32::MAX;
        for (i, offset) in bounds.iter().enumerate() {
            let distance = (offset - x).abs();
            if distance < best_distance {
                best_distance = distance;
                best = i;
            }
        }
        best.min(len)
    }

    fn delete_selection(&self) -> bool {
        let sel = self.selection();
        let Some((a, b)) = sel else { return false };
        if a == b {
            self.with_state(|s| s.anchor = None);
            return false;
        }
        let value = self.value.clone();
        self.with_state(|s| {
            s.engine.ensure(&value);
            let len = s.engine.len_chars();
            let (a, b) = (a.min(len), b.min(len));
            let (ab, bb) = (s.engine.to_bytes(a), s.engine.to_bytes(b));
            s.engine.delete(ab..bb);
            s.engine.commit(&value);
            s.caret = s.engine.to_chars(ab);
            s.anchor = None;
        });
        self.bump();
        true
    }

    pub fn selected_text(&self) -> Option<String> {
        let (a, b) = self.selection()?;
        if a == b {
            return None;
        }
        let value = self.value.clone();
        self.with_state(|s| {
            s.engine.ensure(&value);
            let len = s.engine.len_chars();
            let (a, b) = (a.min(len), b.min(len));
            let text = s
                .engine
                .buffer()
                .substring(s.engine.to_bytes(a)..s.engine.to_bytes(b));
            if text.is_empty() { None } else { Some(text) }
        })
    }

    pub fn select_range(&self, a: usize, b: usize) {
        let value = self.value.clone();
        self.with_state(|s| {
            s.engine.ensure(&value);
            let len = s.engine.len_chars();
            let (a, b) = (a.min(len), b.min(len));
            s.caret = b;
            s.anchor = if a == b { None } else { Some(a) };
        });
        self.bump();
    }

    pub fn clear_selection(&self) -> bool {
        let had = self.with_state(|s| s.anchor.take().is_some());
        if had {
            self.bump();
        }
        had
    }

    fn insert(&self, c: char) {
        self.insert_str(&c.to_string());
    }

    fn insert_str(&self, text: &str) {
        if text.is_empty() {
            return;
        }

        let value = self.value.clone();
        self.with_state(|st| {
            st.engine.ensure(&value);
            let total = st.engine.len_chars();

            let sel_bytes = match st.anchor.map(|a| (a.min(st.caret), a.max(st.caret))) {
                None => None,
                Some((a, b)) if a == b => {
                    st.anchor = None;
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
                    let caret_b = st.engine.to_bytes(st.caret.min(total));
                    st.engine.insert(caret_b, ins);
                    caret_b + ins.len()
                }
            };
            st.engine.commit(&value);
            st.caret = st.engine.to_chars(caret_b).min(st.engine.len_chars());
            st.anchor = None;
        });
        self.bump();
    }

    fn backspace(&self) {
        if self.delete_selection() {
            return;
        }
        let value = self.value.clone();
        self.with_state(|s| {
            s.engine.ensure(&value);
            let caret = s.caret.min(s.engine.len_chars());
            if caret == 0 {
                return;
            }
            let caret_b = s.engine.to_bytes(caret);
            let from_b = sequoia_buffer::prev_grapheme(s.engine.buffer(), caret_b);
            s.engine.delete(from_b..caret_b);
            s.engine.commit(&value);
            s.caret = s.engine.to_chars(from_b);
            s.anchor = None;
        });
        self.bump();
    }

    fn delete(&self) {
        if self.delete_selection() {
            return;
        }
        let value = self.value.clone();
        self.with_state(|s| {
            s.engine.ensure(&value);
            let len = s.engine.len_chars();
            let caret = s.caret.min(len);
            if caret >= len {
                return;
            }
            let caret_b = s.engine.to_bytes(caret);
            let to_b = sequoia_buffer::next_grapheme(s.engine.buffer(), caret_b);
            s.engine.delete(caret_b..to_b);
            s.engine.commit(&value);
            s.caret = s.engine.to_chars(caret_b);
            s.anchor = None;
        });
        self.bump();
    }

    fn move_caret(&self, delta: isize, extend: bool) {
        let value = self.value.clone();
        let next = self.with_state(|s| {
            s.engine.ensure(&value);
            let mut caret_b = s.engine.to_bytes(s.caret.min(s.engine.len_chars()));
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
            s.engine.to_chars(caret_b)
        });
        self.set_caret_extend(next, extend);
        self.bump();
    }

    fn move_home(&self, extend: bool) {
        self.set_caret_extend(0, extend);
        self.bump();
    }

    fn move_end(&self, extend: bool) {
        let value = self.value.clone();
        let end = self.with_state(|s| {
            s.engine.ensure(&value);
            s.engine.len_chars()
        });
        self.set_caret_extend(end, extend);
        self.bump();
    }

    pub fn select_all(&self) {
        let value = self.value.clone();
        self.with_state(|s| {
            s.engine.ensure(&value);
            let len = s.engine.len_chars();
            s.anchor = Some(0);
            s.caret = len;
        });
        self.bump();
    }

    pub fn submit(&self) {
        let Some(cb) = self.on_submit.clone() else {
            return;
        };
        let has_preedit = self.with_state(|s| !s.preedit.is_empty());
        if has_preedit {
            return;
        }
        let cur = self.value.get_untracked();
        self.with_state(|s| {
            s.session_text = Some(cur.clone());
        });
        cb(cur);
    }

    fn undo(&self) {
        let value = self.value.clone();
        let caret = self.with_state(|s| {
            s.engine.ensure(&value);
            match s.engine.undo() {
                Some((a, _)) => {
                    s.engine.commit(&value);
                    Some(s.engine.to_chars(a))
                }
                None => None,
            }
        });
        if let Some(caret) = caret {
            self.with_state(|s| {
                s.caret = caret;
                s.anchor = None;
            });
            self.bump();
        }
    }

    fn redo(&self) {
        let value = self.value.clone();
        let caret = self.with_state(|s| {
            s.engine.ensure(&value);
            match s.engine.redo() {
                Some((_, b)) => {
                    s.engine.commit(&value);
                    Some(s.engine.to_chars(b))
                }
                None => None,
            }
        });
        if let Some(caret) = caret {
            self.with_state(|s| {
                s.caret = caret;
                s.anchor = None;
            });
            self.bump();
        }
    }

    fn on_key(&self, key: &KeyEvent) {
        let extend = key.modifiers.contains(Modifiers::SHIFT);
        let len = self.with_state(|s| {
            s.engine.ensure(&self.value);
            s.engine.len_chars()
        });
        match key.key {
            Key::ENTER => self.submit(),
            Key::BACKSPACE => self.backspace(),
            Key::DELETE => self.delete(),
            Key::ARROW_LEFT => self.move_caret(-1, extend),
            Key::ARROW_RIGHT => self.move_caret(1, extend),
            Key::HOME | Key::PAGE_UP => {
                if extend {
                    self.move_home(true);
                } else {
                    self.set_caret(0);
                    self.bump();
                }
            }
            Key::END | Key::PAGE_DOWN => {
                if extend {
                    self.move_end(true);
                } else {
                    self.set_caret(len);
                    self.bump();
                }
            }
            Key::SPACE => {
                if extend {
                    self.move_caret(1, true);
                } else {
                    self.insert(' ');
                }
            }

            Key::ESCAPE | Key::TAB => {
                let had = self.with_state(|s| {
                    let had = s.anchor.is_some() || !s.preedit.is_empty();
                    s.anchor = None;
                    s.preedit.clear();
                    had
                });
                if had {
                    self.bump();
                }
            }
            other => {
                if let Some(c) = other.char()
                    && !c.is_control()
                {
                    let cmd = key.modifiers.contains(Modifiers::CONTROL)
                        || key.modifiers.contains(Modifiers::SUPER);
                    if cmd {
                        return;
                    }
                    self.insert(c);
                }
            }
        }
    }

    fn on_ime(&self, ime: &ImeEvent) {
        match ime {
            ImeEvent::Enabled => {}
            ImeEvent::Preedit { value, .. } => {
                self.with_state(|s| s.preedit = value.clone());
                self.bump();
            }
            ImeEvent::Commit(value) => {
                self.with_state(|s| s.preedit.clear());
                self.insert_str(value);
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

    fn on_clipboard_key(&self, key: &KeyEvent, ctx: &EventCtx) -> bool {
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
            self.select_all();
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
                    clip.0.set_text(&self.value.get_untracked());
                }
            }
            'x' => {
                if let Some(sel) = self.selected_text() {
                    clip.0.set_text(&sel);
                    self.delete_selection();
                } else {
                    clip.0.set_text(&self.value.get_untracked());
                    self.value.set(String::new());
                    self.set_caret(0);
                    self.bump();
                }
            }
            _ => {
                if let Some(paste) = clip.0.get_text() {
                    let clean: String = paste
                        .chars()
                        .filter_map(|ch| {
                            if ch == '\n' || ch == '\r' {
                                Some(' ')
                            } else if ch.is_control() {
                                None
                            } else {
                                Some(ch)
                            }
                        })
                        .collect();
                    self.insert_str(&clean);
                }
            }
        }
        true
    }
}

impl View for TextField {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn is_leaf(&self) -> bool {
        true
    }

    fn accessibility(&self) -> Option<sequoia_view::Accessible> {
        Some(sequoia_view::Accessible::new(
            sequoia_view::AccessibleRole::TextField,
            if self.placeholder.is_empty() {
                "pole tekstowe".to_string()
            } else {
                self.placeholder.clone()
            },
        ))
    }

    fn focusable(&self) -> bool {
        true
    }

    fn has_visual_state(&self) -> bool {
        true
    }

    fn reuse_state(&self, old: &Self) {
        if !Rc::ptr_eq(&self.state, &old.state) {
            *self.state.borrow_mut() = old.state.borrow().clone();
        }
        self.revision.set(old.revision.get_untracked());
    }

    fn keyed_children(&self) -> Option<Vec<(u64, AnyView)>> {
        let _ = self.value.get();
        let _ = self.revision.get();

        Some(Vec::new())
    }

    fn cursor_icon(&self, _state: VisualState) -> CursorIcon {
        CursorIcon::Text
    }

    fn layout_style(&self) -> LayoutStyle {
        let mut style = self.style;
        style.margin = self.margin;
        style
    }

    fn intrinsic_size(&self, ctx: &mut LayoutCtx) -> Option<Size> {
        let appearance = self.appearance.unwrap_or_default();
        let size = self.font_size.or(appearance.font_size).unwrap_or(14.0);
        let weight = self.font_weight();
        let line_width = self.resolved_line_width();
        let (pad_x, pad_y) = self.resolved_pad();
        let bw = self.approx_border_w();
        let w = ctx
            .text_width_weighted(&self.value.get_untracked(), size, weight)
            .max(ctx.text_width_weighted(&self.placeholder, size, weight));
        let h = if line_width > 0.0 {
            underline::content_height(size, line_width)
        } else {
            LayoutCtx::text_line_height(size)
        };
        Some(Size::new(
            w + pad_x * 2.0 + bw * 2.0,
            h + pad_y * 2.0 + bw * 2.0,
        ))
    }

    fn on_event(&self, event: &Event, ctx: &mut EventCtx) {
        if ctx.phase != EventPhase::Target {
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
                let len = self.chars().len();
                let (pad_x, _) = self.resolved_pad();
                let ix = pad_x + self.approx_border_w();
                let scroll = self.with_state(|s| s.scroll);
                let x = pos.x - (ctx.rect.left() + ix) + scroll;
                let caret = self.caret_at(x, len);
                let extend = modifiers.contains(Modifiers::SHIFT);
                self.set_caret_extend(caret, extend);
                self.bump();
                ctx.request_focus();

                ctx.capture_pointer();
                ctx.handled();
            }
            Event::Pointer(PointerEvent::Move { pos, .. }) => {
                if ctx.state.pressed {
                    let len = self.chars().len();
                    let (pad_x, _) = self.resolved_pad();
                    let ix = pad_x + self.approx_border_w();
                    let scroll = self.with_state(|s| s.scroll);
                    let x = pos.x - (ctx.rect.left() + ix) + scroll;
                    let caret = self.caret_at(x, len);
                    self.set_caret_extend(caret, true);
                    self.bump();
                    ctx.handled();
                }
            }
            Event::Key(key) => {
                if !self.on_clipboard_key(key, ctx) {
                    self.on_key(key);
                }
                ctx.handled();
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
        let line_width = self.resolved_line_width();
        let placeholder_color = self.resolved_placeholder_color(&theme);
        let (pad_x, pad_y) = self.resolved_pad();
        let background = self.resolved_background();
        let radius = self.resolved_radius(&theme);
        let border = self.resolved_border(&theme);
        let colors = underline::field_colors(
            &theme,
            ctx.state.hovered || ctx.state.focused,
            ctx.state.focused,
            self.resolved_text_color(),
            self.resolved_line_color(),
        );

        if background.a > 0.0 || border.is_some() {
            match border {
                Some((w, color)) => {
                    ctx.bordered_quad(rect, background, radius, w, color);
                }
                None => {
                    ctx.quad(rect, background, radius);
                }
            }
        }

        let bw = border.map(|(w, _)| w).unwrap_or(0.0);
        let content_h = if line_width > 0.0 {
            underline::content_height(size, line_width)
        } else {
            LayoutCtx::text_line_height(size)
        };
        let inner_h = (rect.size.height - bw * 2.0 - pad_y * 2.0).max(0.0);
        let yoff = if inner_h > content_h {
            (inner_h - content_h) * 0.5
        } else {
            0.0
        };
        let (ipx, ipy) = (pad_x + bw, pad_y + bw + yoff);

        let line_w = (rect.size.width - ipx * 2.0).max(0.0);
        if line_width > 0.0 && colors.line.a > 0.0 {
            let line = underline::line_rect_padded(rect, size, line_w, line_width, ipx, ipy);
            ctx.quad(line, colors.line, line_width * 0.5);
        }

        let text = self.value.get_untracked();
        let chars: Vec<char> = text.chars().collect();
        let (caret, anchor, preedit) = self.with_state(|s| (s.caret, s.anchor, s.preedit.clone()));
        let caret = caret.min(chars.len());
        let vis_w = (rect.size.width - ipx * 2.0).max(0.0);

        let mut caret_x = 0.0;
        let mut bounds: Vec<f32>;
        if ctx.state.hovered || ctx.state.focused || anchor.is_some() {
            bounds = ctx.caret_positions(&text, size, weight);

            if !preedit.is_empty() {
                let pw = ctx.text_width_weighted(&preedit, size, weight);

                for b in bounds.iter_mut().skip(caret) {
                    *b += pw;
                }
            }
            caret_x = bounds.get(caret).copied().unwrap_or(0.0);

            if !preedit.is_empty() {
                caret_x += ctx.text_width_weighted(&preedit, size, weight);
            }

            let text_w = bounds.last().copied().unwrap_or(0.0);
            self.with_state(|s| {
                if caret_x + CARET_W - s.scroll > vis_w {
                    s.scroll = caret_x + CARET_W - vis_w;
                }
                if caret_x - s.scroll < 0.0 {
                    s.scroll = caret_x;
                }
                s.scroll = s.scroll.clamp(0.0, (text_w + CARET_W - vis_w).max(0.0));
            });
            self.store_bounds(bounds);
        }
        let scroll = self.with_state(|s| s.scroll);
        let x = rect.left() + ipx - scroll;
        let y = underline::baseline_padded(rect, size, ipy);

        let clip_box = Rect::from_xywh(
            rect.left() + ipx,
            rect.top() + bw,
            vis_w,
            (rect.size.height - bw * 2.0).max(0.0),
        );
        ctx.push_clip(clip_box);

        if let Some(a) = anchor {
            let (s0, s1) = (a.min(caret).min(chars.len()), a.max(caret).min(chars.len()));
            if s1 > s0 {
                let stored = self.stored_bounds();

                let x0 = stored.get(s0).copied().unwrap_or(0.0);
                let x1 = stored.get(s1).copied().unwrap_or(x0);
                if x1 > x0 {
                    let top = y - size * 0.8;
                    let sel = Rect::from_xywh(x + x0, top, x1 - x0, size * 1.05);
                    ctx.quad(sel, theme.accent_pressed, 0.0);
                }
            }
        }

        if chars.is_empty() && preedit.is_empty() && !self.placeholder.is_empty() {
            ctx.draw_text_weighted(&self.placeholder, (x, y), size, weight, placeholder_color);
        } else if preedit.is_empty() {
            ctx.draw_text_weighted(&text, (x, y), size, weight, colors.text);
        } else {
            let before: String = chars[..caret.min(chars.len())].iter().collect();
            let after: String = chars[caret.min(chars.len())..].iter().collect();
            let composed = format!("{before}{preedit}{after}");
            ctx.draw_text_weighted(&composed, (x, y), size, weight, colors.text);

            let stored = self.stored_bounds();
            let x0 = stored.get(caret).copied().unwrap_or(0.0);
            let pw = ctx.text_width_weighted(&preedit, size, weight);
            let line_y =
                underline::line_rect_padded(rect, size, line_w, line_width, ipx, ipy).top();
            ctx.quad(
                Rect::from_xywh(x + x0, line_y - 2.0, pw.max(1.0), 1.5),
                colors.text,
                0.0,
            );
        }

        if ctx.state.focused {
            let top = y - size * 0.8;
            ctx.quad(
                Rect::from_xywh(x + caret_x, top, CARET_W, size * 1.05),
                colors.text,
                0.0,
            );
        }

        ctx.pop_clip();
    }
}

fn sanitize_margin(v: f32) -> f32 {
    if v.is_finite() { v.max(0.0) } else { 0.0 }
}
