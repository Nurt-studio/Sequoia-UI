use std::collections::HashMap;

use crate::animation::{Animation, AnimationFrame};
use crate::value::Value;

#[derive(Clone, Copy, Debug, Default)]
pub struct Mixer;

impl Mixer {
    pub fn mix_frames<'a>(
        frames: impl IntoIterator<Item = (&'a AnimationFrame, f32)>,
    ) -> AnimationFrame {
        let mut groups: HashMap<&str, Vec<(&Value, f32)>> = HashMap::new();
        let mut order: Vec<&str> = Vec::new();
        for (frame, weight) in frames {
            if weight <= 0.0 {
                continue;
            }
            for (name, value) in &frame.properties {
                if !groups.contains_key(name.as_str()) {
                    groups.insert(name.as_str(), Vec::new());
                    order.push(name.as_str());
                }
                groups.get_mut(name.as_str()).unwrap().push((value, weight));
            }
        }
        let mut out = AnimationFrame::new();
        for name in order {
            let samples = &groups[name];
            if let Some(v) = Value::mix(samples.iter().copied()) {
                out.properties.insert(name.to_string(), v);
            }
        }
        out
    }

    pub fn mix<'a>(entries: impl IntoIterator<Item = (&'a Animation, f64, f32)>) -> AnimationFrame {
        let frames: Vec<(AnimationFrame, f32)> = entries
            .into_iter()
            .map(|(a, t, w)| (a.sample(t), w))
            .collect();
        Self::mix_frames(frames.iter().map(|(f, w)| (f, *w)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::{Keyframe, LoopMode, Track};
    use crate::value::Value;

    fn anim(name: &str, from: f32, to: f32) -> Animation {
        Animation::new(name, 1.0).with_track(
            "x",
            Track::try_new(vec![
                Keyframe::new(0.0, Value::px(from)),
                Keyframe::new(1.0, Value::px(to)),
            ])
            .unwrap(),
        )
    }

    #[test]
    fn mix_frames_blends_shared_properties() {
        let a = AnimationFrame::new();
        let mut b = AnimationFrame::new();
        b.insert("x", Value::px(0.0));
        let mut c = AnimationFrame::new();
        c.insert("x", Value::px(100.0));
        let out = Mixer::mix_frames([(&b, 0.75), (&c, 0.25)]);
        assert_eq!(out.get("x"), Some(&Value::px(25.0)));
        let _ = a;
    }

    #[test]
    fn mix_animations_samples_then_blends() {
        let idle = anim("idle", 0.0, 0.0);
        let wave = anim("wave", 0.0, 100.0);

        let out = Mixer::mix([(&idle, 0.0, 0.5), (&wave, 0.5, 0.5)]);
        assert_eq!(out.get("x"), Some(&Value::px(25.0)));
    }

    #[test]
    fn zero_weight_entries_are_ignored() {
        let a = anim("a", 10.0, 10.0);
        let b = anim("b", 0.0, 100.0);
        let out = Mixer::mix([(&a, 0.0, 0.0), (&b, 1.0, 1.0)]);
        assert_eq!(out.get("x"), Some(&Value::px(100.0)));
    }

    #[test]
    fn looping_animation_mixes_its_wrapped_sample() {
        let b = anim("b", 0.0, 100.0).with_loop(LoopMode::Loop);

        let out = Mixer::mix([(&b, 1.5, 1.0)]);
        assert_eq!(out.get("x"), Some(&Value::px(50.0)));
    }
}
