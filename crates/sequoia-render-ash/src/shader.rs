use crate::AshError;

/// Compiles the renderer's shared WGSL module to SPIR-V.
///
/// The module is emitted once with every entry point, matching the way the
/// wgpu backend links one WGSL module into several pipelines.
pub(crate) fn compile_render_module() -> Result<Vec<u32>, AshError> {
    let module = naga::front::wgsl::parse_str(sequoia_shader::WGSL_RENDER)
        .map_err(|error| AshError::Shader(format!("WGSL parse failed: {error}")))?;

    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    );
    let info = validator
        .validate(&module)
        .map_err(|error| AshError::Shader(format!("WGSL validation failed: {error}")))?;

    let options = naga::back::spv::Options {
        fake_missing_bindings: true,
        ..Default::default()
    };

    naga::back::spv::write_vec(&module, &info, &options, None)
        .map_err(|error| AshError::Shader(format!("SPIR-V generation failed: {error}")))
}
