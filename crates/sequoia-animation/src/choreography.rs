use crate::animation::{Animation, AnimationFrame, LoopMode};
use crate::mixer::Mixer;

#[derive(Clone, Debug)]
struct Entry {
    animation: Animation,
    at: f64,
    weight: f32,
}

#[derive(Clone, Debug, Default)]
pub struct Choreography {
    entries: Vec<Entry>,
    loop_mode: LoopMode,
}

impl Choreography {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_loop(mut self, mode: LoopMode) -> Self {
        self.loop_mode = mode;
        self
    }

    pub fn add(mut self, animation: Animation, at_secs: f64) -> Self {
        self.entries.push(Entry {
            animation,
            at: at_secs.max(0.0),
            weight: 1.0,
        });
        self
    }

    pub fn add_weighted(mut self, animation: Animation, at_secs: f64, weight: f32) -> Self {
        self.entries.push(Entry {
            animation,
            at: at_secs.max(0.0),
            weight,
        });
        self
    }

    pub fn stagger(animations: impl IntoIterator<Item = Animation>, step: f64) -> Self {
        Self::new().add_staggered(animations, step)
    }

    pub fn add_staggered(
        mut self,
        animations: impl IntoIterator<Item = Animation>,
        step: f64,
    ) -> Self {
        let mut at = self.duration();
        for animation in animations {
            self.entries.push(Entry {
                animation,
                at,
                weight: 1.0,
            });
            at += step.max(0.0);
        }
        self
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn loop_mode(&self) -> LoopMode {
        self.loop_mode
    }

    pub fn duration(&self) -> f64 {
        self.entries
            .iter()
            .map(|e| {
                let len = match e.animation.loop_mode() {
                    LoopMode::Once => e.animation.duration(),

                    LoopMode::Loop | LoopMode::PingPong => e.animation.duration(),
                };
                e.at + len.max(0.0)
            })
            .fold(0.0, f64::max)
    }

    pub fn is_finished_at(&self, t: f64) -> bool {
        self.loop_mode == LoopMode::Once && t >= self.duration()
    }

    pub fn sample(&self, t: f64) -> AnimationFrame {
        let t = self.wrap_time(t);
        let samples: Vec<(AnimationFrame, f32)> = self
            .entries
            .iter()
            .filter_map(|e| {
                let local = t - e.at;
                if local < 0.0 || e.weight <= 0.0 {
                    return None;
                }
                Some((e.animation.sample(local), e.weight))
            })
            .collect();
        if samples.is_empty() {
            return AnimationFrame::new();
        }
        Mixer::mix_frames(samples.iter().map(|(f, w)| (f, *w)))
    }

    fn wrap_time(&self, t: f64) -> f64 {
        let duration = self.duration();
        if duration <= 0.0 {
            return 0.0;
        }
        match self.loop_mode {
            LoopMode::Once => t.max(0.0).min(duration),
            LoopMode::Loop => t.max(0.0) % duration,
            LoopMode::PingPong => {
                let period = duration * 2.0;
                let t = t.max(0.0) % period;
                if t <= duration { t } else { period - t }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::{Keyframe, Track};
    use crate::value::Value;

    fn fade(dur: f64) -> Animation {
        Animation::new("fade", dur).with_track(
            "opacity",
            Track::try_new(vec![
                Keyframe::new(0.0, Value::f32(0.0)),
                Keyframe::new(dur, Value::f32(1.0)),
            ])
            .unwrap(),
        )
    }

    fn slide(name: &str, dur: f64, to: f32) -> Animation {
        Animation::new(name, dur).with_track(
            "x",
            Track::try_new(vec![
                Keyframe::new(0.0, Value::px(0.0)),
                Keyframe::new(dur, Value::px(to)),
            ])
            .unwrap(),
        )
    }

    #[test]
    fn empty_choreography_is_inert() {
        let c = Choreography::new();
        assert!(c.is_empty());
        assert_eq!(c.duration(), 0.0);
        assert!(c.sample(1.0).is_empty());
        assert!(c.is_finished_at(0.0));
    }

    #[test]
    fn duration_is_latest_clip_end() {
        let c = Choreography::new()
            .add(slide("a", 0.4, 10.0), 0.0)
            .add(slide("b", 0.4, 10.0), 0.25);
        assert!((c.duration() - 0.65).abs() < 1e-9);
        assert_eq!(c.len(), 2);
    }

    #[test]
    fn clips_before_their_offset_contribute_nothing() {
        let c = Choreography::new()
            .add(slide("a", 0.2, 100.0), 0.0)
            .add(slide("b", 0.2, 100.0), 0.5);

        let f0 = c.sample(0.0);
        assert_eq!(f0.get("x"), Some(&Value::px(0.0)));

        let f1 = c.sample(0.1);
        assert_eq!(f1.get("x"), Some(&Value::px(50.0)));
    }

    #[test]
    fn stagger_offsets_each_clip() {
        let c = Choreography::stagger([fade(0.2), fade(0.2), fade(0.2)], 0.1);

        assert!((c.duration() - 0.4).abs() < 1e-9);

        assert_eq!(c.sample(0.0).get("opacity"), Some(&Value::f32(0.0)));

        let mixed = c.sample(0.1).get("opacity").copied().unwrap();
        assert!(matches!(mixed, Value::F32(v) if (v - 0.25).abs() < 1e-6));

        assert_eq!(c.sample(10.0).get("opacity"), Some(&Value::f32(1.0)));
    }

    #[test]
    fn weighted_entries_blend() {
        let a = slide("a", 1.0, 100.0);
        let b = slide("b", 1.0, -100.0);
        let c = Choreography::new()
            .add_weighted(a, 0.0, 0.75)
            .add_weighted(b, 0.0, 0.25);
        let v = c.sample(1.0).get("x").copied().unwrap();
        assert_eq!(v, Value::px(50.0));
    }

    #[test]
    fn once_holds_last_frame_and_reports_finished() {
        let c = Choreography::new().add(fade(0.2), 0.0);
        assert!(!c.is_finished_at(0.1));
        assert!(c.is_finished_at(0.2));
        assert_eq!(c.sample(5.0).get("opacity"), Some(&Value::f32(1.0)));
    }

    #[test]
    fn looping_timeline_wraps() {
        let c = Choreography::new()
            .add(fade(0.2), 0.0)
            .with_loop(LoopMode::Loop);
        assert!(!c.is_finished_at(100.0));

        let v = c.sample(0.25).get("opacity").copied().unwrap();
        assert!(matches!(v, Value::F32(x) if (x - 0.25).abs() < 1e-5));
    }

    #[test]
    fn pingpong_reverses_at_the_end() {
        let c = Choreography::new()
            .add(fade(0.2), 0.0)
            .with_loop(LoopMode::PingPong);

        let v = c.sample(0.3).get("opacity").copied().unwrap();
        assert!(matches!(v, Value::F32(x) if (x - 0.5).abs() < 1e-5));
    }

    #[test]
    fn add_staggered_chains_from_the_end() {
        let c = Choreography::new()
            .add(fade(0.3), 0.0)
            .add_staggered([fade(0.2), fade(0.2)], 0.05);

        assert!((c.duration() - 0.55).abs() < 1e-9);
    }
}
