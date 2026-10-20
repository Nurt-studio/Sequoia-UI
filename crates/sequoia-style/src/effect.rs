#![forbid(unsafe_code)]

use sequoia_core::{Color, SpanMask};

use crate::filter::Mask;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Backdrop {
    pub blur: f32,

    pub dim: Color,

    pub noise: f32,

    pub radius: f32,

    pub mask: Option<Mask>,
}

impl Backdrop {
    pub const NONE: Self = Self {
        blur: 0.0,
        dim: Color::TRANSPARENT,
        noise: 0.0,
        radius: 0.0,
        mask: None,
    };

    pub const XS: Self = Self::blur(2.0);
    pub const SM: Self = Self::blur(3.0);
    pub const MD: Self = Self::blur(4.0);

    pub const fn blur(radius: f32) -> Self {
        Self {
            blur: if radius > 0.0 { radius } else { 0.0 },
            ..Self::NONE
        }
    }

    pub fn blur_radius(mut self, radius: f32) -> Self {
        self.blur = radius.max(0.0);
        self
    }

    pub const fn dim(mut self, color: Color) -> Self {
        self.dim = color;
        self
    }

    pub const fn noise(mut self, amount: f32) -> Self {
        self.noise = if amount < 0.0 {
            0.0
        } else if amount > 1.0 {
            1.0
        } else {
            amount
        };
        self
    }

    pub const fn radius(mut self, radius: f32) -> Self {
        self.radius = if radius > 0.0 { radius } else { 0.0 };
        self
    }

    pub fn mask(mut self, mask: Mask) -> Self {
        self.mask = Some(mask);
        self
    }

    pub fn is_empty(self) -> bool {
        !(self.blur > 0.0 && self.blur.is_finite() || self.dim.a > 0.0 || self.noise > 0.0)
    }

    pub fn blur_px(self) -> f32 {
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

impl Default for Backdrop {
    fn default() -> Self {
        Self::NONE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn none_is_empty() {
        assert!(Backdrop::NONE.is_empty());
        assert!(Backdrop::blur(0.0).is_empty());
        assert!(Backdrop::blur(f32::NAN).is_empty());
    }

    #[test]
    fn blur_dim_noise_are_visible() {
        assert!(!Backdrop::blur(16.0).is_empty());
        assert!(!Backdrop::NONE.dim(Color::BLACK.with_alpha(0.25)).is_empty());
        assert!(!Backdrop::NONE.noise(0.06).is_empty());
    }

    #[test]
    fn builder_chain() {
        let b = Backdrop::blur(16.0)
            .dim(Color::BLACK.with_alpha(0.25))
            .noise(0.06)
            .radius(12.0);
        assert_eq!(b.blur, 16.0);
        assert_eq!(b.radius, 12.0);
        assert!((b.noise - 0.06).abs() < 1e-6);

        assert!(Backdrop::NONE.radius(12.0).is_empty());

        assert_eq!(Backdrop::blur(-4.0).blur, 0.0);
    }
}
