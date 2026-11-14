#![forbid(unsafe_code)]

/// Version of the vertex/fragment ABI defined by the WGSL sources in this crate.
///
/// Bump it whenever the vertex inputs, the [`WGSL_PRELUDE`] `ShaderVarying`
/// members, or the uniform block layout change: the sources here and the
/// renderer's bind group layouts must be updated together, and shaders authored
/// against an older ABI would otherwise silently read the wrong inputs.
pub const ABI_VERSION: u32 = 1;

/// Maximum size of one shader's uniform payload, in bytes.
pub const MAX_UNIFORM_BYTES: usize = 256;

/// Maximum number of texture inputs a shader may declare.
pub const MAX_TEXTURES: usize = 6;

pub const WGSL_PRELUDE: &str = include_str!("../wgsl/prelude.wgsl");
pub const WGSL_ARRAY_UNIFORMS: &str = include_str!("../wgsl/array_uniforms.wgsl");
pub const WGSL_EPILOGUE: &str = include_str!("../wgsl/epilogue.wgsl");
pub const WGSL_STD: &str = include_str!("../wgsl/std.wgsl");

pub const WGSL_APPLY_MASK: &str = concat!(
    include_str!("../wgsl/prelude.wgsl"),
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
    include_str!("../wgsl/epilogue.wgsl"),
);
