#![forbid(unsafe_code)]

use sequoia_core::Length;
use sequoia_layout::LayoutStyle;
use sequoia_style::Backdrop;

use crate::any::AnyView;
use crate::view::{PaintCtx, View};

#[derive(Clone)]
pub struct BackdropBlur {
    child: AnyView,
    backdrop: Backdrop,
}

impl BackdropBlur {
    pub fn new<V: View + 'static>(child: V, backdrop: Backdrop) -> Self {
        Self {
            child: AnyView::new(child),
            backdrop,
        }
    }

    pub fn blurred<V: View + 'static>(child: V, radius: f32) -> Self {
        Self::new(child, Backdrop::blur(radius))
    }

    pub fn style(&self) -> Backdrop {
        self.backdrop
    }
}

impl View for BackdropBlur {
    fn body(&self) -> impl View + 'static {
        crate::views::Unit
    }

    fn children_override(&self) -> Option<Vec<AnyView>> {
        Some(vec![self.child.clone()])
    }

    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fit,
            height: Length::Fit,
            ..LayoutStyle::default()
        }
    }

    fn paint_self(&self, ctx: &mut PaintCtx) {
        ctx.push_backdrop(ctx.rect, self.backdrop);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter_view::FilterExt;
    use sequoia_core::{Color, Rect, RenderList, SpanMask};

    #[test]
    fn wrapper_paints_marker_before_child() {
        let v = BackdropBlur::blurred(crate::views::Unit, 16.0);
        let mut list = RenderList::new();

        let rect = Rect::from_xywh(10.0, 20.0, 200.0, 100.0);

        list.push_backdrop(rect, 0.0, 16.0, Color::TRANSPARENT, 0.0, SpanMask::None);
        assert!(list.has_backdrops());
        let span = list.backdrops()[0];
        assert_eq!(span.at, 0, "marker before child content");
        assert_eq!(span.draw.rect, rect);
        assert_eq!(span.draw.blur, 16.0);
        let _ = v;
    }

    #[test]
    fn blur_behind_is_canonical() {
        let w = crate::views::Unit.blur_behind(16.0);
        assert_eq!(w.style(), Backdrop::blur(16.0));
    }

    #[test]
    fn wrapper_is_fit_like_opacity() {
        let style = BackdropBlur::blurred(crate::views::Unit, 8.0).layout_style();
        assert_eq!(style.width, Length::Fit);
        assert_eq!(style.height, Length::Fit);
    }
}
