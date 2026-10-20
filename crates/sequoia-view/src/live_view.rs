#![forbid(unsafe_code)]

use sequoia_core::{Color, Length, Vec2};
use sequoia_layout::LayoutStyle;

use crate::any::AnyView;
use crate::live::{AnimSpec, Live, Prop, SpringSpec};
use crate::runtime::{ElementId, Runtime};
use crate::view::{PaintCtx, View};

#[derive(Clone)]
pub struct LiveOpacity {
    child: AnyView,
    live: Live<f32>,
}

impl LiveOpacity {
    pub fn new<V: View + 'static>(child: V, live: impl Into<Live<f32>>) -> Self {
        Self {
            child: AnyView::new(child),
            live: live.into(),
        }
    }

    pub fn animate(mut self, spec: impl Into<AnimSpec>) -> Self {
        self.live.animate_mut(spec);
        self
    }

    pub fn spring(mut self, spec: SpringSpec) -> Self {
        self.live.spring_mut(spec);
        self
    }
}

impl View for LiveOpacity {
    fn body(&self) -> impl View + 'static {
        crate::views::Unit
    }

    fn children_override(&self) -> Option<Vec<AnyView>> {
        let _ = self.live.target();
        Some(vec![self.child.clone()])
    }

    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fit,
            height: Length::Fit,
            ..LayoutStyle::default()
        }
    }

    fn paint_opacity_ctx(&self, rt: &Runtime, el: ElementId) -> Option<f32> {
        Some(
            rt.resolve_live_f32(el, Prop::OPACITY, &self.live)
                .clamp(0.0, 1.0),
        )
    }
}

#[derive(Clone)]
pub struct LiveOffset {
    child: AnyView,
    live: Live<Vec2>,
}

impl LiveOffset {
    pub fn new<V: View + 'static>(child: V, live: impl Into<Live<Vec2>>) -> Self {
        Self {
            child: AnyView::new(child),
            live: live.into(),
        }
    }

    pub fn animate(mut self, spec: impl Into<AnimSpec>) -> Self {
        self.live.animate_mut(spec);
        self
    }

    pub fn spring(mut self, spec: SpringSpec) -> Self {
        self.live.spring_mut(spec);
        self
    }
}

impl View for LiveOffset {
    fn body(&self) -> impl View + 'static {
        crate::views::Unit
    }

    fn children_override(&self) -> Option<Vec<AnyView>> {
        let _ = self.live.target();
        Some(vec![self.child.clone()])
    }

    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fit,
            height: Length::Fit,
            ..LayoutStyle::default()
        }
    }

    fn paint_offset_ctx(&self, rt: &Runtime, el: ElementId) -> Vec2 {
        rt.resolve_live_vec2(el, Prop::OFFSET, &self.live)
    }
}

#[derive(Clone)]
pub struct LiveBackground {
    child: AnyView,
    live: Live<Color>,
    radius: f32,
}

impl LiveBackground {
    pub fn new<V: View + 'static>(child: V, live: impl Into<Live<Color>>) -> Self {
        Self {
            child: AnyView::new(child),
            live: live.into(),
            radius: 0.0,
        }
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius.max(0.0);
        self
    }

    pub fn animate(mut self, spec: impl Into<AnimSpec>) -> Self {
        self.live.animate_mut(spec);
        self
    }
}

impl View for LiveBackground {
    fn body(&self) -> impl View + 'static {
        crate::views::Unit
    }

    fn children_override(&self) -> Option<Vec<AnyView>> {
        let _ = self.live.target();
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
        let target = self.live.target_untracked();
        let (color, _) = ctx.runtime.live_color(
            ctx.element,
            Prop::BACKGROUND,
            target,
            self.live.tween_spec().as_ref(),
        );
        if color.a > 0.0 {
            ctx.quad(ctx.rect, color, self.radius);
        }
    }
}

pub trait LiveExt: View + Sized + 'static {
    fn opacity_live(self, live: impl Into<Live<f32>>) -> LiveOpacity {
        LiveOpacity::new(self, live)
    }

    fn offset_live(self, live: impl Into<Live<Vec2>>) -> LiveOffset {
        LiveOffset::new(self, live)
    }

    fn background_live(self, live: impl Into<Live<Color>>) -> LiveBackground {
        LiveBackground::new(self, live)
    }
}

impl<T: View + 'static> LiveExt for T {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::live::{AnimSpec, SignalLiveExt};
    use sequoia_animation::Easing;
    use sequoia_reactive::Signal;
    use std::time::Duration;

    #[test]
    fn live_wrappers_track_source() {
        let s = Signal::new(0.5f32);
        let w = crate::views::Unit.opacity_live(s.clone().live());

        let kids = w.children_override().expect("jedno dziecko");
        assert_eq!(kids.len(), 1);
        s.set(0.8);
        let kids = w.children_override().expect("jedno dziecko");
        assert_eq!(kids.len(), 1);
    }

    #[test]
    fn animated_live_carries_spec() {
        let s = Signal::new(8.0f32);
        let live = s.live().animate(AnimSpec::from_duration(
            Duration::from_secs_f32(0.15),
            Easing::Linear,
        ));
        assert!(live.is_animated());
        let w = crate::views::Unit.opacity_live(live);
        assert!(w.live.is_animated());
    }

    #[test]
    fn rebuild_does_not_restart_animation() {
        use crate::live::Prop;
        use crate::runtime::{ElementId, Runtime};
        let rt = Runtime::new();

        let el = ElementId {
            index: 7,
            generation: 0,
        };
        let spec = AnimSpec::from_duration(Duration::from_secs_f32(10.0), Easing::Linear);

        let w1 = Live::fixed(0.0f32).animate(spec);
        assert_eq!(rt.resolve_live_f32(el, Prop::OPACITY, &w1), 0.0);

        let w2 = Live::fixed(1.0f32).animate(spec);
        assert_eq!(rt.resolve_live_f32(el, Prop::OPACITY, &w2), 0.0);
        assert_eq!(rt.live_anim_count(), 1);

        let w3 = Live::fixed(1.0f32).animate(spec);
        let _ = rt.resolve_live_f32(el, Prop::BLUR, &w3);
        assert_eq!(rt.live_anim_count(), 2);
    }
}
