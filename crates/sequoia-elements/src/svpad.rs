#![forbid(unsafe_code)]

use sequoia_core::{Color, Edges, Length, Point, Rect};
use sequoia_input::{CursorIcon, Event, EventPhase, PointerButton, PointerEvent};
use sequoia_layout::LayoutStyle;
use sequoia_reactive::Signal;
use sequoia_style::VisualState;
use sequoia_view::{EventCtx, Pad2D as PadMap, PaintCtx, Painter, Unit, View};

pub const KNOB_R: f32 = 7.0;

pub const HEIGHT: f32 = 198.0;

#[derive(Clone)]
pub struct SvPad {
    sat: Signal<f32>,
    val: Signal<f32>,
    hue: Signal<f32>,
    knob_radius: f32,
    style: LayoutStyle,
    margin: Edges,
}

impl SvPad {
    pub fn new(sat: Signal<f32>, val: Signal<f32>, hue: Signal<f32>) -> Self {
        Self {
            sat,
            val,
            hue,
            knob_radius: KNOB_R,
            style: LayoutStyle {
                width: Length::Fill(1.0),
                height: Length::Px(HEIGHT),
                ..LayoutStyle::default()
            },
            margin: Edges::ZERO,
        }
    }

    pub fn knob_radius(mut self, r: f32) -> Self {
        self.knob_radius = r.clamp(3.0, 16.0);
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

    fn apply_at(&self, rect: Rect, pos: Point) {
        let (s, v) = PadMap::new(rect, 0.0).sv_of(pos);
        self.sat.set(s);
        self.val.set(v);
    }

    fn knob_center(&self, rect: Rect) -> Point {
        PadMap::new(rect, 0.0).pos_of(
            self.sat.get_untracked().clamp(0.0, 1.0),
            self.val.get_untracked().clamp(0.0, 1.0),
        )
    }

    fn cur_color(&self) -> Color {
        Color::from_hsva(
            self.hue.get_untracked() / 360.0,
            self.sat.get_untracked().clamp(0.0, 1.0),
            self.val.get_untracked().clamp(0.0, 1.0),
            1.0,
        )
    }
}

impl View for SvPad {
    fn body(&self) -> impl View + 'static {
        let _ = self.sat.get();
        let _ = self.val.get();
        let _ = self.hue.get();
        Unit
    }

    fn is_leaf(&self) -> bool {
        true
    }

    fn focusable(&self) -> bool {
        false
    }

    fn captures_pointer(&self) -> bool {
        true
    }

    fn has_visual_state(&self) -> bool {
        true
    }

    fn cursor_icon(&self, _state: VisualState) -> CursorIcon {
        CursorIcon::Crosshair
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
            Event::Pointer(PointerEvent::Move { pos, .. }) if ctx.state.pressed => {
                self.apply_at(ctx.rect, *pos);
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
            g.sv_field(rect, self.hue.get_untracked() / 360.0);
            g.knob(self.knob_center(rect), self.knob_radius, self.cur_color());
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
    fn defaults_fill_width_fixed_height() {
        let p = SvPad::new(
            Signal::new(0.5f32),
            Signal::new(0.5f32),
            Signal::new(200.0f32),
        );
        assert_eq!(p.layout_style().width, Length::Fill(1.0));
        assert_eq!(p.layout_style().height, Length::Px(HEIGHT));
        assert!(!p.clip_self());
    }

    #[test]
    fn drag_clamps_to_corners() {
        let (s, v, h) = (
            Signal::new(0.5f32),
            Signal::new(0.5f32),
            Signal::new(0.0f32),
        );
        let p = SvPad::new(s.clone(), v.clone(), h);
        let rect = Rect::from_xywh(0.0, 0.0, 200.0, 200.0);
        p.apply_at(rect, Point::new(-50.0, -50.0));
        assert_eq!((s.get_untracked(), v.get_untracked()), (0.0, 1.0));
        p.apply_at(rect, Point::new(500.0, 500.0));
        assert_eq!((s.get_untracked(), v.get_untracked()), (1.0, 0.0));
    }
}
