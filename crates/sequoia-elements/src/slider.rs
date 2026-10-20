use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Instant;

use sequoia_core::{Color, Edges, Length, Rect, Size};
use sequoia_input::{CursorIcon, Event, EventPhase, Key, PointerButton, PointerEvent};
use sequoia_layout::{LayoutCtx, LayoutStyle};
use sequoia_reactive::Signal;
use sequoia_style::{SliderStyle, VisualState};
use sequoia_view::{EventCtx, PaintCtx, Unit, View};

pub const TRACK_COLOR: Color = Color::rgb8(0x3E, 0x3E, 0x3E);

pub const FILL_COLOR: Color = Color::rgb8(0xC9, 0xC9, 0xC9);

pub const THUMB_OUTLINE: Color = Color::rgb8(0xC9, 0xC9, 0xC9);

pub const THUMB_CENTER: Color = Color::rgb8(0x12, 0x12, 0x12);

const THUMB: f32 = 14.0;

const THUMB_BORDER: f32 = 2.0;

const THUMB_GROW: f32 = 1.5;

const ANIM_SECS: f32 = 0.15;
const ANIM_MIN_T: f32 = 0.08;

const TRACK_H: f32 = 4.0;

const HEIGHT: f32 = 20.0;

const KEY_STEP: f32 = 0.05;

#[derive(Clone, Copy)]
struct Anim {
    from: f32,
    to: f32,
    start: Option<Instant>,
}

thread_local! {
    static TEST_NOW: RefCell<Option<Instant>> = const { RefCell::new(None) };
}

fn now() -> Instant {
    let frozen: Option<Instant> = TEST_NOW.with(|c| *c.borrow());
    frozen.unwrap_or_else(Instant::now)
}

#[doc(hidden)]
pub fn __set_test_now(t: Option<Instant>) {
    TEST_NOW.with(|c| *c.borrow_mut() = t);
}

pub struct Slider {
    value: Signal<f32>,
    min: f32,
    max: f32,
    track_height: Option<f32>,
    track: Option<Color>,
    fill: Option<Color>,
    thumb_center: Option<Color>,
    thumb_outline: Option<Color>,
    thumb_size: Option<f32>,
    appearance: Option<SliderStyle>,
    style: LayoutStyle,
    grow: Signal<f32>,
    anim: Rc<Cell<Anim>>,
    margin: Edges,

    gesture_start: Rc<Cell<Option<f32>>>,

    on_change_start: Option<Rc<dyn Fn()>>,

    on_commit: Option<Rc<dyn Fn(f32)>>,
}

impl Slider {
    pub fn new(value: Signal<f32>) -> Self {
        Self {
            value,
            min: 0.0,
            max: 1.0,
            track_height: None,
            track: None,
            fill: None,
            thumb_center: None,
            thumb_outline: None,
            thumb_size: None,
            appearance: None,
            style: LayoutStyle {
                width: Length::Fill(1.0),
                height: Length::Px(HEIGHT),
                ..LayoutStyle::default()
            },
            grow: Signal::new(0.0),
            anim: Rc::new(Cell::new(Anim {
                from: 0.0,
                to: 0.0,
                start: None,
            })),
            gesture_start: Rc::new(Cell::new(None)),
            on_change_start: None,
            on_commit: None,
            margin: Edges::ZERO,
        }
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

    pub fn appearance(mut self, style: SliderStyle) -> Self {
        self.appearance = Some(style);
        self
    }

    pub fn track_color(mut self, color: Color) -> Self {
        self.track = Some(color);
        self
    }

    pub fn fill_color(mut self, color: Color) -> Self {
        self.fill = Some(color);
        self
    }

    pub fn thumb_center(mut self, color: Color) -> Self {
        self.thumb_center = Some(color);
        self
    }

    pub fn thumb_outline(mut self, color: Color) -> Self {
        self.thumb_outline = Some(color);
        self
    }

    pub fn thumb_size(mut self, size: f32) -> Self {
        self.thumb_size = Some(size.max(1.0));
        self
    }

    pub fn track_height(mut self, height: f32) -> Self {
        self.track_height = Some(height.max(1.0));
        self
    }

    fn resolved_track_height(&self) -> f32 {
        self.track_height
            .or_else(|| self.appearance.and_then(|s| s.track_height))
            .unwrap_or(TRACK_H)
    }

    fn resolved_thumb_size(&self) -> f32 {
        self.thumb_size
            .or_else(|| self.appearance.and_then(|s| s.thumb_size))
            .unwrap_or(THUMB)
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.style = self.style.height(height.into());
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.style = self.style.width(width.into());
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

    fn fire_start(&self) {
        self.gesture_start.set(Some(self.value.get_untracked()));
        if let Some(f) = &self.on_change_start {
            f();
        }
    }

    fn fire_commit(&self) {
        if let Some(start) = self.gesture_start.take() {
            let v = self.value.get_untracked();
            if v != start
                && let Some(f) = &self.on_commit
            {
                f(v);
            }
        }
    }

    fn span(&self) -> f32 {
        (self.max - self.min).max(f32::EPSILON)
    }

    fn ratio(&self) -> f32 {
        ((self.value.get_untracked() - self.min) / self.span()).clamp(0.0, 1.0)
    }

    fn set_ratio(&self, t: f32) {
        let v = self.min + t.clamp(0.0, 1.0) * self.span();

        if v.is_finite() {
            self.value.set(v);
        }
    }

    fn thumb_x(&self, rect: Rect) -> f32 {
        let thumb = self.resolved_thumb_size();
        rect.left() + thumb * 0.5 + self.ratio() * (rect.size.width - thumb).max(0.0)
    }
}

impl View for Slider {
    fn body(&self) -> impl View + 'static {
        let _ = self.value.get();
        let _ = self.grow.get();
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

    fn cursor_icon(&self, state: VisualState) -> CursorIcon {
        if state.pressed {
            CursorIcon::Grabbing
        } else {
            CursorIcon::Grab
        }
    }

    fn layout_style(&self) -> LayoutStyle {
        let mut style = self.style;
        style.margin = self.margin;
        style
    }

    fn intrinsic_size(&self, _ctx: &mut LayoutCtx) -> Option<Size> {
        None
    }

    fn reuse_state(&self, old: &Self) {
        self.gesture_start.set(old.gesture_start.get());
        self.grow.set(old.grow.get_untracked());
        self.anim.set(old.anim.get());
    }

    fn on_event(&self, event: &Event, ctx: &mut EventCtx) {
        if ctx.phase != EventPhase::Target {
            return;
        }
        let thumb = self.resolved_thumb_size();
        match event {
            Event::Pointer(PointerEvent::Down {
                button: PointerButton::Left,
                pos,
                ..
            }) => {
                self.fire_start();
                let t = (pos.x - ctx.rect.left() - thumb * 0.5)
                    / (ctx.rect.size.width - thumb).max(1.0);
                self.set_ratio(t);
                ctx.capture_pointer();
                ctx.handled();
            }
            Event::Pointer(PointerEvent::Move { pos, .. }) => {
                if ctx.state.pressed {
                    let t = (pos.x - ctx.rect.left() - thumb * 0.5)
                        / (ctx.rect.size.width - thumb).max(1.0);
                    self.set_ratio(t);
                    ctx.handled();
                }
            }
            Event::Pointer(PointerEvent::Up { .. }) => {
                self.fire_commit();
            }
            Event::Key(key) => {
                let step = match key.key {
                    Key::ARROW_LEFT | Key::ARROW_DOWN => -KEY_STEP,
                    Key::ARROW_RIGHT | Key::ARROW_UP => KEY_STEP,
                    _ => return,
                };
                self.fire_start();
                self.set_ratio(self.ratio() + step);
                self.fire_commit();
                ctx.handled();
            }
            _ => {}
        }
    }

    fn paint_self(&self, ctx: &mut PaintCtx) {
        let rect = ctx.rect;
        let cy = rect.top() + rect.size.height * 0.5;
        let style = self.appearance.unwrap_or_default();
        let track = self.track.or(style.track).unwrap_or(TRACK_COLOR);
        let fill = self.fill.or(style.fill).unwrap_or(FILL_COLOR);
        let thumb_outline = self
            .thumb_outline
            .or(style.thumb_outline)
            .unwrap_or(THUMB_OUTLINE);
        let thumb_center = self
            .thumb_center
            .or(style.thumb_center)
            .unwrap_or(THUMB_CENTER);
        let thumb = self.resolved_thumb_size();
        let thumb_radius = thumb * 0.5;
        let th = self.resolved_track_height().min(rect.size.height).max(0.0);
        if th <= 0.0 || rect.size.width <= 0.0 {
            return;
        }
        let tx = self.thumb_x(rect);

        ctx.quad(
            Rect::from_xywh(rect.left(), cy - th * 0.5, rect.size.width, th),
            track,
            th * 0.5,
        );
        let fill_w = (tx - rect.left()).max(0.0);
        if fill_w > 0.0 {
            ctx.quad(
                Rect::from_xywh(rect.left(), cy - th * 0.5, fill_w, th),
                fill,
                th * 0.5,
            );
        }

        let target = if ctx.state.hovered { 1.0 } else { 0.0 };
        let mut st = self.anim.get();
        if (st.to - target).abs() > f32::EPSILON {
            st = Anim {
                from: self.grow.get_untracked(),
                to: target,
                start: Some(now()),
            };
            self.anim.set(st);
        }
        if let Some(t0) = st.start {
            let t = (now().saturating_duration_since(t0).as_secs_f32() / ANIM_SECS)
                .clamp(ANIM_MIN_T, 1.0);
            let eased = 1.0 - (1.0 - t).powi(3);
            let v = st.from + (st.to - st.from) * eased;
            if t >= 1.0 {
                self.anim.set(Anim {
                    from: st.to,
                    to: st.to,
                    start: None,
                });
            }
            self.grow.set(v);
        }
        let bw = THUMB_BORDER + self.grow.get_untracked() * THUMB_GROW;

        ctx.quad(
            Rect::from_xywh(tx - thumb_radius, cy - thumb_radius, thumb, thumb),
            thumb_outline,
            thumb_radius,
        );
        let inner = (thumb - 2.0 * bw).max(0.0);
        if inner > 0.0 {
            ctx.quad(
                Rect::from_xywh(tx - inner * 0.5, cy - inner * 0.5, inner, inner),
                thumb_center,
                (thumb_radius - bw).max(0.0),
            );
        }
    }
}

fn sanitize_margin(v: f32) -> f32 {
    if v.is_finite() { v.max(0.0) } else { 0.0 }
}
