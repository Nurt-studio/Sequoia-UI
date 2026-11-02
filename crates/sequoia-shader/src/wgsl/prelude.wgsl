struct Screen {
    size: vec4<f32>,
};
@group(0) @binding(0) var<uniform> screen: Screen;

struct ShaderVarying {
    @builtin(position) position: vec4<f32>,
    @location(0) local: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) world: vec2<f32>,
    @location(3) clip_rect: vec4<f32>,
    @location(4) clip_radius: f32,
    @location(5) opacity: f32,
    // Size of the draw rect in physical pixels, so shaders can build
    // rounded-rect shapes without matching uniforms against DPI scale.
    @location(6) rect_size: vec2<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) vi: u32,
    @location(0) rect: vec4<f32>,
    @location(1) uv: vec4<f32>,
    @location(2) clip_rect: vec4<f32>,
    @location(3) clip_radius: f32,
    @location(4) opacity: f32,
    @location(5) transform_a: vec4<f32>,
    @location(6) transform_b: vec2<f32>,
    @location(7) shader_id: u32,
    @location(8) uniform_offset: u32,
) -> ShaderVarying {
    let c = vec2<f32>(f32(vi & 1u), f32(vi >> 1u));
    var world = rect.xy + c * rect.zw;
    world = vec2<f32>(
        transform_a.x * world.x + transform_a.z * world.y + transform_b.x,
        transform_a.y * world.x + transform_a.w * world.y + transform_b.y,
    );
    var out: ShaderVarying;
    out.position = vec4<f32>(
        world.x / screen.size.x * 2.0 - 1.0,
        1.0 - world.y / screen.size.y * 2.0,
        0.0,
        1.0
    );
    out.local = c * rect.zw;
    out.uv = mix(uv.xy, uv.zw, c);
    out.world = world;
    out.clip_rect = clip_rect;
    out.clip_radius = clip_radius;
    out.opacity = opacity;
    out.rect_size = rect.zw;
    return out;
}

fn sequoia_clip_cover(world: vec2<f32>, clip_rect: vec4<f32>, clip_radius: f32) -> f32 {
    let half = clip_rect.zw * 0.5;
    if (half.x <= 0.0 || half.y <= 0.0) {
        return 1.0;
    }
    let center = clip_rect.xy + half;
    let p = abs(world - center);
    let rr = clamp(clip_radius, 0.0, min(half.x, half.y));
    let b = half - vec2<f32>(rr, rr);
    let d = p - b;
    let dist = length(max(d, vec2<f32>(0.0, 0.0))) + min(max(d.x, d.y), 0.0) - rr;
    let aa = max(fwidth(dist), 0.0001);
    return clamp(0.5 - dist / aa, 0.0, 1.0);
}
