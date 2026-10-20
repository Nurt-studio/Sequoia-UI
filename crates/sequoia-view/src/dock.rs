use sequoia_core::{Color, Edges, Length};
use sequoia_layout::{
    Align, Dock, DockStyle, Indents, Inset, LayoutStyle, Overflow, Position, sanitize_gap,
};
use sequoia_style::BoxStyle;

use crate::any::AnyView;
use crate::view::{PaintCtx, View};
use crate::views::{IntoChildren, Unit};

#[derive(Clone, Default)]
pub struct DockArea {
    children: Vec<AnyView>,
    docks: Vec<Dock>,
    style: LayoutStyle,
    gap: f32,
    last_child_fill: bool,
    ordered: bool,
    deco: BoxStyle,
}

impl DockArea {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            docks: Vec::new(),
            style: LayoutStyle {
                width: Length::Fit,
                height: Length::Fit,
                ..LayoutStyle::default()
            },
            gap: 0.0,
            last_child_fill: false,
            ordered: false,
            deco: BoxStyle::new(),
        }
    }

    pub fn dock<V: View + 'static>(mut self, child: V, dock: Dock) -> Self {
        self.children.push(AnyView::new(child));
        self.docks.push(dock);
        self.debug_check_fill_order();
        self
    }

    pub fn dock_if<V: View + 'static>(
        self,
        cond: bool,
        make: impl FnOnce() -> V,
        dock: Dock,
    ) -> Self {
        if cond { self.dock(make(), dock) } else { self }
    }

    pub fn dock_opt<V: View + 'static>(self, child: Option<V>, dock: Dock) -> Self {
        match child {
            Some(v) => self.dock(v, dock),
            None => self,
        }
    }

    pub fn children(mut self, children: impl IntoChildren, dock: Dock) -> Self {
        for child in children.into_children() {
            self.children.push(child);
            self.docks.push(dock);
        }
        self.debug_check_fill_order();
        self
    }

    pub fn len(&self) -> usize {
        self.children.len()
    }

    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }

    pub fn docks(&self) -> &[Dock] {
        &self.docks
    }

    pub fn get_dock(&self, index: usize) -> Option<Dock> {
        self.docks.get(index).copied()
    }

    pub fn is_ordered(&self) -> bool {
        self.ordered
    }

    pub fn is_border(&self) -> bool {
        !self.ordered
    }

    pub fn dock_at<V: View + 'static>(mut self, index: usize, child: V, dock: Dock) -> Self {
        let i = index.min(self.children.len());
        self.children.insert(i, AnyView::new(child));
        self.docks.insert(i, dock);
        self.debug_check_fill_order();
        self
    }

    pub fn remove(mut self, index: usize) -> Self {
        if index < self.children.len() {
            self.children.remove(index);
            self.docks.remove(index);
        }
        self
    }

    pub fn move_child(mut self, from: usize, slot: usize) -> Self {
        crate::dnd::move_item(&mut self.children, from, slot);

        crate::dnd::move_item(&mut self.docks, from, slot);
        self.debug_check_fill_order();
        self
    }

    pub fn set_dock(mut self, index: usize, dock: Dock) -> Self {
        if let Some(d) = self.docks.get_mut(index) {
            *d = dock;
        }
        self.debug_check_fill_order();
        self
    }

    pub fn clear(mut self) -> Self {
        self.children.clear();
        self.docks.clear();
        self
    }

    pub fn dock_mut<V: View + 'static>(&mut self, child: V, dock: Dock) {
        self.children.push(AnyView::new(child));
        self.docks.push(dock);
        self.debug_check_fill_order();
    }

    pub fn dock_at_mut<V: View + 'static>(&mut self, index: usize, child: V, dock: Dock) {
        let i = index.min(self.children.len());
        self.children.insert(i, AnyView::new(child));
        self.docks.insert(i, dock);
        self.debug_check_fill_order();
    }

    pub fn remove_mut(&mut self, index: usize) {
        if index < self.children.len() {
            self.children.remove(index);
            self.docks.remove(index);
        }
    }

    pub fn move_child_mut(&mut self, from: usize, slot: usize) {
        crate::dnd::move_item(&mut self.children, from, slot);
        crate::dnd::move_item(&mut self.docks, from, slot);
        self.debug_check_fill_order();
    }

    pub fn set_dock_mut(&mut self, index: usize, dock: Dock) {
        if let Some(d) = self.docks.get_mut(index) {
            *d = dock;
        }
        self.debug_check_fill_order();
    }

    pub fn clear_mut(&mut self) {
        self.children.clear();
        self.docks.clear();
    }

    fn debug_check_fill_order(&self) {
        #[cfg(debug_assertions)]
        {
            let n = self.docks.len();
            if n == 0 {
                return;
            }
            if !self.ordered && self.last_child_fill {
                log::warn!(
                    "sequoia-view::DockArea: .last_child_fill(true) is ignored in Border mode - remove it or set it to .ordered(true)"
                );
            }
            if self.ordered {
                for i in 0..n {
                    let effective =
                        sequoia_layout::effective_dock(&self.docks, self.last_child_fill, i, n);
                    if effective == Dock::Fill && i + 1 != n {
                        log::warn!(
                            "sequoia-view::DockArea(ordered): Fill at position {i}/{n} - children after it will get 0x0; put Fill last or use Border mode"
                        );
                        break;
                    }
                }
            } else {
                let fills = self.docks.iter().filter(|d| **d == Dock::Fill).count();
                if fills > 1 {
                    log::warn!(
                        "sequoia-view::DockArea: {fills}× Fill - the first one takes the rest, the next ones will get 0x0"
                    );
                }
            }
        }
    }

    pub fn left<V: View + 'static>(self, child: V) -> Self {
        self.dock(child, Dock::Left)
    }

    pub fn right<V: View + 'static>(self, child: V) -> Self {
        self.dock(child, Dock::Right)
    }

    pub fn top<V: View + 'static>(self, child: V) -> Self {
        self.dock(child, Dock::Top)
    }

    pub fn bottom<V: View + 'static>(self, child: V) -> Self {
        self.dock(child, Dock::Bottom)
    }

    pub fn fill<V: View + 'static>(self, child: V) -> Self {
        self.dock(child, Dock::Fill)
    }

    pub fn center<V: View + 'static>(self, child: V) -> Self {
        self.fill(child)
    }

    pub fn left_if<V: View + 'static>(self, cond: bool, make: impl FnOnce() -> V) -> Self {
        self.dock_if(cond, make, Dock::Left)
    }

    pub fn right_if<V: View + 'static>(self, cond: bool, make: impl FnOnce() -> V) -> Self {
        self.dock_if(cond, make, Dock::Right)
    }

    pub fn top_if<V: View + 'static>(self, cond: bool, make: impl FnOnce() -> V) -> Self {
        self.dock_if(cond, make, Dock::Top)
    }

    pub fn bottom_if<V: View + 'static>(self, cond: bool, make: impl FnOnce() -> V) -> Self {
        self.dock_if(cond, make, Dock::Bottom)
    }

    pub fn fill_if<V: View + 'static>(self, cond: bool, make: impl FnOnce() -> V) -> Self {
        self.dock_if(cond, make, Dock::Fill)
    }

    pub fn center_if<V: View + 'static>(self, cond: bool, make: impl FnOnce() -> V) -> Self {
        self.dock_if(cond, make, Dock::Fill)
    }

    pub fn left_opt<V: View + 'static>(self, child: Option<V>) -> Self {
        self.dock_opt(child, Dock::Left)
    }

    pub fn right_opt<V: View + 'static>(self, child: Option<V>) -> Self {
        self.dock_opt(child, Dock::Right)
    }

    pub fn top_opt<V: View + 'static>(self, child: Option<V>) -> Self {
        self.dock_opt(child, Dock::Top)
    }

    pub fn bottom_opt<V: View + 'static>(self, child: Option<V>) -> Self {
        self.dock_opt(child, Dock::Bottom)
    }

    pub fn fill_opt<V: View + 'static>(self, child: Option<V>) -> Self {
        self.dock_opt(child, Dock::Fill)
    }

    pub fn center_opt<V: View + 'static>(self, child: Option<V>) -> Self {
        self.dock_opt(child, Dock::Fill)
    }

    pub fn last_child_fill(mut self, fill: bool) -> Self {
        self.last_child_fill = fill;
        self.debug_check_fill_order();
        self
    }

    pub fn ordered(mut self, ordered: bool) -> Self {
        self.ordered = ordered;
        self.debug_check_fill_order();
        self
    }

    pub fn border_layout(mut self, border: bool) -> Self {
        self.ordered = !border;
        self.debug_check_fill_order();
        self
    }

    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = sanitize_gap(gap);
        self
    }

    pub fn padding(mut self, padding: impl Into<Edges>) -> Self {
        self.style = self.style.padding(padding);
        self
    }

    pub fn pad(self, padding: impl Into<Edges>) -> Self {
        self.padding(padding)
    }

    pub fn padding_xy(mut self, x: f32, y: f32) -> Self {
        self.style = self.style.padding_xy(x, y);
        self
    }

    pub fn padding_x(mut self, x: f32) -> Self {
        self.style = self.style.padding_x(x);
        self
    }

    pub fn padding_y(mut self, y: f32) -> Self {
        self.style = self.style.padding_y(y);
        self
    }

    pub fn padding_left(mut self, v: f32) -> Self {
        self.style = self.style.padding_left(v);
        self
    }

    pub fn padding_right(mut self, v: f32) -> Self {
        self.style = self.style.padding_right(v);
        self
    }

    pub fn padding_top(mut self, v: f32) -> Self {
        self.style = self.style.padding_top(v);
        self
    }

    pub fn padding_bottom(mut self, v: f32) -> Self {
        self.style = self.style.padding_bottom(v);
        self
    }

    pub fn margin(mut self, margin: impl Into<Edges>) -> Self {
        self.style = self.style.margin(margin);
        self
    }

    pub fn margin_xy(mut self, x: f32, y: f32) -> Self {
        self.style = self.style.margin_xy(x, y);
        self
    }

    pub fn margin_x(mut self, x: f32) -> Self {
        self.style = self.style.margin_x(x);
        self
    }

    pub fn margin_y(mut self, y: f32) -> Self {
        self.style = self.style.margin_y(y);
        self
    }

    pub fn margin_left(mut self, v: f32) -> Self {
        self.style = self.style.margin_left(v);
        self
    }

    pub fn margin_right(mut self, v: f32) -> Self {
        self.style = self.style.margin_right(v);
        self
    }

    pub fn margin_top(mut self, v: f32) -> Self {
        self.style = self.style.margin_top(v);
        self
    }

    pub fn margin_bottom(mut self, v: f32) -> Self {
        self.style = self.style.margin_bottom(v);
        self
    }

    pub fn indent_top(mut self, v: f32) -> Self {
        self.style = self.style.indent_top(v);
        self
    }

    pub fn indent_bottom(mut self, v: f32) -> Self {
        self.style = self.style.indent_bottom(v);
        self
    }

    pub fn indent_left(mut self, v: f32) -> Self {
        self.style = self.style.indent_left(v);
        self
    }

    pub fn indent_right(mut self, v: f32) -> Self {
        self.style = self.style.indent_right(v);
        self
    }

    pub fn clear_indents(mut self) -> Self {
        self.style = self.style.clear_indents();
        self
    }

    pub fn indent(mut self, indents: Indents) -> Self {
        self.style = self.style.indent(indents);
        self
    }

    pub fn indent_all(mut self, v: f32) -> Self {
        self.style = self.style.indent_all(v);
        self
    }

    pub fn indent_xy(mut self, x: f32, y: f32) -> Self {
        self.style = self.style.indent_xy(x, y);
        self
    }

    pub fn indent_axes(mut self, horizontal: f32, vertical: f32) -> Self {
        self.style = self.style.indent_axes(horizontal, vertical);
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

    pub fn size(mut self, width: impl Into<Length>, height: impl Into<Length>) -> Self {
        self.style = self.style.width(width.into()).height(height.into());
        self
    }

    pub fn square(mut self, side: impl Into<Length>) -> Self {
        let side = side.into();
        self.style = self.style.width(side).height(side);
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

    pub fn overflow(mut self, overflow: Overflow) -> Self {
        self.style = self.style.overflow(overflow);
        self
    }

    pub fn flex(mut self, weight: f32) -> Self {
        self.style = self.style.flex(weight);
        self
    }

    pub fn flex_grow(mut self, weight: f32) -> Self {
        self.style = self.style.flex_grow(weight);
        self
    }

    pub fn flex_shrink(mut self, shrink: f32) -> Self {
        self.style = self.style.flex_shrink(shrink);
        self
    }

    pub fn flex_basis(mut self, basis: Option<Length>) -> Self {
        self.style = self.style.flex_basis(basis);
        self
    }

    pub fn align_self(mut self, align: Option<Align>) -> Self {
        self.style = self.style.align_self(align);
        self
    }

    pub fn align_self_value(mut self, align: Align) -> Self {
        self.style = self.style.align_self_value(align);
        self
    }

    pub fn aspect_ratio(mut self, ratio: Option<f32>) -> Self {
        self.style = self.style.aspect_ratio(ratio);
        self
    }

    pub fn aspect(mut self, ratio: f32) -> Self {
        self.style = self.style.aspect(ratio);
        self
    }

    pub fn position(mut self, position: Position) -> Self {
        self.style = self.style.position(position);
        self
    }

    pub fn absolute(mut self) -> Self {
        self.style = self.style.absolute();
        self
    }

    pub fn absolute_at(mut self, left: f32, top: f32) -> Self {
        self.style = self.style.absolute_at(left, top);
        self
    }

    pub fn absolute_inset(mut self, inset: Inset) -> Self {
        self.style = self.style.absolute_inset(inset);
        self
    }

    pub fn fit(mut self) -> Self {
        self.style = self.style.fit();
        self
    }

    pub fn hit_testable(mut self, on: bool) -> Self {
        self.style = self.style.hit_testable(on);
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

    pub fn background(mut self, color: Color) -> Self {
        self.deco = self.deco.background(color);
        self
    }

    pub fn border(mut self, width: f32, color: Color) -> Self {
        self.deco = self.deco.border(width, color);
        self
    }

    pub fn border_left(mut self, width: f32, color: Color) -> Self {
        self.deco = self.deco.border_left(width, color);
        self
    }

    pub fn border_right(mut self, width: f32, color: Color) -> Self {
        self.deco = self.deco.border_right(width, color);
        self
    }

    pub fn border_top(mut self, width: f32, color: Color) -> Self {
        self.deco = self.deco.border_top(width, color);
        self
    }

    pub fn border_bottom(mut self, width: f32, color: Color) -> Self {
        self.deco = self.deco.border_bottom(width, color);
        self
    }

    pub fn border_x(mut self, width: f32, color: Color) -> Self {
        self.deco = self.deco.border_x(width, color);
        self
    }

    pub fn border_y(mut self, width: f32, color: Color) -> Self {
        self.deco = self.deco.border_y(width, color);
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

    pub fn style(self, style: BoxStyle) -> Self {
        self.box_style(style)
    }
}

impl View for DockArea {
    fn body(&self) -> impl View + 'static {
        Unit
    }
    fn children_override(&self) -> Option<Vec<AnyView>> {
        Some(self.children.clone())
    }
    fn layout_style(&self) -> LayoutStyle {
        self.style
    }
    fn dock(&self) -> Option<DockStyle> {
        Some(DockStyle {
            docks: self.docks.clone(),
            gap: self.gap,
            last_child_fill: self.last_child_fill,
            ordered: self.ordered,
        })
    }
    fn paint_self(&self, ctx: &mut PaintCtx) {
        ctx.paint_box(self.deco);
    }
}
