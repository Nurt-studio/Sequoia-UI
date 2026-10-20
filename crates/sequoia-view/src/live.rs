#![forbid(unsafe_code)]

use std::rc::Rc;
use std::time::{Duration, Instant};

use sequoia_animation::Easing;
use sequoia_core::{Color, Vec2};
use sequoia_reactive::{Memo, Signal};

use crate::animated::MAX_ANIM_DT;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Prop(pub u64);

impl Prop {
    pub const OPACITY: Self = Self(1);
    pub const OFFSET: Self = Self(2);
    pub const OFFSET_X: Self = Self(3);
    pub const OFFSET_Y: Self = Self(4);
    pub const BLUR: Self = Self(5);
    pub const BACKGROUND: Self = Self(6);
    pub const SHADOW: Self = Self(7);
    pub const COLOR: Self = Self(8);
    pub const RADIUS: Self = Self(9);

    pub const SCALE: Self = Self(10);

    pub const ROTATION: Self = Self(11);

    pub const WIDTH: Self = Self(12);

    pub const HEIGHT: Self = Self(13);

    pub const fn custom(id: u64) -> Self {
        Self(id)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnimSpec {
    pub duration_secs: f32,
    pub delay_secs: f32,
    pub easing: Easing,
}

impl AnimSpec {
    pub const FAST: Self = Self {
        duration_secs: 0.12,
        delay_secs: 0.0,
        easing: Easing::Linear,
    };

    pub const SLOW: Self = Self {
        duration_secs: 0.25,
        delay_secs: 0.0,
        easing: Easing::EaseOutCubic,
    };

    pub const SMOOTH: Self = Self {
        duration_secs: 0.20,
        delay_secs: 0.0,
        easing: Easing::EaseOutCubic,
    };

    pub const SNAPPY: Self = Self {
        duration_secs: 0.12,
        delay_secs: 0.0,
        easing: Easing::EaseOutCubic,
    };

    pub const GENTLE: Self = Self {
        duration_secs: 0.35,
        delay_secs: 0.0,
        easing: Easing::EaseInOutCubic,
    };

    pub fn fast() -> Self {
        Self::FAST
    }
    pub fn smooth() -> Self {
        Self::SMOOTH
    }
    pub fn snappy() -> Self {
        Self::SNAPPY
    }
    pub fn gentle() -> Self {
        Self::GENTLE
    }
    pub fn slow() -> Self {
        Self::SLOW
    }

    pub fn linear(duration: Duration) -> Self {
        Self {
            duration_secs: duration.as_secs_f32().max(0.0),
            delay_secs: 0.0,
            easing: Easing::Linear,
        }
    }

    pub fn ease_out_cubic(duration: Duration) -> Self {
        Self {
            duration_secs: duration.as_secs_f32().max(0.0),
            delay_secs: 0.0,
            easing: Easing::EaseOutCubic,
        }
    }

    pub fn ease_out(duration: Duration) -> Self {
        Self::ease_out_cubic(duration)
    }

    pub fn ease_in_out(duration: Duration) -> Self {
        Self {
            duration_secs: duration.as_secs_f32().max(0.0),
            delay_secs: 0.0,
            easing: Easing::EaseInOutCubic,
        }
    }

    pub fn from_duration(duration: Duration, easing: Easing) -> Self {
        Self {
            duration_secs: duration.as_secs_f32().max(0.0),
            delay_secs: 0.0,
            easing,
        }
    }

    pub fn delay_duration(mut self, d: Duration) -> Self {
        self.delay_secs = d.as_secs_f32().max(0.0);
        self
    }

    pub fn sample(&self, from: f32, to: f32, elapsed_secs: f32) -> f32 {
        let t = elapsed_secs.max(0.0);
        if t < self.delay_secs {
            return from;
        }
        if self.duration_secs <= 0.0 {
            return to;
        }
        let k = ((t - self.delay_secs) / self.duration_secs).clamp(0.0, 1.0);
        from + (to - from) * self.easing.apply(k)
    }

    pub fn is_running(&self, elapsed_secs: f32) -> bool {
        elapsed_secs < self.delay_secs + self.duration_secs
    }
}

impl From<Duration> for AnimSpec {
    fn from(d: Duration) -> Self {
        Self {
            duration_secs: d.as_secs_f32().max(0.0),
            delay_secs: 0.0,
            easing: Easing::EaseOutCubic,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpringSpec {
    pub stiffness: f32,
    pub damping: f32,
    pub mass: f32,
}

impl SpringSpec {
    pub const DEFAULT: Self = Self {
        stiffness: 180.0,
        damping: 20.0,
        mass: 1.0,
    };

    pub const SNAPPY: Self = Self {
        stiffness: 320.0,
        damping: 22.0,
        mass: 1.0,
    };

    pub const BOUNCY: Self = Self {
        stiffness: 170.0,
        damping: 11.0,
        mass: 1.0,
    };

    pub const GENTLE: Self = Self {
        stiffness: 90.0,
        damping: 19.0,
        mass: 1.0,
    };

    pub fn snappy() -> Self {
        Self::SNAPPY
    }
    pub fn bouncy() -> Self {
        Self::BOUNCY
    }
    pub fn gentle() -> Self {
        Self::GENTLE
    }

    pub fn stiffness(mut self, k: f32) -> Self {
        self.stiffness = k.max(0.0);
        self
    }

    pub fn damping(mut self, c: f32) -> Self {
        self.damping = c.max(0.0);
        self
    }

    pub fn mass(mut self, m: f32) -> Self {
        self.mass = m.max(0.0001);
        self
    }
}

impl Default for SpringSpec {
    fn default() -> Self {
        Self::DEFAULT
    }
}

enum Source<T> {
    Fixed(T),
    Signal(Signal<T>),
    Memo(Memo<T>),
    Derived(Rc<dyn Fn() -> T>),
}

impl<T: Clone> Clone for Source<T> {
    fn clone(&self) -> Self {
        match self {
            Source::Fixed(v) => Source::Fixed(v.clone()),
            Source::Signal(s) => Source::Signal(s.clone()),
            Source::Memo(m) => Source::Memo(m.clone()),
            Source::Derived(f) => Source::Derived(Rc::clone(f)),
        }
    }
}

pub struct Live<T> {
    source: Source<T>,
    tween: Option<AnimSpec>,
    spring: Option<SpringSpec>,
}

impl<T: Clone> Clone for Live<T> {
    fn clone(&self) -> Self {
        Self {
            source: self.source.clone(),
            tween: self.tween,
            spring: self.spring,
        }
    }
}

impl<T: Clone + PartialEq + 'static> Live<T> {
    pub fn fixed(value: T) -> Self {
        Self {
            source: Source::Fixed(value),
            tween: None,
            spring: None,
        }
    }

    pub fn of_signal(signal: Signal<T>) -> Self {
        Self {
            source: Source::Signal(signal),
            tween: None,
            spring: None,
        }
    }

    pub fn of_memo(memo: Memo<T>) -> Self {
        Self {
            source: Source::Memo(memo),
            tween: None,
            spring: None,
        }
    }

    pub fn derived(f: impl Fn() -> T + 'static) -> Self {
        Self {
            source: Source::Derived(Rc::new(f)),
            tween: None,
            spring: None,
        }
    }

    pub fn animate(mut self, spec: impl Into<AnimSpec>) -> Self {
        self.tween = Some(spec.into());
        self.spring = None;
        self
    }

    pub fn spring(mut self, spec: SpringSpec) -> Self {
        self.spring = Some(spec);
        self.tween = None;
        self
    }

    pub fn animate_mut(&mut self, spec: impl Into<AnimSpec>) {
        self.tween = Some(spec.into());
        self.spring = None;
    }

    pub fn spring_mut(&mut self, spec: SpringSpec) {
        self.spring = Some(spec);
        self.tween = None;
    }

    pub fn tween_spec(&self) -> Option<AnimSpec> {
        self.tween
    }

    pub fn spring_spec(&self) -> Option<SpringSpec> {
        self.spring
    }

    pub fn is_animated(&self) -> bool {
        self.tween.is_some() || self.spring.is_some()
    }

    pub fn target(&self) -> T {
        match &self.source {
            Source::Fixed(v) => v.clone(),
            Source::Signal(s) => s.get(),
            Source::Memo(m) => m.get(),
            Source::Derived(f) => f(),
        }
    }

    pub fn target_untracked(&self) -> T
    where
        T: Clone,
    {
        match &self.source {
            Source::Fixed(v) => v.clone(),
            Source::Signal(s) => s.get_untracked(),
            Source::Memo(m) => m.get_untracked(),
            Source::Derived(f) => sequoia_reactive::untracked(|| f()),
        }
    }
}

impl<T: Clone + PartialEq + 'static> From<T> for Live<T> {
    fn from(value: T) -> Self {
        Self::fixed(value)
    }
}

impl<T: Clone + PartialEq + 'static> From<Signal<T>> for Live<T> {
    fn from(signal: Signal<T>) -> Self {
        Self::of_signal(signal)
    }
}

impl<T: Clone + PartialEq + 'static> From<Memo<T>> for Live<T> {
    fn from(memo: Memo<T>) -> Self {
        Self::of_memo(memo)
    }
}

impl From<f64> for Live<f32> {
    fn from(v: f64) -> Self {
        Self::fixed(v as f32)
    }
}

pub trait SignalLiveExt<T: Clone + PartialEq + 'static> {
    fn live(self) -> Live<T>;
    fn map_live<U: Clone + PartialEq + 'static>(&self, f: impl Fn(&T) -> U + 'static) -> Live<U>;
    fn animate(self, spec: impl Into<AnimSpec>) -> Live<T>;
    fn spring(self, spec: SpringSpec) -> Live<T>;
}

impl<T: Clone + PartialEq + 'static> SignalLiveExt<T> for Signal<T> {
    fn live(self) -> Live<T> {
        Live::of_signal(self)
    }

    fn map_live<U: Clone + PartialEq + 'static>(&self, f: impl Fn(&T) -> U + 'static) -> Live<U> {
        let s = self.clone();
        Live::derived(move || f(&s.get()))
    }

    fn animate(self, spec: impl Into<AnimSpec>) -> Live<T> {
        Live::of_signal(self).animate(spec)
    }

    fn spring(self, spec: SpringSpec) -> Live<T> {
        Live::of_signal(self).spring(spec)
    }
}

pub trait MemoLiveExt<T: Clone + PartialEq + 'static> {
    fn live(self) -> Live<T>;
    fn map_live<U: Clone + PartialEq + 'static>(&self, f: impl Fn(&T) -> U + 'static) -> Live<U>;
    fn animate(self, spec: impl Into<AnimSpec>) -> Live<T>;
    fn spring(self, spec: SpringSpec) -> Live<T>;
}

impl<T: Clone + PartialEq + 'static> MemoLiveExt<T> for Memo<T> {
    fn live(self) -> Live<T> {
        Live::of_memo(self)
    }

    fn map_live<U: Clone + PartialEq + 'static>(&self, f: impl Fn(&T) -> U + 'static) -> Live<U> {
        let m = self.clone();
        Live::derived(move || f(&m.get()))
    }

    fn animate(self, spec: impl Into<AnimSpec>) -> Live<T> {
        Live::of_memo(self).animate(spec)
    }

    fn spring(self, spec: SpringSpec) -> Live<T> {
        Live::of_memo(self).spring(spec)
    }
}

fn lerp_f32(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    Color::rgba(
        lerp_f32(a.r, b.r, t),
        lerp_f32(a.g, b.g, t),
        lerp_f32(a.b, b.b, t),
        lerp_f32(a.a, b.a, t),
    )
}

fn lerp_vec2(a: Vec2, b: Vec2, t: f32) -> Vec2 {
    Vec2::new(lerp_f32(a.x, b.x, t), lerp_f32(a.y, b.y, t))
}

#[derive(Clone, Debug)]
pub(crate) enum AnimState {
    F32(F32State),
    Color(ColorState),
    Vec2(Vec2State),
}

#[derive(Clone, Debug)]
pub(crate) struct F32State {
    pub current: f32,
    pub from: f32,
    pub start: Instant,
    pub last_target: f32,
    pub tween: Option<AnimSpec>,
    pub spring: Option<SpringSpec>,
    pub velocity: f32,
    pub last_tick: Instant,
    pub initialized: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct ColorState {
    pub current: Color,
    pub from: Color,
    pub start: Instant,
    pub last_target: Color,
    pub tween: Option<AnimSpec>,

    pub smooth_rate: Option<f32>,
    pub last_tick: Instant,
    pub initialized: bool,
}

pub fn exp_blend(current: Color, target: Color, dt_secs: f32, rate: f32) -> Color {
    let t = 1.0 - (-rate.max(0.0) * dt_secs.max(0.0)).exp();
    Color::rgba(
        current.r + (target.r - current.r) * t,
        current.g + (target.g - current.g) * t,
        current.b + (target.b - current.b) * t,
        current.a + (target.a - current.a) * t,
    )
}

pub const EXP_SETTLE_EPS: f32 = 0.004;

pub fn exp_settled(current: Color, target: Color) -> bool {
    (current.r - target.r).abs() < EXP_SETTLE_EPS
        && (current.g - target.g).abs() < EXP_SETTLE_EPS
        && (current.b - target.b).abs() < EXP_SETTLE_EPS
        && (current.a - target.a).abs() < EXP_SETTLE_EPS
}

#[derive(Clone, Debug)]
pub(crate) struct Vec2State {
    pub current: Vec2,
    pub from: Vec2,
    pub start: Instant,
    pub last_target: Vec2,
    pub tween: Option<AnimSpec>,
    pub spring: Option<SpringSpec>,
    pub velocity: Vec2,
    pub last_tick: Instant,
    pub initialized: bool,
}

fn spring_step(current: f32, velocity: f32, target: f32, spec: &SpringSpec, dt: f32) -> (f32, f32) {
    let m = spec.mass.max(0.0001);
    let k = spec.stiffness.max(0.0);
    let c = spec.damping.max(0.0);

    let force = -k * (current - target) - c * velocity;
    let vel = velocity + force / m * dt;
    let pos = current + vel * dt;
    (pos, vel)
}

impl AnimState {
    pub(crate) fn sample_f32(
        state: &mut F32State,
        target: f32,
        tween: Option<&AnimSpec>,
        spring: Option<&SpringSpec>,
        now: Instant,
    ) -> (f32, bool) {
        if !state.initialized {
            state.current = target;
            state.from = target;
            state.last_target = target;
            state.tween = tween.copied();
            state.spring = spring.copied();
            state.start = now;
            state.last_tick = now;
            state.velocity = 0.0;
            state.initialized = true;
            return (target, false);
        }
        let spec_changed = state.tween != tween.copied() || state.spring != spring.copied();
        if target != state.last_target || spec_changed {
            state.from = state.current;
            state.last_target = target;
            state.tween = tween.copied();
            state.spring = spring.copied();
            state.start = now;
            state.last_tick = now;
            if spring.is_some() {
            } else {
                state.velocity = 0.0;
            }
        }
        if let Some(sp) = spring {
            let dt = now
                .duration_since(state.last_tick)
                .as_secs_f32()
                .clamp(0.0, MAX_ANIM_DT);
            state.last_tick = now;

            let steps = 4;
            let h = dt / steps as f32;
            for _ in 0..steps {
                let (p, v) = spring_step(state.current, state.velocity, target, sp, h.max(0.0));
                state.current = p;
                state.velocity = v;
            }
            let settled = (state.current - target).abs() < 0.0005 && state.velocity.abs() < 0.0005;
            if settled {
                state.current = target;
                state.velocity = 0.0;
                return (target, false);
            }
            return (state.current, true);
        }
        if let Some(spec) = tween {
            let elapsed = now.duration_since(state.start).as_secs_f32();
            let v = spec.sample(state.from, target, elapsed);
            state.current = v;
            return (v, spec.is_running(elapsed));
        }
        state.current = target;
        (target, false)
    }

    pub(crate) fn sample_color(
        state: &mut ColorState,
        target: Color,
        tween: Option<&AnimSpec>,
        smooth_rate: Option<f32>,
        now: Instant,
    ) -> (Color, bool) {
        if !state.initialized {
            state.current = target;
            state.from = target;
            state.last_target = target;
            state.tween = tween.copied();
            state.smooth_rate = smooth_rate;
            state.start = now;
            state.last_tick = now;
            state.initialized = true;
            return (target, false);
        }
        if target != state.last_target
            || state.tween != tween.copied()
            || state.smooth_rate != smooth_rate
        {
            state.from = state.current;
            state.last_target = target;
            state.tween = tween.copied();
            state.smooth_rate = smooth_rate;
            state.start = now;
            state.last_tick = now;
        }
        if let Some(rate) = smooth_rate {
            let dt = now
                .duration_since(state.last_tick)
                .as_secs_f32()
                .clamp(0.0, MAX_ANIM_DT);
            state.last_tick = now;
            if exp_settled(state.current, target) {
                state.current = target;
                return (target, false);
            }
            let next = exp_blend(state.current, target, dt, rate);
            state.current = next;
            if exp_settled(next, target) {
                state.current = target;
                return (target, false);
            }
            return (next, true);
        }
        if let Some(spec) = tween {
            let elapsed = now.duration_since(state.start).as_secs_f32();
            let k = if spec.duration_secs <= 0.0 || elapsed < spec.delay_secs {
                if elapsed < spec.delay_secs { 0.0 } else { 1.0 }
            } else {
                spec.easing
                    .apply(((elapsed - spec.delay_secs) / spec.duration_secs).clamp(0.0, 1.0))
            };
            let v = lerp_color(state.from, target, k);
            state.current = v;
            return (v, spec.is_running(elapsed));
        }
        state.current = target;
        (target, false)
    }

    pub(crate) fn sample_vec2(
        state: &mut Vec2State,
        target: Vec2,
        tween: Option<&AnimSpec>,
        spring: Option<&SpringSpec>,
        now: Instant,
    ) -> (Vec2, bool) {
        if !state.initialized {
            state.current = target;
            state.from = target;
            state.last_target = target;
            state.tween = tween.copied();
            state.spring = spring.copied();
            state.start = now;
            state.last_tick = now;
            state.velocity = Vec2::ZERO;
            state.initialized = true;
            return (target, false);
        }
        let spec_changed = state.tween != tween.copied() || state.spring != spring.copied();
        if target != state.last_target || spec_changed {
            state.from = state.current;
            state.last_target = target;
            state.tween = tween.copied();
            state.spring = spring.copied();
            state.start = now;
            state.last_tick = now;
        }
        if let Some(sp) = spring {
            let dt = now
                .duration_since(state.last_tick)
                .as_secs_f32()
                .clamp(0.0, MAX_ANIM_DT);
            state.last_tick = now;
            let steps = 4;
            let h = dt / steps as f32;
            for _ in 0..steps {
                let (px, vx) =
                    spring_step(state.current.x, state.velocity.x, target.x, sp, h.max(0.0));
                let (py, vy) =
                    spring_step(state.current.y, state.velocity.y, target.y, sp, h.max(0.0));
                state.current = Vec2::new(px, py);
                state.velocity = Vec2::new(vx, vy);
            }
            let settled = (state.current.x - target.x).abs() < 0.0005
                && (state.current.y - target.y).abs() < 0.0005
                && state.velocity.x.abs() < 0.0005
                && state.velocity.y.abs() < 0.0005;
            if settled {
                state.current = target;
                state.velocity = Vec2::ZERO;
                return (target, false);
            }
            return (state.current, true);
        }
        if let Some(spec) = tween {
            let elapsed = now.duration_since(state.start).as_secs_f32();
            let k = if elapsed < spec.delay_secs {
                0.0
            } else if spec.duration_secs <= 0.0 {
                1.0
            } else {
                spec.easing
                    .apply(((elapsed - spec.delay_secs) / spec.duration_secs).clamp(0.0, 1.0))
            };
            let v = lerp_vec2(state.from, target, k);
            state.current = v;
            return (v, spec.is_running(elapsed));
        }
        state.current = target;
        (target, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_fixed_and_signal_targets() {
        let a = Live::fixed(3.0f32);
        assert_eq!(a.target_untracked(), 3.0);
        assert!(!a.is_animated());
        let s = Signal::new(2.0f32);
        let b = s.clone().live().animate(AnimSpec::FAST);
        assert!(b.is_animated());
        assert_eq!(b.target_untracked(), 2.0);
        s.set(5.0);
        assert_eq!(b.target_untracked(), 5.0);
    }

    #[test]
    fn signal_map_live_derives() {
        let hovered = Signal::new(false);
        let bg = hovered.map_live(|v| if *v { 20.0f32 } else { 8.0f32 });
        assert_eq!(bg.target_untracked(), 8.0);
        hovered.set(true);
        assert_eq!(bg.target_untracked(), 20.0);
    }

    #[test]
    fn tween_state_retargets_from_current() {
        let now = Instant::now();
        let spec = AnimSpec::FAST;
        let mut st = F32State {
            current: 0.0,
            from: 0.0,
            start: now,
            last_target: 0.0,
            tween: None,
            spring: None,
            velocity: 0.0,
            last_tick: now,
            initialized: false,
        };

        let (v0, r0) = AnimState::sample_f32(&mut st, 0.0, Some(&spec), None, now);
        assert_eq!(v0, 0.0);
        assert!(!r0, "first sample anchors without gear");

        let (v1a, r1a) = AnimState::sample_f32(&mut st, 1.0, Some(&spec), None, now);
        assert_eq!(v1a, 0.0, "retarget nie skacze w tej samej klatce");
        assert!(r1a);
        let later = now + Duration::from_millis(60);
        let (v1, r1) = AnimState::sample_f32(&mut st, 1.0, Some(&spec), None, later);
        assert!((v1 - 0.5).abs() < 0.05, "half way: {v1}");
        assert!(r1);

        let (v2, _) = AnimState::sample_f32(&mut st, 0.0, Some(&spec), None, later);
        assert!((v2 - v1).abs() < 1e-5, "retarget nie skacze: {v2} vs {v1}");
    }

    #[test]
    fn exp_color_matches_button_blend_math() {
        use sequoia_core::Color;

        let step = exp_blend(Color::BLACK, Color::WHITE, 1.0 / 60.0, 12.0);
        assert!(step.r > 0.1 && step.r < 1.0, "pierwsza klatka: {}", step.r);
        assert!(!exp_settled(Color::BLACK, Color::WHITE));
        assert!(exp_settled(Color::WHITE, Color::WHITE));

        let now = Instant::now();
        let mut st = ColorState {
            current: Color::BLACK,
            from: Color::BLACK,
            start: now,
            last_target: Color::BLACK,
            tween: None,
            smooth_rate: None,
            last_tick: now,
            initialized: false,
        };
        let (v0, r0) = AnimState::sample_color(&mut st, Color::BLACK, None, Some(12.0), now);
        assert_eq!(v0, Color::BLACK);
        assert!(!r0);
        let mut t = now;
        let mut running = true;
        for _ in 0..240 {
            t += Duration::from_millis(16);
            let (_, r) = AnimState::sample_color(&mut st, Color::WHITE, None, Some(12.0), t);
            running = r;
            if !running {
                break;
            }
        }
        assert!(!running, "240 klatek musi dobiec");
        assert_eq!(st.current, Color::WHITE);
    }

    #[test]
    fn spring_state_settles() {
        let now = Instant::now();
        let spec = SpringSpec::DEFAULT;
        let mut st = F32State {
            current: 0.0,
            from: 0.0,
            start: now,
            last_target: 0.0,
            tween: None,
            spring: None,
            velocity: 0.0,
            last_tick: now,
            initialized: false,
        };
        let mut t = now;
        let _ = AnimState::sample_f32(&mut st, 100.0, None, Some(&spec), t);
        for _ in 0..600 {
            t += Duration::from_millis(16);
            let _ = AnimState::sample_f32(&mut st, 100.0, None, Some(&spec), t);
        }
        assert!(
            (st.current - 100.0).abs() < 1.0,
            "spring reaches: {}",
            st.current
        );
    }
}
