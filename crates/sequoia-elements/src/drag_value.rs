use std::cell::RefCell;
use std::rc::Rc;

use sequoia_core::{Color, Edges, Length, Point, Rect, Size};
use sequoia_input::{
    CursorIcon, Event, EventPhase, Key, KeyEvent, Modifiers, PointerButton, PointerEvent,
};
use sequoia_layout::{Align, Axis, LayoutCtx, LayoutStyle};
use sequoia_reactive::Signal;
use sequoia_style::{DragValueStyle, Theme, VisualState};
use sequoia_view::{AnyView, EventCtx, FontWeight, PaintCtx, Unit, View};

use crate::underline;

const HEIGHT: f32 = 24.0;

const KEY_FRACTION: f32 = 20.0;

const DRAG_PX_FULL_RANGE: f32 = 200.0;

const MAX_PRECISION: usize = 9;

const CLICK_SLOP: f32 = 3.0;

const VARIANT_DRAG_PX: f32 = 20.0;

const CARET: char = '|';

#[derive(Clone, Debug, PartialEq)]
pub struct DragValueVariant {
    value: Option<f32>,
    label: Option<String>,
}

impl DragValueVariant {
    pub fn new(value: f32) -> Self {
        Self {
            value: Some(value),
            label: None,
        }
    }

    pub fn labeled(value: f32, label: impl Into<String>) -> Self {
        Self {
            value: Some(value),
            label: Some(label.into()),
        }
    }

    pub fn text(label: impl Into<String>) -> Self {
        Self {
            value: None,
            label: Some(label.into()),
        }
    }

    pub fn value(mut self, value: f32) -> Self {
        self.value = Some(value);
        self
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    fn resolved_value(&self, index: usize) -> f32 {
        self.value.unwrap_or(index as f32)
    }
}

impl From<f32> for DragValueVariant {
    fn from(value: f32) -> Self {
        Self::new(value)
    }
}

impl From<(f32, &str)> for DragValueVariant {
    fn from((value, label): (f32, &str)) -> Self {
        Self::labeled(value, label)
    }
}

impl From<(f32, String)> for DragValueVariant {
    fn from((value, label): (f32, String)) -> Self {
        Self::labeled(value, label)
    }
}

impl From<&str> for DragValueVariant {
    fn from(label: &str) -> Self {
        Self::text(label)
    }
}

impl From<String> for DragValueVariant {
    fn from(label: String) -> Self {
        Self::text(label)
    }
}

#[derive(Clone, Copy)]
struct DragState {
    origin: Point,
    value: f32,
}

#[derive(Clone, Default)]
struct Gesture {
    drag: Option<DragState>,
    edit: Option<String>,

    fresh: bool,

    announced: bool,
}

pub struct DragValue {
    value: Signal<f32>,
    reset: Option<f32>,
    min: f32,
    max: f32,
    sensitivity: Option<f32>,
    step: Option<f32>,
    precision: usize,
    orientation: Axis,
    text_color: Option<Color>,
    line_color: Option<Color>,
    line_width: Option<f32>,
    line_full: Option<bool>,
    background: Option<Color>,
    hover_background: Option<Color>,
    pressed_background: Option<Color>,
    border_color: Option<Color>,
    border_width: Option<f32>,
    radius: Option<f32>,
    padding: Option<(f32, f32)>,
    trim_zeros: bool,
    font_size: Option<f32>,
    weight: Option<FontWeight>,
    appearance: Option<DragValueStyle>,
    variants: Vec<DragValueVariant>,
    wrap_variants: bool,
    style: LayoutStyle,
    margin: Edges,
    align: Align,

    revision: Signal<u32>,

    gesture: Rc<RefCell<Gesture>>,

    on_change_start: Option<Rc<dyn Fn()>>,

    on_commit: Option<Rc<dyn Fn(f32)>>,
}

impl DragValue {
    pub fn new(value: Signal<f32>) -> Self {
        Self {
            value,
            reset: None,
            min: 0.0,
            max: 1.0,
            sensitivity: None,
            step: None,
            precision: 2,
            orientation: Axis::Horizontal,
            text_color: None,
            line_color: None,
            line_width: None,
            line_full: None,
            background: None,
            hover_background: None,
            pressed_background: None,
            border_color: None,
            border_width: None,
            radius: None,
            padding: None,
            trim_zeros: false,
            font_size: None,
            weight: None,
            appearance: None,
            variants: Vec::new(),
            wrap_variants: false,
            style: LayoutStyle {
                width: Length::Fit,
                height: Length::Px(HEIGHT),
                ..LayoutStyle::default()
            },
            margin: Edges::ZERO,
            align: Align::Start,
            revision: Signal::new(0),
            gesture: Rc::new(RefCell::new(Gesture::default())),
            on_change_start: None,
            on_commit: None,
        }
    }

    pub fn reset_to(mut self, value: f32) -> Self {
        self.reset = Some(value);
        self
    }

    pub fn min(mut self, min: f32) -> Self {
        if min.is_finite() {
            self.min = min;
        }
        self
    }

    pub fn max(mut self, max: f32) -> Self {
        if max.is_finite() {
            self.max = max;
        }
        self
    }

    pub fn sensitivity(mut self, units_per_px: f32) -> Self {
        self.sensitivity = Some(units_per_px.max(0.0));
        self
    }

    pub fn step(mut self, step: f32) -> Self {
        self.step = Some(step.max(0.0));
        self
    }

    pub fn precision(mut self, precision: usize) -> Self {
        self.precision = precision.min(MAX_PRECISION);
        self
    }

    pub fn orientation(mut self, axis: Axis) -> Self {
        self.orientation = axis;
        self
    }

    pub fn appearance(mut self, style: DragValueStyle) -> Self {
        self.appearance = Some(style);
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.text_color = Some(color);
        self
    }

    pub fn line_color(mut self, color: Color) -> Self {
        self.line_color = Some(color);
        self
    }

    pub fn line_width(mut self, width: f32) -> Self {
        self.line_width = Some(width.max(0.0));
        self
    }

    pub fn line_full(mut self, full: bool) -> Self {
        self.line_full = Some(full);
        self
    }

    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    pub fn hover_background(mut self, color: Color) -> Self {
        self.hover_background = Some(color);
        self
    }

    pub fn pressed_background(mut self, color: Color) -> Self {
        self.pressed_background = Some(color);
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

    pub fn padding(mut self, pad_x: f32, pad_y: f32) -> Self {
        self.padding = Some((pad_x.max(0.0), pad_y.max(0.0)));
        self
    }

    pub fn padding_xy(self, pad_x: f32, pad_y: f32) -> Self {
        self.padding(pad_x, pad_y)
    }

    pub fn variants<I, V>(mut self, variants: I) -> Self
    where
        I: IntoIterator<Item = V>,
        V: Into<DragValueVariant>,
    {
        self.variants = variants
            .into_iter()
            .map(Into::into)
            .filter(|v| v.value.is_none_or(f32::is_finite))
            .collect();
        self
    }

    pub fn variant(mut self, variant: impl Into<DragValueVariant>) -> Self {
        let variant = variant.into();
        if variant.value.is_none_or(f32::is_finite) {
            self.variants.push(variant);
        }
        self
    }

    pub fn wrap_variants(mut self, wrap: bool) -> Self {
        self.wrap_variants = wrap;
        self
    }

    pub fn trim_zeros(mut self, trim: bool) -> Self {
        self.trim_zeros = trim;
        self
    }

    /// Horizontal alignment of the value inside the field.
    pub fn align(mut self, align: Align) -> Self {
        self.align = align;
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

    pub fn on_change_start(mut self, f: impl Fn() + 'static) -> Self {
        self.on_change_start = Some(Rc::new(f));
        self
    }

    pub fn on_commit(mut self, f: impl Fn(f32) + 'static) -> Self {
        self.on_commit = Some(Rc::new(f));
        self
    }

    fn announce(&self) {
        let need = self.with_gesture(|g| {
            if g.announced {
                false
            } else {
                g.announced = true;
                true
            }
        });
        if need && let Some(f) = &self.on_change_start {
            f();
        }
    }

    fn commit_if_changed(&self, before: f32) {
        let after = self.value.get_untracked();
        if after != before
            && let Some(f) = &self.on_commit
        {
            f(after);
        }
    }

    fn with_gesture<R>(&self, f: impl FnOnce(&mut Gesture) -> R) -> R {
        f(&mut self.gesture.borrow_mut())
    }

    fn anchor(&self) -> Option<DragState> {
        self.with_gesture(|g| g.drag)
    }

    fn is_editing(&self) -> bool {
        self.with_gesture(|g| g.edit.is_some())
    }

    fn edit_buffer(&self) -> Option<String> {
        self.with_gesture(|g| g.edit.clone())
    }

    fn start_edit(&self, buffer: String, fresh: bool) {
        self.with_gesture(|g| {
            g.edit = Some(buffer);
            g.fresh = fresh;
        });
        self.bump();
    }

    fn stop_edit(&self) {
        self.with_gesture(|g| {
            g.edit = None;
            g.fresh = false;
        });
        self.bump();
    }

    fn push_char(&self, c: char) {
        self.with_gesture(|g| {
            if g.edit.is_none() {
                return;
            }
            if g.fresh {
                g.edit = Some(String::new());
                g.fresh = false;
            }
            if let Some(buffer) = g.edit.as_mut() {
                buffer.push(c);
            }
        });
        self.bump();
    }

    fn pop_char(&self) {
        self.with_gesture(|g| {
            g.fresh = false;
            if let Some(buffer) = g.edit.as_mut() {
                buffer.pop();
            }
        });
        self.bump();
    }

    fn commit_edit(&self) {
        let Some(buffer) = self.edit_buffer() else {
            return;
        };
        let before = self.value.get_untracked();
        self.stop_edit();
        if let Ok(parsed) = buffer.trim().replace(',', ".").parse::<f32>() {
            if self.variants.is_empty() {
                self.set_value(parsed);
            } else {
                self.set_value_raw(parsed);
            }
        }
        self.commit_if_changed(before);
    }

    fn bump(&self) {
        self.revision
            .set(self.revision.get_untracked().wrapping_add(1));
    }

    fn span(&self) -> f32 {
        (self.max - self.min).abs().max(f32::EPSILON)
    }

    fn units_per_px(&self) -> f32 {
        self.sensitivity.unwrap_or(self.span() / DRAG_PX_FULL_RANGE)
    }

    fn key_step(&self) -> f32 {
        self.step.unwrap_or(self.span() / KEY_FRACTION)
    }

    fn clamped(&self, v: f32) -> f32 {
        let lo = self.min.min(self.max);
        let hi = self.min.max(self.max);
        let scale = 10f32.powi(self.precision as i32);
        let q = ((v.clamp(lo, hi) * scale).round() / scale).clamp(lo, hi);
        if q == 0.0 { 0.0 } else { q }
    }

    fn set_value(&self, v: f32) {
        let next = self.clamped(v);

        if !next.is_finite() {
            return;
        }
        if next != self.value.get_untracked() {
            self.announce();
            self.value.set(next);
        }
    }

    fn text(&self, value: f32) -> String {
        let s = format!("{value:.prec$}", prec = self.precision);
        if self.trim_zeros && self.precision > 0 && (s.contains('.') || s.contains(',')) {
            let t = s.trim_end_matches('0');
            t.strip_suffix('.')
                .or_else(|| t.strip_suffix(','))
                .unwrap_or(t)
                .to_owned()
        } else {
            s
        }
    }

    fn display(&self, value: f32) -> String {
        match self.edit_buffer() {
            Some(buffer) => format!("{buffer}{CARET}"),
            None => self.display_value(value),
        }
    }

    fn display_value(&self, value: f32) -> String {
        if let Some(index) = self.variant_exact_index(value)
            && let Some(label) = &self.variants[index].label
        {
            return label.clone();
        }
        self.text(value)
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

    fn resolved_line_full(&self) -> bool {
        self.line_full
            .or_else(|| self.appearance.and_then(|s| s.line_full))
            .unwrap_or(false)
    }

    fn resolved_pad(&self) -> (f32, f32) {
        self.padding
            .or_else(|| self.appearance.and_then(|s| s.padding))
            .unwrap_or((underline::PAD_X, 0.0))
    }

    fn resolved_background(&self, state: VisualState) -> Color {
        let style = self.appearance.unwrap_or_default();
        let idle = self
            .background
            .or(style.background)
            .unwrap_or(Color::TRANSPARENT);
        if state.pressed {
            self.pressed_background
                .or(style.pressed_background)
                .or(self.hover_background)
                .or(style.hover_background)
                .unwrap_or(idle)
        } else if state.hovered {
            self.hover_background
                .or(style.hover_background)
                .unwrap_or(idle)
        } else {
            idle
        }
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

    fn current_value(&self) -> f32 {
        let value = self.value.get_untracked();
        if self.variants.is_empty() {
            self.clamped(value)
        } else {
            value
        }
    }

    fn variant_value(&self, index: usize) -> f32 {
        self.variants[index].resolved_value(index)
    }

    fn variant_exact_index(&self, value: f32) -> Option<usize> {
        (0..self.variants.len()).find(|&i| {
            let delta = (self.variant_value(i) - value).abs();
            delta <= 1e-6 * value.abs().max(1.0)
        })
    }

    fn variant_index_for(&self, value: f32) -> usize {
        let mut best = 0;
        let mut best_delta = f32::INFINITY;
        for i in 0..self.variants.len() {
            let delta = (self.variant_value(i) - value).abs();
            if delta < best_delta {
                best_delta = delta;
                best = i;
            }
        }
        best
    }

    fn variant_after(&self, index: usize, steps: isize) -> usize {
        let len = self.variants.len();
        if len == 0 {
            return 0;
        }
        let len_i = len as isize;
        if self.wrap_variants {
            let steps = steps.rem_euclid(len_i);
            ((index as isize + steps) % len_i) as usize
        } else {
            (index as isize + steps.clamp(-len_i, len_i)).clamp(0, len_i - 1) as usize
        }
    }

    fn step_variant(&self, steps: isize) {
        self.step_variant_from(self.value.get_untracked(), steps);
    }

    fn step_variant_from(&self, value: f32, steps: isize) {
        if self.variants.is_empty() {
            return;
        }
        let start = self.variant_index_for(value);
        let next = self.variant_after(start, steps);
        self.set_variant(next);
    }

    fn set_variant(&self, index: usize) {
        if self.variants.is_empty() {
            return;
        }
        self.set_value_raw(self.variant_value(index));
    }

    fn set_value_raw(&self, value: f32) {
        if !value.is_finite() {
            return;
        }
        if value != self.value.get_untracked() {
            self.announce();
            self.value.set(value);
        }
    }
}

fn is_value_char(c: char) -> bool {
    c.is_ascii_digit() || c == '.' || c == ',' || c == '-'
}

fn normalize_char(c: char) -> char {
    if c == ',' { '.' } else { c }
}

impl View for DragValue {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn is_leaf(&self) -> bool {
        true
    }

    fn focusable(&self) -> bool {
        true
    }

    fn captures_pointer(&self) -> bool {
        true
    }

    fn has_visual_state(&self) -> bool {
        true
    }

    fn keyed_children(&self) -> Option<Vec<(u64, AnyView)>> {
        let _ = self.value.get();
        let _ = self.revision.get();
        Some(Vec::new())
    }

    fn cursor_icon(&self, state: VisualState) -> CursorIcon {
        if state.pressed {
            match self.orientation {
                Axis::Horizontal => CursorIcon::ColResize,
                Axis::Vertical => CursorIcon::RowResize,
            }
        } else {
            CursorIcon::Grab
        }
    }

    fn layout_style(&self) -> LayoutStyle {
        let mut s = self.style;
        s.margin = self.margin;
        s
    }

    fn intrinsic_size(&self, ctx: &mut LayoutCtx) -> Option<Size> {
        let appearance = self.appearance.unwrap_or_default();
        let size = self.font_size.or(appearance.font_size).unwrap_or(14.0);
        let text = self.display(self.current_value());
        let w = ctx.text_width_weighted(&text, size, self.font_weight());
        let line_width = self.resolved_line_width();
        let h = if line_width > 0.0 {
            underline::content_height(size, line_width)
        } else {
            LayoutCtx::text_line_height(size)
        };
        let (pad_x, pad_y) = self.resolved_pad();
        let bw = self.approx_border_w();
        Some(Size::new(
            w + pad_x * 2.0 + bw * 2.0,
            h + pad_y * 2.0 + bw * 2.0,
        ))
    }

    fn reuse_state(&self, old: &Self) {
        if !Rc::ptr_eq(&self.gesture, &old.gesture) {
            *self.gesture.borrow_mut() = old.gesture.borrow().clone();
        }
        self.revision.set(old.revision.get_untracked());
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
                if modifiers.contains(Modifiers::CONTROL) {
                    if let Some(reset) = self.reset {
                        let before = self.value.get_untracked();
                        self.stop_edit();
                        self.with_gesture(|g| g.drag = None);
                        if self.variants.is_empty() {
                            self.set_value(reset);
                        } else {
                            self.set_value_raw(reset);
                        }
                        self.commit_if_changed(before);
                        ctx.handled();
                    }
                    return;
                }

                if self.is_editing() {
                    self.commit_edit();
                }
                self.with_gesture(|g| {
                    g.drag = Some(DragState {
                        origin: *pos,
                        value: self.value.get_untracked(),
                    });
                    g.announced = false;
                });
                ctx.capture_pointer();
                ctx.handled();
            }
            Event::Pointer(PointerEvent::Move { pos, .. }) => {
                let Some(drag) = self.anchor() else { return };

                if !ctx.state.pressed {
                    self.with_gesture(|g| g.drag = None);
                    self.commit_if_changed(drag.value);
                    return;
                }
                let delta = match self.orientation {
                    Axis::Horizontal => pos.x - drag.origin.x,

                    Axis::Vertical => drag.origin.y - pos.y,
                };
                if self.variants.is_empty() {
                    self.set_value(drag.value + delta * self.units_per_px());
                } else if delta.is_finite() {
                    let len = self.variants.len() as f32;
                    let steps = (delta / VARIANT_DRAG_PX).round().clamp(-len, len) as isize;
                    self.step_variant_from(drag.value, steps);
                }
                ctx.handled();
            }
            Event::Pointer(PointerEvent::Up { pos, .. }) => {
                let Some(drag) = self.anchor() else { return };
                self.with_gesture(|g| g.drag = None);

                let moved = (pos.x - drag.origin.x)
                    .abs()
                    .max((pos.y - drag.origin.y).abs());
                if moved <= CLICK_SLOP {
                    self.start_edit(self.text(self.value.get_untracked()), true);
                } else {
                    self.bump();
                    self.commit_if_changed(drag.value);
                }
                ctx.handled();
            }
            Event::Key(key) => {
                if self.is_editing() {
                    self.on_edit_key(key);
                    ctx.handled();
                    return;
                }

                if let Some(c) = key.key.char().filter(|c| is_value_char(*c)) {
                    self.start_edit(normalize_char(c).to_string(), false);
                    ctx.request_focus();
                    ctx.handled();
                    return;
                }
                if !self.variants.is_empty() {
                    let steps = match key.key {
                        Key::ARROW_LEFT | Key::ARROW_DOWN => -1,
                        Key::ARROW_RIGHT | Key::ARROW_UP => 1,
                        _ => return,
                    };
                    let before = self.value.get_untracked();
                    self.step_variant(steps);
                    self.commit_if_changed(before);
                    ctx.handled();
                    return;
                }
                let delta = match key.key {
                    Key::ARROW_LEFT | Key::ARROW_DOWN => -self.key_step(),
                    Key::ARROW_RIGHT | Key::ARROW_UP => self.key_step(),
                    _ => return,
                };
                let before = self.value.get_untracked();
                self.set_value(self.value.get_untracked() + delta);
                self.commit_if_changed(before);
                ctx.handled();
            }

            Event::Pointer(PointerEvent::Wheel {
                delta, modifiers, ..
            }) => {
                if self.is_editing() {
                    return;
                }
                let dir = if delta.y < 0.0 {
                    1
                } else if delta.y > 0.0 {
                    -1
                } else {
                    0
                };
                if dir != 0 && !self.variants.is_empty() {
                    let before = self.value.get_untracked();
                    self.step_variant(dir);
                    self.commit_if_changed(before);
                } else if dir != 0 {
                    let mult = if modifiers.contains(Modifiers::SHIFT) {
                        0.1
                    } else if modifiers.contains(Modifiers::CONTROL)
                        || modifiers.contains(Modifiers::SUPER)
                    {
                        10.0
                    } else {
                        1.0
                    };
                    let before = self.value.get_untracked();
                    self.set_value(before + self.span() / 100.0 * mult * dir as f32);
                    self.commit_if_changed(before);
                }
                ctx.handled();
            }
            _ => {}
        }
    }

    fn paint_self(&self, ctx: &mut PaintCtx) {
        if !ctx.state.focused && self.is_editing() {
            self.commit_edit();
        }

        let rect = ctx.rect;
        if rect.size.width <= 0.0 || rect.size.height <= 0.0 {
            return;
        }
        let theme = ctx.theme();
        let size = self.resolved_font_size(&theme);
        let weight = self.font_weight();
        let line_width = self.resolved_line_width();
        let editing = self.is_editing();
        let (pad_x, pad_y) = self.resolved_pad();
        let background = self.resolved_background(ctx.state);
        let radius = self.resolved_radius(&theme);
        let border = self.resolved_border(&theme);
        let colors = underline::field_colors(
            &theme,
            ctx.state.hovered || ctx.state.pressed || editing,
            ctx.state.pressed || editing,
            self.resolved_text_color(),
            self.resolved_line_color(),
        );
        let text = self.display(self.current_value());
        let text_width = ctx.text_width_weighted(&text, size, weight);

        if background.a > 0.0 || border.is_some() {
            match border {
                Some((w, color)) => ctx.bordered_quad(rect, background, radius, w, color),
                None => ctx.quad(rect, background, radius),
            }
        }

        let bw = border.map(|(w, _)| w).unwrap_or(0.0);
        let inner_w = (rect.size.width - (pad_x + bw) * 2.0).max(0.0);
        let inner_h = (rect.size.height - (pad_y + bw) * 2.0).max(0.0);
        let line_h = LayoutCtx::text_line_height(size);
        let draws_line = line_width > 0.0 && colors.line.a > 0.0;
        let text_top = if draws_line {
            rect.top() + bw + pad_y
        } else {
            rect.top() + bw + pad_y + ((inner_h - line_h) * 0.5).max(0.0)
        };

        if draws_line {
            let w = if self.resolved_line_full() {
                inner_w
            } else {
                text_width.min(inner_w)
            };
            let line = Rect::from_xywh(
                rect.left() + pad_x + bw,
                text_top + line_h + underline::LINE_GAP,
                w,
                line_width,
            );
            ctx.quad(line, colors.line, line_width * 0.5);
        }

        let x = match self.align {
            Align::Center => rect.left() + pad_x + bw + (inner_w - text_width) * 0.5,
            Align::End => rect.right() - pad_x - bw - text_width,
            _ => rect.left() + pad_x + bw,
        };
        let x = x.max(rect.left());
        let y = text_top + line_h * 0.5 + size * 0.35;
        ctx.draw_text_weighted(&text, (x, y), size, weight, colors.text);
    }
}

impl DragValue {
    fn on_edit_key(&self, key: &KeyEvent) {
        match key.key {
            Key::ENTER => self.commit_edit(),
            Key::ESCAPE => self.stop_edit(),
            Key::BACKSPACE => self.pop_char(),
            other => {
                if let Some(c) = other.char().filter(|c| is_value_char(*c)) {
                    self.push_char(normalize_char(c));
                }
            }
        }
    }
}

fn sanitize_margin(v: f32) -> f32 {
    if v.is_finite() { v.max(0.0) } else { 0.0 }
}
