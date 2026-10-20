#![forbid(unsafe_code)]

use std::ops::Range;
use std::time::Duration;

use sequoia_core::Vec2;

use crate::{Animation, Easing, Keyframe, Lerp, Spring, Track};

fn dur_secs(d: Duration) -> f32 {
    d.as_secs_f32().max(0.0)
}

#[derive(Clone, Debug)]
pub struct Animate {
    pub opacity: Option<(f32, f32)>,
    pub offset: Option<(Vec2, Vec2)>,
    pub duration_secs: f32,
    pub delay_secs: f32,
    pub easing: Easing,
}

impl Animate {
    fn sample_one(from: f32, to: f32, delay: f32, dur: f32, ease: Easing, t: f32) -> f32 {
        let t = t.max(0.0);
        if t < delay {
            return from;
        }
        if dur <= 0.0 {
            return to;
        }
        let k = ((t - delay) / dur).clamp(0.0, 1.0);
        from + (to - from) * ease.apply(k)
    }

    pub fn sample_at(&self, elapsed_secs: f32) -> AnimateFrame {
        let op = self.opacity.map(|(a, b)| {
            Self::sample_one(
                a,
                b,
                self.delay_secs,
                self.duration_secs,
                self.easing,
                elapsed_secs,
            )
        });
        let off = self.offset.map(|(a, b)| {
            let x = Self::sample_one(
                a.x,
                b.x,
                self.delay_secs,
                self.duration_secs,
                self.easing,
                elapsed_secs,
            );
            let y = Self::sample_one(
                a.y,
                b.y,
                self.delay_secs,
                self.duration_secs,
                self.easing,
                elapsed_secs,
            );
            Vec2::new(x, y)
        });
        AnimateFrame {
            opacity: op,
            offset: off,
        }
    }

    pub fn is_running(&self, elapsed_secs: f32) -> bool {
        elapsed_secs < self.delay_secs + self.duration_secs
    }

    pub fn into_animation(&self, name: impl Into<String>) -> Option<Animation> {
        let name = name.into();
        let mut anim = Animation::new(name, self.duration_secs.max(0.0) as f64);
        let mut any = false;
        if let Some((a, b)) = self.opacity {
            let track = Track::try_new(vec![
                Keyframe::new(0.0, crate::Value::f32(a)),
                Keyframe::new(self.duration_secs.max(0.0) as f64, crate::Value::f32(b)),
            ])?;
            anim = anim.with_track("opacity", track);
            any = true;
        }
        if let Some((a, b)) = self.offset {
            let dx = Track::try_new(vec![
                Keyframe::new(0.0, crate::Value::px(a.x)),
                Keyframe::new(self.duration_secs.max(0.0) as f64, crate::Value::px(b.x)),
            ])?;
            anim = anim.with_track("x", dx);
            let dy = Track::try_new(vec![
                Keyframe::new(0.0, crate::Value::px(a.y)),
                Keyframe::new(self.duration_secs.max(0.0) as f64, crate::Value::px(b.y)),
            ])?;
            anim = anim.with_track("y", dy);
            any = true;
        }
        if any { Some(anim) } else { None }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnimateFrame {
    pub opacity: Option<f32>,
    pub offset: Option<Vec2>,
}

impl From<AnimateBuilder> for Animate {
    fn from(b: AnimateBuilder) -> Self {
        b.build()
    }
}

pub fn animate() -> AnimateBuilder {
    AnimateBuilder {
        opacity: None,
        offset: None,
        duration_secs: 0.25,
        delay_secs: 0.0,
        easing: Easing::EaseOutCubic,
    }
}

#[derive(Clone, Debug)]
pub struct AnimateBuilder {
    opacity: Option<(f32, f32)>,
    offset: Option<(Vec2, Vec2)>,
    duration_secs: f32,
    delay_secs: f32,
    easing: Easing,
}

impl AnimateBuilder {
    pub fn opacity(mut self, range: Range<f32>) -> Self {
        self.opacity = Some((range.start.clamp(0.0, 1.0), range.end.clamp(0.0, 1.0)));
        self
    }

    pub fn translate(mut self, from: Vec2, to: Vec2) -> Self {
        self.offset = Some((from, to));
        self
    }

    pub fn translate_xy(mut self, from: (f32, f32), to: (f32, f32)) -> Self {
        self.offset = Some((Vec2::new(from.0, from.1), Vec2::new(to.0, to.1)));
        self
    }

    pub fn translate_y(mut self, range: Range<f32>) -> Self {
        let (from, to) = (Vec2::new(0.0, range.start), Vec2::new(0.0, range.end));
        self.offset = Some((from, to));
        self
    }

    pub fn translate_x(mut self, range: Range<f32>) -> Self {
        let (from, to) = (Vec2::new(range.start, 0.0), Vec2::new(range.end, 0.0));
        self.offset = Some((from, to));
        self
    }

    pub fn duration(mut self, d: Duration) -> Self {
        self.duration_secs = dur_secs(d);
        self
    }

    pub fn delay(mut self, d: Duration) -> Self {
        self.delay_secs = dur_secs(d);
        self
    }

    pub fn ease(mut self, easing: Easing) -> Self {
        self.easing = easing;
        self
    }

    pub fn ease_out(mut self) -> Self {
        self.easing = Easing::EaseOutCubic;
        self
    }

    pub fn ease_in_out(mut self) -> Self {
        self.easing = Easing::EaseInOutCubic;
        self
    }

    pub fn linear(mut self) -> Self {
        self.easing = Easing::Linear;
        self
    }

    pub fn build(self) -> Animate {
        Animate {
            opacity: self.opacity,
            offset: self.offset,
            duration_secs: self.duration_secs,
            delay_secs: self.delay_secs,
            easing: self.easing,
        }
    }

    pub fn sample_at(&self, elapsed_secs: f32) -> AnimateFrame {
        self.clone().build().sample_at(elapsed_secs)
    }
}

pub fn animation(from: f32, to: f32) -> TweenBuilder {
    TweenBuilder {
        from,
        to,
        duration_secs: 0.25,
        delay_secs: 0.0,
        easing: Easing::EaseOutCubic,
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TweenBuilder {
    from: f32,
    to: f32,
    duration_secs: f32,
    delay_secs: f32,
    easing: Easing,
}

impl TweenBuilder {
    pub fn duration(mut self, d: Duration) -> Self {
        self.duration_secs = dur_secs(d);
        self
    }

    pub fn delay(mut self, d: Duration) -> Self {
        self.delay_secs = dur_secs(d);
        self
    }

    pub fn ease(mut self, easing: Easing) -> Self {
        self.easing = easing;
        self
    }

    pub fn ease_out(self) -> Self {
        self.ease(Easing::EaseOutCubic)
    }

    pub fn sample_at(&self, elapsed_secs: f32) -> f32 {
        Animate::sample_one(
            self.from,
            self.to,
            self.delay_secs,
            self.duration_secs,
            self.easing,
            elapsed_secs,
        )
    }

    pub fn is_running(&self, elapsed_secs: f32) -> bool {
        elapsed_secs < self.delay_secs + self.duration_secs
    }
}

pub fn spring(target: f32) -> SpringBuilder {
    SpringBuilder {
        from: 0.0,
        target,
        stiffness: 180.0,
        damping: 20.0,
        mass: 1.0,
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SpringBuilder {
    from: f32,
    target: f32,
    stiffness: f32,
    damping: f32,
    mass: f32,
}

impl SpringBuilder {
    pub fn from(mut self, value: f32) -> Self {
        self.from = value;
        self
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

    pub fn spring(&self) -> Spring {
        Spring::from_stiffness_damping(self.stiffness, self.damping, self.mass)
    }

    pub fn sample_at(&self, t_secs: f32) -> f32 {
        let s = self.spring().value(t_secs.max(0.0));
        self.from + (self.target - self.from) * s
    }

    pub fn settle_time(&self, eps: f32) -> f32 {
        self.spring().settle_time(eps)
    }
}

pub fn lerp_sample<T: Lerp>(from: T, to: T, easing: Easing, progress: f32) -> T {
    from.lerp(to, easing.apply(progress.clamp(0.0, 1.0)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DurationExt;

    #[test]
    fn animate_opacity_walks_zero_to_one() {
        let a = animate()
            .opacity(0.0..1.0)
            .duration(200.ms())
            .ease_out()
            .build();
        assert_eq!(a.sample_at(0.0).opacity, Some(0.0));
        let mid = a.sample_at(0.1).opacity.unwrap();
        assert!(mid > 0.0 && mid < 1.0, "mid={mid}");
        assert_eq!(a.sample_at(10.0).opacity, Some(1.0));
        assert!(a.is_running(0.1));
        assert!(!a.is_running(10.0));
    }

    #[test]
    fn animate_translate_y_and_delay() {
        use std::time::Duration;
        let a = animate()
            .translate_y(-8.0..0.0)
            .duration(Duration::from_millis(200))
            .delay(Duration::from_millis(100))
            .linear()
            .build();
        assert_eq!(a.sample_at(0.0).offset, Some(Vec2::new(0.0, -8.0)));
        assert_eq!(a.sample_at(0.05).offset, Some(Vec2::new(0.0, -8.0)));
        let mid = a.sample_at(0.2).offset.unwrap();
        assert!(mid.y > -8.0 && mid.y < 0.0, "mid={mid:?}");
        assert_eq!(a.sample_at(10.0).offset, Some(Vec2::ZERO));
    }

    #[test]
    fn tween_builder_samples() {
        let t = animation(0.0, 1.0).duration(200.ms()).ease_out();
        assert_eq!(t.sample_at(0.0), 0.0);
        assert_eq!(t.sample_at(10.0), 1.0);
    }

    #[test]
    fn spring_builder_settles_and_maps_params() {
        let s = spring(100.0).from(0.0).stiffness(180.0).damping(20.0);
        assert_eq!(s.sample_at(0.0), 0.0);
        let settled = s.sample_at(s.settle_time(0.001) + 1.0);
        assert!((settled - 100.0).abs() < 0.5, "settled={settled}");

        let stiff = spring(1.0).from(0.0).stiffness(200.0).damping(40.0);
        let mut peak = 0.0f32;
        let mut t = 0.0;
        while t < 2.0 {
            peak = peak.max(stiff.sample_at(t));
            t += 0.005;
        }
        assert!(peak <= 1.001, "peak={peak}");
    }

    #[test]
    fn animate_lowers_to_animation_clip() {
        let a = animate().opacity(0.0..1.0).duration(200.ms()).build();
        let clip = a.into_animation("fade").expect("ma track");
        let f = clip.sample(0.0);
        assert!(f.get("opacity").is_some());
    }
}
