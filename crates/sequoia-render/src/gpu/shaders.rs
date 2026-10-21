//! Embedded WGSL shader shared by all render pipelines.
//!
//! Each pipeline selects a vertex/fragment entry point from this module, so the
//! geometry helpers and effect code stay identical across draw paths.

/// WGSL source implementing quads, glyphs, blits, backdrops, and gradients.
pub(crate) const SHADER: &str = r#"
// Screen size in physical pixels.
struct Screen {
    size: vec4<f32>,
};
@group(0) @binding(0) var<uniform> screen: Screen;

struct VsQuad {
    @builtin(position) pos: vec4<f32>,
    @location(0) local: vec2<f32>,
    @location(1) half_size: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) radius: f32,
    @location(4) border_color: vec4<f32>,
    @location(5) border_width: f32,
    @location(6) world: vec2<f32>,
    @location(7) clip_rect: vec4<f32>,
    @location(8) clip_radius: f32,
    @location(9) softness: f32,
    @location(10) spread: f32,
    @location(11) shadow_offset: vec2<f32>,
    @location(12) inset: f32,
};

@vertex
fn vs_quad(
    @builtin(vertex_index) vi: u32,
    @location(0) rect: vec4<f32>,
    @location(1) color: vec4<f32>,
    @location(2) radius: f32,
    @location(3) border_color: vec4<f32>,
    @location(4) border_width: f32,
    @location(5) clip_rect: vec4<f32>,
    @location(6) clip_radius: f32,
    @location(7) softness: f32,
    @location(8) spread: f32,
    @location(9) offset: vec2<f32>,
    @location(10) inset: f32,
    @location(11) transform_a: vec4<f32>,
    @location(12) transform_b: vec2<f32>,
) -> VsQuad {
    let c = vec2<f32>(f32(vi & 1u), f32(vi >> 1u));
    var origin = rect.xy;
    var size = rect.zw;
    if (softness > 0.0 && inset < 0.5) {
        let m = spread + softness;
        origin = rect.xy - vec2<f32>(m, m) + offset;
        size = rect.zw + vec2<f32>(2.0 * m, 2.0 * m);
    }
    var world = origin + c * size;
    world = vec2<f32>(
        transform_a.x * world.x + transform_a.z * world.y + transform_b.x,
        transform_a.y * world.x + transform_a.w * world.y + transform_b.y,
    );
    var out: VsQuad;
    out.pos = vec4<f32>(
        world.x / screen.size.x * 2.0 - 1.0,
        1.0 - world.y / screen.size.y * 2.0,
        0.0,
        1.0
    );
    out.local = (c - vec2<f32>(0.5, 0.5)) * size;
    out.half_size = size * 0.5;
    out.color = color;
    out.radius = radius;
    out.border_color = border_color;
    out.border_width = border_width;
    out.world = world;
    out.clip_rect = clip_rect;
    out.clip_radius = clip_radius;
    out.softness = softness;
    out.spread = spread;
    out.shadow_offset = offset;
    out.inset = inset;
    return out;
}

// Signed-distance helpers for rounded rectangles and clipping.
fn sd_rounded_box(p: vec2<f32>, half_size: vec2<f32>, r: f32) -> f32 {
    let rr = clamp(r, 0.0, min(half_size.x, half_size.y));
    let b = half_size - vec2<f32>(rr, rr);
    let d = p - b;
    return length(max(d, vec2<f32>(0.0, 0.0))) + min(max(d.x, d.y), 0.0) - rr;
}

fn rounded_box_cover(p: vec2<f32>, half_size: vec2<f32>, r: f32) -> f32 {
    let dist = sd_rounded_box(p, half_size, r);
    let aa = max(fwidth(dist), 0.0001);
    return clamp(0.5 - dist / aa, 0.0, 1.0);
}

fn rounded_clip_cover(world: vec2<f32>, clip_rect: vec4<f32>, clip_radius: f32) -> f32 {
    let half = clip_rect.zw * 0.5;
    if (half.x <= 0.0 || half.y <= 0.0) {
        return 1.0;
    }
    let center = clip_rect.xy + half;
    let p = abs(world - center);
    return rounded_box_cover(p, half, max(clip_radius, 0.0));
}

@fragment
fn fs_quad(frag: VsQuad) -> @location(0) vec4<f32> {
    let clip = rounded_clip_cover(frag.world, frag.clip_rect, frag.clip_radius);
    if (clip <= 0.001) {
        discard;
    }
    if (frag.softness > 0.0) {
        var d: f32;
        var t: f32;
        if (frag.inset > 0.5) {
            let half = frag.half_size - vec2<f32>(frag.spread, frag.spread);
            d = sd_rounded_box(abs(frag.local - frag.shadow_offset), half, frag.radius);
            let shape = rounded_box_cover(abs(frag.local), frag.half_size, frag.radius + frag.spread);
            t = clamp(0.5 + d / frag.softness, 0.0, 1.0) * shape;
        } else {
            d = sd_rounded_box(abs(frag.local), frag.half_size - vec2<f32>(frag.softness, frag.softness), frag.radius);
            t = clamp(0.5 - d / frag.softness, 0.0, 1.0);
        }
        let alpha = t * clip * frag.color.a;
        if (alpha <= 0.0) {
            discard;
        }
        return vec4<f32>(frag.color.rgb * alpha, alpha);
    }
    let p = abs(frag.local);
    let outer = rounded_box_cover(p, frag.half_size, frag.radius);
    let bw = max(frag.border_width, 0.0);
    if (bw <= 0.0 || frag.border_color.a <= 0.0) {
        let alpha = outer * clip * frag.color.a;
        return vec4<f32>(frag.color.rgb * alpha, alpha);
    }
    let min_half = min(frag.half_size.x, frag.half_size.y);
    var inner = 0.0;
    if (bw < min_half) {
        inner = rounded_box_cover(p, frag.half_size - vec2<f32>(bw, bw), frag.radius - bw);
    }
    let ring = clamp(outer - inner, 0.0, 1.0);
    let a_b = frag.border_color.a * ring;
    let a_f = frag.color.a * inner;
    var a_out = a_b + a_f * (1.0 - a_b);
    a_out = a_out * clip;
    if (a_out <= 0.0) {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }
    let rgb = (frag.border_color.rgb * a_b + frag.color.rgb * a_f * (1.0 - a_b)) / max(a_b + a_f * (1.0 - a_b), 0.0001);
    return vec4<f32>(rgb * a_out, a_out);
}

// Glyph atlas sampling.
@group(1) @binding(0) var atlas_tex: texture_2d<f32>;
@group(1) @binding(1) var atlas_samp: sampler;

struct VsGlyph {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) color_glyph: f32,
    @location(3) world: vec2<f32>,
    @location(4) clip_rect: vec4<f32>,
    @location(5) clip_radius: f32,
};

@vertex
fn vs_glyph(
    @builtin(vertex_index) vi: u32,
    @location(0) rect: vec4<f32>,
    @location(1) uv: vec4<f32>,
    @location(2) color: vec4<f32>,
    @location(3) color_glyph: f32,
    @location(4) clip_rect: vec4<f32>,
    @location(5) clip_radius: f32,
    @location(6) transform_a: vec4<f32>,
    @location(7) transform_b: vec2<f32>,
) -> VsGlyph {
    let c = vec2<f32>(f32(vi & 1u), f32(vi >> 1u));
    var world = rect.xy + c * rect.zw;
    world = vec2<f32>(
        transform_a.x * world.x + transform_a.z * world.y + transform_b.x,
        transform_a.y * world.x + transform_a.w * world.y + transform_b.y,
    );
    var out: VsGlyph;
    out.pos = vec4<f32>(
        world.x / screen.size.x * 2.0 - 1.0,
        1.0 - world.y / screen.size.y * 2.0,
        0.0,
        1.0
    );
    out.uv = mix(uv.xy, uv.zw, c);
    out.color = color;
    out.color_glyph = color_glyph;
    out.world = world;
    out.clip_rect = clip_rect;
    out.clip_radius = clip_radius;
    return out;
}

@fragment
fn fs_glyph(frag: VsGlyph) -> @location(0) vec4<f32> {
    let clip = rounded_clip_cover(frag.world, frag.clip_rect, frag.clip_radius);
    if (clip <= 0.001) {
        discard;
    }
    let t = textureSample(atlas_tex, atlas_samp, frag.uv);
    let a = t.a * frag.color.a * clip;
    if frag.color_glyph > 0.5 {
        return vec4<f32>(t.rgb * frag.color.a * clip, a);
    }
    return vec4<f32>(frag.color.rgb * a, a);
}

// Blit sampling for images and cached sub-surfaces.
@group(1) @binding(0) var blit_tex: texture_2d<f32>;
@group(1) @binding(1) var blit_samp: sampler;

struct VsBlit {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) world: vec2<f32>,
    @location(2) clip_rect: vec4<f32>,
    @location(3) clip_radius: f32,
};

@vertex
fn vs_blit(
    @builtin(vertex_index) vi: u32,
    @location(0) rect: vec4<f32>,
    @location(1) uv: vec4<f32>,
    @location(2) clip_rect: vec4<f32>,
    @location(3) clip_radius: f32,
    @location(4) transform_a: vec4<f32>,
    @location(5) transform_b: vec2<f32>,
) -> VsBlit {
    let c = vec2<f32>(f32(vi & 1u), f32(vi >> 1u));
    var world = rect.xy + c * rect.zw;
    world = vec2<f32>(
        transform_a.x * world.x + transform_a.z * world.y + transform_b.x,
        transform_a.y * world.x + transform_a.w * world.y + transform_b.y,
    );
    var out: VsBlit;
    out.pos = vec4<f32>(
        world.x / screen.size.x * 2.0 - 1.0,
        1.0 - world.y / screen.size.y * 2.0,
        0.0,
        1.0
    );
    out.uv = mix(uv.xy, uv.zw, c);
    out.world = world;
    out.clip_rect = clip_rect;
    out.clip_radius = clip_radius;
    return out;
}

// Cached surfaces and images are authored in sRGB.
fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        return c / 12.92;
    }
    return pow((c + 0.055) / 1.055, 2.4);
}

@fragment
fn fs_blit(frag: VsBlit) -> @location(0) vec4<f32> {
    let clip = rounded_clip_cover(frag.world, frag.clip_rect, frag.clip_radius);
    if (clip <= 0.001) {
        discard;
    }
    let t = textureSample(blit_tex, blit_samp, frag.uv);
    let rgb = vec3<f32>(
        srgb_to_linear(t.r),
        srgb_to_linear(t.g),
        srgb_to_linear(t.b),
    );
    let a = t.a * clip;
    return vec4<f32>(rgb * a, a);
}

// Backdrop blur, dimming, noise, and masking.
struct VsBackdrop {
    @builtin(position) pos: vec4<f32>,
    @location(0) rect: vec4<f32>,
    @location(1) world: vec2<f32>,
    @location(2) dim: vec4<f32>,
    @location(3) blur: f32,
    @location(4) noise: f32,
    @location(5) radius: f32,
    @location(6) clip_rect: vec4<f32>,
    @location(7) clip_radius: f32,
    @location(8) mask: vec4<f32>,
};

fn effect_mask(world: vec2<f32>, rect: vec4<f32>, mask: vec4<f32>) -> f32 {
    if (mask.x < 0.5) {
        return 1.0;
    }
    if (mask.x < 1.5) {
        let dir = vec2<f32>(cos(mask.w), sin(mask.w));
        let span = dot(rect.zw, abs(dir));
        var t = 0.0;
        if (span > 0.0001) {
            t = clamp(dot(world - rect.xy, dir) / span, 0.0, 1.0);
        }
        return clamp(mask.y + (mask.z - mask.y) * t, 0.0, 1.0);
    }
    return clamp(mask.y, 0.0, 1.0);
}

@vertex
fn vs_backdrop(
    @builtin(vertex_index) vi: u32,
    @location(0) rect: vec4<f32>,
    @location(1) dim: vec4<f32>,
    @location(2) params: vec4<f32>,
    @location(3) clip_rect: vec4<f32>,
    @location(4) mask: vec4<f32>,
) -> VsBackdrop {
    let c = vec2<f32>(f32(vi & 1u), f32(vi >> 1u));
    let world = rect.xy + c * rect.zw;
    var out: VsBackdrop;
    out.pos = vec4<f32>(
        world.x / screen.size.x * 2.0 - 1.0,
        1.0 - world.y / screen.size.y * 2.0,
        0.0,
        1.0
    );
    out.rect = rect;
    out.world = world;
    out.dim = dim;
    out.blur = params.x;
    out.noise = params.y;
    out.radius = params.z;
    out.clip_rect = clip_rect;
    out.clip_radius = params.w;
    out.mask = mask;
    return out;
}

fn backdrop_weight(d: i32) -> f32 {
    if (d == 0) {
        return 6.0;
    }
    if (d == 1 || d == -1) {
        return 4.0;
    }
    return 1.0;
}

fn hash12(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.xyx) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

@fragment
fn fs_backdrop(frag: VsBackdrop) -> @location(0) vec4<f32> {
    let center = frag.rect.xy + frag.rect.zw * 0.5;
    let p = abs(frag.world - center);
    let shape = rounded_box_cover(p, frag.rect.zw * 0.5, max(frag.radius, 0.0));
    let clip = rounded_clip_cover(frag.world, frag.clip_rect, frag.clip_radius);
    let m = shape * clip * effect_mask(frag.world, frag.rect, frag.mask);
    if (m <= 0.001) {
        discard;
    }
    let uv = frag.world / screen.size.xy;
    var b: vec4<f32>;
    if (frag.blur < 0.5) {
        b = textureSample(blit_tex, blit_samp, uv);
    } else {
        let step = clamp(frag.blur * 0.45, 1.0, 10.0) / screen.size.xy;
        var acc = vec4<f32>(0.0, 0.0, 0.0, 0.0);
        for (var oy = -2; oy <= 2; oy++) {
            for (var ox = -2; ox <= 2; ox++) {
                let w = backdrop_weight(ox) * backdrop_weight(oy) / 256.0;
                acc += textureSample(blit_tex, blit_samp, uv + vec2<f32>(f32(ox), f32(oy)) * step) * w;
            }
        }
        b = acc;
    }
    let d = frag.dim;
    let oa = d.a + b.a * (1.0 - d.a);
    var orgb = d.rgb * d.a + b.rgb * (1.0 - d.a);
    orgb += (hash12(frag.world) - 0.5) * frag.noise * oa;
    return vec4<f32>(orgb * m, oa * m);
}

@fragment
fn fs_present(frag: VsBlit) -> @location(0) vec4<f32> {
    let t = textureSample(blit_tex, blit_samp, frag.uv);
    return vec4<f32>(t.rgb, t.a);
}

// Gradient stops, positions, and linear/radial evaluation.
struct VsGradient {
    @builtin(position) pos: vec4<f32>,
    @location(0) world: vec2<f32>,
    @location(1) rect: vec4<f32>,
    @location(2) stop0: vec4<f32>,
    @location(3) stop1: vec4<f32>,
    @location(4) stop2: vec4<f32>,
    @location(5) stop3: vec4<f32>,
    @location(6) stop_pos: vec4<f32>,
    @location(7) params: vec4<f32>,
    @location(8) clip_rect: vec4<f32>,
    @location(9) clip_radius: f32,
};

@vertex
fn vs_gradient(
    @builtin(vertex_index) vi: u32,
    @location(0) rect: vec4<f32>,
    @location(1) stop0: vec4<f32>,
    @location(2) stop1: vec4<f32>,
    @location(3) stop2: vec4<f32>,
    @location(4) stop3: vec4<f32>,
    @location(5) stop_pos: vec4<f32>,
    @location(6) params: vec4<f32>,
    @location(7) clip_rect: vec4<f32>,
    @location(8) clip_radius: f32,
    @location(9) transform_a: vec4<f32>,
    @location(10) transform_b: vec2<f32>,
) -> VsGradient {
    let c = vec2<f32>(f32(vi & 1u), f32(vi >> 1u));
    var world = rect.xy + c * rect.zw;
    world = vec2<f32>(
        transform_a.x * world.x + transform_a.z * world.y + transform_b.x,
        transform_a.y * world.x + transform_a.w * world.y + transform_b.y,
    );
    var out: VsGradient;
    out.pos = vec4<f32>(
        world.x / screen.size.x * 2.0 - 1.0,
        1.0 - world.y / screen.size.y * 2.0,
        0.0,
        1.0
    );
    out.world = world;
    out.rect = rect;
    out.stop0 = stop0;
    out.stop1 = stop1;
    out.stop2 = stop2;
    out.stop3 = stop3;
    out.stop_pos = stop_pos;
    out.params = params;
    out.clip_rect = clip_rect;
    out.clip_radius = clip_radius;
    return out;
}

fn gradient_sample(
    t: f32,
    stop0: vec4<f32>,
    stop1: vec4<f32>,
    stop2: vec4<f32>,
    stop3: vec4<f32>,
    stop_pos: vec4<f32>,
    count: f32,
) -> vec4<f32> {
    let tc = clamp(t, 0.0, 1.0);
    if (count < 1.5) {
        return stop0;
    }
    if (count < 2.5) {
        let span = max(stop_pos.y - stop_pos.x, 0.0001);
        let k = clamp((tc - stop_pos.x) / span, 0.0, 1.0);
        return mix(stop0, stop1, k);
    }
    if (count < 3.5) {
        if (tc <= stop_pos.y) {
            let span = max(stop_pos.y - stop_pos.x, 0.0001);
            return mix(stop0, stop1, clamp((tc - stop_pos.x) / span, 0.0, 1.0));
        }
        let span = max(stop_pos.z - stop_pos.y, 0.0001);
        return mix(stop1, stop2, clamp((tc - stop_pos.y) / span, 0.0, 1.0));
    }
    if (tc <= stop_pos.y) {
        let span = max(stop_pos.y - stop_pos.x, 0.0001);
        return mix(stop0, stop1, clamp((tc - stop_pos.x) / span, 0.0, 1.0));
    }
    if (tc <= stop_pos.z) {
        let span = max(stop_pos.z - stop_pos.y, 0.0001);
        return mix(stop1, stop2, clamp((tc - stop_pos.y) / span, 0.0, 1.0));
    }
    let span = max(stop_pos.w - stop_pos.z, 0.0001);
    return mix(stop2, stop3, clamp((tc - stop_pos.z) / span, 0.0, 1.0));
}

@fragment
fn fs_gradient(frag: VsGradient) -> @location(0) vec4<f32> {
    let center = frag.rect.xy + frag.rect.zw * 0.5;
    let p = abs(frag.world - center);
    let shape = rounded_box_cover(p, frag.rect.zw * 0.5, max(frag.params.z, 0.0));
    let clip = rounded_clip_cover(frag.world, frag.clip_rect, frag.clip_radius);
    let m = shape * clip;
    if (m <= 0.001) {
        discard;
    }
    var t = 0.0;
    if (frag.params.x < 0.5) {
        let dir = vec2<f32>(cos(frag.params.y), sin(frag.params.y));
        let span = dot(frag.rect.zw, abs(dir));
        if (span > 0.0001) {
            t = clamp(dot(frag.world - frag.rect.xy, dir) / span, 0.0, 1.0);
        }
    } else {
        var extent = length(frag.rect.zw * 0.5);
        if (frag.params.y > 0.0001) {
            extent = frag.params.y;
        }
        if (extent > 0.0001) {
            t = clamp(length(frag.world - center) / extent, 0.0, 1.0);
        }
    }
    var col = gradient_sample(
        t, frag.stop0, frag.stop1, frag.stop2, frag.stop3, frag.stop_pos, frag.params.w
    );
    col += (hash12(frag.world) - 0.5) * (1.2 / 255.0) * col.a;
    let alpha = col.a * m;
    if (alpha <= 0.0) {
        discard;
    }
    return vec4<f32>(col.rgb * alpha, alpha);
}
"#;
