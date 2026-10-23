use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use sequoia_animation::{Animate, Easing};
use sequoia_core::{Length, Vec2};
use sequoia_layout::LayoutStyle;
use sequoia_reactive::Signal;

use crate::any::AnyView;
use crate::runtime::{ElementId, Runtime};
use crate::view::{PaintCtx, View};
use crate::views::Unit;

pub const ENTER_SECS: f32 = 0.2;
pub const EXIT_SECS: f32 = 0.2;

pub fn enter_sample(from: f32, delay: f32, duration: f32, easing: Easing, elapsed: f32) -> f32 {
    sample_to(from, 1.0, delay, duration, easing, elapsed)
}

pub fn enter_progress(delay: f32, duration: f32, easing: Easing, elapsed: f32) -> f32 {
    crate::animated::slide_progress(delay, duration, easing, elapsed)
}

pub fn exit_sample(delay: f32, duration: f32, easing: Easing, elapsed: f32) -> f32 {
    let p = exit_progress(delay, duration, easing, elapsed);
    1.0 - p
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

fn sample_to(from: f32, to: f32, delay: f32, duration: f32, easing: Easing, elapsed: f32) -> f32 {
    let p = enter_progress(delay, duration, easing, elapsed);
    from + (to - from) * p
}

fn lerp_vec(from: Vec2, to: Vec2, t: f32) -> Vec2 {
    Vec2::new(from.x + (to.x - from.x) * t, from.y + (to.y - from.y) * t)
}

#[derive(Clone, Copy)]
struct PresenceState {
    was_present: bool,
    enter_start: Option<f32>,
    exit_start: Option<f32>,
    exit_fired: bool,

    current_opacity: f32,
    current_offset: Vec2,
    current_scale: f32,

    enter_from: f32,
    enter_offset: Vec2,
    enter_scale: f32,

    exit_from: f32,
    exit_offset: Vec2,
    exit_scale: f32,
}

impl PresenceState {
    fn new() -> Self {
        Self {
            was_present: false,
            enter_start: None,
            exit_start: None,
            exit_fired: false,
            current_opacity: 0.0,
            current_offset: Vec2::ZERO,
            current_scale: 1.0,
            enter_from: 0.0,
            enter_offset: Vec2::ZERO,
            enter_scale: 1.0,
            exit_from: 1.0,
            exit_offset: Vec2::ZERO,
            exit_scale: 1.0,
        }
    }
}

#[derive(Clone)]
pub struct Presence {
    present: Signal<bool>,
    child: Option<AnyView>,
    enter_from: f32,
    enter_to: f32,
    enter_has_opacity: bool,
    enter_secs: f32,
    enter_delay: f32,
    enter_easing: Easing,
    exit_to: f32,
    exit_has_opacity: bool,
    exit_secs: f32,
    exit_delay: f32,
    exit_easing: Easing,
    enter_offset: Vec2,
    enter_offset_to: Vec2,
    exit_offset_to: Vec2,
    enter_scale: f32,
    exit_scale: f32,
    on_exit_complete: Option<Rc<dyn Fn()>>,
    state: Rc<RefCell<PresenceState>>,
}

impl Presence {
    pub fn new(present: Signal<bool>) -> Self {
        Self {
            present,
            child: None,
            enter_from: 0.0,
            enter_to: 1.0,
            enter_has_opacity: true,
            enter_secs: ENTER_SECS,
            enter_delay: 0.0,
            enter_easing: Easing::EaseOutCubic,
            exit_to: 0.0,
            exit_has_opacity: true,
            exit_secs: EXIT_SECS,
            exit_delay: 0.0,
            exit_easing: Easing::EaseOutCubic,
            enter_offset: Vec2::ZERO,
            enter_offset_to: Vec2::ZERO,
            exit_offset_to: Vec2::ZERO,
            enter_scale: 1.0,
            exit_scale: 1.0,
            on_exit_complete: None,
            state: Rc::new(RefCell::new(PresenceState::new())),
        }
    }

    pub fn child<V: View + 'static>(mut self, child: V) -> Self {
        self.child = Some(AnyView::new(child));
        self
    }

    pub fn from(mut self, opacity: f32) -> Self {
        self.enter_from = opacity.clamp(0.0, 1.0);
        self
    }

    pub fn enter_duration(mut self, secs: f32) -> Self {
        self.enter_secs = secs.max(0.0);
        self
    }

    pub fn enter_duration_d(mut self, d: Duration) -> Self {
        self.enter_secs = d.as_secs_f32().max(0.0);
        self
    }

    pub fn enter_delay(mut self, secs: f32) -> Self {
        self.enter_delay = secs.max(0.0);
        self
    }

    pub fn enter_easing(mut self, easing: Easing) -> Self {
        self.enter_easing = easing;
        self
    }

    pub fn exit_duration(mut self, secs: f32) -> Self {
        self.exit_secs = secs.max(0.0);
        self
    }

    pub fn exit_duration_d(mut self, d: Duration) -> Self {
        self.exit_secs = d.as_secs_f32().max(0.0);
        self
    }

    pub fn exit_delay(mut self, secs: f32) -> Self {
        self.exit_delay = secs.max(0.0);
        self
    }

    pub fn exit_easing(mut self, easing: Easing) -> Self {
        self.exit_easing = easing;
        self
    }

    pub fn durations(mut self, enter_secs: f32, exit_secs: f32) -> Self {
        self.enter_secs = enter_secs.max(0.0);
        self.exit_secs = exit_secs.max(0.0);
        self
    }

    pub fn durations_d(mut self, enter: Duration, exit: Duration) -> Self {
        self.enter_secs = enter.as_secs_f32().max(0.0);
        self.exit_secs = exit.as_secs_f32().max(0.0);
        self
    }

    pub fn easing(mut self, easing: Easing) -> Self {
        self.enter_easing = easing;
        self.exit_easing = easing;
        self
    }

    pub fn slide(mut self, dx: f32, dy: f32) -> Self {
        let v = Vec2::new(dx, dy);
        self.enter_offset = v;
        self.enter_offset_to = Vec2::ZERO;
        self.exit_offset_to = v;
        self
    }

    pub fn scale(mut self, scale: f32) -> Self {
        let s = if scale.is_finite() {
            scale.max(0.0)
        } else {
            1.0
        };
        self.enter_scale = s;
        self.exit_scale = s;
        self
    }

    pub fn enter(mut self, anim: impl Into<Animate>) -> Self {
        let a = anim.into();
        if let Some((from, to)) = a.opacity {
            self.enter_from = from;
            self.enter_to = to;
            self.enter_has_opacity = true;
        } else {
            self.enter_has_opacity = false;
        }
        self.enter_secs = a.duration_secs;
        self.enter_delay = a.delay_secs;
        self.enter_easing = a.easing;
        if let Some((from, to)) = a.offset {
            self.enter_offset = from;
            self.enter_offset_to = to;
        }
        self
    }

    pub fn exit(mut self, anim: impl Into<Animate>) -> Self {
        let a = anim.into();
        if let Some((_, to)) = a.opacity {
            self.exit_to = to;
            self.exit_has_opacity = true;
        } else {
            self.exit_has_opacity = false;
        }
        self.exit_secs = a.duration_secs;
        self.exit_delay = a.delay_secs;
        self.exit_easing = a.easing;
        if let Some((_, to)) = a.offset {
            self.exit_offset_to = to;
        }
        self
    }

    pub fn on_exit_complete(mut self, f: impl Fn() + 'static) -> Self {
        self.on_exit_complete = Some(Rc::new(f));
        self
    }

    pub fn sample_enter_at(&self, elapsed_secs: f32) -> f32 {
        sample_to(
            self.enter_from,
            self.enter_to,
            self.enter_delay,
            self.enter_secs,
            self.enter_easing,
            elapsed_secs,
        )
    }

    pub fn sample_exit_at(&self, elapsed_secs: f32) -> f32 {
        sample_to(
            1.0,
            self.exit_to,
            self.exit_delay,
            self.exit_secs,
            self.exit_easing,
            elapsed_secs,
        )
    }

    pub fn is_mounted(&self) -> bool {
        if self.child.is_none() {
            return false;
        }
        if self.present.get_untracked() {
            return true;
        }
        let st = *self.state.borrow();
        st.was_present && !st.exit_fired
    }

    fn begin_frame(&self, now: f32) {
        let present = self.present.get_untracked();
        let mut st = self.state.borrow_mut();
        if present {
            let reversing = st.exit_start.is_some();
            if reversing {
                st.enter_from = st.current_opacity;
                st.enter_offset = st.current_offset;
                st.enter_scale = st.current_scale;
            } else if !st.was_present || st.enter_start.is_none() {
                st.enter_from = self.enter_from;
                st.enter_offset = self.enter_offset;
                st.enter_scale = self.enter_scale;
            }
            st.was_present = true;
            st.exit_start = None;
            st.exit_fired = false;
            if reversing || st.enter_start.is_none() {
                st.enter_start = Some(now);
            }
        } else if st.was_present && st.exit_start.is_none() {
            st.exit_from = st.current_opacity;
            st.exit_offset = st.current_offset;
            st.exit_scale = st.current_scale;
            st.exit_start = Some(now);
            st.enter_start = None;
            st.exit_fired = false;
        }
    }

    fn opacity_at(&self, now: f32, reduced: bool) -> Option<f32> {
        self.begin_frame(now);
        let present = self.present.get_untracked();
        let st = *self.state.borrow();
        if present {
            if !self.enter_has_opacity {
                return None;
            }
            let value = if reduced {
                self.enter_to
            } else {
                let elapsed = st.enter_start.map(|s| now - s).unwrap_or(0.0);
                sample_to(
                    st.enter_from,
                    self.enter_to,
                    self.enter_delay,
                    self.enter_secs,
                    self.enter_easing,
                    elapsed,
                )
            };
            self.state.borrow_mut().current_opacity = value;
            Some(value)
        } else {
            if !st.was_present {
                return None;
            }
            if !self.exit_has_opacity {
                return None;
            }
            let value = if reduced {
                self.exit_to
            } else {
                let elapsed = st.exit_start.map(|s| now - s).unwrap_or(0.0);
                sample_to(
                    st.exit_from,
                    self.exit_to,
                    self.exit_delay,
                    self.exit_secs,
                    self.exit_easing,
                    elapsed,
                )
            };
            self.state.borrow_mut().current_opacity = value;
            Some(value)
        }
    }

    fn offset_at(&self, now: f32, reduced: bool) -> Vec2 {
        self.begin_frame(now);
        let present = self.present.get_untracked();
        let st = *self.state.borrow();
        if present {
            let value = if reduced {
                self.enter_offset_to
            } else {
                let elapsed = st.enter_start.map(|s| now - s).unwrap_or(0.0);
                let p = enter_progress(
                    self.enter_delay,
                    self.enter_secs,
                    self.enter_easing,
                    elapsed,
                );
                lerp_vec(st.enter_offset, self.enter_offset_to, p)
            };
            self.state.borrow_mut().current_offset = value;
            value
        } else {
            if !st.was_present {
                return self.exit_offset_to;
            }
            let value = if reduced {
                self.exit_offset_to
            } else {
                let elapsed = st.exit_start.map(|s| now - s).unwrap_or(0.0);
                let p = exit_progress(self.exit_delay, self.exit_secs, self.exit_easing, elapsed);
                lerp_vec(st.exit_offset, self.exit_offset_to, p)
            };
            self.state.borrow_mut().current_offset = value;
            value
        }
    }

    fn scale_at(&self, now: f32, reduced: bool) -> f32 {
        self.begin_frame(now);
        let present = self.present.get_untracked();
        let st = *self.state.borrow();
        if present {
            let value = if reduced {
                1.0
            } else {
                let elapsed = st.enter_start.map(|s| now - s).unwrap_or(0.0);
                let p = enter_progress(
                    self.enter_delay,
                    self.enter_secs,
                    self.enter_easing,
                    elapsed,
                );
                st.enter_scale + (1.0 - st.enter_scale) * p
            };
            self.state.borrow_mut().current_scale = value;
            value
        } else {
            if !st.was_present {
                return self.exit_scale;
            }
            let value = if reduced {
                self.exit_scale
            } else {
                let elapsed = st.exit_start.map(|s| now - s).unwrap_or(0.0);
                let p = exit_progress(self.exit_delay, self.exit_secs, self.exit_easing, elapsed);
                st.exit_scale + (self.exit_scale - st.exit_scale) * p
            };
            self.state.borrow_mut().current_scale = value;
            value
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
                st.enter_start = None;
                st.exit_start = None;
                st.exit_fired = false;
            }
            Some(vec![(0, child)])
        } else if st.was_present && !st.exit_fired {
            Some(vec![(0, child)])
        } else {
            st.was_present = false;
            st.enter_start = None;
            st.exit_start = None;
            Some(Vec::new())
        }
    }

    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fit,
            height: Length::Fit,
            ..LayoutStyle::default()
        }
    }

    fn paint_opacity_ctx(&self, rt: &Runtime, _el: ElementId) -> Option<f32> {
        self.opacity_at(rt.elapsed_secs(), rt.reduced_motion())
    }

    fn paint_offset_ctx(&self, rt: &Runtime, _el: ElementId) -> Vec2 {
        self.offset_at(rt.elapsed_secs(), rt.reduced_motion())
    }

    fn paint_scale_ctx(&self, rt: &Runtime, _el: ElementId) -> f32 {
        self.scale_at(rt.elapsed_secs(), rt.reduced_motion())
    }

    fn paint_self(&self, ctx: &mut PaintCtx) {
        let now = ctx.runtime().elapsed_secs();
        let reduced = ctx.runtime().reduced_motion();
        self.begin_frame(now);
        let present = self.present.get_untracked();
        if present {
            let st = *self.state.borrow();
            let elapsed = st.enter_start.map(|s| now - s).unwrap_or(0.0);
            if !reduced && elapsed < self.enter_delay + self.enter_secs {
                ctx.request_frame();
            }
            return;
        }

        let st = *self.state.borrow();
        if !st.was_present {
            return;
        }
        let elapsed = st.exit_start.map(|s| now - s).unwrap_or(0.0);
        if reduced || exit_done(self.exit_delay, self.exit_secs, elapsed) {
            let just_fired = {
                let mut st = self.state.borrow_mut();
                if st.exit_fired {
                    false
                } else {
                    st.exit_fired = true;
                    st.was_present = false;
                    st.enter_start = None;
                    st.exit_start = None;
                    st.current_opacity = self.exit_to;
                    st.current_offset = self.exit_offset_to;
                    st.current_scale = self.exit_scale;
                    true
                }
            };
            if just_fired {
                ctx.runtime().request_rebuild(ctx.element);
                if let Some(f) = &self.on_exit_complete {
                    f();
                }
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
        old.begin_frame(0.0);
        old.state.borrow_mut().enter_start = Some(0.0);
        let fresh = Presence::new(Signal::new(true)).child(Unit);
        fresh.reuse_state(&old);
        let op = fresh.opacity_at(10.0, false).unwrap_or(0.0);
        assert!(op > 0.99, "rebuilding does not restart input: {op}");
    }

    #[test]
    fn reversing_mid_enter_starts_from_current_opacity() {
        let p = Presence::new(Signal::new(true))
            .child(Unit)
            .durations(1.0, 1.0)
            .easing(Easing::Linear);
        p.begin_frame(0.0);
        p.state.borrow_mut().enter_start = Some(0.0);
        let mid = p.opacity_at(0.5, false).unwrap();
        assert!((mid - 0.5).abs() < 0.1, "mid enter: {mid}");

        p.present.set(false);
        p.begin_frame(0.5);
        let first = p.opacity_at(0.5, false).unwrap();
        assert!(
            (first - mid).abs() < 1e-4,
            "exit starts from the current value, not 1.0: {first} vs {mid}"
        );
    }
}
