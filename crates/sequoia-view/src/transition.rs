#![forbid(unsafe_code)]

use sequoia_animation::{Animate, Easing, animate};
use sequoia_core::Vec2;

pub fn fade_in() -> Animate {
    animate().opacity(0.0..1.0).build()
}

pub fn slide_in(dy: f32) -> Animate {
    animate().opacity(0.0..1.0).translate_y(dy..0.0).build()
}

pub fn pop_in(dy: f32) -> Animate {
    animate()
        .opacity(0.0..1.0)
        .translate_y(dy..0.0)
        .ease(Easing::EaseOutBack)
        .build()
}

#[derive(Clone, Debug, Default)]
pub struct Sequence {
    steps: Vec<(Animate, f32)>,
}

impl Sequence {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(mut self, anim: Animate, at_secs: f32) -> Self {
        self.steps.push((anim, at_secs.max(0.0)));
        self
    }

    pub fn sample_at(&self, t_secs: f32) -> SequenceFrame {
        let mut opacity: Option<f32> = None;
        let mut offset: Option<Vec2> = None;
        for (anim, at) in &self.steps {
            let f = anim.sample_at((t_secs - at).max(0.0));
            if f.opacity.is_some() {
                opacity = f.opacity;
            }
            if f.offset.is_some() {
                offset = f.offset;
            }
        }
        SequenceFrame { opacity, offset }
    }

    pub fn is_running(&self, t_secs: f32) -> bool {
        self.steps
            .iter()
            .any(|(a, at)| a.is_running((t_secs - at).max(0.0)))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SequenceFrame {
    pub opacity: Option<f32>,
    pub offset: Option<Vec2>,
}

pub fn stagger(
    n: usize,
    each_delay_secs: f32,
    make: impl Fn(usize) -> Animate,
) -> Vec<(Animate, f32)> {
    (0..n)
        .map(|i| (make(i), i as f32 * each_delay_secs.max(0.0)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convenience_builders_cover_fade_slide_pop() {
        let f = fade_in();
        assert_eq!(f.sample_at(0.0).opacity, Some(0.0));
        assert_eq!(f.sample_at(10.0).opacity, Some(1.0));
        let s = slide_in(-8.0);
        assert_eq!(s.sample_at(10.0).offset, Some(Vec2::ZERO));
        let p = pop_in(12.0);
        assert_eq!(p.sample_at(10.0).opacity, Some(1.0));
    }

    #[test]
    fn sequence_layers_steps() {
        use std::time::Duration;
        let seq = Sequence::new()
            .add(
                animate()
                    .opacity(0.0..1.0)
                    .duration(Duration::from_millis(200))
                    .build(),
                0.0,
            )
            .add(
                animate()
                    .translate_y(-8.0..0.0)
                    .duration(Duration::from_millis(200))
                    .build(),
                0.1,
            );
        let f = seq.sample_at(0.05);
        assert!(f.opacity.is_some());

        assert_eq!(f.offset, Some(Vec2::new(0.0, -8.0)));
        assert!(seq.is_running(0.05));
        assert!(!seq.is_running(10.0));
    }

    #[test]
    fn stagger_offsets_starts() {
        let steps = stagger(3, 0.05, |_| fade_in());
        assert_eq!(steps.len(), 3);
        assert_eq!(steps[1].1, 0.05);
        assert_eq!(steps[2].1, 0.1);
    }
}
