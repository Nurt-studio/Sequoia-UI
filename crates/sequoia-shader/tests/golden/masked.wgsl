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

fn sq_luminance(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
}

fn sq_srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    let lo = c / 12.92;
    let hi = pow((c + vec3<f32>(0.055, 0.055, 0.055)) / 1.055, vec3<f32>(2.4, 2.4, 2.4));
    return select(hi, lo, c <= vec3<f32>(0.04045, 0.04045, 0.04045));
}

fn sq_linear_to_srgb(c: vec3<f32>) -> vec3<f32> {
    let lo = c * 12.92;
    let hi = 1.055 * pow(c, vec3<f32>(1.0 / 2.4, 1.0 / 2.4, 1.0 / 2.4)) - vec3<f32>(0.055, 0.055, 0.055);
    return select(hi, lo, c <= vec3<f32>(0.0031308, 0.0031308, 0.0031308));
}

fn sq_hash21(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}

fn sq_noise21(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (vec2<f32>(3.0, 3.0) - 2.0 * f);
    let a = sq_hash21(i);
    let b = sq_hash21(i + vec2<f32>(1.0, 0.0));
    let c = sq_hash21(i + vec2<f32>(0.0, 1.0));
    let d = sq_hash21(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

fn sq_fbm21(p: vec2<f32>, octaves: i32) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var point = p;
    let count = clamp(octaves, 1, 8);
    for (var i = 0; i < count; i = i + 1) {
        value = value + amplitude * sq_noise21(point);
        point = point * 2.0 + vec2<f32>(17.0, 31.0);
        amplitude = amplitude * 0.5;
    }
    return value;
}

fn sq_rotate(p: vec2<f32>, radians: f32) -> vec2<f32> {
    let s = sin(radians);
    let c = cos(radians);
    return vec2<f32>(p.x * c - p.y * s, p.x * s + p.y * c);
}

fn sq_remap(x: f32, in_min: f32, in_max: f32, out_min: f32, out_max: f32) -> f32 {
    let span = in_max - in_min;
    if (abs(span) < 0.000001) {
        return out_min;
    }
    return out_min + (x - in_min) * (out_max - out_min) / span;
}

fn sq_contrast(c: vec3<f32>, amount: f32) -> vec3<f32> {
    return clamp((c - vec3<f32>(0.5, 0.5, 0.5)) * amount + vec3<f32>(0.5, 0.5, 0.5), vec3<f32>(0.0, 0.0, 0.0), vec3<f32>(1.0, 1.0, 1.0));
}

fn sq_saturate(c: vec3<f32>, amount: f32) -> vec3<f32> {
    let l = sq_luminance(c);
    return clamp(vec3<f32>(l, l, l) + (c - vec3<f32>(l, l, l)) * amount, vec3<f32>(0.0, 0.0, 0.0), vec3<f32>(1.0, 1.0, 1.0));
}

fn sq_checker_uv(uv: vec2<f32>, scale: f32) -> f32 {
    let cell = floor(uv * scale);
    return select(0.0, 1.0, (i32(cell.x) + i32(cell.y)) % 2 == 0);
}

fn sq_rgb_to_hsv(c: vec3<f32>) -> vec3<f32> {
    let mx = max(max(c.r, c.g), c.b);
    let mn = min(min(c.r, c.g), c.b);
    let delta = mx - mn;
    var h = 0.0;
    if (delta > 0.00001) {
        if (mx == c.r) {
            h = (c.g - c.b) / delta;
        } else if (mx == c.g) {
            h = 2.0 + (c.b - c.r) / delta;
        } else {
            h = 4.0 + (c.r - c.g) / delta;
        }
        h = fract(h / 6.0);
    }
    let s = select(0.0, delta / mx, mx > 0.00001);
    return vec3<f32>(h, s, mx);
}

fn sq_hsv_to_rgb(hsv: vec3<f32>) -> vec3<f32> {
    let h = fract(hsv.x) * 6.0;
    let sector = floor(h);
    let f = h - sector;
    let s = clamp(hsv.y, 0.0, 1.0);
    let v = hsv.z;
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));
    if (sector < 1.0) {
        return vec3<f32>(v, t, p);
    }
    if (sector < 2.0) {
        return vec3<f32>(q, v, p);
    }
    if (sector < 3.0) {
        return vec3<f32>(p, v, t);
    }
    if (sector < 4.0) {
        return vec3<f32>(p, q, v);
    }
    if (sector < 5.0) {
        return vec3<f32>(t, p, v);
    }
    return vec3<f32>(v, p, q);
}

fn sq_hue_to_rgb(p: f32, q: f32, t_in: f32) -> f32 {
    let t = fract(t_in / 6.0);
    if (t < 1.0 / 6.0) {
        return p + (q - p) * 6.0 * t;
    }
    if (t < 0.5) {
        return q;
    }
    if (t < 2.0 / 3.0) {
        return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
    }
    return p;
}

fn sq_hsl_to_rgb(hsl: vec3<f32>) -> vec3<f32> {
    let s = clamp(hsl.y, 0.0, 1.0);
    let l = hsl.z;
    if (s <= 0.00001) {
        return vec3<f32>(l, l, l);
    }
    let q = select(l + s - l * s, l * (1.0 + s), l < 0.5);
    let p = 2.0 * l - q;
    let h = fract(hsl.x) * 6.0;
    return vec3<f32>(
        sq_hue_to_rgb(p, q, h + 2.0),
        sq_hue_to_rgb(p, q, h),
        sq_hue_to_rgb(p, q, h - 2.0),
    );
}

fn sq_rgb_to_hsl(c: vec3<f32>) -> vec3<f32> {
    let mx = max(max(c.r, c.g), c.b);
    let mn = min(min(c.r, c.g), c.b);
    let l = (mx + mn) * 0.5;
    let delta = mx - mn;
    if (delta <= 0.00001) {
        return vec3<f32>(0.0, 0.0, l);
    }
    let s = select(delta / (2.0 - mx - mn), delta / (mx + mn), l < 0.5);
    var h = 0.0;
    if (mx == c.r) {
        h = (c.g - c.b) / delta;
    } else if (mx == c.g) {
        h = 2.0 + (c.b - c.r) / delta;
    } else {
        h = 4.0 + (c.r - c.g) / delta;
    }
    return vec3<f32>(fract(h / 6.0), s, l);
}

fn sq_mix3(a: vec3<f32>, b: vec3<f32>, t: f32) -> vec3<f32> {
    return a + (b - a) * t;
}

fn sq_mix4(a: vec4<f32>, b: vec4<f32>, t: f32) -> vec4<f32> {
    return a + (b - a) * t;
}

fn sq_lerp(a: f32, b: f32, t: f32) -> f32 {
    return a + (b - a) * t;
}

fn sq_clamp01(v: f32) -> f32 {
    return clamp(v, 0.0, 1.0);
}

fn sq_smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = clamp((x - edge0) / max(edge1 - edge0, 0.000001), 0.0, 1.0);
    return t * t * (3.0 - 2.0 * t);
}

fn sq_overlay(base: vec3<f32>, blend: vec3<f32>) -> vec3<f32> {
    let lo = 2.0 * base * blend;
    let hi = vec3<f32>(1.0, 1.0, 1.0) - 2.0 * (vec3<f32>(1.0, 1.0, 1.0) - base) * (vec3<f32>(1.0, 1.0, 1.0) - blend);
    return clamp(select(lo, hi, base > vec3<f32>(0.5, 0.5, 0.5)), vec3<f32>(0.0, 0.0, 0.0), vec3<f32>(1.0, 1.0, 1.0));
}

fn sq_soft_light_curve(base: f32) -> f32 {
    if (base > 0.25) {
        return sqrt(base);
    }
    return ((16.0 * base - 12.0) * base + 4.0) * base;
}

fn sq_soft_light(base: vec3<f32>, blend: vec3<f32>) -> vec3<f32> {
    let d = vec3<f32>(
        sq_soft_light_curve(base.x),
        sq_soft_light_curve(base.y),
        sq_soft_light_curve(base.z),
    );
    let lo = 2.0 * blend * base + base * base * (vec3<f32>(1.0, 1.0, 1.0) - 2.0 * blend);
    let hi = base + (2.0 * blend - vec3<f32>(1.0, 1.0, 1.0)) * (d - base);
    return select(lo, hi, blend > vec3<f32>(0.5, 0.5, 0.5));
}

fn sq_hard_light(base: vec3<f32>, blend: vec3<f32>) -> vec3<f32> {
    let lo = 2.0 * base * blend;
    let hi = vec3<f32>(1.0, 1.0, 1.0) - 2.0 * (vec3<f32>(1.0, 1.0, 1.0) - base) * (vec3<f32>(1.0, 1.0, 1.0) - blend);
    return select(lo, hi, blend > vec3<f32>(0.5, 0.5, 0.5));
}

fn sq_screen(base: vec3<f32>, blend: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(1.0, 1.0, 1.0) - (vec3<f32>(1.0, 1.0, 1.0) - base) * (vec3<f32>(1.0, 1.0, 1.0) - blend);
}

fn sq_multiply(base: vec3<f32>, blend: vec3<f32>) -> vec3<f32> {
    return base * blend;
}

fn sq_add_blend(base: vec3<f32>, blend: vec3<f32>) -> vec3<f32> {
    return min(base + blend, vec3<f32>(1.0, 1.0, 1.0));
}

fn sq_vignette(uv: vec2<f32>, strength: f32) -> f32 {
    let d = length(uv - vec2<f32>(0.5, 0.5)) * 1.41421356;
    return clamp(1.0 - d * strength, 0.0, 1.0);
}

struct SequoiaUniforms {
    _reserved: vec4<f32>,
}
@group(0) @binding(1) var<uniform> sequoia_u: SequoiaUniforms;
@group(1) @binding(0) var mask: texture_2d<f32>;
@group(1) @binding(1) var mask_sampler: sampler;
fn fs_user(frag: ShaderVarying) -> vec4<f32> {
    let uv = frag.uv;

let m = textureSample(mask, mask_sampler, uv); let edge =
sq_smoothstep(0.0, 1.0, m.a); return vec4 < f32 > (m.rgb, m.a * edge);
}
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
