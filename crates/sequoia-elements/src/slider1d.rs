#![forbid(unsafe_code)]

use std::rc::Rc;

use sequoia_core::{BrushStop, Color, Edges, Fill, GradientStops, Length, Point, Rect};
use sequoia_input::{CursorIcon, Event, EventPhase, Key, PointerButton, PointerEvent};
use sequoia_layout::LayoutStyle;
use sequoia_reactive::Signal;
use sequoia_style::VisualState;
use sequoia_view::{EventCtx, Orientation, PaintCtx, Painter, Track1D, Unit, View};

pub const TRACK_H: f32 = 8.0;

pub const KNOB_R: f32 = 7.0;

pub const HEIGHT: f32 = 26.0;

pub const WIDTH: f32 = 26.0;

#[derive(Clone)]
pub struct Slider1D {
    value: Signal<f32>,
    min: f32,
    max: f32,
    stops: GradientStops,
    orientation: Orientation,
    solid: Color,
    track_height: f32,
    knob_radius: f32,
    style: LayoutStyle,
    margin: Edges,
    on_commit: Option<Rc<dyn Fn(f32)>>,
}

impl Slider1D {
    pub fn new(value: Signal<f32>, min: f32, max: f32) -> Self {
        Self {
            value,
            min: min.min(max),
            max: min.max(max),
            stops: GradientStops::new(Vec::new()),
            orientation: Orientation::Horizontal,
            solid: Color::hex(0x3E3E3E),
            track_height: TRACK_H,
            knob_radius: KNOB_R,
            style: LayoutStyle {
                width: Length::Fill(1.0),
                height: Length::Px(HEIGHT),
                ..LayoutStyle::default()
            },
            margin: Edges::ZERO,
            on_commit: None,
        }
    }

    pub fn vertical(value: Signal<f32>, min: f32, max: f32) -> Self {
        let mut s = Self::new(value, min, max);
        s.orientation = Orientation::Vertical;
        s.style = LayoutStyle {
            width: Length::Px(WIDTH),
            height: Length::Fill(1.0),
            ..LayoutStyle::default()
        };
        s
    }

    pub fn gradient(mut self, stops: Vec<BrushStop>, orientation: Orientation) -> Self {
        self.stops = GradientStops::new(stops);
        self.orientation = orientation;
        self
    }

    pub fn solid(mut self, color: Color) -> Self {
        self.solid = color;
        self
    }

    pub fn track_height(mut self, h: f32) -> Self {
        self.track_height = h.max(1.0);
        self
    }

    pub fn knob_radius(mut self, r: f32) -> Self {
        self.knob_radius = r.clamp(3.0, 16.0);
        self
    }

    pub fn on_commit(mut self, f: impl Fn(f32) + 'static) -> Self {
        self.on_commit = Some(Rc::new(f));
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.style = self.style.height(height.into());
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.style = self.style.width(width.into());
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

    fn span(&self) -> f32 {
        (self.max - self.min).max(f32::EPSILON)
    }

    fn ratio(&self) -> f32 {
        ((self.value.get_untracked() - self.min) / self.span()).clamp(0.0, 1.0)
    }

    fn set_ratio(&self, t: f32) {
        self.value.set(self.min + t.clamp(0.0, 1.0) * self.span());
    }

    fn track_of(&self, rect: Rect) -> Track1D {
        match self.orientation {
            Orientation::Horizontal => Track1D::new(rect.left(), rect.size.width, 0.0),
            Orientation::Vertical => Track1D::new(rect.top(), rect.size.height, 0.0),
        }
    }

    fn bar_rect(&self, rect: Rect) -> Rect {
        let th = self.track_height.min(if self.is_horizontal() {
            rect.size.height
        } else {
            rect.size.width
        });
        if self.is_horizontal() {
            let cy = rect.top() + rect.size.height * 0.5;
            Rect::from_xywh(rect.left(), cy - th * 0.5, rect.size.width, th)
        } else {
            let cx = rect.left() + rect.size.width * 0.5;
            Rect::from_xywh(cx - th * 0.5, rect.top(), th, rect.size.height)
        }
    }

    fn is_horizontal(&self) -> bool {
        self.orientation == Orientation::Horizontal
    }

    fn knob_fill(&self, t: f32) -> Color {
        if self.stops.is_empty() {
            return Color::WHITE;
        }

        let [r, g, b, a] = self.stops.sample_linear(t.clamp(0.0, 1.0));
        Color { r, g, b, a }.from_linear()
    }

    fn bar_fill(&self) -> Fill {
        if self.stops.is_empty() {
            return Fill::solid(self.solid);
        }
        match self.orientation {
            Orientation::Horizontal => Fill::horizontal(self.stops.clone()),
            Orientation::Vertical => Fill::vertical(self.stops.clone()),
        }
    }

    fn apply_at(&self, rect: Rect, pos: Point) {
        let track = self.track_of(rect);
        let t = if self.is_horizontal() {
            track.t_of(pos.x)
        } else {
            track.t_of(pos.y)
        };
        self.set_ratio(t);
    }
}

impl View for Slider1D {
    fn body(&self) -> impl View + 'static {
        let _ = self.value.get();
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

    fn cursor_icon(&self, _state: VisualState) -> CursorIcon {
        if self.is_horizontal() {
            CursorIcon::ColResize
        } else {
            CursorIcon::RowResize
        }
    }

    fn layout_style(&self) -> LayoutStyle {
        let mut s = self.style;
        s.margin = self.margin;
        s
    }

    fn clip_self(&self) -> bool {
        false
    }

    fn on_event(&self, event: &Event, ctx: &mut EventCtx) {
        if ctx.phase != EventPhase::Target {
            return;
        }
        match event {
            Event::Pointer(PointerEvent::Down {
                button: PointerButton::Left,
                pos,
                ..
            }) => {
                self.apply_at(ctx.rect, *pos);
                ctx.capture_pointer();
                ctx.handled();
            }
            Event::Pointer(PointerEvent::Move { pos, .. }) => {
                if ctx.state.pressed {
                    self.apply_at(ctx.rect, *pos);
                    ctx.handled();
                }
            }
            Event::Pointer(PointerEvent::Up {
                button: PointerButton::Left,
                ..
            }) => {
                if let Some(f) = &self.on_commit {
                    f(self.value.get_untracked());
                }
            }
            Event::Pointer(PointerEvent::Wheel {
                delta, modifiers, ..
            }) => {
                let mult = if modifiers.contains(sequoia_input::Modifiers::SHIFT) {
                    0.1
                } else if modifiers.contains(sequoia_input::Modifiers::CONTROL) {
                    10.0
                } else {
                    1.0
                };
                let dir = if delta.y < 0.0 { 1.0 } else { -1.0 };
                let step = self.span() / 100.0 * mult * dir;
                self.value
                    .set((self.value.get_untracked() + step).clamp(self.min, self.max));
                ctx.handled();
            }
            Event::Key(key) => {
                let step = match key.key {
                    Key::ARROW_LEFT | Key::ARROW_DOWN => -self.span() * 0.05,
                    Key::ARROW_RIGHT | Key::ARROW_UP => self.span() * 0.05,
                    _ => return,
                };
                self.value
                    .set((self.value.get_untracked() + step).clamp(self.min, self.max));
                if let Some(f) = &self.on_commit {
                    f(self.value.get_untracked());
                }
                ctx.handled();
            }
            _ => {}
        }
    }

    fn paint_self(&self, ctx: &mut PaintCtx) {
        let mut g = Painter::new(ctx);
        let rect = g.rect();
        if rect.is_empty() {
            return;
        }

        let overflow = rect.inflate(self.knob_radius, self.knob_radius);
        g.clipped(overflow, |g| {
            let bar = self.bar_rect(rect);
            if !bar.is_empty() {
                g.fill(bar, self.bar_fill());
            }
            let t = self.ratio();
            let track = self.track_of(rect);
            let c = track.pos_of(t);
            let center = if self.is_horizontal() {
                Point::new(c, bar.top() + bar.size.height * 0.5)
            } else {
                Point::new(bar.left() + bar.size.width * 0.5, c)
            };
            g.knob(center, self.knob_radius, self.knob_fill(t));
        });
    }
}

fn sanitize_margin(v: f32) -> f32 {
    if v.is_finite() { v.max(0.0) } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_horizontal_fill() {
        let v = Signal::new(0.5f32);
        let s = Slider1D::new(v, 0.0, 1.0);
        assert_eq!(s.layout_style().width, Length::Fill(1.0));
        assert!(!s.clip_self(), "knob sticks out - extended clip");
        let vert = Slider1D::vertical(Signal::new(0.0f32), 0.0, 360.0);
        assert_eq!(vert.layout_style().width, Length::Px(WIDTH));
    }

    #[test]
    fn ratio_clamps_out_of_range_values() {
        let v = Signal::new(99.0f32);
        let s = Slider1D::new(v, 0.0, 1.0);
        assert_eq!(s.ratio(), 1.0);
        s.set_ratio(-5.0);
        assert_eq!(s.value.get_untracked(), 0.0);
        s.set_ratio(0.25);
        assert!((s.value.get_untracked() - 0.25).abs() < 1e-6);
    }

    #[test]
    fn knob_samples_gradient_at_position() {
        let v = Signal::new(0.0f32);
        let s = Slider1D::new(v, 0.0, 1.0).gradient(
            vec![
                BrushStop::new(0.0, Color::BLACK),
                BrushStop::new(1.0, Color::WHITE),
            ],
            Orientation::Horizontal,
        );
        let lo = s.knob_fill(0.0).to_rgb8();
        let hi = s.knob_fill(1.0).to_rgb8();
        assert_eq!(lo, (0, 0, 0));
        assert_eq!(hi, (255, 255, 255));
    }
}
