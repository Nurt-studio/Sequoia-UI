use std::rc::Rc;

use sequoia_core::{Color, Constraints, Length, Point, Rect, Size, Vec2};
use sequoia_input::{Event, EventPhase, Key, Modifiers, PointerEvent};
use sequoia_layout::{LayoutCtx, LayoutStyle};

use crate::any::AnyView;
use crate::runtime::{ElementId, Runtime};
use crate::view::{EventCtx, PaintCtx, View};

const DEFAULT_SCROLLBAR_W: f32 = 8.0;

const SCROLLBAR_PAD: f32 = 2.0;

const THUMB_MIN: f32 = 24.0;

const DEFAULT_STEP: f32 = 40.0;

const HOVER_EXTRA: f32 = 4.0;

const TOUCH_PAN_THRESHOLD_SQ: f32 = 64.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum ScrollAxis {
    #[default]
    Vertical,

    Horizontal,

    Both,
}

impl ScrollAxis {
    pub fn scrolls_x(self) -> bool {
        matches!(self, Self::Horizontal | Self::Both)
    }

    pub fn scrolls_y(self) -> bool {
        matches!(self, Self::Vertical | Self::Both)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum ScrollbarVisibility {
    #[default]
    Auto,

    Always,

    Never,

    OnHover,

    Overlay,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Drag {
    V {
        grab: f32,
    },

    H {
        grab: f32,
    },

    Touch {
        id: u64,
        start: Point,
        last: Point,
        moved: bool,
    },
}

struct Bars {
    v_track: Option<Rect>,
    v_thumb: Option<Rect>,
    h_track: Option<Rect>,
    h_thumb: Option<Rect>,
}

#[derive(Clone, Copy, Default)]
struct ScrollState {
    offset: Vec2,
    content: Size,
    viewport: Size,
    drag: Option<Drag>,
    arranged: Vec2,

    arranged_epoch: u64,
    frame: Rect,

    bar_hover: bool,
}

#[derive(Clone)]
pub struct ScrollView {
    child: AnyView,
    axis: ScrollAxis,
    scrollbar: ScrollbarVisibility,
    bar_width: f32,
    track_color: Option<Color>,
    thumb_color: Option<Color>,
    step: f32,
    on_scroll: Option<Rc<dyn Fn(Vec2)>>,
}

impl ScrollView {
    pub fn new(child: impl View + 'static) -> Self {
        Self {
            child: AnyView::new(child),
            axis: ScrollAxis::Vertical,
            scrollbar: ScrollbarVisibility::Auto,
            bar_width: DEFAULT_SCROLLBAR_W,
            track_color: None,
            thumb_color: None,
            step: DEFAULT_STEP,
            on_scroll: None,
        }
    }

    pub fn with_child(&self, child: impl View + 'static) -> Self {
        let mut out = self.clone();
        out.child = AnyView::new(child);
        out
    }

    pub fn axis(mut self, axis: ScrollAxis) -> Self {
        self.axis = axis;
        self
    }

    pub fn vertical(mut self) -> Self {
        self.axis = ScrollAxis::Vertical;
        self
    }

    pub fn horizontal(mut self) -> Self {
        self.axis = ScrollAxis::Horizontal;
        self
    }

    pub fn both(mut self) -> Self {
        self.axis = ScrollAxis::Both;
        self
    }

    pub fn scrollbar_visibility(mut self, visibility: ScrollbarVisibility) -> Self {
        self.scrollbar = visibility;
        self
    }

    pub fn show_scrollbar(mut self, show: bool) -> Self {
        self.scrollbar = if show {
            ScrollbarVisibility::Always
        } else {
            ScrollbarVisibility::Never
        };
        self
    }

    pub fn scrollbar_width(mut self, width: f32) -> Self {
        self.bar_width = width;
        self
    }

    pub fn scrollbar_track(mut self, color: Color) -> Self {
        self.track_color = Some(color);
        self
    }

    pub fn scrollbar_thumb(mut self, color: Color) -> Self {
        self.thumb_color = Some(color);
        self
    }

    pub fn scroll_step(mut self, step: f32) -> Self {
        self.step = step.max(0.0);
        self
    }

    pub fn on_scroll(mut self, handler: impl Fn(Vec2) + 'static) -> Self {
        self.on_scroll = Some(Rc::new(handler));
        self
    }

    pub fn scroll_axis(&self) -> ScrollAxis {
        self.axis
    }

    fn load(rt: &Runtime, el: ElementId) -> ScrollState {
        rt.read_state::<ScrollState, ScrollState>(el, |s| *s)
    }

    pub fn offset(&self, rt: &Runtime, el: ElementId) -> f32 {
        Self::load(rt, el).offset.y
    }

    pub fn offset_x(&self, rt: &Runtime, el: ElementId) -> f32 {
        Self::load(rt, el).offset.x
    }

    pub fn offset_y(&self, rt: &Runtime, el: ElementId) -> f32 {
        Self::load(rt, el).offset.y
    }

    pub fn offset_vec(&self, rt: &Runtime, el: ElementId) -> Vec2 {
        Self::load(rt, el).offset
    }

    pub fn content_height(&self, rt: &Runtime, el: ElementId) -> f32 {
        Self::load(rt, el).content.height
    }

    pub fn content_width(&self, rt: &Runtime, el: ElementId) -> f32 {
        Self::load(rt, el).content.width
    }

    pub fn content_size(&self, rt: &Runtime, el: ElementId) -> Size {
        Self::load(rt, el).content
    }

    pub fn viewport_size(&self, rt: &Runtime, el: ElementId) -> Size {
        Self::load(rt, el).viewport
    }

    pub fn viewport_width(&self, rt: &Runtime, el: ElementId) -> f32 {
        Self::load(rt, el).viewport.width
    }

    pub fn viewport_height(&self, rt: &Runtime, el: ElementId) -> f32 {
        Self::load(rt, el).viewport.height
    }

    pub fn max_offset(&self, rt: &Runtime, el: ElementId) -> f32 {
        let st = Self::load(rt, el);
        (st.content.height - st.viewport.height).max(0.0)
    }

    pub fn max_offset_x(&self, rt: &Runtime, el: ElementId) -> f32 {
        let st = Self::load(rt, el);
        (st.content.width - st.viewport.width).max(0.0)
    }

    pub fn max_offset_y(&self, rt: &Runtime, el: ElementId) -> f32 {
        self.max_offset(rt, el)
    }

    pub fn max_offset_vec(&self, rt: &Runtime, el: ElementId) -> Vec2 {
        Vec2::new(self.max_offset_x(rt, el), self.max_offset(rt, el))
    }

    pub fn can_scroll_x(&self, rt: &Runtime, el: ElementId) -> bool {
        self.axis.scrolls_x() && self.max_offset_x(rt, el) > 0.0
    }

    pub fn can_scroll_y(&self, rt: &Runtime, el: ElementId) -> bool {
        self.axis.scrolls_y() && self.max_offset(rt, el) > 0.0
    }

    pub fn scroll_fraction(&self, rt: &Runtime, el: ElementId) -> Vec2 {
        Vec2::new(
            self.scroll_fraction_x(rt, el),
            self.scroll_fraction_y(rt, el),
        )
    }

    pub fn scroll_fraction_x(&self, rt: &Runtime, el: ElementId) -> f32 {
        let max = self.max_offset_x(rt, el);
        if max > 0.0 {
            (Self::load(rt, el).offset.x / max).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    pub fn scroll_fraction_y(&self, rt: &Runtime, el: ElementId) -> f32 {
        let max = self.max_offset(rt, el);
        if max > 0.0 {
            (Self::load(rt, el).offset.y / max).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    pub fn scroll_to(&self, rt: &Runtime, el: ElementId, offset: Vec2) {
        let max = self.max_offset_vec(rt, el);
        let o = Self::load(rt, el).offset;
        let next = Vec2::new(
            if self.axis.scrolls_x() {
                offset.x.clamp(0.0, max.x)
            } else {
                o.x.clamp(0.0, max.x)
            },
            if self.axis.scrolls_y() {
                offset.y.clamp(0.0, max.y)
            } else {
                o.y.clamp(0.0, max.y)
            },
        );
        self.apply(rt, el, next);
    }

    pub fn scroll_to_xy(&self, rt: &Runtime, el: ElementId, x: f32, y: f32) {
        self.scroll_to(rt, el, Vec2::new(x, y));
    }

    pub fn scroll_to_x(&self, rt: &Runtime, el: ElementId, x: f32) {
        let o = Self::load(rt, el).offset;
        self.scroll_to(rt, el, Vec2::new(x, o.y));
    }

    pub fn scroll_to_y(&self, rt: &Runtime, el: ElementId, y: f32) {
        let o = Self::load(rt, el).offset;
        self.scroll_to(rt, el, Vec2::new(o.x, y));
    }

    pub fn scroll_by(&self, rt: &Runtime, el: ElementId, delta: Vec2) {
        let o = Self::load(rt, el).offset;
        self.scroll_to(rt, el, Vec2::new(o.x + delta.x, o.y + delta.y));
    }

    pub fn scroll_to_top(&self, rt: &Runtime, el: ElementId) {
        self.scroll_to_y(rt, el, 0.0);
    }

    pub fn scroll_to_bottom(&self, rt: &Runtime, el: ElementId) {
        self.scroll_to_y(rt, el, self.max_offset(rt, el));
    }

    pub fn scroll_into_view(&self, rt: &Runtime, el: ElementId, rect: Rect) {
        let st = Self::load(rt, el);
        let mut next = st.offset;
        if self.axis.scrolls_y() {
            let vh = st.viewport.height;
            if rect.top() < next.y {
                next.y = rect.top();
            } else if rect.bottom() > next.y + vh {
                next.y = rect.bottom() - vh;
            }
        }
        if self.axis.scrolls_x() {
            let vw = st.viewport.width;
            if rect.left() < next.x {
                next.x = rect.left();
            } else if rect.right() > next.x + vw {
                next.x = rect.right() - vw;
            }
        }
        self.scroll_to(rt, el, next);
    }

    pub fn scroll_element_into_view(&self, rt: &Runtime, el: ElementId, child: ElementId) {
        let Some(r) = rt.rect_of(child) else {
            return;
        };
        let (frame, off) = rt.read_state(el, |s: &ScrollState| (s.frame, s.offset));
        let content = Rect::from_xywh(
            r.left() - frame.left() + off.x,
            r.top() - frame.top() + off.y,
            r.size.width,
            r.size.height,
        );
        self.scroll_into_view(rt, el, content);
    }

    pub fn scroll_to_left(&self, rt: &Runtime, el: ElementId) {
        self.scroll_to_x(rt, el, 0.0);
    }

    pub fn scroll_to_right(&self, rt: &Runtime, el: ElementId) {
        self.scroll_to_x(rt, el, self.max_offset_x(rt, el));
    }

    pub fn page_up(&self, rt: &Runtime, el: ElementId) {
        let st = Self::load(rt, el);
        self.scroll_to(
            rt,
            el,
            Vec2::new(st.offset.x, st.offset.y - st.viewport.height),
        );
    }

    pub fn page_down(&self, rt: &Runtime, el: ElementId) {
        let st = Self::load(rt, el);
        self.scroll_to(
            rt,
            el,
            Vec2::new(st.offset.x, st.offset.y + st.viewport.height),
        );
    }

    pub fn page_left(&self, rt: &Runtime, el: ElementId) {
        let st = Self::load(rt, el);
        self.scroll_to(
            rt,
            el,
            Vec2::new(st.offset.x - st.viewport.width, st.offset.y),
        );
    }

    pub fn page_right(&self, rt: &Runtime, el: ElementId) {
        let st = Self::load(rt, el);
        self.scroll_to(
            rt,
            el,
            Vec2::new(st.offset.x + st.viewport.width, st.offset.y),
        );
    }

    pub fn ensure_visible(&self, rt: &Runtime, el: ElementId, rect: Rect) -> bool {
        let st = Self::load(rt, el);
        let vp = st.viewport;
        let o = st.offset;
        let max = self.max_offset_vec(rt, el);
        let mut next = o;
        if self.axis.scrolls_x() && vp.width > 0.0 {
            if rect.size.width >= vp.width || rect.left() < o.x {
                next.x = rect.left();
            } else if rect.right() > o.x + vp.width {
                next.x = rect.right() - vp.width;
            }
            next.x = next.x.clamp(0.0, max.x);
        }
        if self.axis.scrolls_y() && vp.height > 0.0 {
            if rect.size.height >= vp.height || rect.top() < o.y {
                next.y = rect.top();
            } else if rect.bottom() > o.y + vp.height {
                next.y = rect.bottom() - vp.height;
            }
            next.y = next.y.clamp(0.0, max.y);
        }
        if next != o {
            self.apply(rt, el, next);
            true
        } else {
            false
        }
    }

    fn apply(&self, rt: &Runtime, el: ElementId, next: Vec2) {
        let prev = rt.with_state::<ScrollState, Vec2>(el, |s| {
            let prev = s.offset;
            s.offset = next;
            prev
        });
        if next != prev
            && let Some(cb) = &self.on_scroll
        {
            cb(next);
        }
    }

    fn shows_v(&self, st: &ScrollState) -> bool {
        if !self.axis.scrolls_y() || self.bar_width <= 0.0 {
            return false;
        }
        match self.scrollbar {
            ScrollbarVisibility::Never => false,
            ScrollbarVisibility::Always => true,
            ScrollbarVisibility::Auto => st.content.height > st.viewport.height,
            ScrollbarVisibility::OnHover | ScrollbarVisibility::Overlay => {
                (st.bar_hover || st.drag.is_some()) && st.content.height > st.viewport.height
            }
        }
    }

    fn shows_h(&self, st: &ScrollState) -> bool {
        if !self.axis.scrolls_x() || self.bar_width <= 0.0 {
            return false;
        }
        match self.scrollbar {
            ScrollbarVisibility::Never => false,
            ScrollbarVisibility::Always => true,
            ScrollbarVisibility::Auto => st.content.width > st.viewport.width,
            ScrollbarVisibility::OnHover | ScrollbarVisibility::Overlay => {
                (st.bar_hover || st.drag.is_some()) && st.content.width > st.viewport.width
            }
        }
    }

    fn gutter(&self) -> f32 {
        (self.bar_width + SCROLLBAR_PAD * 2.0 + HOVER_EXTRA).max(0.0)
    }

    fn in_gutter(&self, rect: Rect, pos: Point) -> bool {
        let g = self.gutter();
        if g <= 0.0 {
            return false;
        }
        let v = self.axis.scrolls_y()
            && pos.x >= rect.right() - g
            && pos.x <= rect.right()
            && pos.y >= rect.top()
            && pos.y <= rect.bottom();
        let h = self.axis.scrolls_x()
            && pos.y >= rect.bottom() - g
            && pos.y <= rect.bottom()
            && pos.x >= rect.left()
            && pos.x <= rect.right();
        v || h
    }

    fn track_hover(&self, rt: &Runtime, el: ElementId, rect: Rect, pos: Point) -> bool {
        if !matches!(
            self.scrollbar,
            ScrollbarVisibility::OnHover | ScrollbarVisibility::Overlay
        ) {
            return false;
        }
        let hov = self.in_gutter(rect, pos);
        rt.with_state(el, |s: &mut ScrollState| {
            let old = s.bar_hover;
            s.bar_hover = hov;
            old != hov
        })
    }

    fn v_bar(&self, st: &ScrollState, viewport: Rect) -> Option<(Rect, Rect)> {
        if !self.shows_v(st) {
            return None;
        }
        let w = self.bar_width;
        let track = Rect::from_xywh(
            viewport.right() - w - SCROLLBAR_PAD,
            viewport.top() + SCROLLBAR_PAD,
            w,
            (viewport.size.height - SCROLLBAR_PAD * 2.0).max(0.0),
        );
        if track.is_empty() {
            return None;
        }
        Some((track, self.v_thumb(st, track)))
    }

    fn v_thumb(&self, st: &ScrollState, track: Rect) -> Rect {
        let content = st.content.height;
        let vh = st.viewport.height;
        let max_off = (content - vh).max(0.0);
        if max_off <= 0.0 || content <= 0.0 {
            return track;
        }
        let thumb_h = (track.size.height * vh / content)
            .max(THUMB_MIN)
            .min(track.size.height);
        let travel = (track.size.height - thumb_h).max(0.0);
        let t = (st.offset.y / max_off).clamp(0.0, 1.0);
        Rect::from_xywh(
            track.left(),
            track.top() + t * travel,
            track.size.width,
            thumb_h,
        )
    }

    fn h_bar(&self, st: &ScrollState, viewport: Rect) -> Option<(Rect, Rect)> {
        if !self.shows_h(st) {
            return None;
        }
        let h = self.bar_width;
        let track = Rect::from_xywh(
            viewport.left() + SCROLLBAR_PAD,
            viewport.bottom() - h - SCROLLBAR_PAD,
            (viewport.size.width - SCROLLBAR_PAD * 2.0).max(0.0),
            h,
        );
        if track.is_empty() {
            return None;
        }
        Some((track, self.h_thumb(st, track)))
    }

    fn h_thumb(&self, st: &ScrollState, track: Rect) -> Rect {
        let content = st.content.width;
        let vw = st.viewport.width;
        let max_off = (content - vw).max(0.0);
        if max_off <= 0.0 || content <= 0.0 {
            return track;
        }
        let thumb_w = (track.size.width * vw / content)
            .max(THUMB_MIN)
            .min(track.size.width);
        let travel = (track.size.width - thumb_w).max(0.0);
        let t = (st.offset.x / max_off).clamp(0.0, 1.0);
        Rect::from_xywh(
            track.left() + t * travel,
            track.top(),
            thumb_w,
            track.size.height,
        )
    }

    fn bars(&self, st: &ScrollState, viewport: Rect) -> Bars {
        let (v_track, v_thumb) = self.v_bar(st, viewport).unzip();
        let (h_track, h_thumb) = self.h_bar(st, viewport).unzip();
        Bars {
            v_track,
            v_thumb,
            h_track,
            h_thumb,
        }
    }

    fn drag_offset(
        &self,
        st: &ScrollState,
        bars: &Bars,
        drag: Drag,
        x: f32,
        y: f32,
    ) -> Option<Vec2> {
        let o = st.offset;
        match drag {
            Drag::Touch { .. } => None,
            Drag::V { grab } => {
                let (track, thumb) = bars.v_track.zip(bars.v_thumb)?;
                let travel = (track.size.height - thumb.size.height).max(0.0);
                let max_off = (st.content.height - st.viewport.height).max(0.0);
                if travel <= 0.0 || max_off <= 0.0 {
                    return None;
                }
                let top = (y - grab - track.top()).clamp(0.0, travel);
                Some(Vec2::new(o.x, top / travel * max_off))
            }
            Drag::H { grab } => {
                let (track, thumb) = bars.h_track.zip(bars.h_thumb)?;
                let travel = (track.size.width - thumb.size.width).max(0.0);
                let max_off = (st.content.width - st.viewport.width).max(0.0);
                if travel <= 0.0 || max_off <= 0.0 {
                    return None;
                }
                let left = (x - grab - track.left()).clamp(0.0, travel);
                Some(Vec2::new(left / travel * max_off, o.y))
            }
        }
    }

    fn on_wheel(
        &self,
        rt: &Runtime,
        el: ElementId,
        delta: Vec2,
        modifiers: Modifiers,
        ctx: &mut EventCtx,
    ) {
        let (mut dx, mut dy) = (delta.x, delta.y);
        if modifiers.contains(Modifiers::SHIFT) && self.axis.scrolls_x() {
            dx = dy;
            dy = 0.0;
        }
        let st = Self::load(rt, el);
        let o = st.offset;
        let max = Vec2::new(
            (st.content.width - st.viewport.width).max(0.0),
            (st.content.height - st.viewport.height).max(0.0),
        );
        let nx = if self.axis.scrolls_x() {
            (o.x - dx).clamp(0.0, max.x)
        } else {
            o.x.clamp(0.0, max.x)
        };
        let ny = if self.axis.scrolls_y() {
            (o.y - dy).clamp(0.0, max.y)
        } else {
            o.y.clamp(0.0, max.y)
        };
        let next = Vec2::new(nx, ny);
        if next != o {
            self.apply(rt, el, next);
            ctx.request_frame();
            ctx.handled();
        }
    }

    fn on_key(
        &self,
        rt: &Runtime,
        el: ElementId,
        key: Key,
        modifiers: Modifiers,
        ctx: &mut EventCtx,
    ) {
        let st = Self::load(rt, el);
        let o = st.offset;
        let max = Vec2::new(
            (st.content.width - st.viewport.width).max(0.0),
            (st.content.height - st.viewport.height).max(0.0),
        );
        let vp = st.viewport;
        let step = if self.step.is_finite() {
            self.step.max(0.0)
        } else {
            0.0
        };
        let next = match (key, modifiers) {
            (Key::ARROW_UP, Modifiers::NONE) if self.axis.scrolls_y() => Vec2::new(o.x, o.y - step),
            (Key::ARROW_DOWN, Modifiers::NONE) if self.axis.scrolls_y() => {
                Vec2::new(o.x, o.y + step)
            }
            (Key::ARROW_LEFT, Modifiers::NONE) if self.axis.scrolls_x() => {
                Vec2::new(o.x - step, o.y)
            }
            (Key::ARROW_RIGHT, Modifiers::NONE) if self.axis.scrolls_x() => {
                Vec2::new(o.x + step, o.y)
            }
            (Key::PAGE_UP, Modifiers::NONE) if self.axis.scrolls_y() => {
                Vec2::new(o.x, o.y - vp.height)
            }
            (Key::PAGE_DOWN, Modifiers::NONE) if self.axis.scrolls_y() => {
                Vec2::new(o.x, o.y + vp.height)
            }
            (Key::HOME, Modifiers::NONE) if self.axis.scrolls_y() => Vec2::new(o.x, 0.0),
            (Key::END, Modifiers::NONE) if self.axis.scrolls_y() => Vec2::new(o.x, max.y),
            (Key::HOME, m) if m == Modifiers::CONTROL => Vec2::new(0.0, 0.0),
            (Key::END, m) if m == Modifiers::CONTROL => Vec2::new(max.x, max.y),
            _ => return,
        };
        let next = Vec2::new(next.x.clamp(0.0, max.x), next.y.clamp(0.0, max.y));
        if next != o {
            self.apply(rt, el, next);
            ctx.request_frame();
            ctx.handled();
        }
    }
}

impl View for ScrollView {
    fn body(&self) -> impl View + 'static {
        crate::views::Unit
    }

    fn children_override(&self) -> Option<Vec<AnyView>> {
        Some(vec![self.child.clone()])
    }

    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fill(1.0),
            height: Length::Fill(1.0),
            ..LayoutStyle::default()
        }
    }

    fn intrinsic_size(&self, _ctx: &mut LayoutCtx) -> Option<Size> {
        None
    }

    fn contain_size(&self) -> bool {
        true
    }

    fn arrange_override(
        &self,
        rt: &Runtime,
        el: ElementId,
        rect: Rect,
        ctx: &mut LayoutCtx,
    ) -> bool {
        let children = rt.children(el);
        let Some(&child) = children.first() else {
            rt.with_state(el, |s: &mut ScrollState| {
                s.viewport = rect.size;
                s.content = Size::ZERO;
                s.frame = rect;
            });
            return true;
        };

        let gx = if self.scrollbar == ScrollbarVisibility::OnHover && self.axis.scrolls_y() {
            self.gutter()
        } else {
            0.0
        };
        let gy = if self.scrollbar == ScrollbarVisibility::OnHover && self.axis.scrolls_x() {
            self.gutter()
        } else {
            0.0
        };
        let max_w = if self.axis.scrolls_x() {
            f32::INFINITY
        } else {
            (rect.size.width - gx).max(0.0)
        };
        let max_h = if self.axis.scrolls_y() {
            f32::INFINITY
        } else {
            (rect.size.height - gy).max(0.0)
        };
        let unconstrained = Constraints::loose(Size::new(max_w, max_h));

        let content_size = rt.measure_node(child, unconstrained, ctx);

        let max = Vec2::new(
            (content_size.width - rect.size.width).max(0.0),
            (content_size.height - rect.size.height).max(0.0),
        );
        let off = rt.with_state(el, |s: &mut ScrollState| {
            s.offset = Vec2::new(s.offset.x.clamp(0.0, max.x), s.offset.y.clamp(0.0, max.y));
            s.offset
        });

        let (prev_frame, prev_content, prev_off, prev_epoch) = rt
            .read_state(el, |s: &ScrollState| {
                (s.frame, s.content, s.arranged, s.arranged_epoch)
            });
        let prev_content_w = if self.axis.scrolls_x() {
            prev_content.width
        } else {
            prev_content.width.max(prev_frame.size.width)
        };
        let prev_content_h = if self.axis.scrolls_y() {
            prev_content.height
        } else {
            prev_content.height.max(prev_frame.size.height)
        };
        let new_w = if self.axis.scrolls_x() {
            content_size.width
        } else {
            content_size.width.max(rect.size.width - gx)
        };
        let new_h = if self.axis.scrolls_y() {
            content_size.height
        } else {
            content_size.height.max(rect.size.height - gy)
        };

        rt.with_state(el, |s: &mut ScrollState| {
            s.viewport = rect.size;
            s.content = content_size;
        });

        if !prev_frame.size.is_empty()
            && prev_content_w == new_w
            && prev_content_h == new_h
            && prev_epoch == rt.layout_epoch()
        {
            let dx = (rect.left() - prev_frame.left()) - (off.x - prev_off.x);
            let dy = (rect.top() - prev_frame.top()) - (off.y - prev_off.y);
            if dx != 0.0 || dy != 0.0 {
                rt.translate_subtree(child, Vec2::new(dx, dy));
            }
            rt.with_state(el, |s: &mut ScrollState| {
                s.arranged = off;
                s.frame = rect;
            });
            return true;
        }

        let w = new_w;
        let h = new_h;
        let child_rect = Rect::from_xywh(rect.left() - off.x, rect.top() - off.y, w, h);
        let epoch = rt.layout_epoch();
        rt.arrange_node(child, child_rect, ctx);
        rt.with_state(el, |s: &mut ScrollState| {
            s.arranged = off;
            s.arranged_epoch = epoch;
            s.frame = rect;
        });
        true
    }

    fn clip_self(&self) -> bool {
        true
    }

    fn captures_pointer(&self) -> bool {
        false
    }

    fn wants_hover_updates(&self) -> bool {
        matches!(
            self.scrollbar,
            ScrollbarVisibility::OnHover | ScrollbarVisibility::Overlay
        )
    }

    fn captures_pointer_ctx(&self, rt: &Runtime, el: ElementId) -> bool {
        rt.read_state(el, |s: &ScrollState| s.drag.is_some())
    }

    fn on_event(&self, event: &Event, ctx: &mut EventCtx) {
        let rt = ctx.runtime().clone();
        let el = ctx.element();
        match event {
            Event::Pointer(PointerEvent::Wheel {
                delta, modifiers, ..
            }) => {
                if matches!(ctx.phase, EventPhase::Target | EventPhase::Bubble) {
                    self.on_wheel(&rt, el, *delta, *modifiers, ctx);
                }
            }
            Event::Pointer(PointerEvent::Down { pos, .. }) => {
                if !matches!(ctx.phase, EventPhase::Target | EventPhase::Bubble) {
                    return;
                }

                if self.track_hover(&rt, el, ctx.rect, *pos) {
                    ctx.request_frame();
                }
                let st = Self::load(&rt, el);
                if st.drag.is_some() {
                    return;
                }
                let bars = self.bars(&st, ctx.rect);

                if let Some(thumb) = bars.v_thumb
                    && thumb.contains(*pos)
                {
                    rt.with_state(el, |s: &mut ScrollState| {
                        s.drag = Some(Drag::V {
                            grab: pos.y - thumb.top(),
                        });
                    });
                    ctx.request_frame();
                    ctx.handled();
                    return;
                }
                if let Some(thumb) = bars.h_thumb
                    && thumb.contains(*pos)
                {
                    rt.with_state(el, |s: &mut ScrollState| {
                        s.drag = Some(Drag::H {
                            grab: pos.x - thumb.left(),
                        });
                    });
                    ctx.request_frame();
                    ctx.handled();
                    return;
                }
                if let Some(track) = bars.v_track
                    && track.contains(*pos)
                    && let Some(thumb) = bars.v_thumb
                {
                    let travel = (track.size.height - thumb.size.height).max(0.0);
                    let max_off = (st.content.height - st.viewport.height).max(0.0);
                    if travel > 0.0 && max_off > 0.0 {
                        let top =
                            (pos.y - thumb.size.height * 0.5 - track.top()).clamp(0.0, travel);
                        let o = st.offset;
                        self.apply(&rt, el, Vec2::new(o.x, top / travel * max_off));
                    }
                    rt.with_state(el, |s: &mut ScrollState| {
                        s.drag = Some(Drag::V {
                            grab: thumb.size.height * 0.5,
                        });
                    });
                    ctx.request_frame();
                    ctx.handled();
                    return;
                }
                if let Some(track) = bars.h_track
                    && track.contains(*pos)
                    && let Some(thumb) = bars.h_thumb
                {
                    let travel = (track.size.width - thumb.size.width).max(0.0);
                    let max_off = (st.content.width - st.viewport.width).max(0.0);
                    if travel > 0.0 && max_off > 0.0 {
                        let left =
                            (pos.x - thumb.size.width * 0.5 - track.left()).clamp(0.0, travel);
                        let o = st.offset;
                        self.apply(&rt, el, Vec2::new(left / travel * max_off, o.y));
                    }
                    rt.with_state(el, |s: &mut ScrollState| {
                        s.drag = Some(Drag::H {
                            grab: thumb.size.width * 0.5,
                        });
                    });
                    ctx.request_frame();
                    ctx.handled();
                }
            }
            Event::Pointer(PointerEvent::Move { pos, .. }) => {
                if ctx.phase != EventPhase::Target {
                    return;
                }
                let st = Self::load(&rt, el);
                if let Some(drag) = st.drag {
                    let bars = self.bars(&st, ctx.rect);
                    if let Some(next) = self.drag_offset(&st, &bars, drag, pos.x, pos.y) {
                        let max = Vec2::new(
                            (st.content.width - st.viewport.width).max(0.0),
                            (st.content.height - st.viewport.height).max(0.0),
                        );
                        let next = Vec2::new(next.x.clamp(0.0, max.x), next.y.clamp(0.0, max.y));
                        if next != st.offset {
                            self.apply(&rt, el, next);
                            ctx.request_frame();
                        }
                        ctx.handled();
                    }
                }
            }
            Event::Pointer(PointerEvent::Up { pos, .. }) => {
                if ctx.phase != EventPhase::Target {
                    return;
                }

                let mut reframed = false;
                if matches!(
                    self.scrollbar,
                    ScrollbarVisibility::OnHover | ScrollbarVisibility::Overlay
                ) && self.track_hover(&rt, el, ctx.rect, *pos)
                {
                    ctx.request_frame();
                    reframed = true;
                }
                let had = rt.with_state(el, |s: &mut ScrollState| s.drag.take().is_some());
                if had && !reframed {
                    ctx.request_frame();
                }
                if had {
                    ctx.handled();
                }
            }
            Event::Pointer(PointerEvent::TouchDown { id, pos, .. }) => {
                if !matches!(ctx.phase, EventPhase::Target | EventPhase::Bubble) {
                    return;
                }
                let st = Self::load(&rt, el);

                if st.drag.is_some() {
                    return;
                }

                rt.with_state(el, |s: &mut ScrollState| {
                    s.drag = Some(Drag::Touch {
                        id: *id,
                        start: *pos,
                        last: *pos,
                        moved: false,
                    });
                });
            }
            Event::Pointer(PointerEvent::TouchMove { id, pos, .. }) => {
                if !matches!(ctx.phase, EventPhase::Target | EventPhase::Bubble) {
                    return;
                }
                let st = Self::load(&rt, el);
                let Some(Drag::Touch {
                    id: did,
                    start,
                    last,
                    moved,
                }) = st.drag
                else {
                    return;
                };
                if did != *id {
                    return;
                }
                let dx_total = pos.x - start.x;
                let dy_total = pos.y - start.y;
                if !moved && dx_total * dx_total + dy_total * dy_total < TOUCH_PAN_THRESHOLD_SQ {
                    return;
                }
                if !moved {
                    ctx.capture_pointer();
                }

                let max = Vec2::new(
                    (st.content.width - st.viewport.width).max(0.0),
                    (st.content.height - st.viewport.height).max(0.0),
                );
                let next = Vec2::new(
                    if self.axis.scrolls_x() {
                        (st.offset.x - (pos.x - last.x)).clamp(0.0, max.x)
                    } else {
                        st.offset.x
                    },
                    if self.axis.scrolls_y() {
                        (st.offset.y - (pos.y - last.y)).clamp(0.0, max.y)
                    } else {
                        st.offset.y
                    },
                );
                rt.with_state(el, |s: &mut ScrollState| {
                    s.drag = Some(Drag::Touch {
                        id: *id,
                        start,
                        last: *pos,
                        moved: true,
                    });
                });
                if next != st.offset {
                    self.apply(&rt, el, next);
                    ctx.request_frame();
                }
                ctx.handled();
            }
            Event::Pointer(PointerEvent::TouchUp { id, .. }) => {
                if !matches!(ctx.phase, EventPhase::Target | EventPhase::Bubble) {
                    return;
                }

                let is_touch = matches!(
                    Self::load(&rt, el).drag,
                    Some(Drag::Touch { id: did, .. }) if did == *id
                );
                if !is_touch {
                    return;
                }
                let moved = rt.with_state(el, |s: &mut ScrollState| {
                    let moved = matches!(s.drag, Some(Drag::Touch { moved: true, .. }));
                    s.drag.take();
                    moved
                });
                if moved {
                    ctx.release_pointer();
                    ctx.request_frame();
                    ctx.handled();
                }
            }
            Event::Pointer(PointerEvent::Cancel) => {
                if ctx.phase != EventPhase::Target {
                    return;
                }
                let had = rt.with_state(el, |s: &mut ScrollState| s.drag.take().is_some());
                let unhov = rt.with_state(el, |s: &mut ScrollState| {
                    let old = s.bar_hover;
                    s.bar_hover = false;
                    old
                });
                if had || unhov {
                    ctx.request_frame();
                }
                if had {
                    ctx.handled();
                }
            }
            Event::Key(key) => {
                if matches!(ctx.phase, EventPhase::Target | EventPhase::Bubble) {
                    self.on_key(&rt, el, key.key, key.modifiers, ctx);
                }
            }
            Event::Ime(_) => {}
            Event::FilesHovered { .. } | Event::FilesDropped { .. } | Event::FilesCancelled => {}

            _ => {}
        }
    }

    fn paint_self(&self, ctx: &mut PaintCtx) {
        let rt = ctx.runtime().clone();
        let el = ctx.element();
        let viewport = ctx.rect;
        rt.with_state(el, |s: &mut ScrollState| {
            s.viewport = viewport.size;
        });
    }

    fn paint_foreground(&self, ctx: &mut PaintCtx) {
        let rt = ctx.runtime().clone();
        let el = ctx.element();
        let viewport = ctx.rect;

        let hov = matches!(
            self.scrollbar,
            ScrollbarVisibility::OnHover | ScrollbarVisibility::Overlay
        ) && rt
            .pointer_pos()
            .is_some_and(|p| self.in_gutter(viewport, p));
        rt.with_state(el, |s: &mut ScrollState| {
            s.viewport = viewport.size;
            if matches!(
                self.scrollbar,
                ScrollbarVisibility::OnHover | ScrollbarVisibility::Overlay
            ) {
                s.bar_hover = hov;
            }
        });
        let st = Self::load(&rt, el);

        let bars = self.bars(&st, viewport);
        if bars.v_track.is_none() && bars.h_track.is_none() {
            return;
        }

        let theme = ctx.theme();
        let track_color = self.track_color.unwrap_or(theme.border);
        let thumb_color = self.thumb_color.unwrap_or(theme.text_secondary);
        let radius = self.bar_width * 0.375;
        if let (Some(track), Some(thumb)) = (bars.v_track, bars.v_thumb) {
            ctx.quad(track, track_color, radius);
            ctx.quad(thumb, thumb_color, radius);
        }
        if let (Some(track), Some(thumb)) = (bars.h_track, bars.h_thumb) {
            ctx.quad(track, track_color, radius);
            ctx.quad(thumb, thumb_color, radius);
        }

        if let (Some(v), Some(h)) = (bars.v_track, bars.h_track) {
            let corner = Rect::from_xywh(v.left(), h.top(), v.size.width, h.size.height);
            if !corner.is_empty() {
                ctx.quad(corner, track_color, 0.0);
            }
        }
    }
}
