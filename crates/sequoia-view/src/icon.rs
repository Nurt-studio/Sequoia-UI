use sequoia_assets::{AssetRegistry, icon_key};
use sequoia_core::{Color, Edges, Length, Rect, Size};
use sequoia_layout::{Indents, LayoutCtx, LayoutStyle};

use crate::view::{PaintCtx, View};
use crate::views::Unit;

const DEFAULT_SIZE: f32 = 16.0;

pub struct Icon {
    name: String,
    size: Option<f32>,
    color: Option<Color>,
    style: LayoutStyle,
}

impl Icon {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            size: None,
            color: None,
            style: LayoutStyle {
                width: Length::Fit,
                height: Length::Fit,
                ..LayoutStyle::default()
            },
        }
    }

    pub fn size(mut self, px: f32) -> Self {
        self.size = Some(px);
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
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

    pub fn padding(mut self, padding: impl Into<Edges>) -> Self {
        self.style = self.style.padding(padding);
        self
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

    pub fn hit_testable(mut self, on: bool) -> Self {
        self.style = self.style.hit_testable(on);
        self
    }

    fn px(&self) -> f32 {
        self.size.unwrap_or(DEFAULT_SIZE)
    }
}

impl View for Icon {
    fn body(&self) -> impl View + 'static {
        Unit
    }
    fn is_leaf(&self) -> bool {
        true
    }
    fn layout_style(&self) -> LayoutStyle {
        self.style
    }
    fn intrinsic_size(&self, _ctx: &mut LayoutCtx) -> Option<Size> {
        let px = self.px();
        let pad = self.style.padding;
        Some(Size::new(
            px + pad.horizontal_sum(),
            px + pad.vertical_sum(),
        ))
    }
    fn paint_self(&self, ctx: &mut PaintCtx) {
        let Some(assets) = ctx.environment.get::<AssetRegistry>() else {
            return;
        };
        let px = self.px();

        let raster_px = ((px * ctx.text.scale_factor()).round() as u32).max(1);
        let Ok(sprite) = assets.rasterize_icon(&self.name, raster_px) else {
            return;
        };
        let pad = self.style.padding;
        let r = Rect::from_xywh(
            ctx.rect.left() + pad.left,
            ctx.rect.top() + pad.top,
            (ctx.rect.size.width - pad.horizontal_sum()).max(0.0),
            (ctx.rect.size.height - pad.vertical_sum()).max(0.0),
        );
        let dst = Rect::from_xywh(
            r.left() + (r.size.width - px) * 0.5,
            r.top() + (r.size.height - px) * 0.5,
            px,
            px,
        );
        let color = self.color.unwrap_or_else(|| ctx.theme().text);
        ctx.draw_sprite(
            icon_key(&self.name, raster_px),
            sprite.width,
            sprite.height,
            &sprite.alpha,
            dst,
            color,
        );
    }
}
