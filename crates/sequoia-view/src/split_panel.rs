use std::cell::Cell;
use std::rc::Rc;

use sequoia_core::{Color, Constraints, Edges, Length, Rect, Size};
use sequoia_input::{CursorIcon, Event, EventPhase, PointerButton, PointerEvent};
use sequoia_layout::{Axis, Indents, LayoutCtx, LayoutStyle};
use sequoia_reactive::Signal;
use sequoia_style::{BoxStyle, SplitPanelStyle, VisualState};

use crate::any::AnyView;
use crate::runtime::{ElementId, Runtime};
use crate::view::{EventCtx, PaintCtx, View};
use crate::views::Unit;

const DEFAULT_HANDLE: f32 = 7.0;

const MIN_HANDLE: f32 = 3.0;

const LINE_IDLE: f32 = 1.0;

const LINE_ACTIVE: f32 = 2.0;

const GRAB_PAD: f32 = 6.0;

const IDLE_COLOR: Color = Color::rgb8(0x75, 0x75, 0x75);

const ACTIVE_COLOR: Color = Color::rgb8(0x0D, 0x99, 0xFF);

#[derive(Clone, Copy)]
struct SplitDrag {
    origin: f32,
    fraction: f32,
}

fn split_lengths(
    total: f32,
    handle: f32,
    fraction: f32,
    min_first: f32,
    min_second: f32,
) -> (f32, f32, f32) {
    let frac = if fraction.is_finite() {
        fraction.clamp(0.0, 1.0)
    } else {
        0.5
    };
    let hw = if handle.is_finite() {
        handle.max(0.0)
    } else {
        DEFAULT_HANDLE
    };
    let avail = (total - hw).max(0.0);
    let max_first = (avail - min_second.max(0.0)).max(0.0);
    let first = (avail * frac).clamp(min_first.max(0.0).min(max_first), max_first);
    let second = (avail - first).max(0.0);
    (first, first, second)
}

#[derive(Clone)]
pub struct SplitPanel {
    fraction: Signal<f32>,
    first: Option<AnyView>,
    second: Option<AnyView>,
    axis: Axis,
    handle_width: Option<f32>,
    min_first: f32,
    min_second: f32,
    handle: Option<Color>,
    handle_active: Option<Color>,
    style: Option<SplitPanelStyle>,
    layout: LayoutStyle,
    deco: BoxStyle,
    drag: Rc<Cell<Option<SplitDrag>>>,
}

impl SplitPanel {
    pub fn new(fraction: Signal<f32>) -> Self {
        Self {
            fraction,
            first: None,
            second: None,
            axis: Axis::Horizontal,
            handle_width: None,
            min_first: 0.0,
            min_second: 0.0,
            handle: None,
            handle_active: None,
            style: None,
            layout: LayoutStyle {
                width: Length::Fill(1.0),
                height: Length::Fill(1.0),
                ..LayoutStyle::default()
            },
            deco: BoxStyle::new(),
            drag: Rc::new(Cell::new(None)),
        }
    }

    pub fn first<V: View + 'static>(mut self, child: V) -> Self {
        self.first = Some(AnyView::new(child));
        self
    }

    pub fn second<V: View + 'static>(mut self, child: V) -> Self {
        self.second = Some(AnyView::new(child));
        self
    }

    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }

    pub fn horizontal(self) -> Self {
        self.axis(Axis::Horizontal)
    }

    pub fn vertical(self) -> Self {
        self.axis(Axis::Vertical)
    }

    pub fn handle_width(mut self, width: f32) -> Self {
        self.handle_width = Some(if width.is_finite() {
            width.max(MIN_HANDLE)
        } else {
            DEFAULT_HANDLE
        });
        self
    }

    pub fn min_first(mut self, min: f32) -> Self {
        self.min_first = if min.is_finite() { min.max(0.0) } else { 0.0 };
        self
    }

    pub fn min_second(mut self, min: f32) -> Self {
        self.min_second = if min.is_finite() { min.max(0.0) } else { 0.0 };
        self
    }

    pub fn min_pane(mut self, min: f32) -> Self {
        let min = if min.is_finite() { min.max(0.0) } else { 0.0 };
        self.min_first = min;
        self.min_second = min;
        self
    }

    pub fn handle_color(mut self, color: Color) -> Self {
        self.handle = Some(color);
        self
    }

    pub fn handle_active_color(mut self, color: Color) -> Self {
        self.handle_active = Some(color);
        self
    }

    pub fn style(mut self, style: SplitPanelStyle) -> Self {
        self.style = Some(style);
        self
    }

    pub fn fraction_value(&self) -> f32 {
        let f = self.fraction.get_untracked();
        if f.is_finite() {
            f.clamp(0.0, 1.0)
        } else {
            0.5
        }
    }

    fn resolved_handle_width(&self) -> f32 {
        let style = self.style.unwrap_or_default();
        self.handle_width
            .or(style.handle_width)
            .unwrap_or(DEFAULT_HANDLE)
    }

    fn resolved_handle(&self) -> Color {
        let style = self.style.unwrap_or_default();
        self.handle.or(style.handle).unwrap_or(IDLE_COLOR)
    }

    fn resolved_handle_active(&self) -> Color {
        let style = self.style.unwrap_or_default();
        self.handle_active
            .or(style.handle_active)
            .unwrap_or(ACTIVE_COLOR)
    }

    fn axis_pos(&self, x: f32, y: f32) -> f32 {
        match self.axis {
            Axis::Horizontal => x,
            Axis::Vertical => y,
        }
    }

    fn content_rect(&self, rect: Rect) -> Rect {
        let pad = self.layout.padding;
        Rect::from_xywh(
            rect.left() + pad.left,
            rect.top() + pad.top,
            (rect.size.width - pad.horizontal_sum()).max(0.0),
            (rect.size.height - pad.vertical_sum()).max(0.0),
        )
    }

    fn handle_range(&self, content: Rect) -> (f32, f32) {
        let total = match self.axis {
            Axis::Horizontal => content.size.width,
            Axis::Vertical => content.size.height,
        };
        let (_, off, _) = split_lengths(
            total,
            self.resolved_handle_width(),
            self.fraction_value(),
            self.min_first,
            self.min_second,
        );
        let start = match self.axis {
            Axis::Horizontal => content.left(),
            Axis::Vertical => content.top(),
        };
        (start + off, start + off + self.resolved_handle_width())
    }

    pub fn padding(mut self, padding: impl Into<Edges>) -> Self {
        self.layout = self.layout.padding(padding);
        self
    }

    pub fn pad(self, padding: impl Into<Edges>) -> Self {
        self.padding(padding)
    }

    pub fn padding_xy(mut self, x: f32, y: f32) -> Self {
        self.layout = self.layout.padding_xy(x, y);
        self
    }

    pub fn padding_x(mut self, x: f32) -> Self {
        self.layout = self.layout.padding_x(x);
        self
    }

    pub fn padding_y(mut self, y: f32) -> Self {
        self.layout = self.layout.padding_y(y);
        self
    }

    pub fn padding_left(mut self, v: f32) -> Self {
        self.layout = self.layout.padding_left(v);
        self
    }

    pub fn padding_right(mut self, v: f32) -> Self {
        self.layout = self.layout.padding_right(v);
        self
    }

    pub fn padding_top(mut self, v: f32) -> Self {
        self.layout = self.layout.padding_top(v);
        self
    }

    pub fn padding_bottom(mut self, v: f32) -> Self {
        self.layout = self.layout.padding_bottom(v);
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.layout = self.layout.width(width.into());
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.layout = self.layout.height(height.into());
        self
    }

    pub fn size(mut self, width: impl Into<Length>, height: impl Into<Length>) -> Self {
        self.layout = self.layout.width(width.into()).height(height.into());
        self
    }

    pub fn min_width(mut self, v: f32) -> Self {
        self.layout = self.layout.min_width(v);
        self
    }

    pub fn max_width(mut self, v: f32) -> Self {
        self.layout = self.layout.max_width(v);
        self
    }

    pub fn min_height(mut self, v: f32) -> Self {
        self.layout = self.layout.min_height(v);
        self
    }

    pub fn max_height(mut self, v: f32) -> Self {
        self.layout = self.layout.max_height(v);
        self
    }

    pub fn overflow(mut self, overflow: sequoia_layout::Overflow) -> Self {
        self.layout = self.layout.overflow(overflow);
        self
    }

    pub fn margin(mut self, margin: impl Into<Edges>) -> Self {
        self.layout = self.layout.margin(margin);
        self
    }

    pub fn margin_xy(mut self, x: f32, y: f32) -> Self {
        self.layout = self.layout.margin_xy(x, y);
        self
    }

    pub fn margin_x(mut self, x: f32) -> Self {
        self.layout = self.layout.margin_x(x);
        self
    }

    pub fn margin_y(mut self, y: f32) -> Self {
        self.layout = self.layout.margin_y(y);
        self
    }

    pub fn margin_left(mut self, v: f32) -> Self {
        self.layout = self.layout.margin_left(v);
        self
    }

    pub fn margin_right(mut self, v: f32) -> Self {
        self.layout = self.layout.margin_right(v);
        self
    }

    pub fn margin_top(mut self, v: f32) -> Self {
        self.layout = self.layout.margin_top(v);
        self
    }

    pub fn margin_bottom(mut self, v: f32) -> Self {
        self.layout = self.layout.margin_bottom(v);
        self
    }

    pub fn indent_top(mut self, v: f32) -> Self {
        self.layout = self.layout.indent_top(v);
        self
    }

    pub fn indent_bottom(mut self, v: f32) -> Self {
        self.layout = self.layout.indent_bottom(v);
        self
    }

    pub fn indent_left(mut self, v: f32) -> Self {
        self.layout = self.layout.indent_left(v);
        self
    }

    pub fn indent_right(mut self, v: f32) -> Self {
        self.layout = self.layout.indent_right(v);
        self
    }

    pub fn clear_indents(mut self) -> Self {
        self.layout = self.layout.clear_indents();
        self
    }

    pub fn indent(mut self, indents: Indents) -> Self {
        self.layout = self.layout.indent(indents);
        self
    }

    pub fn indent_all(mut self, v: f32) -> Self {
        self.layout = self.layout.indent_all(v);
        self
    }

    pub fn indent_xy(mut self, x: f32, y: f32) -> Self {
        self.layout = self.layout.indent_xy(x, y);
        self
    }

    pub fn indent_axes(mut self, horizontal: f32, vertical: f32) -> Self {
        self.layout = self.layout.indent_axes(horizontal, vertical);
        self
    }

    pub fn flex(mut self, weight: f32) -> Self {
        self.layout = self.layout.flex(weight);
        self
    }

    pub fn flex_shrink(mut self, shrink: f32) -> Self {
        self.layout = self.layout.flex_shrink(shrink);
        self
    }

    pub fn flex_basis(mut self, basis: Option<Length>) -> Self {
        self.layout = self.layout.flex_basis(basis);
        self
    }

    pub fn align_self(mut self, align: Option<sequoia_layout::Align>) -> Self {
        self.layout = self.layout.align_self(align);
        self
    }

    pub fn expand(self) -> Self {
        self.flex(1.0)
    }

    pub fn fill_width(mut self) -> Self {
        self.layout = self.layout.width(Length::Fill(1.0));
        self
    }

    pub fn fill_height(mut self) -> Self {
        self.layout = self.layout.height(Length::Fill(1.0));
        self
    }

    pub fn fill_size(self) -> Self {
        self.fill_width().fill_height()
    }

    pub fn fit(mut self) -> Self {
        self.layout = self.layout.fit();
        self
    }

    pub fn hit_testable(mut self, on: bool) -> Self {
        self.layout = self.layout.hit_testable(on);
        self
    }

    pub fn position(mut self, position: sequoia_layout::Position) -> Self {
        self.layout = self.layout.position(position);
        self
    }

    pub fn absolute(mut self) -> Self {
        self.layout = self.layout.absolute();
        self
    }

    pub fn background(mut self, color: Color) -> Self {
        self.deco = self.deco.background(color);
        self
    }

    pub fn border(mut self, width: f32, color: Color) -> Self {
        self.deco = self.deco.border(width, color);
        self
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.deco = self.deco.radius(radius);
        self
    }

    pub fn box_style(mut self, style: BoxStyle) -> Self {
        self.deco = style;
        self
    }
}

impl View for SplitPanel {
    fn body(&self) -> impl View + 'static {
        Unit
    }
    fn children_override(&self) -> Option<Vec<AnyView>> {
        let _ = self.fraction.get();
        let mut out = Vec::with_capacity(2);
        if let Some(f) = &self.first {
            out.push(f.clone());
        }
        if let Some(s) = &self.second {
            out.push(s.clone());
        }
        Some(out)
    }
    fn layout_style(&self) -> LayoutStyle {
        self.layout
    }
    fn captures_pointer(&self) -> bool {
        false
    }
    fn has_visual_state(&self) -> bool {
        true
    }
    fn cursor_icon(&self, state: VisualState) -> CursorIcon {
        if self.drag.get().is_some() || state.pressed || state.hovered {
            match self.axis {
                Axis::Horizontal => CursorIcon::ColResize,
                Axis::Vertical => CursorIcon::RowResize,
            }
        } else {
            CursorIcon::Default
        }
    }
    fn arrange_override(
        &self,
        rt: &Runtime,
        el: ElementId,
        rect: Rect,
        ctx: &mut LayoutCtx,
    ) -> bool {
        let kids = rt.children(el);
        if kids.is_empty() {
            return true;
        }
        let content = self.content_rect(rect);
        let hw = self.resolved_handle_width();
        let (total_main, cross) = match self.axis {
            Axis::Horizontal => (content.size.width, content.size.height),
            Axis::Vertical => (content.size.height, content.size.width),
        };
        let has_first = self.first.is_some();
        let has_second = self.second.is_some();
        let mut slots: Vec<(ElementId, Rect)> = Vec::with_capacity(2);
        if has_first && has_second && kids.len() >= 2 {
            let (a, off, b) = split_lengths(
                total_main,
                hw,
                self.fraction_value(),
                self.min_first,
                self.min_second,
            );
            let (ra, rb) = match self.axis {
                Axis::Horizontal => (
                    Rect::from_xywh(content.left(), content.top(), a, cross),
                    Rect::from_xywh(content.left() + off + hw, content.top(), b, cross),
                ),
                Axis::Vertical => (
                    Rect::from_xywh(content.left(), content.top(), cross, a),
                    Rect::from_xywh(content.left(), content.top() + off + hw, cross, b),
                ),
            };
            slots.push((kids[0], ra));
            slots.push((kids[1], rb));
        } else {
            let whole = match self.axis {
                Axis::Horizontal => {
                    Rect::from_xywh(content.left(), content.top(), total_main, cross)
                }
                Axis::Vertical => Rect::from_xywh(content.left(), content.top(), cross, total_main),
            };
            slots.push((kids[0], whole));
        }
        for (child, slot) in slots {
            let max = Size::new(slot.size.width.max(0.0), slot.size.height.max(0.0));
            let _ = rt.measure_node(
                child,
                Constraints {
                    min: Size::ZERO,
                    max,
                },
                ctx,
            );
            rt.arrange_node(child, slot, ctx);
        }
        true
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
                let content = self.content_rect(ctx.rect);
                let (lo, hi) = self.handle_range(content);
                let p = self.axis_pos(pos.x, pos.y);

                if p >= lo - GRAB_PAD && p <= hi + GRAB_PAD {
                    self.drag.set(Some(SplitDrag {
                        origin: p,
                        fraction: self.fraction_value(),
                    }));
                    ctx.capture_pointer();
                    ctx.handled();
                }
            }
            Event::Pointer(PointerEvent::Move { pos, .. }) => {
                if let Some(st) = self.drag.get() {
                    let content = self.content_rect(ctx.rect);
                    let total = match self.axis {
                        Axis::Horizontal => content.size.width,
                        Axis::Vertical => content.size.height,
                    };
                    let avail = (total - self.resolved_handle_width()).max(0.0);
                    if avail > 0.0 {
                        let p = self.axis_pos(pos.x, pos.y);
                        let next = st.fraction + (p - st.origin) / avail;
                        self.fraction.set(next.clamp(0.0, 1.0));
                    }
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
        ctx.paint_box(self.deco);
        let content = self.content_rect(ctx.rect);
        if !self.first.is_some() || !self.second.is_some() {
            return;
        }
        let (lo, _) = self.handle_range(content);
        let hw = self.resolved_handle_width();
        if hw <= 0.0 {
            return;
        }
        let active = self.drag.get().is_some() || ctx.state.pressed || ctx.state.hovered;
        let (w, c) = if active {
            (LINE_ACTIVE, self.resolved_handle_active())
        } else {
            (LINE_IDLE, self.resolved_handle())
        };
        match self.axis {
            Axis::Horizontal => {
                let cx = lo + hw * 0.5;
                ctx.quad(
                    Rect::from_xywh(cx - w * 0.5, content.top(), w, content.size.height),
                    c,
                    0.0,
                );
            }
            Axis::Vertical => {
                let cy = lo + hw * 0.5;
                ctx.quad(
                    Rect::from_xywh(content.left(), cy - w * 0.5, content.size.width, w),
                    c,
                    0.0,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn even_split_without_minimums() {
        let (a, off, b) = split_lengths(400.0, 8.0, 0.5, 0.0, 0.0);
        assert_eq!((a, off, b), (196.0, 196.0, 196.0));
    }

    #[test]
    fn fraction_moves_divider() {
        let (a, off, b) = split_lengths(400.0, 8.0, 0.25, 0.0, 0.0);
        assert_eq!((a, off, b), (98.0, 98.0, 294.0));
    }

    #[test]
    fn minimums_clamp_without_panic() {
        let (a, _, b) = split_lengths(400.0, 8.0, 0.0, 100.0, 0.0);
        assert_eq!((a, b), (100.0, 292.0));

        let (a, _, b) = split_lengths(100.0, 0.0, 0.5, 80.0, 80.0);
        assert_eq!((a, b), (20.0, 80.0));
    }

    #[test]
    fn degenerate_inputs_collapse() {
        assert_eq!(split_lengths(4.0, 8.0, 0.5, 0.0, 0.0), (0.0, 0.0, 0.0));
        assert_eq!(split_lengths(400.0, 8.0, f32::NAN, 0.0, 0.0).0, 196.0);
        assert_eq!(split_lengths(0.0, 8.0, 0.5, 0.0, 0.0), (0.0, 0.0, 0.0));
    }

    #[test]
    fn fraction_value_sanitizes() {
        let s = SplitPanel::new(Signal::new(2.0));
        assert_eq!(s.fraction_value(), 1.0);
        let s = SplitPanel::new(Signal::new(f32::NAN));
        assert_eq!(s.fraction_value(), 0.5);
    }
}
