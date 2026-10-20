#![forbid(unsafe_code)]

use sequoia_core::Vec2;

use crate::any::AnyView;
use crate::live::Live;
use crate::runtime::{ElementId, Runtime};
use crate::view::View;
use sequoia_animation::Animate;

#[derive(Clone)]
pub struct Scrub {
    child: AnyView,
    progress: Live<f32>,
    anim: Animate,
}

impl Scrub {
    pub fn new<V: View + 'static>(child: V, progress: impl Into<Live<f32>>, anim: Animate) -> Self {
        Self {
            child: AnyView::new(child),
            progress: progress.into(),
            anim,
        }
    }

    pub fn progress_untracked(&self) -> f32 {
        self.progress.target_untracked().clamp(0.0, 1.0)
    }

    fn frame(&self) -> (Option<f32>, Option<Vec2>) {
        let p = self.progress_untracked();
        let total = (self.anim.duration_secs + self.anim.delay_secs).max(0.0);
        let f = self.anim.sample_at(p * total);
        (f.opacity, f.offset)
    }
}

impl View for Scrub {
    fn body(&self) -> impl View + 'static {
        crate::views::Unit
    }

    fn children_override(&self) -> Option<Vec<AnyView>> {
        let _ = self.progress.target();
        Some(vec![self.child.clone()])
    }

    fn paint_opacity(&self) -> Option<f32> {
        self.frame().0
    }

    fn paint_offset(&self) -> Vec2 {
        self.frame().1.unwrap_or(Vec2::ZERO)
    }

    fn paint_opacity_ctx(&self, _rt: &Runtime, _el: ElementId) -> Option<f32> {
        self.frame().0
    }

    fn paint_offset_ctx(&self, _rt: &Runtime, _el: ElementId) -> Vec2 {
        self.frame().1.unwrap_or(Vec2::ZERO)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::live::SignalLiveExt;
    use sequoia_reactive::Signal;

    fn anim() -> Animate {
        sequoia_animation::animate()
            .opacity(0.0..1.0)
            .translate_y(-8.0..0.0)
            .build()
    }

    #[test]
    fn scrub_maps_progress_to_frame() {
        let p = Signal::new(0.0f32);
        let s = Scrub::new(crate::views::Unit, p.clone().live(), anim());
        assert_eq!(s.paint_opacity(), Some(0.0));
        assert_eq!(s.paint_offset(), Vec2::new(0.0, -8.0));
        p.set(1.0);
        assert_eq!(s.paint_opacity(), Some(1.0));
        assert_eq!(s.paint_offset(), Vec2::ZERO);
    }

    #[test]
    fn scrub_clamps_progress() {
        let s = Scrub::new(crate::views::Unit, Live::fixed(-5.0f32), anim());
        assert_eq!(s.paint_opacity(), Some(0.0));
        let s = Scrub::new(crate::views::Unit, Live::fixed(99.0f32), anim());
        assert_eq!(s.paint_opacity(), Some(1.0));
    }

    #[test]
    fn scrub_tracks_signal() {
        let p = Signal::new(0.25f32);
        let s = Scrub::new(crate::views::Unit, p.clone().live(), anim());
        let kids = s.children_override().expect("dziecko");
        assert_eq!(kids.len(), 1);
        p.set(0.75);
        assert!((s.progress_untracked() - 0.75).abs() < 1e-6);
    }
}
