use std::cell::Cell;
use std::rc::Rc;

use sequoia_core::{Color, Length, Rect};
use sequoia_input::{Event, EventPhase, Key, Modifiers, PointerEvent};
use sequoia_layout::{LayoutCtx, LayoutStyle};
use sequoia_reactive::Signal;

use crate::any::AnyView;
use crate::runtime::{ElementId, Runtime};
use crate::scroll_view::ScrollbarVisibility;
use crate::view::{EventCtx, PaintCtx, View};

const FALLBACK_WINDOW: usize = 64;

const DEFAULT_ITEM_HEIGHT: f32 = 28.0;

const DEFAULT_OVERSCAN: usize = 4;

const DEFAULT_BAR_W: f32 = 8.0;

const BAR_PAD: f32 = 2.0;

const THUMB_MIN: f32 = 24.0;

const DEFAULT_STEP: f32 = 40.0;

struct ItemSlot {
    child: AnyView,
    height: f32,
}

impl View for ItemSlot {
    fn body(&self) -> impl View + 'static {
        crate::views::Unit
    }

    fn children_override(&self) -> Option<Vec<AnyView>> {
        Some(vec![self.child.clone()])
    }

    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fill(1.0),
            height: Length::Px(self.height),
            ..LayoutStyle::default()
        }
    }
}

pub struct VirtualList<T> {
    items: Rc<dyn Fn() -> Vec<T>>,
    key: Rc<dyn Fn(&T) -> u64>,
    item: Rc<dyn Fn(&T) -> AnyView>,

    item_idx: Option<Rc<dyn Fn(usize, &T) -> AnyView>>,
    item_height: f32,
    overscan: usize,
    offset: Signal<f32>,
    viewport_h: Rc<Cell<f32>>,
    viewport_hint: Signal<u64>,
    window_start: Rc<Cell<usize>>,
    style: LayoutStyle,
    scrollbar: ScrollbarVisibility,
    bar_width: f32,
    track_color: Option<Color>,
    thumb_color: Option<Color>,
    step: f32,
    on_scroll: Option<Rc<dyn Fn(f32)>>,

    drag: Rc<Cell<Option<f32>>>,
}

impl<T: 'static> Clone for VirtualList<T> {
    fn clone(&self) -> Self {
        Self {
            items: Rc::clone(&self.items),
            key: Rc::clone(&self.key),
            item: Rc::clone(&self.item),
            item_idx: self.item_idx.clone(),
            item_height: self.item_height,
            overscan: self.overscan,
            offset: self.offset.clone(),
            viewport_h: Rc::clone(&self.viewport_h),
            viewport_hint: self.viewport_hint.clone(),
            window_start: Rc::clone(&self.window_start),
            style: self.style,
            scrollbar: self.scrollbar,
            bar_width: self.bar_width,
            track_color: self.track_color,
            thumb_color: self.thumb_color,
            step: self.step,
            on_scroll: self.on_scroll.clone(),
            drag: Rc::clone(&self.drag),
        }
    }
}

impl<T: 'static> VirtualList<T> {
    pub fn new<F: Fn() -> Vec<T> + 'static>(items: F) -> Self {
        Self {
            items: Rc::new(items),
            key: Rc::new(|_| 0),
            item: Rc::new(|_| AnyView::new(crate::views::Unit)),
            item_idx: None,
            item_height: DEFAULT_ITEM_HEIGHT,
            overscan: DEFAULT_OVERSCAN,
            offset: Signal::new(0.0),
            viewport_h: Rc::new(Cell::new(0.0)),
            viewport_hint: Signal::new(0),
            window_start: Rc::new(Cell::new(0)),
            scrollbar: ScrollbarVisibility::Auto,
            bar_width: DEFAULT_BAR_W,
            track_color: None,
            thumb_color: None,
            step: DEFAULT_STEP,
            on_scroll: None,
            drag: Rc::new(Cell::new(None)),
            style: LayoutStyle {
                width: Length::Fill(1.0),
                height: Length::Fill(1.0),
                ..LayoutStyle::default()
            },
        }
    }

    pub fn key<K: Fn(&T) -> u64 + 'static>(mut self, key: K) -> Self {
        self.key = Rc::new(key);
        self
    }

    pub fn item<I, F>(mut self, item: F) -> Self
    where
        I: View + 'static,
        F: Fn(&T) -> I + 'static,
    {
        self.item = Rc::new(move |t: &T| AnyView::new(item(t)));
        self
    }

    pub fn item_indexed<I, F>(mut self, item: F) -> Self
    where
        I: View + 'static,
        F: Fn(usize, &T) -> I + 'static,
    {
        self.item_idx = Some(Rc::new(move |i: usize, t: &T| AnyView::new(item(i, t))));
        self
    }

    pub fn item_height(mut self, h: f32) -> Self {
        self.item_height = h.max(1.0);
        self
    }

    pub fn overscan(mut self, n: usize) -> Self {
        self.overscan = n;
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

    pub fn on_scroll(mut self, handler: impl Fn(f32) + 'static) -> Self {
        self.on_scroll = Some(Rc::new(handler));
        self
    }

    pub fn scroll_step(mut self, step: f32) -> Self {
        self.step = step.max(0.0);
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

    pub fn fill_width(mut self) -> Self {
        self.style = self.style.width(Length::Fill(1.0));
        self
    }

    pub fn fill_height(mut self) -> Self {
        self.style = self.style.height(Length::Fill(1.0));
        self
    }

    pub fn fill_size(self) -> Self {
        self.fill_width().fill_height()
    }

    pub fn offset(&self) -> f32 {
        self.offset.get_untracked()
    }

    pub fn scroll_to(&self, y: f32) {
        self.apply(y.clamp(0.0, self.max_offset()));
    }

    fn apply(&self, next: f32) {
        let prev = self.offset.get_untracked();
        if next != prev {
            self.offset.set(next);
            if let Some(cb) = &self.on_scroll {
                cb(next);
            }
        }
    }

    pub fn scroll_by(&self, dy: f32) {
        self.scroll_to(self.offset.get_untracked() + dy);
    }

    pub fn scroll_to_top(&self) {
        self.scroll_to(0.0);
    }

    pub fn scroll_to_bottom(&self) {
        self.scroll_to(self.max_offset());
    }

    pub fn page_up(&self) {
        self.scroll_by(-self.viewport_h.get());
    }

    pub fn page_down(&self) {
        self.scroll_by(self.viewport_h.get());
    }

    fn total_height(&self) -> f32 {
        (self.items)().len() as f32 * self.item_height
    }

    pub fn max_offset(&self) -> f32 {
        (self.total_height() - self.viewport_h.get()).max(0.0)
    }

    fn first_visible(&self, off: f32) -> usize {
        if self.item_height > 0.0 {
            (off / self.item_height).floor().max(0.0) as usize
        } else {
            0
        }
    }

    fn window_bounds(&self, item_count: usize, off: f32) -> (usize, usize) {
        let vh = self.viewport_h.get();
        let first = self.first_visible(off);
        let start = first.saturating_sub(self.overscan).min(item_count);
        if vh <= 0.0 {
            let end = (start + FALLBACK_WINDOW).min(item_count);
            return (start, end);
        }
        let capacity = (vh / self.item_height).ceil() as usize + 1;
        let end = (first + capacity + self.overscan).min(item_count);
        (start, end.min(item_count).max(start))
    }

    fn shows_bar(&self, viewport: Rect) -> bool {
        if self.bar_width <= 0.0 {
            return false;
        }
        match self.scrollbar {
            ScrollbarVisibility::Never => false,
            ScrollbarVisibility::Always => true,
            ScrollbarVisibility::Auto => self.total_height() > viewport.size.height,

            ScrollbarVisibility::OnHover | ScrollbarVisibility::Overlay => {
                self.total_height() > viewport.size.height
            }
        }
    }

    fn track_rect(&self, viewport: Rect) -> Option<Rect> {
        if !self.shows_bar(viewport) {
            return None;
        }
        let track = Rect::from_xywh(
            viewport.right() - self.bar_width - BAR_PAD,
            viewport.top() + BAR_PAD,
            self.bar_width,
            (viewport.size.height - BAR_PAD * 2.0).max(0.0),
        );
        if track.is_empty() { None } else { Some(track) }
    }

    fn thumb_rect(&self, track: Rect) -> Rect {
        let content = self.total_height();
        let vh = self.viewport_h.get();
        let max_off = (content - vh).max(0.0);
        if max_off <= 0.0 || content <= 0.0 {
            return track;
        }
        let thumb_h = (track.size.height * vh / content)
            .max(THUMB_MIN)
            .min(track.size.height);
        let travel = (track.size.height - thumb_h).max(0.0);
        let t = (self.offset.get_untracked() / max_off).clamp(0.0, 1.0);
        Rect::from_xywh(
            track.left(),
            track.top() + t * travel,
            track.size.width,
            thumb_h,
        )
    }

    fn offset_from_drag(&self, track: Rect, thumb_h: f32, y: f32, grab: f32) -> Option<f32> {
        let travel = (track.size.height - thumb_h).max(0.0);
        let max_off = self.max_offset();
        if travel <= 0.0 || max_off <= 0.0 {
            return None;
        }
        let top = (y - grab - track.top()).clamp(0.0, travel);
        Some(top / travel * max_off)
    }

    fn on_wheel(&self, delta_y: f32, ctx: &mut EventCtx) {
        let next = (self.offset.get_untracked() - delta_y).clamp(0.0, self.max_offset());
        if next != self.offset.get_untracked() {
            self.apply(next);
            ctx.request_frame();
            ctx.handled();
        }
    }

    fn on_key(&self, key: Key, modifiers: Modifiers, ctx: &mut EventCtx) {
        let o = self.offset.get_untracked();
        let max = self.max_offset();
        let vh = self.viewport_h.get().max(0.0);
        let step = if self.step.is_finite() {
            self.step.max(0.0)
        } else {
            0.0
        };
        let next = match (key, modifiers) {
            (Key::ARROW_UP, Modifiers::NONE) => o - step,
            (Key::ARROW_DOWN, Modifiers::NONE) => o + step,
            (Key::PAGE_UP, Modifiers::NONE) => o - vh,
            (Key::PAGE_DOWN, Modifiers::NONE) => o + vh,
            (Key::HOME, Modifiers::NONE) => 0.0,
            (Key::END, Modifiers::NONE) => max,
            _ => return,
        };
        let next = next.clamp(0.0, max);
        if next != o {
            self.apply(next);
            ctx.request_frame();
            ctx.handled();
        }
    }
}

impl<T: 'static> View for VirtualList<T> {
    fn reuse_state(&self, old: &Self) {
        self.offset.set(old.offset.get_untracked());
        self.viewport_h.set(old.viewport_h.get());
        self.viewport_hint.set(old.viewport_hint.get_untracked());
        self.window_start.set(old.window_start.get());
    }

    fn body(&self) -> impl View + 'static {
        crate::views::Unit
    }

    fn keyed_children(&self) -> Option<Vec<(u64, AnyView)>> {
        let items = (self.items)();
        let off = self.offset.get().clamp(0.0, self.max_offset());
        let _hint = self.viewport_hint.get();
        let (start, end) = self.window_bounds(items.len(), off);
        self.window_start.set(start);
        let slot_h = self.item_height;

        Some(
            items[start..end]
                .iter()
                .enumerate()
                .map(|(i, it)| {
                    let k = (self.key)(it);
                    let v = match &self.item_idx {
                        Some(f) => f(start + i, it),
                        None => (self.item)(it),
                    };
                    (
                        k,
                        AnyView::new(ItemSlot {
                            child: v,
                            height: slot_h,
                        }),
                    )
                })
                .collect(),
        )
    }

    fn layout_style(&self) -> LayoutStyle {
        self.style
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
        self.viewport_h.set(rect.size.height);
        let hint = (rect.size.height * 64.0) as u64;
        if self.viewport_hint.get_untracked() != hint {
            self.viewport_hint.set(hint);
        }

        let off = self.offset.get_untracked().clamp(0.0, self.max_offset());

        let start = self.window_start.get();
        let children = rt.children(el);
        for (i, &child) in children.iter().enumerate() {
            let index = start + i;
            let y = rect.top() + index as f32 * self.item_height - off;
            let child_rect = Rect::from_xywh(rect.left(), y, rect.size.width, self.item_height);
            rt.arrange_node(child, child_rect, ctx);
        }
        true
    }

    fn clip_self(&self) -> bool {
        true
    }

    fn focusable(&self) -> bool {
        true
    }

    fn captures_pointer(&self) -> bool {
        false
    }

    fn captures_pointer_ctx(&self, _rt: &Runtime, _el: ElementId) -> bool {
        self.drag.get().is_some()
    }

    fn on_event(&self, event: &Event, ctx: &mut EventCtx) {
        match event {
            Event::Pointer(PointerEvent::Wheel { delta, .. }) => {
                if matches!(ctx.phase, EventPhase::Target | EventPhase::Bubble) {
                    self.on_wheel(delta.y, ctx);
                }
            }
            Event::Pointer(PointerEvent::Down { pos, .. }) => {
                if !matches!(ctx.phase, EventPhase::Target | EventPhase::Bubble) {
                    return;
                }
                if self.drag.get().is_some() {
                    return;
                }
                let Some(track) = self.track_rect(ctx.rect) else {
                    return;
                };
                let thumb = self.thumb_rect(track);

                if thumb.contains(*pos) {
                    self.drag.set(Some(pos.y - thumb.top()));
                    ctx.request_frame();
                    ctx.handled();
                    return;
                }

                if track.contains(*pos) {
                    let travel = (track.size.height - thumb.size.height).max(0.0);
                    if travel > 0.0 {
                        let top =
                            (pos.y - thumb.size.height * 0.5 - track.top()).clamp(0.0, travel);
                        let max_off = self.max_offset();
                        if max_off > 0.0 {
                            self.apply(top / travel * max_off);
                        }
                    }
                    self.drag.set(Some(thumb.size.height * 0.5));
                    ctx.request_frame();
                    ctx.handled();
                }
            }
            Event::Pointer(PointerEvent::Move { pos, .. }) => {
                if ctx.phase != EventPhase::Target {
                    return;
                }
                if let Some(grab) = self.drag.get() {
                    let Some(track) = self.track_rect(ctx.rect) else {
                        return;
                    };
                    let thumb_h = self.thumb_rect(track).size.height;
                    if let Some(next) = self.offset_from_drag(track, thumb_h, pos.y, grab) {
                        if next != self.offset.get_untracked() {
                            self.apply(next);
                            ctx.request_frame();
                        }
                        ctx.handled();
                    }
                }
            }
            Event::Pointer(PointerEvent::Up { .. }) | Event::Pointer(PointerEvent::Cancel) => {
                if ctx.phase != EventPhase::Target {
                    return;
                }
                if self.drag.take().is_some() {
                    ctx.request_frame();
                    ctx.handled();
                }
            }
            Event::Key(key) => {
                if matches!(ctx.phase, EventPhase::Target | EventPhase::Bubble) {
                    self.on_key(key.key, key.modifiers, ctx);
                }
            }
            Event::Ime(_) => {}
            Event::FilesHovered { .. } | Event::FilesDropped { .. } | Event::FilesCancelled => {}

            _ => {}
        }
    }

    fn paint_foreground(&self, ctx: &mut PaintCtx) {
        let viewport = ctx.rect;
        let Some(track) = self.track_rect(viewport) else {
            return;
        };
        let thumb = self.thumb_rect(track);
        let theme = ctx.theme();
        let track_color = self.track_color.unwrap_or(theme.border);
        let thumb_color = self.thumb_color.unwrap_or(theme.text_secondary);
        let radius = self.bar_width * 0.375;
        ctx.quad(track, track_color, radius);
        ctx.quad(thumb, thumb_color, radius);
    }
}
