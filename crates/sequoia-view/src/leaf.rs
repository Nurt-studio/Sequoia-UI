use sequoia_core::Size;
use sequoia_input::{CursorIcon, Event};
use sequoia_layout::{LayoutCtx, LayoutStyle};
use sequoia_style::VisualState;

use crate::view::{EventCtx, PaintCtx};

pub trait Leaf: 'static {
    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle::default()
    }

    fn intrinsic_size(&self, _ctx: &mut LayoutCtx) -> Option<Size> {
        None
    }

    fn paint(&self, ctx: &mut PaintCtx);

    fn on_event(&self, _event: &Event, _ctx: &mut EventCtx) {}

    fn focusable(&self) -> bool {
        false
    }

    fn has_visual_state(&self) -> bool {
        false
    }

    fn cursor(&self, _state: VisualState) -> CursorIcon {
        CursorIcon::Default
    }

    fn track(&self) {}
}

#[macro_export]
macro_rules! impl_view_for_leaf {
    ($t:ty) => {
        impl $crate::View for $t {
            fn body(&self) -> impl $crate::View + 'static {
                <$t as $crate::Leaf>::track(self);
                $crate::Unit
            }
            fn is_leaf(&self) -> bool {
                true
            }
            fn layout_style(&self) -> $crate::LayoutStyle {
                <$t as $crate::Leaf>::layout_style(self)
            }
            fn intrinsic_size(&self, ctx: &mut $crate::LayoutCtx) -> Option<$crate::Size> {
                <$t as $crate::Leaf>::intrinsic_size(self, ctx)
            }
            fn paint_self(&self, ctx: &mut $crate::PaintCtx) {
                <$t as $crate::Leaf>::paint(self, ctx)
            }
            fn on_event(&self, event: &$crate::Event, ctx: &mut $crate::EventCtx) {
                <$t as $crate::Leaf>::on_event(self, event, ctx)
            }
            fn focusable(&self) -> bool {
                <$t as $crate::Leaf>::focusable(self)
            }
            fn has_visual_state(&self) -> bool {
                <$t as $crate::Leaf>::has_visual_state(self)
            }
            fn cursor_icon(&self, state: $crate::VisualState) -> $crate::CursorIcon {
                <$t as $crate::Leaf>::cursor(self, state)
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::View;
    use crate::views::{Row, Unit};
    use sequoia_core::{Color, Length, Point};

    #[derive(Clone)]
    struct Dot {
        color: Color,
    }

    impl Leaf for Dot {
        fn layout_style(&self) -> LayoutStyle {
            LayoutStyle::default()
                .width(Length::Px(12.0))
                .height(Length::Px(12.0))
        }

        fn paint(&self, ctx: &mut PaintCtx) {
            let c = Point::new(ctx.rect.origin.x + 6.0, ctx.rect.origin.y + 6.0);
            ctx.disk(c, 6.0, self.color);
        }
    }

    impl_view_for_leaf!(Dot);

    #[test]
    fn leaf_is_a_leaf_view() {
        let d = Dot { color: Color::RED };
        assert!(View::is_leaf(&d));
        let _ = View::body(&d);
    }

    #[test]
    fn leaf_paints_disk_without_panicking() {
        use crate::runtime::Runtime;
        use sequoia_core::Rect;
        use sequoia_text::TextSystem;

        let rt = Runtime::new();
        rt.mount(Row::new().child(Dot {
            color: Color::GREEN,
        }));
        let mut text = TextSystem::new();
        rt.layout(Rect::from_xywh(0.0, 0.0, 200.0, 100.0), &mut text);
        let mut list = sequoia_core::RenderList::new();
        rt.paint(&mut list, &mut text);
        assert!(!list.is_empty(), "leaf should paint something");
        assert!(rt.errors().is_empty());
    }

    #[test]
    fn leaf_with_signal_tracks() {
        use sequoia_reactive::Signal;

        #[derive(Clone)]
        struct Badge {
            label: Signal<String>,
        }

        impl Leaf for Badge {
            fn track(&self) {
                let _ = self.label.get();
            }

            fn paint(&self, ctx: &mut PaintCtx) {
                let s = self.label.get_untracked();
                ctx.draw_text(
                    &s,
                    (ctx.rect.left(), ctx.rect.top() + 12.0),
                    12.0,
                    Color::WHITE,
                );
            }
        }

        impl_view_for_leaf!(Badge);

        let sig = Signal::new(String::from("hi"));
        let b = Badge { label: sig.clone() };
        let _ = View::body(&b);
        assert_eq!(b.label.get_untracked(), "hi");
        let _ = Unit;
    }
}
