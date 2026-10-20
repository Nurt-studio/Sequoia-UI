use std::collections::HashMap;

use sequoia_core::Color;

use crate::Easing;
use crate::Spring;
use crate::bezier::CubicBezier;
use crate::value::Value;

#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum Interpolation {
    Hold,

    Linear,

    Bezier(CubicBezier),

    Spring(Spring),

    Easing(Easing),
}

impl Interpolation {
    pub fn apply(&self, t: f32) -> f32 {
        match self {
            Self::Hold => 0.0,
            Self::Linear => t,
            Self::Bezier(b) => b.solve(t),
            Self::Spring(s) => s.value(t),
            Self::Easing(e) => e.apply(t),
        }
    }

    pub fn from_easing(easing: Easing) -> Self {
        match easing {
            Easing::CubicBezier(x1, y1, x2, y2) => Self::Bezier(CubicBezier::new(x1, y1, x2, y2)),
            Easing::Linear => Self::Linear,
            other => Self::Easing(other),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Keyframe {
    pub time: f64,
    pub value: Value,
    pub interpolation: Interpolation,
}

impl Keyframe {
    pub fn new(time: f64, value: impl Into<Value>) -> Self {
        Self {
            time,
            value: value.into(),
            interpolation: Interpolation::Linear,
        }
    }
    pub fn hold(time: f64, value: impl Into<Value>) -> Self {
        Self {
            time,
            value: value.into(),
            interpolation: Interpolation::Hold,
        }
    }
    pub fn with_interpolation(mut self, interpolation: Interpolation) -> Self {
        self.interpolation = interpolation;
        self
    }

    pub fn with_easing(self, easing: Easing) -> Self {
        self.with_interpolation(Interpolation::from_easing(easing))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Track {
    keyframes: Vec<Keyframe>,
    duration: f64,
}

impl Track {
    pub fn try_new(keyframes: Vec<Keyframe>) -> Option<Self> {
        if keyframes.is_empty() {
            return None;
        }
        if keyframes
            .iter()
            .any(|k| !k.time.is_finite() || k.time < 0.0)
        {
            return None;
        }
        if keyframes.windows(2).any(|w| w[0].time > w[1].time) {
            return None;
        }
        let duration = keyframes.last()?.time;
        Some(Self {
            keyframes,
            duration,
        })
    }

    pub fn keyframes(&self) -> &[Keyframe] {
        &self.keyframes
    }

    pub fn px(points: &[(f64, f32)]) -> Option<Self> {
        Self::try_new(
            points
                .iter()
                .map(|&(t, v)| Keyframe::new(t, Value::px(v)))
                .collect(),
        )
    }

    pub fn f32(points: &[(f64, f32)]) -> Option<Self> {
        Self::try_new(
            points
                .iter()
                .map(|&(t, v)| Keyframe::new(t, Value::f32(v)))
                .collect(),
        )
    }

    pub fn color(points: &[(f64, Color)]) -> Option<Self> {
        Self::try_new(
            points
                .iter()
                .map(|&(t, c)| Keyframe::new(t, Value::color(c)))
                .collect(),
        )
    }

    pub fn with_easing(mut self, easing: Easing) -> Self {
        let interpolation = Interpolation::from_easing(easing);
        for keyframe in &mut self.keyframes {
            keyframe.interpolation = interpolation;
        }
        self
    }

    pub fn duration(&self) -> f64 {
        self.duration
    }

    pub fn sample(&self, t: f64) -> Option<Value> {
        let t = t.max(0.0);
        let first = self.keyframes.first()?;
        if t < first.time {
            return Some(first.value);
        }
        let last = self.keyframes.last()?;
        if t >= last.time {
            return Some(last.value);
        }
        for pair in self.keyframes.windows(2) {
            let (a, b) = (&pair[0], &pair[1]);
            if t >= a.time && t <= b.time {
                if a.time == b.time {
                    return Some(b.value);
                }
                let seg_t = ((t - a.time) / (b.time - a.time)) as f32;
                return Some(match a.interpolation {
                    Interpolation::Hold => a.value,
                    Interpolation::Spring(s) => {
                        let seg_dur = (b.time - a.time).max(1e-6) as f32;
                        a.value.lerp(&b.value, s.value(seg_t * seg_dur))
                    }
                    _ => a.value.lerp(&b.value, a.interpolation.apply(seg_t)),
                });
            }
        }

        Some(first.value)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum LoopMode {
    #[default]
    Once,

    Loop,

    PingPong,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AnimationFrame {
    pub properties: HashMap<String, Value>,
}

impl AnimationFrame {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn get(&self, name: &str) -> Option<&Value> {
        self.properties.get(name)
    }
    pub fn insert(&mut self, name: impl Into<String>, value: impl Into<Value>) {
        self.properties.insert(name.into(), value.into());
    }
    pub fn len(&self) -> usize {
        self.properties.len()
    }
    pub fn is_empty(&self) -> bool {
        self.properties.is_empty()
    }
}

#[derive(Clone, Debug)]
pub struct Animation {
    name: String,
    duration: f64,
    speed: f32,
    loop_mode: LoopMode,
    tracks: HashMap<String, Track>,
}

impl Animation {
    pub fn new(name: impl Into<String>, duration: f64) -> Self {
        Self {
            name: name.into(),
            duration,
            speed: 1.0,
            loop_mode: LoopMode::Once,
            tracks: HashMap::new(),
        }
    }

    pub fn with_speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }
    pub fn with_loop(mut self, mode: LoopMode) -> Self {
        self.loop_mode = mode;
        self
    }
    pub fn with_track(mut self, name: impl Into<String>, track: Track) -> Self {
        self.tracks.insert(name.into(), track);
        self
    }

    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn duration(&self) -> f64 {
        self.duration
    }
    pub fn speed(&self) -> f32 {
        self.speed
    }
    pub fn loop_mode(&self) -> LoopMode {
        self.loop_mode
    }
    pub fn tracks(&self) -> &HashMap<String, Track> {
        &self.tracks
    }

    pub fn sample(&self, t: f64) -> AnimationFrame {
        let t = self.wrap_time(t);
        let mut frame = AnimationFrame::new();
        for (name, track) in &self.tracks {
            if let Some(v) = track.sample(t) {
                frame.properties.insert(name.clone(), v);
            }
        }
        frame
    }

    pub fn is_finished_at(&self, t: f64) -> bool {
        self.loop_mode == LoopMode::Once && t >= self.duration
    }

    fn wrap_time(&self, t: f64) -> f64 {
        if self.duration <= 0.0 {
            return 0.0;
        }
        match self.loop_mode {
            LoopMode::Once => t.max(0.0).min(self.duration),
            LoopMode::Loop => {
                let t = t.max(0.0);
                t % self.duration
            }
            LoopMode::PingPong => {
                let period = self.duration * 2.0;
                let t = t.max(0.0) % period;
                if t <= self.duration { t } else { period - t }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keyframes() -> Vec<Keyframe> {
        vec![
            Keyframe::new(0.0, Value::px(0.0)),
            Keyframe::new(0.5, Value::px(50.0)),
            Keyframe::new(1.0, Value::px(100.0)),
        ]
    }

    #[test]
    fn track_rejects_invalid_input() {
        assert!(Track::try_new(vec![]).is_none());
        assert!(Track::try_new(vec![Keyframe::new(-1.0, 0.0f32)]).is_none());
        assert!(Track::try_new(vec![Keyframe::new(f64::NAN, 0.0f32)]).is_none());
        assert!(
            Track::try_new(vec![
                Keyframe::new(0.0, 0.0f32),
                Keyframe::new(0.1, 1.0f32),
                Keyframe::new(0.05, 2.0f32),
            ])
            .is_none()
        );
    }

    #[test]
    fn track_samples_segments() {
        let t = Track::try_new(keyframes()).expect("valid");
        assert_eq!(t.duration(), 1.0);
        assert_eq!(t.sample(-5.0), Some(Value::px(0.0)));
        assert_eq!(t.sample(0.25), Some(Value::px(25.0)));
        assert_eq!(t.sample(1.5), Some(Value::px(100.0)));
    }

    #[test]
    fn hold_keyframe_does_not_interpolate() {
        let t = Track::try_new(vec![
            Keyframe::hold(0.0, 0.0f32),
            Keyframe::new(1.0, 100.0f32),
        ])
        .expect("valid");
        assert_eq!(t.sample(0.5), Some(Value::f32(0.0)));
        assert_eq!(t.sample(0.99), Some(Value::f32(0.0)));
        assert_eq!(t.sample(1.0), Some(Value::f32(100.0)));
    }

    #[test]
    fn spring_keyframes_overshoot_then_settle() {
        let spring = Spring::new(0.4, 2.0);
        let t = Track::try_new(vec![
            Keyframe::new(0.0, 0.0f32).with_interpolation(Interpolation::Spring(spring)),
            Keyframe::new(1.0, 100.0f32),
        ])
        .expect("valid");

        assert!(t.sample(0.1).unwrap().as_f32().unwrap() > 0.0);
        let mid = t.sample(0.5).unwrap().as_f32().unwrap();
        assert!(mid != 50.0, "spring must not be linear");

        assert_eq!(t.sample(10.0), Some(Value::f32(100.0)));
    }

    #[test]
    fn duplicate_times_do_not_divide_by_zero() {
        let t = Track::try_new(vec![Keyframe::new(0.0, 1.0f32), Keyframe::new(0.0, 2.0f32)])
            .expect("duplicate times are non-decreasing");
        assert_eq!(t.sample(0.0), Some(Value::f32(2.0)));
    }

    #[test]
    fn loop_modes_wrap_time() {
        let a = Animation::new("a", 1.0).with_track("x", Track::try_new(keyframes()).unwrap());
        assert_eq!(a.sample(1.5).get("x"), Some(&Value::px(100.0)));

        let looped = Animation::new("a", 1.0)
            .with_loop(LoopMode::Loop)
            .with_track("x", Track::try_new(keyframes()).unwrap());
        assert_eq!(looped.sample(1.5).get("x"), Some(&Value::px(50.0)));

        let pingpong = Animation::new("a", 1.0)
            .with_loop(LoopMode::PingPong)
            .with_track("x", Track::try_new(keyframes()).unwrap());

        assert_eq!(pingpong.sample(1.5).get("x"), Some(&Value::px(50.0)));
    }

    #[test]
    fn once_animation_reports_finished() {
        let a = Animation::new("a", 1.0);
        assert!(a.is_finished_at(1.0));
        assert!(!a.is_finished_at(0.9));
        let l = a.with_loop(LoopMode::Loop);
        assert!(!l.is_finished_at(10.0));
    }

    #[test]
    fn builder_helpers_make_typed_tracks() {
        let px = Track::px(&[(0.0, 0.0), (1.0, 100.0)]).unwrap();
        assert_eq!(px.sample(0.5), Some(Value::px(50.0)));
        let f = Track::f32(&[(0.0, 1.0), (1.0, 0.0)]).unwrap();
        assert_eq!(f.sample(0.5), Some(Value::f32(0.5)));
        let c = Track::color(&[(0.0, Color::BLACK), (1.0, Color::WHITE)]).unwrap();
        assert!(c.sample(0.5).and_then(|v| v.as_color()).is_some());
        assert!(Track::px(&[]).is_none(), "still validated");
    }

    #[test]
    fn with_easing_reshapes_the_whole_track() {
        let linear = Track::px(&[(0.0, 0.0), (1.0, 100.0)]).unwrap();
        let eased = linear.clone().with_easing(Easing::EaseInQuad);

        let v = eased.sample(0.5).unwrap().as_px().unwrap();
        assert!((v - 25.0).abs() < 1e-3, "eased: {v}");
        assert_eq!(
            linear.sample(0.5),
            Some(Value::px(50.0)),
            "source unchanged"
        );
    }

    #[test]
    fn interpolation_from_easing_bridges_curves() {
        assert_eq!(
            Interpolation::from_easing(Easing::Linear),
            Interpolation::Linear
        );
        assert!(matches!(
            Interpolation::from_easing(Easing::EaseOutBack),
            Interpolation::Easing(Easing::EaseOutBack)
        ));
        assert!(matches!(
            Interpolation::from_easing(Easing::CubicBezier(0.3, 1.4, 0.4, 1.0)),
            Interpolation::Bezier(_)
        ));

        let kf = Keyframe::new(0.0, Value::px(0.0)).with_easing(Easing::EaseOutQuart);
        assert!(matches!(kf.interpolation, Interpolation::Easing(_)));
    }
}
