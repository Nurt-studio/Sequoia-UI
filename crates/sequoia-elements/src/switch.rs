use sequoia_core::{Color, Edges, Length, Point, Rect, Size};
use sequoia_input::{CursorIcon, Event, EventPhase, Key, PointerButton, PointerEvent};
use sequoia_layout::{LayoutCtx, LayoutStyle};
use sequoia_reactive::Signal;
use sequoia_style::{SwitchStyle, VisualState};
use sequoia_view::{AnyView, EventCtx, PaintCtx, Unit, View};

const TRACK_W: f32 = 40.0;
const TRACK_H: f32 = 22.0;
const GAP: f32 = 8.0;
const BORDER_W: f32 = 1.0;

pub struct Switch {
    checked: Signal<bool>,
    label: Option<String>,
    style: Option<SwitchStyle>,
    track_on: Option<Color>,
    track_off: Option<Color>,
    track_disabled: Option<Color>,
    thumb: Option<Color>,
    thumb_on: Option<Color>,
    thumb_disabled: Option<Color>,
    border: Option<Color>,
    border_on: Option<Color>,
    text_color: Option<Color>,
    font_size: Option<f32>,
    track_width: Option<f32>,
    track_height: Option<f32>,
    thumb_size: Option<f32>,
    disabled: bool,
    on_toggle: Option<Box<dyn Fn(bool)>>,
    margin: Edges,
}

impl Switch {
    pub fn new(checked: Signal<bool>) -> Self {
        Self {
            checked,
            label: None,
            style: None,
            track_on: None,
            track_off: None,
            track_disabled: None,
            thumb: None,
            thumb_on: None,
            thumb_disabled: None,
            border: None,
            border_on: None,
            text_color: None,
            font_size: None,
            track_width: None,
            track_height: None,
            thumb_size: None,
            disabled: false,
            on_toggle: None,
            margin: Edges::ZERO,
        }
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn style(mut self, style: SwitchStyle) -> Self {
        self.style = Some(style);
        self
    }

    pub fn track_on(mut self, color: Color) -> Self {
        self.track_on = Some(color);
        self
    }

    pub fn track_off(mut self, color: Color) -> Self {
        self.track_off = Some(color);
        self
    }

    pub fn track_disabled(mut self, color: Color) -> Self {
        self.track_disabled = Some(color);
        self
    }

    pub fn thumb_color(mut self, color: Color) -> Self {
        self.thumb = Some(color);
        self
    }

    pub fn thumb_on_color(mut self, color: Color) -> Self {
        self.thumb_on = Some(color);
        self
    }

    pub fn thumb_disabled_color(mut self, color: Color) -> Self {
        self.thumb_disabled = Some(color);
        self
    }

    pub fn border_color(mut self, color: Color) -> Self {
        self.border = Some(color);
        self
    }

    pub fn border_on_color(mut self, color: Color) -> Self {
        self.border_on = Some(color);
        self
    }

    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color = Some(color);
        self
    }

    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = Some(size);
        self
    }

    pub fn track_width(mut self, width: f32) -> Self {
        self.track_width = Some(width.max(8.0));
        self
    }

    pub fn track_height(mut self, height: f32) -> Self {
        self.track_height = Some(height.max(8.0));
        self
    }

    pub fn thumb_size(mut self, size: f32) -> Self {
        self.thumb_size = Some(size.max(4.0));
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_toggle(mut self, f: impl Fn(bool) + 'static) -> Self {
        self.on_toggle = Some(Box::new(f));
        self
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

    pub fn is_on(&self) -> bool {
        self.checked.get_untracked()
    }

    fn resolved_track_size(&self) -> (f32, f32) {
        let style = self.style.unwrap_or_default();
        let w = self.track_width.or(style.width).unwrap_or(TRACK_W).max(8.0);
        let h = self
            .track_height
            .or(style.height)
            .unwrap_or(TRACK_H)
            .max(8.0);
        (w, h)
    }

    fn resolved_thumb_d(&self, track_h: f32) -> f32 {
        let style = self.style.unwrap_or_default();
        self.thumb_size
            .or(style.thumb_size)
            .unwrap_or((track_h - 4.0).max(4.0))
            .clamp(4.0, track_h.max(4.0))
    }

    fn toggle(&self) {
        if self.disabled {
            return;
        }
        let next = !self.checked.get_untracked();
        self.checked.set(next);
        if let Some(f) = &self.on_toggle {
            f(next);
        }
    }
}

fn sanitize_margin(v: f32) -> f32 {
    if v.is_finite() { v.max(0.0) } else { 0.0 }
}

impl View for Switch {
    fn body(&self) -> impl View + 'static {
        Unit
    }
    fn is_leaf(&self) -> bool {
        true
    }
    fn focusable(&self) -> bool {
        !self.disabled
    }
    fn has_visual_state(&self) -> bool {
        true
    }
    fn cursor_icon(&self, _state: VisualState) -> CursorIcon {
        if self.disabled {
            CursorIcon::Default
        } else {
            CursorIcon::Pointer
        }
    }
    fn keyed_children(&self) -> Option<Vec<(u64, AnyView)>> {
        let _ = self.checked.get();
        Some(Vec::new())
    }
    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fit,
            height: Length::Fit,
            margin: self.margin,
            ..LayoutStyle::default()
        }
    }
    fn intrinsic_size(&self, ctx: &mut LayoutCtx) -> Option<Size> {
        let style = self.style.unwrap_or_default();
        let size = self.font_size.or(style.font_size).unwrap_or(14.0);
        let (track_w, track_h) = self.resolved_track_size();
        let label_w = self
            .label
            .as_deref()
            .map(|l| ctx.text_width(l, size))
            .unwrap_or(0.0);
        let gap = if self.label.is_some() { GAP } else { 0.0 };
        let h = track_h.max(LayoutCtx::text_line_height(size));
        Some(Size::new(track_w + gap + label_w, h))
    }
    fn on_event(&self, event: &Event, ctx: &mut EventCtx) {
        if self.disabled || ctx.phase != EventPhase::Target {
            return;
        }
        if let Event::Pointer(PointerEvent::Up {
            button: PointerButton::Left,
            ..
        }) = event
        {
            if ctx.state.pressed && ctx.hover_self {
                self.toggle();
                ctx.handled();
            }
            return;
        }
        if let Event::Key(key) = event
            && ctx.state.focused
            && (key.key == Key::ENTER || key.key == Key::SPACE)
        {
            self.toggle();
            ctx.handled();
        }
    }
    fn paint_self(&self, ctx: &mut PaintCtx) {
        let theme = ctx.theme();
        let mut vs = ctx.state;
        vs.disabled = self.disabled;
        vs.selected = self.checked.get_untracked();
        let style = self.style.unwrap_or_default();
        let (track_w, track_h) = self.resolved_track_size();
        let thumb_d = self.resolved_thumb_d(track_h);
        let thumb_r = thumb_d * 0.5;
        let pad = ((track_h - thumb_d) * 0.5).max(1.0);

        let rect = ctx.rect;
        let track_y = rect.top() + (rect.size.height - track_h) * 0.5;
        let track_rect = Rect::from_xywh(rect.left(), track_y, track_w, track_h);
        let radius = track_h * 0.5;

        let track = if vs.disabled {
            self.track_disabled
                .or(style.track_disabled)
                .unwrap_or(theme.disabled)
        } else if vs.selected {
            self.track_on
                .or(style.track_on)
                .unwrap_or(Color::hex(0x3A3A3A))
        } else {
            self.track_off.or(style.track_off).unwrap_or(theme.surface)
        };
        let border = if vs.selected {
            self.border_on
                .or(style.border_on)
                .or(self.border)
                .or(style.border)
        } else {
            self.border.or(style.border)
        };
        let has_border = border.is_some_and(|c| c.a > 0.0);
        if has_border {
            ctx.bordered_quad(track_rect, track, radius, BORDER_W, border.unwrap());
        } else {
            ctx.quad(track_rect, track, radius);
        }

        let thumb = if vs.disabled {
            self.thumb_disabled
                .or(style.thumb_disabled)
                .or(self.thumb)
                .or(style.thumb)
                .unwrap_or(Color::WHITE)
        } else if vs.selected {
            self.thumb_on
                .or(style.thumb_on)
                .or(self.thumb)
                .or(style.thumb)
                .unwrap_or(Color::WHITE)
        } else {
            self.thumb.or(style.thumb).unwrap_or(Color::WHITE)
        };
        let cx = if vs.selected {
            track_rect.right() - pad - thumb_r
        } else {
            track_rect.left() + pad + thumb_r
        };
        let cy = track_rect.top() + track_h * 0.5;
        ctx.disk(Point::new(cx, cy), thumb_r, thumb);

        if let Some(label) = &self.label {
            let size = self
                .font_size
                .or(style.font_size)
                .unwrap_or(theme.font_size);
            let text = self.text_color.or(style.text).unwrap_or(theme.text);
            let x = rect.left() + track_w + GAP;
            let y = rect.top() + rect.size.height * 0.5 + size * 0.35;
            ctx.draw_text(label, (x, y), size, text);
        }
    }
}
