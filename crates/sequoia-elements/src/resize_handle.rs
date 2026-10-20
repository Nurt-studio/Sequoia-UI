use std::cell::Cell;
use std::rc::Rc;

use sequoia_core::{Color, Edges, Length, Point, Rect, Size};
use sequoia_input::{CursorIcon, Event, EventPhase, PointerButton, PointerEvent};
use sequoia_layout::{Axis, LayoutCtx, LayoutStyle};
use sequoia_reactive::Signal;
use sequoia_style::{ResizeHandleStyle, VisualState};
use sequoia_view::{EventCtx, PaintCtx, Unit, View};

const DEFAULT_HIT: f32 = 7.0;

const LINE_IDLE: f32 = 1.0;

const LINE_ACTIVE: f32 = 2.0;

const IDLE_COLOR: Color = Color::rgb8(0x75, 0x75, 0x75);

const ACTIVE_COLOR: Color = Color::rgb8(0x0D, 0x99, 0xFF);

#[derive(Clone, Copy)]
struct DragState {
    origin: f32,
    value: f32,
}

#[derive(Clone)]
pub struct ResizeHandle {
    value: Signal<f32>,
    min: f32,
    max: f32,
    hit: f32,
    axis: Axis,
    invert: bool,
    gutter: Color,
    style: Option<ResizeHandleStyle>,
    idle: Option<Color>,
    active: Option<Color>,
    idle_width: Option<f32>,
    active_width: Option<f32>,
    drag: Rc<Cell<Option<DragState>>>,
    margin: Edges,
}

impl ResizeHandle {
    pub fn new(value: Signal<f32>) -> Self {
        Self {
            value,
            min: 0.0,
            max: f32::INFINITY,
            hit: DEFAULT_HIT,
            axis: Axis::Horizontal,
            invert: false,
            gutter: Color::TRANSPARENT,
            style: None,
            idle: None,
            active: None,
            idle_width: None,
            active_width: None,
            drag: Rc::new(Cell::new(None)),
            margin: Edges::ZERO,
        }
    }

    pub fn min(mut self, min: f32) -> Self {
        self.min = if min.is_finite() { min.max(0.0) } else { 0.0 };
        self
    }

    pub fn max(mut self, max: f32) -> Self {
        self.max = if max.is_nan() {
            f32::INFINITY
        } else {
            max.max(0.0)
        };
        self
    }

    pub fn hit_width(mut self, hit: f32) -> Self {
        self.hit = if hit.is_finite() {
            hit.max(3.0)
        } else {
            DEFAULT_HIT
        };
        self
    }

    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }

    pub fn invert(mut self, invert: bool) -> Self {
        self.invert = invert;
        self
    }

    pub fn gutter(mut self, color: Color) -> Self {
        self.gutter = color;
        self
    }

    pub fn style(mut self, style: ResizeHandleStyle) -> Self {
        self.style = Some(style);
        self
    }

    pub fn idle_color(mut self, color: Color) -> Self {
        self.idle = Some(color);
        self
    }

    pub fn active_color(mut self, color: Color) -> Self {
        self.active = Some(color);
        self
    }

    pub fn idle_width(mut self, width: f32) -> Self {
        self.idle_width = Some(width);
        self
    }

    pub fn active_width(mut self, width: f32) -> Self {
        self.active_width = Some(width);
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

    fn resolved_gutter(&self) -> Color {
        if self.gutter != Color::TRANSPARENT {
            self.gutter
        } else {
            self.style
                .unwrap_or_default()
                .gutter
                .unwrap_or(Color::TRANSPARENT)
        }
    }

    fn resolved_idle(&self) -> Color {
        let style = self.style.unwrap_or_default();
        self.idle.or(style.idle).unwrap_or(IDLE_COLOR)
    }

    fn resolved_active(&self) -> Color {
        let style = self.style.unwrap_or_default();
        self.active.or(style.active).unwrap_or(ACTIVE_COLOR)
    }

    fn resolved_idle_width(&self) -> f32 {
        let style = self.style.unwrap_or_default();
        self.idle_width.or(style.idle_width).unwrap_or(LINE_IDLE)
    }

    fn resolved_active_width(&self) -> f32 {
        let style = self.style.unwrap_or_default();
        self.active_width
            .or(style.active_width)
            .unwrap_or(LINE_ACTIVE)
    }

    fn clamped(&self, v: f32) -> f32 {
        v.clamp(self.min, self.max)
    }

    fn axis_pos(&self, pos: &Point) -> f32 {
        match self.axis {
            Axis::Horizontal => pos.x,
            Axis::Vertical => pos.y,
        }
    }
}

impl View for ResizeHandle {
    fn reuse_state(&self, old: &Self) {
        self.drag.set(old.drag.get());
    }

    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn is_leaf(&self) -> bool {
        true
    }

    fn captures_pointer(&self) -> bool {
        true
    }

    fn has_visual_state(&self) -> bool {
        true
    }

    fn cursor_icon(&self, _state: VisualState) -> CursorIcon {
        match self.axis {
            Axis::Horizontal => CursorIcon::ColResize,
            Axis::Vertical => CursorIcon::RowResize,
        }
    }

    fn layout_style(&self) -> LayoutStyle {
        match self.axis {
            Axis::Horizontal => LayoutStyle {
                width: Length::Px(self.hit),
                height: Length::Fill(1.0),
                margin: self.margin,
                ..LayoutStyle::default()
            },
            Axis::Vertical => LayoutStyle {
                width: Length::Fill(1.0),
                height: Length::Px(self.hit),
                margin: self.margin,
                ..LayoutStyle::default()
            },
        }
    }

    fn intrinsic_size(&self, _ctx: &mut LayoutCtx) -> Option<Size> {
        Some(match self.axis {
            Axis::Horizontal => Size::new(self.hit, 0.0),
            Axis::Vertical => Size::new(0.0, self.hit),
        })
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
                let p = self.axis_pos(pos);
                self.drag.set(Some(DragState {
                    origin: p,
                    value: self.value.get_untracked(),
                }));
                ctx.capture_pointer();
                ctx.handled();
            }
            Event::Pointer(PointerEvent::Move { pos, .. }) => {
                if let Some(st) = self.drag.get() {
                    let p = self.axis_pos(pos);
                    let delta = if self.invert {
                        st.origin - p
                    } else {
                        p - st.origin
                    };
                    self.value.set(self.clamped(st.value + delta));
                    ctx.handled();
                }
            }
            Event::Pointer(PointerEvent::Up {
                button: PointerButton::Left,
                ..
            }) => {
                self.drag.set(None);
            }
            Event::Pointer(PointerEvent::Cancel) => {
                self.drag.set(None);
            }
            _ => {}
        }
    }

    fn paint_self(&self, ctx: &mut PaintCtx) {
        let r = ctx.rect;
        if r.is_empty() {
            return;
        }

        let gutter = self.resolved_gutter();
        if gutter.a > 0.0 {
            ctx.quad(r, gutter, 0.0);
        }
        let active = ctx.state.hovered || ctx.state.pressed || self.drag.get().is_some();
        let (w, c) = if active {
            (self.resolved_active_width(), self.resolved_active())
        } else {
            (self.resolved_idle_width(), self.resolved_idle())
        };
        match self.axis {
            Axis::Horizontal => {
                let cx = r.left() + r.size.width * 0.5;
                ctx.quad(
                    Rect::from_xywh(cx - w * 0.5, r.top(), w, r.size.height),
                    c,
                    0.0,
                );
            }
            Axis::Vertical => {
                let cy = r.top() + r.size.height * 0.5;
                ctx.quad(
                    Rect::from_xywh(r.left(), cy - w * 0.5, r.size.width, w),
                    c,
                    0.0,
                );
            }
        }
    }
}

fn sanitize_margin(v: f32) -> f32 {
    if v.is_finite() { v.max(0.0) } else { 0.0 }
}
