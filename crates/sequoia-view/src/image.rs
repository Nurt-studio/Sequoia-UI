use std::rc::Rc;

use sequoia_assets::AssetRegistry;
use sequoia_core::{ImageSource, Length, Rect, Size};
use sequoia_layout::{Indents, LayoutCtx, LayoutStyle};

use crate::view::{PaintCtx, View};
use crate::views::Unit;

pub struct Image {
    source: ImageSource,
    style: LayoutStyle,
}

impl Image {
    pub fn file(path: impl Into<String>) -> Self {
        Self {
            source: ImageSource::File(path.into()),
            style: LayoutStyle {
                width: Length::Fill(1.0),
                height: Length::Fill(1.0),
                ..LayoutStyle::default()
            },
        }
    }

    pub fn rgba(id: u64, width: u32, height: u32, data: Vec<u8>) -> Self {
        Self {
            source: ImageSource::Rgba {
                id,
                width: width.max(1),
                height: height.max(1),
                data: Rc::from(data.into_boxed_slice()),
            },
            style: LayoutStyle {
                width: Length::Fill(1.0),
                height: Length::Fill(1.0),
                ..LayoutStyle::default()
            },
        }
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

    pub fn margin(mut self, margin: impl Into<sequoia_core::Edges>) -> Self {
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

    pub fn dynamic(id: u64, width: u32, height: u32, data: Vec<u8>, version: u64) -> Self {
        Self {
            source: ImageSource::Dynamic {
                id,
                width: width.max(1),
                height: height.max(1),
                data: Rc::from(data.into_boxed_slice()),
                version,
            },
            style: LayoutStyle {
                width: Length::Fill(1.0),
                height: Length::Fill(1.0),
                ..LayoutStyle::default()
            },
        }
    }

    #[allow(unreachable_patterns)]
    fn aspect(&self, ctx: &PaintCtx) -> Option<(f32, f32)> {
        match &self.source {
            ImageSource::Rgba { width, height, .. }
            | ImageSource::Dynamic { width, height, .. } => {
                if *width > 0 && *height > 0 {
                    Some((*width as f32, *height as f32))
                } else {
                    None
                }
            }
            ImageSource::File(path) => ctx
                .environment
                .get::<AssetRegistry>()
                .and_then(|assets| assets.photo_size(path).ok())
                .filter(|(w, h)| *w > 0 && *h > 0)
                .map(|(w, h)| (w as f32, h as f32)),

            _ => None,
        }
    }
}

fn contain_fit(outer: Rect, aspect: (f32, f32)) -> Rect {
    let (aw, ah) = aspect;
    if aw <= 0.0 || ah <= 0.0 || outer.size.width <= 0.0 || outer.size.height <= 0.0 {
        return outer;
    }
    let scale = (outer.size.width / aw).min(outer.size.height / ah);
    let w = aw * scale;
    let h = ah * scale;
    Rect::from_xywh(
        outer.left() + (outer.size.width - w) * 0.5,
        outer.top() + (outer.size.height - h) * 0.5,
        w,
        h,
    )
}

impl View for Image {
    fn body(&self) -> impl View + 'static {
        Unit
    }
    fn is_leaf(&self) -> bool {
        true
    }
    fn layout_style(&self) -> LayoutStyle {
        self.style
    }

    #[allow(unreachable_patterns)]
    fn intrinsic_size(&self, _ctx: &mut LayoutCtx) -> Option<Size> {
        match &self.source {
            ImageSource::Rgba { width, height, .. }
            | ImageSource::Dynamic { width, height, .. } => {
                Some(Size::new(*width as f32, *height as f32))
            }
            ImageSource::File(_) => None,

            _ => None,
        }
    }
    fn paint_self(&self, ctx: &mut PaintCtx) {
        let rect = ctx.rect;
        if rect.size.width <= 0.0 || rect.size.height <= 0.0 {
            return;
        }
        let dst = self
            .aspect(ctx)
            .map(|a| contain_fit(rect, a))
            .unwrap_or(rect);
        if dst.is_empty() {
            return;
        }
        ctx.image(dst, [0.0, 0.0, 1.0, 1.0], self.source.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contain_fit_centers_without_distortion() {
        let outer = Rect::from_xywh(0.0, 0.0, 100.0, 100.0);

        let r = contain_fit(outer, (40.0, 30.0));
        assert_eq!((r.size.width, r.size.height), (100.0, 75.0));
        assert_eq!((r.left(), r.top()), (0.0, 12.5));

        let r = contain_fit(outer, (30.0, 40.0));
        assert_eq!((r.size.width, r.size.height), (75.0, 100.0));
        assert_eq!((r.left(), r.top()), (12.5, 0.0));
    }

    #[test]
    fn contain_fit_degenerate_falls_back_to_outer() {
        let outer = Rect::from_xywh(1.0, 2.0, 10.0, 10.0);
        assert_eq!(contain_fit(outer, (0.0, 5.0)), outer);
        assert_eq!(contain_fit(Rect::ZERO, (4.0, 3.0)), Rect::ZERO);
    }
}
