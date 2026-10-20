use std::rc::Rc;

use sequoia_core::{Edges, Length};
use sequoia_layout::LayoutStyle;
use sequoia_style::{BoxStyle, Theme};
use sequoia_view::{AnyView, PaintCtx, Unit, View};

pub use crate::container::Container;
pub use sequoia_view::{Canvas, Column, Icon, Image, Portal, Row, ScrollView, Text};

pub const PRESS_BLEND_RATE: f32 = 12.0;

#[derive(Clone)]
pub struct Deco {
    content: AnyView,
    resolve: Rc<dyn Fn(&Theme) -> BoxStyle>,
    style: LayoutStyle,
    hit: bool,
}

impl Deco {
    pub fn new<V: View + 'static>(
        content: V,
        resolve: impl Fn(&Theme) -> BoxStyle + 'static,
    ) -> Self {
        Self {
            content: AnyView::new(content),
            resolve: Rc::new(resolve),
            style: LayoutStyle {
                width: Length::Fit,
                height: Length::Fit,
                ..LayoutStyle::default()
            },
            hit: true,
        }
    }

    pub fn fixed<V: View + 'static>(content: V, deco: BoxStyle) -> Self {
        Self::new(content, move |_| deco)
    }

    pub fn size_px(mut self, w: f32, h: f32) -> Self {
        self.style = self
            .style
            .width(Length::Px(w.max(0.0)))
            .height(Length::Px(h.max(0.0)));
        self
    }

    pub fn square_px(self, px: f32) -> Self {
        self.size_px(px, px)
    }

    pub fn hit_testable(mut self, on: bool) -> Self {
        self.hit = on;
        self
    }

    pub fn margin(mut self, margin: impl Into<Edges>) -> Self {
        self.style = self.style.margin(margin);
        self
    }

    pub fn margin_xy(self, x: f32, y: f32) -> Self {
        self.margin(Edges::xy(x, y))
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
}

impl View for Deco {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn children_override(&self) -> Option<Vec<AnyView>> {
        Some(vec![self.content.clone()])
    }

    fn layout_style(&self) -> LayoutStyle {
        self.style
    }

    fn hit_testable(&self) -> bool {
        self.hit
    }

    fn paint_self(&self, ctx: &mut PaintCtx) {
        ctx.paint_box((self.resolve)(&ctx.theme()));
    }
}
