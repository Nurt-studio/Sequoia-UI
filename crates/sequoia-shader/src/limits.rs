//! Shader limits shared by the macro, runtime validation, and adapters.
//!
//! The numbers live in `sequoia-shader-wgsl`, because the macro crate and the
//! runtime both need them and neither may depend on the other. This module
//! wraps them in a value type so the renderer can report the limits actually in
//! force on a given adapter instead of repeating the constants.

use sequoia_shader_wgsl::{ABI_VERSION, MAX_UNIFORM_BYTES};

/// Upper bounds every registered shader is checked against.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShaderLimits {
    /// Version of the vertex/fragment ABI the shader was authored against.
    pub abi_version: u32,
    /// Maximum size of the payload written by
    /// [`SequoiShader::encode_uniforms`](crate::SequoiShader::encode_uniforms).
    pub max_uniform_bytes: usize,
    /// Maximum number of entries in
    /// [`SequoiShader::TEXTURES`](crate::SequoiShader::TEXTURES).
    pub max_textures: usize,
}

impl ShaderLimits {
    /// Limits baked into the `shader!` macro and the engine bind group layouts.
    pub const ENGINE: Self = Self {
        abi_version: ABI_VERSION,
        max_uniform_bytes: MAX_UNIFORM_BYTES,
        max_textures: crate::MAX_TEXTURES,
    };

    /// Checks a uniform payload size, returning the message shown to the user.
    ///
    /// # Errors
    ///
    /// Returns a description when `bytes` exceeds
    /// [`ShaderLimits::max_uniform_bytes`].
    pub fn check_uniforms(&self, bytes: usize) -> Result<(), String> {
        if bytes > self.max_uniform_bytes {
            return Err(format!(
                "shader uniforms use {bytes} B, the limit is {} B",
                self.max_uniform_bytes
            ));
        }
        Ok(())
    }

    /// Checks a texture count, returning the message shown to the user.
    ///
    /// # Errors
    ///
    /// Returns a description when `count` exceeds
    /// [`ShaderLimits::max_textures`].
    pub fn check_textures(&self, count: usize) -> Result<(), String> {
        if count > self.max_textures {
            return Err(format!(
                "a shader may declare at most {} textures, got {count}",
                self.max_textures
            ));
        }
        Ok(())
    }

    /// Narrows these limits to what an adapter can bind.
    ///
    /// Device limits can only lower the engine limits; an adapter that reports
    /// more than the engine allows still gets the engine bound.
    #[must_use]
    pub fn narrowed(self, max_uniform_bytes: usize, max_textures: usize) -> Self {
        Self {
            max_uniform_bytes: self.max_uniform_bytes.min(max_uniform_bytes),
            max_textures: self.max_textures.min(max_textures),
            ..self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_limits_match_the_macro() {
        assert_eq!(ShaderLimits::ENGINE.max_uniform_bytes, 256);
        assert_eq!(ShaderLimits::ENGINE.max_textures, 6);
        assert_eq!(ShaderLimits::ENGINE.abi_version, ABI_VERSION);
    }

    #[test]
    fn checks_report_the_limit() {
        let limits = ShaderLimits::ENGINE;
        assert!(limits.check_uniforms(256).is_ok());
        assert!(limits.check_uniforms(0).is_ok());
        assert!(limits.check_uniforms(257).is_err());
        assert!(limits.check_textures(6).is_ok());
        assert!(limits.check_textures(7).is_err());
        assert!(
            limits
                .check_textures(7)
                .unwrap_err()
                .contains("at most 6 textures")
        );
    }

    #[test]
    fn narrowing_only_lowers_limits() {
        let limits = ShaderLimits::ENGINE;
        let wide = limits.narrowed(1024, 16);
        assert_eq!(wide, limits);
        let narrow = limits.narrowed(64, 2);
        assert_eq!(narrow.max_uniform_bytes, 64);
        assert_eq!(narrow.max_textures, 2);
        assert_eq!(narrow.abi_version, limits.abi_version);
    }
}
