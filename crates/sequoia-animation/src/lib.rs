#![warn(clippy::all)]
#![forbid(unsafe_code)]

mod animation;
mod animator;
mod bezier;
mod builder;
mod choreography;
mod import;
mod machine;
mod mixer;
mod spring_machine;
mod spring_value;
mod value;

pub use animation::{Animation, AnimationFrame, Interpolation, Keyframe, LoopMode, Track};
pub use animator::Animator;
pub use bezier::CubicBezier;
pub use builder::{
    Animate, AnimateBuilder, AnimateFrame, SpringBuilder, TweenBuilder, animate, animation,
    lerp_sample, spring,
};
pub use choreography::Choreography;
pub use import::{ImportError, animation_from_json, bundle_from_json, import_lottie};
pub use machine::{
    CmpOp, Condition, FromState, MachineError, MachineInputs, MachineState, StateMachine, ToState,
    Transition,
};
pub use mixer::Mixer;
use sequoia_core::{Color, Point, Size};
pub use spring_machine::{AnimValue, SpringMachine, StateDef, TransitionDef, Trigger};
pub use spring_value::{SpringPoint, SpringValue};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::panic::Location;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;
pub use value::Value;

#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum Easing {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    EaseInQuad,
    EaseOutQuad,
    EaseInOutQuad,
    EaseInCubic,
    EaseOutCubic,
    EaseInOutCubic,
    EaseInQuart,
    EaseOutQuart,
    EaseInOutQuart,
    EaseInQuint,
    EaseOutQuint,
    EaseInOutQuint,
    EaseInExpo,
    EaseOutExpo,
    EaseInOutExpo,
    EaseInCirc,
    EaseOutCirc,
    EaseInOutCirc,
    EaseInSine,
    EaseOutSine,
    EaseInOutSine,
    EaseInBack,
    EaseOutBack,
    EaseInOutBack,
    BounceOut,
    BounceIn,
    ElasticOut,

    CubicBezier(f32, f32, f32, f32),
}

impl Easing {
    pub fn apply(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Easing::Linear => t,
            Easing::EaseIn => t * t * t,
            Easing::EaseOut => 1.0 - (1.0 - t).powi(3),
            Easing::EaseInOut => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
                }
            }
            Easing::EaseInQuad => t * t,
            Easing::EaseOutQuad => t * (2.0 - t),
            Easing::EaseInOutQuad => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    -1.0 + (4.0 - 2.0 * t) * t
                }
            }
            Easing::EaseInCubic => t * t * t,
            Easing::EaseOutCubic => {
                let t = t - 1.0;
                t * t * t + 1.0
            }
            Easing::EaseInOutCubic => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
                }
            }
            Easing::EaseInQuart => t * t * t * t,
            Easing::EaseOutQuart => {
                let t = t - 1.0;
                -(t * t * t * t) + 1.0
            }
            Easing::EaseInOutQuart => {
                if t < 0.5 {
                    8.0 * t * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(4) / 2.0
                }
            }
            Easing::EaseInQuint => t * t * t * t * t,
            Easing::EaseOutQuint => {
                let t = t - 1.0;
                t * t * t * t * t + 1.0
            }
            Easing::EaseInOutQuint => {
                if t < 0.5 {
                    16.0 * t * t * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(5) / 2.0
                }
            }
            Easing::EaseInExpo => {
                if t == 0.0 {
                    0.0
                } else {
                    (2.0_f32).powf(10.0 * (t - 1.0))
                }
            }
            Easing::EaseOutExpo => {
                if t == 1.0 {
                    1.0
                } else {
                    1.0 - (2.0_f32).powf(-10.0 * t)
                }
            }
            Easing::EaseInOutExpo => {
                if t == 0.0 || t == 1.0 {
                    t
                } else if t < 0.5 {
                    (2.0_f32).powf(10.0 * (2.0 * t - 1.0)) / 2.0
                } else {
                    (2.0 - (2.0_f32).powf(-10.0 * (2.0 * t - 1.0))) / 2.0
                }
            }
            Easing::EaseInCirc => 1.0 - (1.0 - t * t).sqrt(),
            Easing::EaseOutCirc => (1.0 - (t - 1.0) * (t - 1.0)).sqrt(),
            Easing::EaseInOutCirc => {
                if t < 0.5 {
                    (1.0 - (1.0 - 4.0 * t * t).sqrt()) / 2.0
                } else {
                    ((1.0 - (-2.0 * t + 2.0).powi(2)).sqrt() + 1.0) / 2.0
                }
            }
            Easing::EaseInSine => 1.0 - (t * std::f32::consts::FRAC_PI_2).cos(),
            Easing::EaseOutSine => (t * std::f32::consts::FRAC_PI_2).sin(),
            Easing::EaseInOutSine => -(std::f32::consts::PI * t).cos() / 2.0 + 0.5,
            Easing::EaseInBack => {
                const C1: f32 = 1.70158;
                const C3: f32 = C1 + 1.0;
                C3 * t * t * t - C1 * t * t
            }
            Easing::EaseOutBack => {
                const C1: f32 = 1.70158;
                const C3: f32 = C1 + 1.0;
                let t = t - 1.0;
                C3 * t * t * t + C1 * t * t + 1.0
            }
            Easing::EaseInOutBack => {
                const C1: f32 = 1.70158;
                const C2: f32 = C1 * 1.525;
                if t < 0.5 {
                    let t = 2.0 * t;
                    (t * t * (C2 + 1.0) * t - C2 * t * t) / 2.0
                } else {
                    let t = 2.0 * t - 2.0;
                    (t * t * (C2 + 1.0) * t + C2 * t * t + 2.0) / 2.0
                }
            }
            Easing::BounceOut => {
                if t < 1.0 / 2.75 {
                    7.5625 * t * t
                } else if t < 2.0 / 2.75 {
                    let t = t - 1.5 / 2.75;
                    7.5625 * t * t + 0.75
                } else if t < 2.5 / 2.75 {
                    let t = t - 2.25 / 2.75;
                    7.5625 * t * t + 0.9375
                } else {
                    let t = t - 2.625 / 2.75;
                    7.5625 * t * t + 0.984375
                }
            }
            Easing::BounceIn => 1.0 - Easing::BounceOut.apply(1.0 - t),
            Easing::CubicBezier(x1, y1, x2, y2) => CubicBezier::new(x1, y1, x2, y2).solve(t),
            Easing::ElasticOut => {
                const C4: f32 = (2.0 * std::f32::consts::PI) / 3.0;
                if t == 0.0 || t == 1.0 {
                    t
                } else {
                    (2.0_f32).powf(-10.0 * t) * ((t * 10.0 - 0.75) * C4).sin() + 1.0
                }
            }
        }
    }
}

pub trait Lerp: Copy {
    fn lerp(self, other: Self, t: f32) -> Self;
}

impl Lerp for f32 {
    fn lerp(self, other: Self, t: f32) -> Self {
        self + (other - self) * t
    }
}

impl Lerp for f64 {
    fn lerp(self, other: Self, t: f32) -> Self {
        self + (other - self) * t as f64
    }
}

impl Lerp for Color {
    fn lerp(self, other: Self, t: f32) -> Self {
        Color::rgba(
            self.r + (other.r - self.r) * t,
            self.g + (other.g - self.g) * t,
            self.b + (other.b - self.b) * t,
            self.a + (other.a - self.a) * t,
        )
    }
}

impl Lerp for Point {
    fn lerp(self, other: Self, t: f32) -> Self {
        Point::new(self.x.lerp(other.x, t), self.y.lerp(other.y, t))
    }
}

impl Lerp for Size {
    fn lerp(self, other: Self, t: f32) -> Self {
        Size::new(
            self.width.lerp(other.width, t),
            self.height.lerp(other.height, t),
        )
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct SessionKey(u64, u32, u32);

impl SessionKey {
    #[track_caller]
    pub fn caller() -> Self {
        let loc = Location::caller();
        let mut h = DefaultHasher::new();
        loc.file().hash(&mut h);
        SessionKey(h.finish(), loc.line(), loc.column())
    }
}

type SessionCellRegistry = HashMap<(SessionKey, TypeId), Box<dyn Any + Send>>;

static SESSION_CELLS: OnceLock<Mutex<SessionCellRegistry>> = OnceLock::new();

pub fn session_cell<T: Default + Send + 'static>(key: SessionKey) -> &'static Mutex<T> {
    let map = SESSION_CELLS.get_or_init(|| Mutex::new(HashMap::new()));
    {
        let map = map.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(cell) = map.get(&(key, TypeId::of::<T>())) {
            return cell
                .downcast_ref::<&'static Mutex<T>>()
                .copied()
                .expect("session cell stored under a different type");
        }
    }

    let cell: &'static Mutex<T> = Box::leak(Box::new(Mutex::new(T::default())));
    let mut map = map.lock().unwrap_or_else(|p| p.into_inner());
    map.entry((key, TypeId::of::<T>()))
        .or_insert_with(|| Box::new(cell))
        .downcast_ref::<&'static Mutex<T>>()
        .copied()
        .expect("session cell stored under a different type")
}

pub fn clear_sessions() {
    if let Some(map) = SESSION_CELLS.get() {
        map.lock().unwrap_or_else(|p| p.into_inner()).clear();
    }
}

pub const MAX_FRAME_DT: f32 = 1.0 / 20.0;

#[derive(Default)]
struct Session {
    started: Option<Instant>,

    last: Option<Instant>,

    elapsed: f64,

    running: bool,

    from: f32,
    to: f32,

    run_started: f64,
}

#[derive(Clone)]
pub struct AnimationController {
    started: bool,

    last: Option<Instant>,

    elapsed: f64,

    step_elapsed: f64,
    duration: f64,
    easing: Easing,
    pub value: f32,
    pub running: bool,
    from: f32,
    to: f32,

    session: Option<SessionKey>,
}

impl AnimationController {
    pub fn new(duration_secs: f64, easing: Easing) -> Self {
        Self {
            started: false,
            last: None,
            elapsed: 0.0,
            step_elapsed: 0.0,
            duration: duration_secs,
            easing,
            value: 0.0,
            running: false,
            from: 0.0,
            to: 1.0,
            session: None,
        }
    }

    pub fn with_session(mut self, key: SessionKey) -> Self {
        self.session = Some(key);
        self
    }

    fn anchor(&mut self) {
        let Some(key) = self.session else {
            self.last = Some(Instant::now());
            self.elapsed = 0.0;
            return;
        };
        let mut s = session_cell::<Session>(key)
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        if s.started.is_none() {
            s.started = Some(Instant::now());
            s.last = Some(Instant::now());
            s.elapsed = 0.0;
            s.from = self.from;
            s.to = self.to;
            s.run_started = 0.0;
            s.running = true;
        } else if self.from != s.from || self.to != s.to {
            s.from = self.from;
            s.to = self.to;
            s.run_started = s.elapsed;
            s.running = true;
        }
    }

    pub fn start(&mut self) {
        self.started = true;
        self.running = true;
        self.from = 0.0;
        self.to = 1.0;
        self.value = 0.0;
        self.step_elapsed = 0.0;
        self.anchor();
    }

    pub fn animate_to(&mut self, target: f32) {
        let target = target.clamp(0.0, 1.0);
        if (target - self.value).abs() < 0.001 {
            self.running = false;
            self.value = target;
            return;
        }
        self.started = true;
        self.from = self.value;
        self.to = target;
        self.running = true;
        self.step_elapsed = 0.0;
        self.anchor();
    }

    pub fn advance(&mut self) -> bool {
        if !self.running {
            return false;
        }
        let now = Instant::now();

        let (elapsed, run_started) = if let Some(key) = self.session {
            let mut s = session_cell::<Session>(key)
                .lock()
                .unwrap_or_else(|p| p.into_inner());
            let dt = now
                .duration_since(s.last.unwrap_or(now))
                .as_secs_f64()
                .min(f64::from(MAX_FRAME_DT));
            s.elapsed += dt;
            s.last = Some(now);
            (s.elapsed, s.run_started)
        } else {
            let dt = now
                .duration_since(self.last.unwrap_or(now))
                .as_secs_f64()
                .min(f64::from(MAX_FRAME_DT));
            self.elapsed += dt;
            self.last = Some(now);
            (self.elapsed, 0.0)
        };
        let t = ((elapsed - run_started) / self.duration) as f32;
        if t >= 1.0 {
            self.value = self.to;
            self.running = false;
            if let Some(key) = self.session {
                session_cell::<Session>(key)
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .running = false;
            }
            true
        } else {
            let eased = self.easing.apply(t);
            self.value = self.from + (self.to - self.from) * eased;
            false
        }
    }

    pub fn reset(&mut self) {
        self.started = false;
        self.last = None;
        self.elapsed = 0.0;
        self.step_elapsed = 0.0;
        self.value = 0.0;
        self.running = false;
        self.from = 0.0;
        self.to = 1.0;
        if let Some(key) = self.session {
            let mut s = session_cell::<Session>(key)
                .lock()
                .unwrap_or_else(|p| p.into_inner());
            s.started = None;
            s.last = None;
            s.elapsed = 0.0;
            s.running = false;
        }
    }

    pub fn advance_by(&mut self, dt: f64) -> bool {
        if !self.running {
            return false;
        }
        self.step_elapsed += dt.max(0.0);
        let t = (self.step_elapsed / self.duration) as f32;
        if t >= 1.0 {
            self.value = self.to;
            self.running = false;
            true
        } else {
            self.value = self.from + (self.to - self.from) * self.easing.apply(t);
            false
        }
    }

    pub fn progress(&self) -> f32 {
        ((self.value - self.from) / (self.to - self.from)).clamp(0.0, 1.0)
    }
}

#[derive(Clone)]
pub struct Tween<T: Lerp> {
    start: Option<Instant>,

    elapsed: f64,
    duration: f64,
    delay: f64,
    easing: Easing,
    from: T,
    to: T,
    pub value: T,
    pub running: bool,
    pub finished: bool,
}

impl<T: Lerp> Tween<T> {
    pub fn new(duration_secs: f64, from: T, to: T, easing: Easing) -> Self {
        Self {
            start: None,
            elapsed: 0.0,
            duration: duration_secs,
            delay: 0.0,
            easing,
            from,
            to,
            value: from,
            running: false,
            finished: false,
        }
    }

    pub fn with_delay(mut self, secs: f64) -> Self {
        self.delay = secs;
        self
    }

    pub fn start(&mut self) {
        self.start = Some(Instant::now());
        self.elapsed = 0.0;
        self.running = true;
        self.finished = false;
        self.value = self.from;
    }

    pub fn advance(&mut self) -> bool {
        if !self.running || self.finished {
            return false;
        }
        let elapsed = self.start.map(|s| s.elapsed().as_secs_f64()).unwrap_or(0.0);
        if elapsed < self.delay {
            return false;
        }
        let t = ((elapsed - self.delay) / self.duration) as f32;
        if t >= 1.0 {
            self.value = self.to;
            self.finished = true;
            self.running = false;
            true
        } else {
            self.value = self.from.lerp(self.to, self.easing.apply(t));
            false
        }
    }

    pub fn advance_by(&mut self, dt: f64) -> bool {
        if !self.running || self.finished {
            return false;
        }
        self.elapsed += dt.max(0.0);
        if self.elapsed < self.delay {
            return false;
        }
        let t = ((self.elapsed - self.delay) / self.duration) as f32;
        if t >= 1.0 {
            self.value = self.to;
            self.finished = true;
            self.running = false;
            true
        } else {
            self.value = self.from.lerp(self.to, self.easing.apply(t));
            false
        }
    }

    pub fn reset(&mut self) {
        self.start = None;
        self.elapsed = 0.0;
        self.value = self.from;
        self.running = false;
        self.finished = false;
    }
}

pub trait AnimationClip: Send {
    fn advance(&mut self) -> bool;
    fn running(&self) -> bool;
    fn finished(&self) -> bool;
}

macro_rules! impl_clip_for_tween {
    ($t:ty) => {
        impl AnimationClip for Tween<$t> {
            fn advance(&mut self) -> bool {
                self.advance()
            }
            fn running(&self) -> bool {
                self.running
            }
            fn finished(&self) -> bool {
                self.finished
            }
        }
    };
}
impl_clip_for_tween!(f32);
impl_clip_for_tween!(f64);
impl_clip_for_tween!(Color);
impl_clip_for_tween!(Point);
impl_clip_for_tween!(Size);

impl AnimationClip for AnimationController {
    fn advance(&mut self) -> bool {
        self.advance()
    }
    fn running(&self) -> bool {
        self.running
    }
    fn finished(&self) -> bool {
        !self.running && self.started
    }
}

pub struct Parallel {
    clips: Vec<Box<dyn AnimationClip>>,
}

impl Parallel {
    pub fn new(clips: Vec<Box<dyn AnimationClip>>) -> Self {
        Self { clips }
    }

    pub fn advance(&mut self) -> bool {
        let mut any_finished = false;
        for clip in &mut self.clips {
            if clip.advance() {
                any_finished = true;
            }
        }
        any_finished
    }

    pub fn running(&self) -> bool {
        self.clips.iter().any(|c| c.running())
    }

    pub fn finished(&self) -> bool {
        self.clips.iter().all(|c| c.finished())
    }
}

pub struct Sequence {
    clips: Vec<Box<dyn AnimationClip>>,
    index: usize,
}

impl Sequence {
    pub fn new(clips: Vec<Box<dyn AnimationClip>>) -> Self {
        Self { clips, index: 0 }
    }

    pub fn advance(&mut self) -> bool {
        if self.index >= self.clips.len() {
            return true;
        }
        let finished = self.clips[self.index].advance();
        if finished && self.clips[self.index].finished() {
            self.index += 1;
        }
        self.index >= self.clips.len()
    }

    pub fn running(&self) -> bool {
        self.index < self.clips.len()
    }

    pub fn finished(&self) -> bool {
        self.index >= self.clips.len()
    }
}

pub struct Timeline<T: Lerp> {
    keyframes: Vec<(f64, T)>,
    duration: f64,
    elapsed: f64,
    pub value: T,
    pub finished: bool,
    easing: Easing,
}

impl<T: Lerp + Copy> Timeline<T> {
    pub fn try_new(keyframes: Vec<(f64, T)>, easing: Easing) -> Option<Self> {
        if keyframes.is_empty() {
            return None;
        }
        if keyframes.iter().any(|&(t, _)| !t.is_finite() || t < 0.0) {
            return None;
        }
        if keyframes.windows(2).any(|w| w[0].0 > w[1].0) {
            return None;
        }
        let duration = keyframes.last().map(|&(t, _)| t)?;
        let value = keyframes.first().map(|&(_, v)| v)?;
        Some(Self {
            keyframes,
            duration,
            elapsed: 0.0,
            value,
            finished: false,
            easing,
        })
    }

    pub fn advance(&mut self, dt: f64) -> bool {
        if self.finished {
            return true;
        }
        self.elapsed = (self.elapsed + dt).min(self.duration);
        self.update_value();
        if self.elapsed >= self.duration {
            self.finished = true;
        }
        self.finished
    }

    pub fn reset(&mut self) {
        self.elapsed = 0.0;
        self.finished = false;
        if let Some(&(_, v)) = self.keyframes.first() {
            self.value = v;
        }
    }

    fn update_value(&mut self) {
        let t = self.elapsed;
        let Some(&(_, first_value)) = self.keyframes.first() else {
            return;
        };
        if t <= 0.0 {
            self.value = first_value;
            return;
        }
        let Some(&(last_t, last_value)) = self.keyframes.last() else {
            return;
        };
        if t >= last_t {
            self.value = last_value;
            return;
        }

        for pair in self.keyframes.windows(2) {
            let (t0, v0) = pair[0];
            let (t1, v1) = pair[1];
            if t >= t0 && t <= t1 {
                let seg_t = ((t - t0) / (t1 - t0)) as f32;
                self.value = v0.lerp(v1, self.easing.apply(seg_t));
                return;
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spring {
    pub damping_ratio: f32,

    pub frequency: f32,
}

impl Spring {
    pub const fn new(damping_ratio: f32, frequency: f32) -> Self {
        Self {
            damping_ratio,
            frequency,
        }
    }

    pub fn value(&self, t: f32) -> f32 {
        let t = t.max(0.0);
        let w0 = std::f32::consts::TAU * self.frequency.max(0.0001);
        let z = self.damping_ratio;
        if z < 1.0 {
            let wd = w0 * (1.0 - z * z).sqrt();
            let c = z / (1.0 - z * z).sqrt();
            let decay = (-z * w0 * t).exp();
            1.0 - decay * ((wd * t).cos() + c * (wd * t).sin())
        } else if z > 1.0 {
            let wd = w0 * (z * z - 1.0).sqrt();
            let c = z / (z * z - 1.0).sqrt();
            let decay = (-z * w0 * t).exp();
            1.0 - decay * ((wd * t).cosh() + c * (wd * t).sinh())
        } else {
            let decay = (-w0 * t).exp();
            1.0 - decay * (1.0 + w0 * t)
        }
    }

    pub fn settle_time(&self, eps: f32) -> f32 {
        let w0 = std::f32::consts::TAU * self.frequency.max(0.0001);
        let z = self.damping_ratio.max(0.0001);
        let envelope = (-eps.ln() / (z * w0)).max(0.0);

        envelope * 1.5
    }

    pub const fn snappy() -> Self {
        Self::new(0.7, 6.0)
    }

    pub const fn bouncy() -> Self {
        Self::new(0.45, 4.0)
    }

    pub const fn gentle() -> Self {
        Self::new(1.0, 2.5)
    }

    pub fn from_stiffness_damping(stiffness: f32, damping: f32, mass: f32) -> Self {
        let m = mass.max(0.0001);
        let k = stiffness.max(0.0);
        let c = damping.max(0.0);
        let omega0 = (k / m).sqrt();
        let freq = omega0 / std::f32::consts::TAU;
        let zeta = if k <= 0.0 || m <= 0.0 {
            1.0
        } else {
            c / (2.0 * (k * m).sqrt())
        };
        Self::new(zeta, freq.max(0.0001))
    }
}

pub trait DurationExt {
    fn ms(self) -> std::time::Duration;
    fn s(self) -> std::time::Duration;
}

macro_rules! impl_duration_ext_int {
    ($($t:ty),*) => {
        $(
            impl DurationExt for $t {
                fn ms(self) -> std::time::Duration {
                    let v = (self as i64).max(0) as u64;
                    std::time::Duration::from_millis(v)
                }
                fn s(self) -> std::time::Duration {
                    let v = (self as f64).max(0.0);
                    std::time::Duration::from_secs_f64(v)
                }
            }
        )*
    };
}
impl_duration_ext_int!(u64, u32, u16, i64, i32, usize);

macro_rules! impl_duration_ext_float {
    ($($t:ty),*) => {
        $(
            impl DurationExt for $t {
                fn ms(self) -> std::time::Duration {
                    let v = (self as f64 / 1000.0).max(0.0);
                    std::time::Duration::from_secs_f64(v)
                }
                fn s(self) -> std::time::Duration {
                    let v = (self as f64).max(0.0);
                    std::time::Duration::from_secs_f64(v)
                }
            }
        )*
    };
}
impl_duration_ext_float!(f32, f64);

pub use Easing::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeline_requires_at_least_one_keyframe() {
        let empty: Vec<(f64, f32)> = Vec::new();
        assert!(Timeline::try_new(empty, Linear).is_none());
    }

    #[test]
    fn timeline_rejects_non_monotonic_times() {
        assert!(Timeline::try_new(vec![(0.0, 0.0), (0.5, 1.0), (0.2, 2.0)], Linear).is_none());
        assert!(Timeline::try_new(vec![(1.0, 0.0), (0.5, 1.0)], Linear).is_none());
    }

    #[test]
    fn timeline_rejects_negative_or_non_finite_times() {
        assert!(Timeline::try_new(vec![(-1.0, 0.0), (1.0, 1.0)], Linear).is_none());
        assert!(
            Timeline::try_new(vec![(0.0, 0.0), (f64::NAN, 1.0)], Linear).is_none(),
            "NaN times are rejected"
        );
    }

    #[test]
    fn timeline_accepts_valid_input() {
        let mut t = Timeline::try_new(vec![(0.0, 0.0f32), (0.5, 50.0f32), (1.0, 100.0f32)], Linear)
            .expect("valid timeline");
        assert_eq!(t.value, 0.0);
        t.advance(0.25);
        assert!(!t.finished);
        assert!(t.value > 0.0 && t.value < 50.0);
        t.advance(1.0);
        assert!(t.finished);
        assert_eq!(t.value, 100.0);
    }

    #[test]
    fn timeline_single_keyframe_is_stable() {
        let mut t = Timeline::try_new(vec![(0.0, 7.0f32)], Linear).expect("single keyframe");
        t.advance(10.0);
        assert!(t.finished);
        assert_eq!(t.value, 7.0);
        t.reset();
        assert!(!t.finished);
        assert_eq!(t.value, 7.0);
    }

    #[test]
    fn timeline_reset_after_advance_recovers_start() {
        let mut t = Timeline::try_new(vec![(0.0, 0.0f32), (1.0, 100.0f32)], Linear)
            .expect("valid timeline");
        t.advance(1.0);
        assert!(t.finished);
        t.reset();
        assert!(!t.finished);
        assert_eq!(t.value, 0.0);

        let flat = Timeline::try_new(vec![(0.0, 1.0f32), (0.0, 2.0f32)], Linear)
            .expect("duplicate times are non-decreasing");
        assert_eq!(flat.value, 1.0);
    }

    #[test]
    fn spring_starts_at_zero_and_settles_at_one() {
        let s = Spring::new(1.0, 4.0);
        assert_eq!(s.value(0.0), 0.0);
        let settled = s.settle_time(0.001);
        assert!(settled > 0.0);
        assert!((s.value(settled) - 1.0).abs() < 0.005);
        assert!((s.value(settled + 1.0) - 1.0).abs() < 0.005);
    }

    #[test]
    fn underdamped_spring_overshoots() {
        let s = Spring::new(0.5, 4.0);
        let mut peak = 0.0f32;
        let mut t = 0.0;
        while t < 2.0 {
            peak = peak.max(s.value(t));
            t += 0.001;
        }
        assert!(peak > 1.05, "under-damped spring should overshoot: {peak}");
        assert!(peak < 1.6, "overshoot should stay bounded: {peak}");
    }

    #[test]
    fn overdamped_spring_never_overshoots() {
        let s = Spring::new(2.0, 4.0);
        let mut peak = 0.0f32;
        let mut t = 0.0;
        while t < 2.0 {
            peak = peak.max(s.value(t));
            t += 0.001;
        }
        assert!(
            peak <= 1.0 + 1e-4,
            "over-damped spring must not overshoot: {peak}"
        );
    }

    #[test]
    fn session_continues_animation_across_rebuilds() {
        let key = SessionKey::caller();
        let mut first = AnimationController::new(0.01, Linear).with_session(key);
        first.start();
        std::thread::sleep(std::time::Duration::from_millis(20));
        assert!(first.advance(), "20 ms must finish a 10 ms animation");

        let mut rebuilt = AnimationController::new(0.01, Linear).with_session(key);
        rebuilt.start();
        assert!(rebuilt.advance());
        assert_eq!(rebuilt.value, 1.0);
        assert!(!rebuilt.running);
    }

    #[test]
    fn session_retarget_animates_without_restarting_or_jumping() {
        let key = SessionKey::caller();
        let mut c = AnimationController::new(0.1, Linear).with_session(key);
        c.start();
        std::thread::sleep(std::time::Duration::from_millis(30));
        c.advance();
        let v_before = c.value;
        assert!(v_before > 0.2 && v_before < 0.5, "value = {v_before}");

        let mut rebuilt = AnimationController::new(0.1, Linear).with_session(key);
        rebuilt.value = v_before;
        rebuilt.animate_to(0.0);
        assert!(rebuilt.running, "re-target must start a new run");
        std::thread::sleep(std::time::Duration::from_millis(20));
        assert!(!rebuilt.advance());
        assert!(
            rebuilt.running,
            "re-target must animate, not settle instantly"
        );
        assert!(
            rebuilt.value < v_before,
            "must head back toward 0 (got {})",
            rebuilt.value
        );

        for _ in 0..8 {
            std::thread::sleep(std::time::Duration::from_millis(30));
            rebuilt.advance();
        }
        assert_eq!(rebuilt.value, 0.0);
        assert!(!rebuilt.running);
        let mut settled = AnimationController::new(0.1, Linear).with_session(key);
        settled.animate_to(0.0);
        assert!(!settled.running, "stray redraw must not replay the run");
    }

    #[test]
    fn session_reset_reanchors_the_clock() {
        let key = SessionKey::caller();
        let mut first = AnimationController::new(0.01, Linear).with_session(key);
        first.start();
        std::thread::sleep(std::time::Duration::from_millis(20));
        first.advance();
        first.reset();

        let mut fresh = AnimationController::new(10.0, Linear).with_session(key);
        fresh.start();
        assert!(!fresh.advance());
        assert!(fresh.running);
        assert!(fresh.value < 0.5);
    }

    #[test]
    fn sessions_are_isolated_per_declaration_site() {
        let key_a = SessionKey::caller();
        let key_b = SessionKey::caller();
        let mut a = AnimationController::new(0.01, Linear).with_session(key_a);
        a.start();
        std::thread::sleep(std::time::Duration::from_millis(20));
        assert!(a.advance());

        let mut b = AnimationController::new(10.0, Linear).with_session(key_b);
        b.start();
        assert!(!b.advance());
        assert!(b.running);
    }

    #[test]
    fn controllers_without_session_keep_legacy_behavior() {
        let mut c = AnimationController::new(10.0, Linear);
        c.start();
        assert!(!c.advance());
        assert!(c.running);
        assert!(c.value < 0.001, "animation should barely have started");
    }

    #[test]
    fn stalled_frame_does_not_teleport_session_animation() {
        let key = SessionKey::caller();
        let mut first = AnimationController::new(1.0, Linear).with_session(key);
        first.start();

        std::thread::sleep(std::time::Duration::from_millis(300));

        let mut rebuilt = AnimationController::new(1.0, Linear).with_session(key);
        rebuilt.start();
        assert!(
            !rebuilt.advance(),
            "a stalled frame must not finish the animation"
        );
        assert!(rebuilt.running);
        assert!(
            rebuilt.value < 0.5,
            "stalled frame advanced too far: {}",
            rebuilt.value
        );
    }
}
