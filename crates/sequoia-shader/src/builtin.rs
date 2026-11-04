//! Shader modules implemented by the engine rather than by the `shader!` macro.
//!
//! These are written by hand so the compositing behaviour is explicit and does
//! not depend on codegen. They use the same vertex/fragment ABI as generated
//! shaders, so the renderer treats them like any other registered shader.

use crate::SequoiShader;
use crate::wgsl::WGSL_APPLY_MASK;

/// Keeps `layer` and multiplies its alpha by the alpha of `mask`.
///
/// Used by `Canvas::with_mask`. Both inputs are offscreen targets: `mask` is
/// bound at group 1 and `layer` at group 2, in that order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ApplyMask;

impl SequoiShader for ApplyMask {
    const NAME: &'static str = "apply_mask";
    const WGSL: &'static str = WGSL_APPLY_MASK;
    const TEXTURES: &'static [&'static str] = &["mask", "layer"];
    const UNIFORM_SIZE: usize = 16;

    fn encode_uniforms(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&[0u8; Self::UNIFORM_SIZE]);
    }
}

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
