@fragment
fn fs_main(frag: ShaderVarying) -> @location(0) vec4<f32> {
    let clip = sequoia_clip_cover(frag.world, frag.clip_rect, frag.clip_radius);
    if (clip <= 0.001) {
        discard;
    }
    let color = fs_user(frag);
    let alpha = clamp(color.a * frag.opacity * clip, 0.0, 1.0);
    return vec4<f32>(color.rgb * alpha, alpha);
}
