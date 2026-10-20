use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use sequoia_animation::{Animate, Easing};
use sequoia_core::{Length, Vec2};
use sequoia_layout::LayoutStyle;
use sequoia_reactive::Signal;

use crate::any::AnyView;
use crate::view::{PaintCtx, View};
use crate::views::Unit;

pub const ENTER_SECS: f32 = 0.2;
pub const EXIT_SECS: f32 = 0.2;

pub fn enter_sample(from: f32, delay: f32, duration: f32, easing: Easing, elapsed: f32) -> f32 {
    crate::animated::fade_sample(from, delay, duration, easing, elapsed)
}

pub fn enter_progress(delay: f32, duration: f32, easing: Easing, elapsed: f32) -> f32 {
    crate::animated::slide_progress(delay, duration, easing, elapsed)
}

pub fn exit_sample(delay: f32, duration: f32, easing: Easing, elapsed: f32) -> f32 {
    let delay = delay.max(0.0);
    if elapsed < delay {
        return 1.0;
    }
    if duration <= 0.0 {
        return 0.0;
    }
    let t = (elapsed - delay) / duration;
    if t <= 0.0 {
        return 1.0;
    }
    if t >= 1.0 {
        return 0.0;
    }
    1.0 - easing.apply(t)
}

pub fn exit_progress(delay: f32, duration: f32, easing: Easing, elapsed: f32) -> f32 {
    let delay = delay.max(0.0);
    if elapsed < delay {
        return 0.0;
    }
    if duration <= 0.0 {
        return 1.0;
    }
    let t = (elapsed - delay) / duration;
    if t <= 0.0 {
        return 0.0;
    }
    if t >= 1.0 {
        return 1.0;
    }
    easing.apply(t)
}

pub fn exit_done(delay: f32, duration: f32, elapsed: f32) -> bool {
    elapsed >= delay.max(0.0) + duration.max(0.0)
}

#[derive(Clone, Copy)]
struct PresenceState {
    was_present: bool,
    enter_start: Option<Instant>,
    exit_start: Option<Instant>,
    exit_fired: bool,
}

#[derive(Clone)]
pub struct Presence {
    present: Signal<bool>,
    child: Option<AnyView>,
    enter_from: f32,
    enter_secs: f32,
    enter_delay: f32,
    enter_easing: Easing,
    exit_secs: f32,
    exit_delay: f32,
    exit_easing: Easing,
    slide: Vec2,
    on_exit_complete: Option<Rc<dyn Fn()>>,
    state: Rc<RefCell<PresenceState>>,

    enter_anim: Option<Animate>,
    exit_anim: Option<Animate>,
}

impl Presence {
    pub fn new(present: Signal<bool>) -> Self {
        Self {
            present,
            child: None,
            enter_from: 0.0,
            enter_secs: ENTER_SECS,
            enter_delay: 0.0,
            enter_easing: Easing::EaseOutCubic,
            exit_secs: EXIT_SECS,
            exit_delay: 0.0,
            exit_easing: Easing::EaseOutCubic,
            slide: Vec2::ZERO,
            on_exit_complete: None,
            state: Rc::new(RefCell::new(PresenceState {
                was_present: false,
                enter_start: None,
                exit_start: None,
                exit_fired: false,
            })),
            enter_anim: None,
            exit_anim: None,
        }
    }

    pub fn child<V: View + 'static>(mut self, child: V) -> Self {
        self.child = Some(AnyView::new(child));
        self
    }

    pub fn from(mut self, opacity: f32) -> Self {
        self.enter_from = opacity.clamp(0.0, 1.0);
        self.enter_anim = None;
        self
    }

    pub fn enter_duration(mut self, secs: f32) -> Self {
        self.enter_secs = secs.max(0.0);
        self.enter_anim = None;
        self
    }

    pub fn enter_duration_d(mut self, d: Duration) -> Self {
        self.enter_secs = d.as_secs_f32().max(0.0);
        self.enter_anim = None;
        self
    }

    pub fn enter_delay(mut self, secs: f32) -> Self {
        self.enter_delay = secs.max(0.0);
        self.enter_anim = None;
        self
    }

    pub fn enter_easing(mut self, easing: Easing) -> Self {
        self.enter_easing = easing;
        self.enter_anim = None;
        self
    }

    pub fn exit_duration(mut self, secs: f32) -> Self {
        self.exit_secs = secs.max(0.0);
        self.exit_anim = None;
        self
    }

    pub fn exit_duration_d(mut self, d: Duration) -> Self {
        self.exit_secs = d.as_secs_f32().max(0.0);
        self.exit_anim = None;
        self
    }

    pub fn exit_delay(mut self, secs: f32) -> Self {
        self.exit_delay = secs.max(0.0);
        self.exit_anim = None;
        self
    }

    pub fn exit_easing(mut self, easing: Easing) -> Self {
        self.exit_easing = easing;
        self.exit_anim = None;
        self
    }

    pub fn durations(mut self, enter_secs: f32, exit_secs: f32) -> Self {
        self.enter_secs = enter_secs.max(0.0);
        self.exit_secs = exit_secs.max(0.0);
        self.enter_anim = None;
        self.exit_anim = None;
        self
    }

    pub fn durations_d(mut self, enter: Duration, exit: Duration) -> Self {
        self.enter_secs = enter.as_secs_f32().max(0.0);
        self.exit_secs = exit.as_secs_f32().max(0.0);
        self.enter_anim = None;
        self.exit_anim = None;
        self
    }

    pub fn easing(mut self, easing: Easing) -> Self {
        self.enter_easing = easing;
        self.exit_easing = easing;
        self.enter_anim = None;
        self.exit_anim = None;
        self
    }

    pub fn slide(mut self, dx: f32, dy: f32) -> Self {
        self.slide = Vec2::new(dx, dy);
        self.enter_anim = None;
        self.exit_anim = None;
        self
    }

    pub fn enter(mut self, anim: impl Into<Animate>) -> Self {
        let a = anim.into();
        if let Some((from, _)) = a.opacity {
            self.enter_from = from;
        }
        self.enter_secs = a.duration_secs;
        self.enter_delay = a.delay_secs;
        self.enter_easing = a.easing;
        if let Some((from, _)) = a.offset {
            self.slide = from;
        }
        self.enter_anim = Some(a);
        self
    }

    pub fn exit(mut self, anim: impl Into<Animate>) -> Self {
        let a = anim.into();
        self.exit_secs = a.duration_secs;
        self.exit_delay = a.delay_secs;
        self.exit_easing = a.easing;
        if let Some((_, to)) = a.offset {
            self.slide = to;
        } else if let Some((from, _)) = a.offset {
            self.slide = from;
        }
        self.exit_anim = Some(a);
        self
    }

    pub fn on_exit_complete(mut self, f: impl Fn() + 'static) -> Self {
        self.on_exit_complete = Some(Rc::new(f));
        self
    }

    pub fn sample_enter_at(&self, elapsed_secs: f32) -> f32 {
        if let Some(a) = &self.enter_anim
            && let Some(v) = a.sample_at(elapsed_secs).opacity
        {
            return v;
        }
        enter_sample(
            self.enter_from,
            self.enter_delay,
            self.enter_secs,
            self.enter_easing,
            elapsed_secs,
        )
    }

    pub fn sample_exit_at(&self, elapsed_secs: f32) -> f32 {
        if let Some(a) = &self.exit_anim
            && let Some(v) = a.sample_at(elapsed_secs).opacity
        {
            return v;
        }
        exit_sample(
            self.exit_delay,
            self.exit_secs,
            self.exit_easing,
            elapsed_secs,
        )
    }

    fn sample_enter_offset_at(&self, elapsed_secs: f32) -> Vec2 {
        if let Some(a) = &self.enter_anim
            && let Some(v) = a.sample_at(elapsed_secs).offset
        {
            return v;
        }
        let p = enter_progress(
            self.enter_delay,
            self.enter_secs,
            self.enter_easing,
            elapsed_secs,
        );
        Vec2::new(self.slide.x * (1.0 - p), self.slide.y * (1.0 - p))
    }

    fn sample_exit_offset_at(&self, elapsed_secs: f32) -> Vec2 {
        if let Some(a) = &self.exit_anim
            && let Some(v) = a.sample_at(elapsed_secs).offset
        {
            return v;
        }
        let p = exit_progress(
            self.exit_delay,
            self.exit_secs,
            self.exit_easing,
            elapsed_secs,
        );
        Vec2::new(self.slide.x * p, self.slide.y * p)
    }

    pub fn is_mounted(&self) -> bool {
        if self.present.get_untracked() {
            return self.child.is_some();
        }
        let st = *self.state.borrow();
        if !st.was_present || self.child.is_none() {
            return false;
        }
        match st.exit_start {
            None => true,
            Some(t0) => !exit_done(
                self.exit_delay,
                self.exit_secs,
                Instant::now().duration_since(t0).as_secs_f32(),
            ),
        }
    }

    fn wall_since(slot: &Option<Instant>) -> f32 {
        match slot {
            Some(t0) => Instant::now().duration_since(*t0).as_secs_f32().min(600.0),
            None => 0.0,
        }
    }
}

impl View for Presence {
    fn body(&self) -> impl View + 'static {
        Unit
    }
    fn reuse_state(&self, old: &Self) {
        if Rc::ptr_eq(&self.state, &old.state) {
            return;
        }
        let prev = *old.state.borrow();
        *self.state.borrow_mut() = prev;
    }
    fn keyed_children(&self) -> Option<Vec<(u64, AnyView)>> {
        let present = self.present.get();
        let Some(child) = self.child.clone() else {
            return Some(Vec::new());
        };
        let mut st = self.state.borrow_mut();
        if present {
            if !st.was_present {
                st.was_present = true;
                st.enter_start = Some(Instant::now());
                st.exit_start = None;
                st.exit_fired = false;
            }
            Some(vec![(0, child)])
        } else {
            if !st.was_present {
                return Some(Vec::new());
            }
            if st.exit_start.is_none() {
                st.exit_start = Some(Instant::now());
            }
            let elapsed = Self::wall_since(&st.exit_start);
            if exit_done(self.exit_delay, self.exit_secs, elapsed) {
                st.was_present = false;
                st.enter_start = None;
                st.exit_start = None;
                Some(Vec::new())
            } else {
                Some(vec![(0, child)])
            }
        }
    }
    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fit,
            height: Length::Fit,
            ..LayoutStyle::default()
        }
    }
    fn paint_opacity(&self) -> Option<f32> {
        let st = *self.state.borrow();
        if self.present.get_untracked() {
            if !st.was_present {
                if let Some(a) = &self.enter_anim {
                    return a.sample_at(0.0).opacity.or(Some(self.enter_from));
                }
                return Some(self.enter_from);
            }
            let el = Self::wall_since(&st.enter_start);
            if self
                .enter_anim
                .as_ref()
                .is_some_and(|a| a.opacity.is_none())
            {
                return None;
            }
            return Some(self.sample_enter_at(el));
        }
        if !st.was_present {
            return None;
        }
        let el = Self::wall_since(&st.exit_start);
        if self.exit_anim.as_ref().is_some_and(|a| a.opacity.is_none()) {
            return None;
        }
        Some(self.sample_exit_at(el))
    }
    fn paint_offset(&self) -> Vec2 {
        let st = *self.state.borrow();
        if self.present.get_untracked() {
            if !st.was_present {
                return self
                    .enter_anim
                    .as_ref()
                    .and_then(|a| a.sample_at(0.0).offset)
                    .unwrap_or(self.slide);
            }
            let el = Self::wall_since(&st.enter_start);
            return self.sample_enter_offset_at(el);
        }
        if !st.was_present {
            return Vec2::ZERO;
        }
        let el = Self::wall_since(&st.exit_start);
        self.sample_exit_offset_at(el)
    }
    fn paint_self(&self, ctx: &mut PaintCtx) {
        let present = self.present.get_untracked();
        let st = *self.state.borrow();
        if present {
            if st.was_present {
                let el = Self::wall_since(&st.enter_start);
                if el < self.enter_delay + self.enter_secs {
                    ctx.request_frame();
                }
            }
            return;
        }
        if !st.was_present {
            if !st.exit_fired && self.child.is_some() {
                self.state.borrow_mut().exit_fired = true;
                if let Some(f) = &self.on_exit_complete {
                    f();
                }
            }
            return;
        }
        let el = Self::wall_since(&st.exit_start);
        if exit_done(self.exit_delay, self.exit_secs, el) {
            self.state.borrow_mut().exit_fired = true;
            if let Some(f) = &self.on_exit_complete {
                f();
            }
        } else {
            ctx.request_frame();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_walks_one_to_zero() {
        let p = Presence::new(Signal::new(false));
        assert_eq!(p.sample_exit_at(-1.0), 1.0, "before delay full");
        let mid = p.sample_exit_at(0.1);
        assert!(mid > 0.0 && mid < 1.0, "output middle: {mid}");
        assert_eq!(p.sample_exit_at(10.0), 0.0);
    }

    #[test]
    fn exit_zero_duration_cuts() {
        let p = Presence::new(Signal::new(false)).exit_duration(0.0);
        assert_eq!(p.sample_exit_at(0.0), 0.0, "zero time = cut");
        assert!(exit_done(0.0, 0.0, 0.0));
        assert!(!exit_done(0.0, 0.2, 0.19));
        assert!(exit_done(0.0, 0.2, 0.2));
    }

    #[test]
    fn exit_delay_holds_full_then_fades() {
        let p = Presence::new(Signal::new(false))
            .exit_delay(0.5)
            .exit_duration(0.2);
        assert_eq!(p.sample_exit_at(0.0), 1.0);
        assert_eq!(p.sample_exit_at(0.49), 1.0);
        assert!(p.sample_exit_at(0.6) < 1.0);
        assert!(!exit_done(0.5, 0.2, 0.69));
        assert!(exit_done(0.5, 0.2, 0.7));
    }

    #[test]
    fn enter_walks_from_to_one() {
        let p = Presence::new(Signal::new(true)).from(0.3);
        assert_eq!(p.sample_enter_at(-1.0), 0.3);
        assert_eq!(p.sample_enter_at(10.0), 1.0);
    }

    #[test]
    fn exit_progress_drives_slide() {
        assert_eq!(exit_progress(0.0, 0.2, Easing::Linear, 0.0), 0.0);
        assert_eq!(exit_progress(0.0, 0.2, Easing::Linear, 10.0), 1.0);
        let mid = exit_progress(0.0, 0.2, Easing::Linear, 0.1);
        assert!((mid - 0.5).abs() < 1e-4, "linear center: {mid}");
    }

    #[test]
    fn reuse_state_keeps_finished_enter() {
        let old = Presence::new(Signal::new(true)).child(Unit);
        let _ = old.keyed_children();
        std::thread::sleep(Duration::from_millis(300));
        let fresh = Presence::new(Signal::new(true)).child(Unit);
        fresh.reuse_state(&old);
        let op = fresh.paint_opacity().unwrap_or(0.0);
        assert!(op > 0.99, "rebuilding does not restart input: {op}");
    }
}
