#![forbid(unsafe_code)]

use sequoia_core::Length;
use sequoia_layout::LayoutStyle;

use crate::any::AnyView;
use crate::live::{AnimSpec, Live, Prop, SpringSpec};
use crate::runtime::{ElementId, Runtime};
use crate::view::View;

#[derive(Clone)]
pub struct AnimatedSize {
    child: AnyView,
    width: Option<Live<f32>>,
    height: Option<Live<f32>>,
}

impl AnimatedSize {
    pub fn new<V: View + 'static>(
        child: V,
        width: Option<Live<f32>>,
        height: Option<Live<f32>>,
    ) -> Self {
        Self {
            child: AnyView::new(child),
            width,
            height,
        }
    }

    pub fn width<V: View + 'static>(child: V, width: impl Into<Live<f32>>) -> Self {
        Self::new(child, Some(width.into()), None)
    }

    pub fn height<V: View + 'static>(child: V, height: impl Into<Live<f32>>) -> Self {
        Self::new(child, None, Some(height.into()))
    }

    pub fn size<V: View + 'static>(
        child: V,
        width: impl Into<Live<f32>>,
        height: impl Into<Live<f32>>,
    ) -> Self {
        Self::new(child, Some(width.into()), Some(height.into()))
    }

    pub fn animate(mut self, spec: impl Into<AnimSpec>) -> Self {
        let spec = spec.into();
        if let Some(w) = &mut self.width {
            w.animate_mut(spec);
        }
        if let Some(h) = &mut self.height {
            h.animate_mut(spec);
        }
        self
    }

    pub fn spring(mut self, spec: SpringSpec) -> Self {
        if let Some(w) = &mut self.width {
            w.spring_mut(spec);
        }
        if let Some(h) = &mut self.height {
            h.spring_mut(spec);
        }
        self
    }

    fn px_or_fit(live: &Option<Live<f32>>) -> Length {
        match live {
            Some(l) => Length::Px(l.target_untracked().max(0.0)),
            None => Length::Fit,
        }
    }
}

impl View for AnimatedSize {
    fn body(&self) -> impl View + 'static {
        crate::views::Unit
    }

    fn children_override(&self) -> Option<Vec<AnyView>> {
        if let Some(w) = &self.width {
            let _ = w.target();
        }
        if let Some(h) = &self.height {
            let _ = h.target();
        }
        Some(vec![self.child.clone()])
    }

    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Self::px_or_fit(&self.width),
            height: Self::px_or_fit(&self.height),
            ..LayoutStyle::default()
        }
    }

    fn layout_style_ctx(&self, rt: &Runtime, el: ElementId) -> LayoutStyle {
        let width = match &self.width {
            Some(live) => Length::Px(rt.resolve_live_f32(el, Prop::WIDTH, live).max(0.0)),
            None => Length::Fit,
        };
        let height = match &self.height {
            Some(live) => Length::Px(rt.resolve_live_f32(el, Prop::HEIGHT, live).max(0.0)),
            None => Length::Fit,
        };
        LayoutStyle {
            width,
            height,
            ..LayoutStyle::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::live::SignalLiveExt;
    use sequoia_reactive::Signal;
    use std::time::Duration;

    #[test]
    fn static_snapshot_without_runtime() {
        let w = AnimatedSize::width(crate::views::Unit, 120.0f32);
        let s = w.layout_style();
        assert_eq!(s.width, Length::Px(120.0));
        assert_eq!(s.height, Length::Fit);
    }

    #[test]
    fn tracks_signal_target() {
        let sig = Signal::new(80.0f32);
        let w = AnimatedSize::width(crate::views::Unit, sig.clone().live());
        let kids = w.children_override().expect("dziecko");
        assert_eq!(kids.len(), 1);
        sig.set(200.0);
        let s = w.layout_style();
        assert_eq!(s.width, Length::Px(200.0));
    }

    #[test]
    fn runtime_anchors_then_animates() {
        let rt = Runtime::new();
        let el = ElementId {
            index: 91,
            generation: 0,
        };
        let spec = AnimSpec::from_duration(
            Duration::from_secs_f32(10.0),
            sequoia_animation::Easing::Linear,
        );
        let live = Live::fixed(80.0f32).animate(spec);
        let w = AnimatedSize::width(crate::views::Unit, live);

        let s1 = w.layout_style_ctx(&rt, el);
        assert_eq!(s1.width, Length::Px(80.0));

        let live2 = Live::fixed(200.0f32).animate(spec);
        let w2 = AnimatedSize::width(crate::views::Unit, live2);
        let s2 = w2.layout_style_ctx(&rt, el);
        assert_eq!(s2.width, Length::Px(80.0));
        assert_eq!(rt.live_anim_count(), 1);
    }

    #[test]
    fn size_sets_both_axes() {
        let w = AnimatedSize::size(crate::views::Unit, 100.0f32, 40.0f32);
        let s = w.layout_style();
        assert_eq!(s.width, Length::Px(100.0));
        assert_eq!(s.height, Length::Px(40.0));
        let w = w.animate(AnimSpec::FAST).spring(SpringSpec::BOUNCY);
        let s = w.layout_style();
        assert_eq!(s.width, Length::Px(100.0));
    }
}
