#![forbid(unsafe_code)]

use sequoia_core::SpanMask;

use crate::effect::Backdrop;

#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum Mask {
    LinearFade {
        from_alpha: f32,
        to_alpha: f32,
        angle_rad: f32,
    },

    Uniform(f32),
}

impl Mask {
    pub fn strength_at(&self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match *self {
            Mask::LinearFade {
                from_alpha,
                to_alpha,
                ..
            } => from_alpha + (to_alpha - from_alpha) * t,
            Mask::Uniform(v) => v.clamp(0.0, 1.0),
        }
    }

    pub fn span_mask(&self) -> SpanMask {
        match *self {
            Mask::LinearFade {
                from_alpha,
                to_alpha,
                angle_rad,
            } => SpanMask::Linear {
                from: from_alpha,
                to: to_alpha,
                angle_rad,
            },
            Mask::Uniform(v) => SpanMask::Uniform(v),
        }
    }
}

pub const VERTICAL: f32 = std::f32::consts::FRAC_PI_2;

pub fn gradient_mask<G>(gradient: G) -> Mask
where
    G: GradientAlphas,
{
    let (a, b) = gradient.endpoint_alphas();
    let (dx, dy) = gradient.direction();
    let angle = if dx == 0.0 && dy == 0.0 {
        VERTICAL
    } else {
        dy.atan2(dx)
    };
    Mask::LinearFade {
        from_alpha: a.clamp(0.0, 1.0),
        to_alpha: b.clamp(0.0, 1.0),
        angle_rad: angle,
    }
}

pub trait GradientAlphas {
    fn endpoint_alphas(&self) -> (f32, f32);

    fn direction(&self) -> (f32, f32) {
        (0.0, 1.0)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Filter {
    pub blur: f32,
    pub mask: Option<Mask>,
}

impl Filter {
    pub const NONE: Self = Self {
        blur: 0.0,
        mask: None,
    };

    pub const fn blur(radius: f32) -> Self {
        Self {
            blur: if radius > 0.0 { radius } else { 0.0 },
            mask: None,
        }
    }

    pub fn blur_radius(mut self, radius: f32) -> Self {
        self.blur = radius.max(0.0);
        self
    }

    pub fn mask(mut self, mask: Mask) -> Self {
        self.mask = Some(mask);
        self
    }

    pub fn is_empty(&self) -> bool {
        !(self.blur > 0.0 && self.blur.is_finite())
    }

    pub fn blur_px(&self) -> f32 {
        if self.blur.is_finite() {
            self.blur.max(0.0)
        } else {
            0.0
        }
    }

    pub fn span_mask(&self) -> SpanMask {
        self.mask.map(|m| m.span_mask()).unwrap_or(SpanMask::None)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum Effect {
    Blur(Filter),
    Shadow(crate::ShadowSpec),
    Backdrop(Backdrop),
}

impl Effect {
    pub fn is_empty(&self) -> bool {
        match self {
            Effect::Blur(b) => b.is_empty(),
            Effect::Shadow(s) => !s.is_visible(),
            Effect::Backdrop(b) => b.is_empty(),
        }
    }
}

pub fn blur(radius: f32) -> Filter {
    Filter::blur(radius.max(0.0))
}

pub fn gradient(from_alpha: f32, to_alpha: f32) -> Mask {
    gradient_angled(from_alpha, to_alpha, VERTICAL)
}

pub fn gradient_angled(from_alpha: f32, to_alpha: f32, angle_rad: f32) -> Mask {
    Mask::LinearFade {
        from_alpha: from_alpha.clamp(0.0, 1.0),
        to_alpha: to_alpha.clamp(0.0, 1.0),
        angle_rad,
    }
}

pub fn backdrop_blur(radius: f32) -> Backdrop {
    Backdrop::blur(radius)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sequoia_core::Color;

    #[test]
    fn filter_vs_backdrop_semantics() {
        assert!(Filter::NONE.is_empty());
        assert!(!Filter::blur(12.0).is_empty());
        assert!(Backdrop::NONE.is_empty());
        assert!(!Backdrop::blur(4.0).is_empty());

        assert!(Backdrop::NONE.radius(12.0).is_empty());
    }

    #[test]
    fn blur_mask_builder() {
        let b = blur(30.0).mask(gradient(1.0, 0.0));
        assert_eq!(b.blur, 30.0);
        assert!(b.mask.is_some());
        let m = b.mask.as_ref().unwrap();
        assert_eq!(m.strength_at(0.0), 1.0, "the mask holds the ramp");
        assert_eq!(m.strength_at(1.0), 0.0);
        assert_eq!(
            m.span_mask(),
            SpanMask::Linear {
                from: 1.0,
                to: 0.0,
                angle_rad: VERTICAL,
            },
            "pionowa rampa do GPU"
        );
    }

    #[test]
    fn angled_mask_encodes_direction() {
        let m = gradient_angled(0.0, 1.0, 0.0);
        assert_eq!(m.span_mask().mask_vec(), [1.0, 0.0, 1.0, 0.0]);
        assert_eq!(Filter::NONE.span_mask(), SpanMask::None);
        assert_eq!(
            Backdrop::blur(4.0).span_mask(),
            SpanMask::None,
            "no mask, full strength"
        );
    }

    #[test]
    fn backdrop_builder_with_mask() {
        let f = Backdrop::blur(16.0)
            .dim(Color::BLACK)
            .noise(0.06)
            .radius(12.0)
            .mask(gradient(1.0, 0.0));
        assert_eq!(f.blur, 16.0);
        assert_eq!(f.radius, 12.0);
        assert!(!f.is_empty());
        assert!(f.mask.is_some());
        assert!(f.span_mask() != SpanMask::None);
    }
}
