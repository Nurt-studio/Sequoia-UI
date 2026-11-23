//! Embedded WGSL shader shared by all render pipelines.
//!
//! Each pipeline selects a vertex/fragment entry point from this module, so the
//! geometry helpers and effect code stay identical across draw paths.

use sequoia_shader::TextureKind;

/// WGSL source implementing quads, glyphs, blits, backdrops, and gradients.
///
/// The source lives in `sequoia-shader-wgsl` so the experimental ash backend
/// can compile the exact same module to SPIR-V.
pub(crate) const SHADER: &str = sequoia_shader::WGSL_RENDER;

/// WGSL declarations shared by every user shader module.
///
/// This is [`sequoia_shader::WGSL_PRELUDE`] plus the uniform block, so a
/// hand-written module and the `shader!` macro describe exactly one ABI. The
/// previous copy had a different `ShaderVarying` (no `rect_size`), which meant
/// hand-written shaders could not compile against the same ABI version.
pub(crate) const USER_SHADER_PRELUDE: &str = sequoia_shader::WGSL_PRELUDE;

/// Uniform block bound at `@group(0) @binding(1)` for every user shader.
const USER_SHADER_UNIFORMS: &str = r#"
struct ShaderUniforms {
    values: array<vec4<f32>, 16>,
};
@group(0) @binding(1) var<uniform> uniforms: ShaderUniforms;
"#;

/// WGSL wrapper that turns `fs_user` into a premultiplied fragment stage.
pub(crate) const USER_SHADER_EPILOGUE: &str = sequoia_shader::WGSL_EPILOGUE;

/// Detects the dimension of each texture group declared by a shader module.
///
/// The renderer needs this to pick `texture_2d` or `texture_3d` bind group
/// layouts. Groups are indexed by their `@group(n)` number minus one, so the
/// result lines up with the `TEXTURES` slot order.
#[must_use]
pub(crate) fn texture_kinds(source: &str) -> Vec<TextureKind> {
    let mut kinds: Vec<TextureKind> = Vec::new();
    for line in source.lines() {
        let Some(start) = line.find("@group(") else {
            continue;
        };
        let rest = &line[start + 7..];
        let Some(end) = rest.find(')') else {
            continue;
        };
        let Ok(group) = rest[..end].trim().parse::<usize>() else {
            continue;
        };
        if group == 0 {
            continue;
        }
        let is_3d = line.contains("texture_3d");
        let is_2d = line.contains("texture_2d");
        if !is_3d && !is_2d {
            continue;
        }
        if kinds.len() < group {
            kinds.resize(group, TextureKind::D2);
        }
        kinds[group - 1] = if is_3d {
            TextureKind::D3
        } else {
            TextureKind::D2
        };
    }
    kinds
}

/// Composes a user fragment function with the engine vertex and uniform ABI.
pub fn compose_user_shader(source: &str) -> String {
    format!("{USER_SHADER_PRELUDE}\n{USER_SHADER_UNIFORMS}\n{source}\n{USER_SHADER_EPILOGUE}")
}

/// Checks the one bind group entry every user shader module must declare.
///
/// Registration used to accept a module whose group 0 differed from the
/// engine's ABI; the mismatch only surfaced later as a validation error while
/// building a pipeline, with no draw and no link back to the shader.
///
/// Only `@group(0) @binding(0)` is required, because the prelude's `vs_main`
/// always reads the screen size from it. Binding 1 and its shape belong to the
/// shader: `shader!` declares a typed `SequoiaUniforms` struct, while uniform-free
/// modules such as the built-in mask shader declare no block at all. Requiring
/// the renderer's canonical `array<vec4<f32>, 16>` here rejected every typed
/// shader at registration, and the draw was then dropped for lack of an id.
///
/// # Errors
///
/// Returns [`crate::RenderError::ShaderCompile`] naming the missing binding.
pub fn validate_shader_abi(source: &str) -> Result<(), crate::RenderError> {
    let compact: String = source.chars().filter(|c| !c.is_whitespace()).collect();
    if !compact.contains("@group(0)@binding(0)") {
        return Err(crate::RenderError::ShaderCompile(
            "shader ABI: missing @group(0) @binding(0) `Screen` uniform".into(),
        ));
    }
    Ok(())
}

/// Parses and validates a complete user shader module.
///
/// The source must be self-contained WGSL with a `vs_main` vertex entry point
/// and an `fs_main` fragment entry point matching the engine ABI. Use
/// [`compose_user_shader`] to wrap a bare `fs_user` function with that ABI.
///
/// # Errors
///
/// Returns [`crate::RenderError::ShaderCompile`] with the naga diagnostic when
/// parsing or validation fails, including the source line and column.
pub fn validate_user_shader(source: &str) -> Result<(), crate::RenderError> {
    let module = wgpu::naga::front::wgsl::parse_str(source)
        .map_err(|e| crate::RenderError::ShaderCompile(e.emit_to_string(source)))?;
    let mut validator = wgpu::naga::valid::Validator::new(
        wgpu::naga::valid::ValidationFlags::all(),
        wgpu::naga::valid::Capabilities::all(),
    );
    validator.validate(&module).map_err(|e| {
        let message = match e.location(source) {
            Some(location) => format!(
                "{} at line {}, column {}",
                e, location.line_number, location.line_position
            ),
            None => e.to_string(),
        };
        crate::RenderError::ShaderCompile(message)
    })?;

    let has_vertex = module
        .entry_points
        .iter()
        .any(|entry| entry.name == "vs_main" && entry.stage == wgpu::naga::ShaderStage::Vertex);
    let has_fragment = module
        .entry_points
        .iter()
        .any(|entry| entry.name == "fs_main" && entry.stage == wgpu::naga::ShaderStage::Fragment);
    if !has_vertex {
        return Err(crate::RenderError::ShaderCompile(
            "user shader is missing the `vs_main` vertex entry point".into(),
        ));
    }
    if !has_fragment {
        return Err(crate::RenderError::ShaderCompile(
            "user shader is missing the `fs_main` fragment entry point".into(),
        ));
    }
    Ok(())
}

/// Returns the built-in passthrough shader as a complete WGSL module.
///
/// The fragment stage returns `uniforms.values[0]`, so the first 16 uniform
/// bytes are the straight RGBA color of the drawn rectangle.
#[must_use]
pub fn passthrough_wgsl() -> String {
    compose_user_shader(PASSTHROUGH_FS)
}

pub(crate) const PASSTHROUGH_FS: &str =
    "fn fs_user(frag: ShaderVarying) -> vec4<f32> { return uniforms.values[0]; }";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_typed_and_uniformless_group_zero_modules() {
        let typed = format!(
            "{}\nstruct SequoiaUniforms {{\n    amount: f32,\n}}\n\
             @group(0) @binding(1) var<uniform> sequoia_u: SequoiaUniforms;\n\
             fn fs_user(frag: ShaderVarying) -> vec4<f32> {{ return vec4<f32>(sequoia_u.amount); }}\n{}",
            sequoia_shader::WGSL_PRELUDE,
            sequoia_shader::WGSL_EPILOGUE
        );
        assert!(
            validate_shader_abi(&typed).is_ok(),
            "the `shader!` macro declares its own uniform struct, not array<vec4<f32>, 16>"
        );
        let uniformless = format!(
            "{}\nfn fs_user(frag: ShaderVarying) -> vec4<f32> {{ return vec4<f32>(1.0); }}\n{}",
            sequoia_shader::WGSL_PRELUDE,
            sequoia_shader::WGSL_EPILOGUE
        );
        assert!(
            validate_shader_abi(&uniformless).is_ok(),
            "a shader that uses no uniforms needs no binding 1"
        );
    }

    #[test]
    fn rejects_a_module_without_the_screen_binding() {
        assert!(validate_shader_abi("fn fs_main() {}").is_err());
    }

    #[test]
    fn detects_texture_dimensions_per_group() {
        let source = "@group(0) @binding(0) var<uniform> u: U;\n\
            @group(1) @binding(0) var photo: texture_2d<f32>;\n\
            @group(1) @binding(1) var photo_sampler: sampler;\n\
            @group(2) @binding(0) var lut: texture_3d<f32>;\n\
            @group(2) @binding(1) var lut_sampler: sampler;";
        assert_eq!(
            texture_kinds(source),
            vec![TextureKind::D2, TextureKind::D3]
        );
        assert!(texture_kinds("fn f() {}").is_empty());
    }
}
