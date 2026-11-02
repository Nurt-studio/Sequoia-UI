/// Engine vertex/fragment ABI declarations shared by every shader module.
pub const WGSL_PRELUDE: &str = include_str!("wgsl/prelude.wgsl");

/// Uniform declaration used by raw Stage 1 shaders (`uniforms.values`).
pub const WGSL_ARRAY_UNIFORMS: &str = include_str!("wgsl/array_uniforms.wgsl");

/// Premultiplied-alpha fragment wrapper calling `fs_user`.
pub const WGSL_EPILOGUE: &str = include_str!("wgsl/epilogue.wgsl");

/// Reusable WGSL helper functions (`sq_*`).
pub const WGSL_STD: &str = include_str!("wgsl/std.wgsl");
