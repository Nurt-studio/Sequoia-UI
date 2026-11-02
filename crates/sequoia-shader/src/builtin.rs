//! Shader modules implemented by the engine rather than by the `shader!` macro.
//!
//! These are written by hand so the compositing behaviour is explicit and does
//! not depend on codegen. They use the same vertex/fragment ABI as generated
//! shaders, so the renderer treats them like any other registered shader.

use crate::SequoiShader;

/// Keeps `layer` and multiplies its alpha by the alpha of `mask`.
///
/// Used by `Canvas::with_mask`. Both inputs are offscreen targets: `mask` is
/// bound at group 1 and `layer` at group 2, in that order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ApplyMask;

impl SequoiShader for ApplyMask {
    const NAME: &'static str = "apply_mask";
    const WGSL: &'static str = APPLY_MASK_WGSL;
    const TEXTURES: &'static [&'static str] = &["mask", "layer"];
    const UNIFORM_SIZE: usize = 16;

    fn encode_uniforms(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&[0u8; Self::UNIFORM_SIZE]);
    }
}

const APPLY_MASK_WGSL: &str = concat!(
    include_str!("wgsl/prelude.wgsl"),
    r#"
@group(1) @binding(0) var mask: texture_2d<f32>;
@group(1) @binding(1) var mask_sampler: sampler;
@group(2) @binding(0) var layer: texture_2d<f32>;
@group(2) @binding(1) var layer_sampler: sampler;

fn fs_user(frag: ShaderVarying) -> vec4<f32> {
    let m = textureSample(mask, mask_sampler, frag.uv).a;
    let l = textureSample(layer, layer_sampler, frag.uv);
    return vec4<f32>(l.rgb, l.a * m);
}
"#,
    include_str!("wgsl/epilogue.wgsl"),
);

/// Shaders registered by the engine on every app, as `(name, wgsl)` pairs.
pub const BUILTINS: &[(&str, &str)] = &[(ApplyMask::NAME, ApplyMask::WGSL)];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_mask_declares_both_targets() {
        assert!(ApplyMask::WGSL.contains("fn vs_main"));
        assert!(ApplyMask::WGSL.contains("@group(1) @binding(0) var mask"));
        assert!(ApplyMask::WGSL.contains("@group(2) @binding(0) var layer"));
        assert!(ApplyMask::WGSL.contains("fs_user"));
        assert_eq!(ApplyMask::TEXTURES, &["mask", "layer"]);
    }

    #[test]
    fn apply_mask_uniforms_are_zeroed() {
        let mut bytes = Vec::new();
        ApplyMask.encode_uniforms(&mut bytes);
        assert_eq!(bytes, vec![0u8; 16]);
        assert_eq!(BUILTINS.len(), 1);
    }
}
