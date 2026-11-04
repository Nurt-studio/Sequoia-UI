struct ShaderUniforms {
    values: array<vec4<f32>, 16>,
};
@group(0) @binding(1) var<uniform> uniforms: ShaderUniforms;
