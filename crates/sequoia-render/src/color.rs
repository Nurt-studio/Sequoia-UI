//! Color-space helpers shared by pipelines, textures, and user renderers.

/// Color space implied by a texture or surface format.
///
/// sRGB formats perform linear-to-sRGB encoding in hardware and return linear
/// values when sampled. Non-sRGB formats store shader output as-is and are
/// normally treated as linear.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorSpace {
    Srgb,
    Linear,
}

impl ColorSpace {
    /// Returns the color space implied by `format`.
    pub fn of(format: wgpu::TextureFormat) -> Self {
        if format.is_srgb() {
            Self::Srgb
        } else {
            Self::Linear
        }
    }

    /// Returns whether this space needs an explicit encode pass.
    pub fn is_srgb(self) -> bool {
        matches!(self, Self::Srgb)
    }
}

/// Decodes one sRGB-encoded channel to linear light.
pub fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.040_45 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// Encodes one linear channel to sRGB.
pub fn linear_to_srgb(c: f32) -> f32 {
    if c <= 0.003_130_8 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

/// Decodes an RGB triplet from sRGB to linear.
pub fn srgb_to_linear_rgb(rgb: [f32; 3]) -> [f32; 3] {
    [
        srgb_to_linear(rgb[0]),
        srgb_to_linear(rgb[1]),
        srgb_to_linear(rgb[2]),
    ]
}

/// Encodes an RGB triplet from linear to sRGB.
pub fn linear_to_srgb_rgb(rgb: [f32; 3]) -> [f32; 3] {
    [
        linear_to_srgb(rgb[0]),
        linear_to_srgb(rgb[1]),
        linear_to_srgb(rgb[2]),
    ]
}
