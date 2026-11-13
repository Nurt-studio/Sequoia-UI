//! Runtime types and compile-time WGSL validation for Sequoia custom shaders.
//!
//! The [`macro@shader`] macro turns a WGSL fragment body into a typed Rust
//! struct implementing [`SequoiShader`]. The macro validates the generated
//! module with naga at compile time, so invalid WGSL fails the build with a
//! source location instead of producing a silently missing draw call.
//!
//! # Thin macro
//!
//! Shaders are written as `shader! { fn name(...) -> Color { /* WGSL */ } }`
//! rather than as an attribute on a Rust function. The body is WGSL, copied
//! verbatim into `fs_user`; there is deliberately no Rust-to-WGSL translation,
//! type inference, or expression walker. Parameters still drive the Rust struct
//! and uniform layout, so call sites stay typed, while the shader source avoids
//! a second, subtly different language. WGSL is checked by naga at compile time
//! and again by the renderer when shaders are registered.
//!
//! ```ignore
//! sequoia_shader::shader! {
//!     pub fn tint(color: Color, amount: f32) -> Color {
//!         return vec4<f32>(color.rgb * amount, color.a);
//!     }
//! }
//!
//! painter.shader(Tint { color: Color::RED, amount: 0.5 }, rect, &[]);
//! ```

#![forbid(unsafe_code)]

pub use bytemuck;
pub use sequoia_shader_macros::shader;

mod builtin;
mod math;
mod registry;
mod sampler;
pub mod stdlib;
mod traits;
mod wgsl;

pub use builtin::{ApplyMask, BUILTINS};
pub use math::{Color, Vec2, Vec3, Vec4};
pub use registry::ShaderRegistry;
pub use sampler::{Sampler2D, Sampler3D};
pub use traits::{
    FragLocal, FragPos, FragUv, FragWorld, MAX_TEXTURES, SequoiShader, TEXTURE_GROUP_BASE,
    TextureKind, VertexIndex,
};
pub use wgsl::{WGSL_ARRAY_UNIFORMS, WGSL_EPILOGUE, WGSL_PRELUDE, WGSL_STD};

#[cfg(test)]
mod golden_tests;

#[cfg(test)]
mod tests {
    use super::*;

    crate::shader! {
        fn solid(color: Color, gain: f32, invert: bool) -> Color {
            return vec4<f32>(color.rgb * gain, select(color.a, 1.0 - color.a, invert));
        }
    }

    crate::shader! {
        fn textured(uv: FragUv, tint: Color, tex: Sampler2D) -> Color {
            let sample = textureSample(tex, tex_sampler, uv);
            return vec4<f32>(sample.rgb * tint.rgb, sample.a);
        }
    }

    crate::shader! {
        fn noise_field(uv: FragUv, scale: f32) -> Color {
            let n = sq_fbm21(uv * scale, 4);
            return vec4<f32>(n, n, n, 1.0);
        }
    }

    crate::shader! {
        fn constant() -> Color {
            return vec4<f32>(1.0, 0.5, 0.25, 1.0);
        }
    }

    crate::shader! {
        fn lut3d_sample(uv: FragUv, lut: Sampler3D) -> Color {
            return textureSample(lut, lut_sampler, vec3<f32>(uv.x, uv.y, 0.5));
        }
    }

    crate::shader! {
        fn array_shader(weights: [f32; 4], vectors: [Vec4; 2]) -> Color {
            return vec4<f32>(weights[0], vectors[0].w, 0.0, 1.0);
        }
    }

    #[test]
    fn generates_typed_uniforms() {
        let shader = Solid {
            color: Color::rgba(0.1, 0.2, 0.3, 0.4),
            gain: 2.0,
            invert: true,
        };
        let mut bytes = Vec::new();
        shader.encode_uniforms(&mut bytes);
        assert_eq!(Solid::UNIFORM_SIZE, 32);
        assert_eq!(bytes.len(), 32);
        assert_eq!(f32::from_le_bytes(bytes[0..4].try_into().unwrap()), 0.1);
        assert_eq!(f32::from_le_bytes(bytes[12..16].try_into().unwrap()), 0.4);
        assert_eq!(f32::from_le_bytes(bytes[16..20].try_into().unwrap()), 2.0);
        assert_eq!(u32::from_le_bytes(bytes[20..24].try_into().unwrap()), 1);
        assert!(Solid::WGSL.contains("vec4<f32>"));
    }

    #[test]
    fn declares_texture_groups_and_names() {
        assert_eq!(Textured::TEXTURES, &["tex"]);
        assert!(Textured::WGSL.contains("@group(1) @binding(0) var tex"));
        assert!(Textured::WGSL.contains("var tex_sampler: sampler"));
        assert!(Textured::WGSL.contains("textureSample(tex, tex_sampler, uv)"));
    }

    #[test]
    fn includes_stdlib_helpers() {
        assert!(NoiseField::WGSL.contains("fn sq_fbm21"));
        assert!(NoiseField::WGSL.contains("sq_fbm21(uv * scale, 4)"));
    }

    #[test]
    fn supports_shaders_without_uniforms() {
        let mut bytes = Vec::new();
        Constant.encode_uniforms(&mut bytes);
        assert_eq!(Constant::UNIFORM_SIZE, 16);
        assert_eq!(bytes.len(), 16);
        assert_eq!(Constant::TEXTURES, &[] as &[&str]);
    }

    #[test]
    fn supports_sampler_3d_and_arrays() {
        assert_eq!(Lut3dSample::TEXTURES, &["lut"]);
        assert_eq!(Lut3dSample::TEXTURE_KINDS, &[TextureKind::D3]);
        assert!(Lut3dSample::WGSL.contains("@group(1) @binding(0) var lut: texture_3d<f32>"));
        assert_eq!(Textured::TEXTURE_KINDS, &[TextureKind::D2]);

        let shader = ArrayShader {
            weights: [1.0, 2.0, 3.0, 4.0],
            vectors: [Vec4::splat(1.0), Vec4::splat(2.0)],
        };
        let mut bytes = Vec::new();
        shader.encode_uniforms(&mut bytes);
        assert_eq!(ArrayShader::UNIFORM_SIZE, 48);
        assert_eq!(bytes.len(), 48);
        // f32 arrays are packed contiguously and rounded up to 16 B.
        assert_eq!(f32::from_le_bytes(bytes[0..4].try_into().unwrap()), 1.0);
        assert_eq!(f32::from_le_bytes(bytes[12..16].try_into().unwrap()), 4.0);
        assert_eq!(f32::from_le_bytes(bytes[16..20].try_into().unwrap()), 1.0);
        assert_eq!(f32::from_le_bytes(bytes[44..48].try_into().unwrap()), 2.0);
        assert!(ArrayShader::WGSL.contains("var weights: array<f32, 4>;"));
        assert!(ArrayShader::WGSL.contains("array<vec4<f32>, 2>"));
    }
}
