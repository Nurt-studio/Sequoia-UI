//! CPU equivalents of the WGSL helpers in [`crate::WGSL_STD`].
//!
//! These are useful in tests and when animating shader uniforms without a GPU.

use crate::{Vec2, Vec3, Vec4};

fn fract(value: f32) -> f32 {
    value - value.floor()
}

fn mix(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn hue_to_rgb(p: f32, q: f32, t: f32) -> f32 {
    let t = fract(t / 6.0);
    if t < 1.0 / 6.0 {
        p + (q - p) * 6.0 * t
    } else if t < 0.5 {
        q
    } else if t < 2.0 / 3.0 {
        p + (q - p) * (2.0 / 3.0 - t) * 6.0
    } else {
        p
    }
}

/// Converts linear RGB to HSV. Hue is normalized to `0..=1`.
#[must_use]
pub fn rgb_to_hsv(color: Vec3) -> Vec3 {
    let max = color.x.max(color.y).max(color.z);
    let min = color.x.min(color.y).min(color.z);
    let delta = max - min;
    let mut hue = 0.0;
    if delta > 0.000_01 {
        let h = if max == color.x {
            (color.y - color.z) / delta
        } else if max == color.y {
            2.0 + (color.z - color.x) / delta
        } else {
            4.0 + (color.x - color.y) / delta
        };
        hue = fract(h / 6.0);
    }
    let saturation = if max > 0.000_01 { delta / max } else { 0.0 };
    Vec3::new(hue, saturation, max)
}

/// Converts HSV (normalized hue) back to linear RGB.
#[must_use]
pub fn hsv_to_rgb(hsv: Vec3) -> Vec3 {
    let h = fract(hsv.x) * 6.0;
    let sector = h.floor();
    let f = h - sector;
    let s = hsv.y.clamp(0.0, 1.0);
    let v = hsv.z;
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));
    match sector as i32 {
        0 => Vec3::new(v, t, p),
        1 => Vec3::new(q, v, p),
        2 => Vec3::new(p, v, t),
        3 => Vec3::new(p, q, v),
        4 => Vec3::new(t, p, v),
        _ => Vec3::new(v, p, q),
    }
}

/// Converts HSL (normalized hue) to linear RGB.
#[must_use]
pub fn hsl_to_rgb(hsl: Vec3) -> Vec3 {
    let s = hsl.y.clamp(0.0, 1.0);
    let l = hsl.z;
    if s <= 0.000_01 {
        return Vec3::splat(l);
    }
    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;
    let h = fract(hsl.x) * 6.0;
    Vec3::new(
        hue_to_rgb(p, q, h + 2.0),
        hue_to_rgb(p, q, h),
        hue_to_rgb(p, q, h - 2.0),
    )
}

/// Converts linear RGB to HSL. Hue is normalized to `0..=1`.
#[must_use]
pub fn rgb_to_hsl(color: Vec3) -> Vec3 {
    let max = color.x.max(color.y).max(color.z);
    let min = color.x.min(color.y).min(color.z);
    let lightness = (max + min) * 0.5;
    let delta = max - min;
    if delta <= 0.000_01 {
        return Vec3::new(0.0, 0.0, lightness);
    }
    let saturation = if lightness < 0.5 {
        delta / (max + min)
    } else {
        delta / (2.0 - max - min)
    };
    let h = if max == color.x {
        (color.y - color.z) / delta
    } else if max == color.y {
        2.0 + (color.z - color.x) / delta
    } else {
        4.0 + (color.x - color.y) / delta
    };
    Vec3::new(fract(h / 6.0), saturation, lightness)
}

/// Element-wise linear interpolation between two colors.
#[must_use]
pub fn mix3(a: Vec3, b: Vec3, t: f32) -> Vec3 {
    Vec3::new(
        a.x + (b.x - a.x) * t,
        a.y + (b.y - a.y) * t,
        a.z + (b.z - a.z) * t,
    )
}

/// Element-wise linear interpolation between two RGBA values.
#[must_use]
pub fn mix4(a: Vec4, b: Vec4, t: f32) -> Vec4 {
    Vec4::new(
        a.x + (b.x - a.x) * t,
        a.y + (b.y - a.y) * t,
        a.z + (b.z - a.z) * t,
        a.w + (b.w - a.w) * t,
    )
}

/// Scalar linear interpolation.
#[must_use]
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Clamps `value` to the `min..=max` range.
#[must_use]
pub fn clamp(value: f32, min: f32, max: f32) -> f32 {
    value.clamp(min, max)
}

/// Hermite interpolation between `edge0` and `edge1`.
#[must_use]
pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0).max(0.000_001)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Photoshop-style overlay blend of `blend` over `base`.
#[must_use]
pub fn overlay(base: Vec3, blend: Vec3) -> Vec3 {
    Vec3::new(
        blend_channel(base.x, blend.x, |b, s| {
            if b > 0.5 {
                1.0 - 2.0 * (1.0 - b) * (1.0 - s)
            } else {
                2.0 * b * s
            }
        }),
        blend_channel(base.y, blend.y, |b, s| {
            if b > 0.5 {
                1.0 - 2.0 * (1.0 - b) * (1.0 - s)
            } else {
                2.0 * b * s
            }
        }),
        blend_channel(base.z, blend.z, |b, s| {
            if b > 0.5 {
                1.0 - 2.0 * (1.0 - b) * (1.0 - s)
            } else {
                2.0 * b * s
            }
        }),
    )
}

fn blend_channel(base: f32, blend: f32, f: impl Fn(f32, f32) -> f32) -> f32 {
    f(base, blend).clamp(0.0, 1.0)
}

/// Soft-light blend of `blend` over `base`.
#[must_use]
pub fn soft_light(base: Vec3, blend: Vec3) -> Vec3 {
    let d = Vec3::new(
        soft_light_curve(base.x),
        soft_light_curve(base.y),
        soft_light_curve(base.z),
    );
    Vec3::new(
        if blend.x > 0.5 {
            base.x + (2.0 * blend.x - 1.0) * (d.x - base.x)
        } else {
            2.0 * blend.x * base.x + base.x * base.x * (1.0 - 2.0 * blend.x)
        },
        if blend.y > 0.5 {
            base.y + (2.0 * blend.y - 1.0) * (d.y - base.y)
        } else {
            2.0 * blend.y * base.y + base.y * base.y * (1.0 - 2.0 * blend.y)
        },
        if blend.z > 0.5 {
            base.z + (2.0 * blend.z - 1.0) * (d.z - base.z)
        } else {
            2.0 * blend.z * base.z + base.z * base.z * (1.0 - 2.0 * blend.z)
        },
    )
}

fn soft_light_curve(base: f32) -> f32 {
    if base > 0.25 {
        base.sqrt()
    } else {
        ((16.0 * base - 12.0) * base + 4.0) * base
    }
}

/// Hard-light blend of `blend` over `base`.
#[must_use]
pub fn hard_light(base: Vec3, blend: Vec3) -> Vec3 {
    Vec3::new(
        hard_light_channel(base.x, blend.x),
        hard_light_channel(base.y, blend.y),
        hard_light_channel(base.z, blend.z),
    )
}

fn hard_light_channel(base: f32, blend: f32) -> f32 {
    if blend > 0.5 {
        1.0 - 2.0 * (1.0 - base) * (1.0 - blend)
    } else {
        2.0 * base * blend
    }
}

/// Screen blend of `blend` over `base`.
#[must_use]
pub fn screen(base: Vec3, blend: Vec3) -> Vec3 {
    Vec3::new(
        1.0 - (1.0 - base.x) * (1.0 - blend.x),
        1.0 - (1.0 - base.y) * (1.0 - blend.y),
        1.0 - (1.0 - base.z) * (1.0 - blend.z),
    )
}

/// Multiply blend of `blend` over `base`.
#[must_use]
pub fn multiply(base: Vec3, blend: Vec3) -> Vec3 {
    Vec3::new(base.x * blend.x, base.y * blend.y, base.z * blend.z)
}

/// Additive blend of `blend` over `base`, clamped to white.
#[must_use]
pub fn add_blend(base: Vec3, blend: Vec3) -> Vec3 {
    Vec3::new(
        (base.x + blend.x).min(1.0),
        (base.y + blend.y).min(1.0),
        (base.z + blend.z).min(1.0),
    )
}

#[must_use]
pub fn luminance(color: Vec3) -> f32 {
    color.x * 0.2126 + color.y * 0.7152 + color.z * 0.0722
}

#[must_use]
pub fn srgb_to_linear(color: Vec3) -> Vec3 {
    let convert = |value: f32| {
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    Vec3::new(convert(color.x), convert(color.y), convert(color.z))
}

#[must_use]
pub fn linear_to_srgb(color: Vec3) -> Vec3 {
    let convert = |value: f32| {
        if value <= 0.003_130_8 {
            value * 12.92
        } else {
            1.055 * value.powf(1.0 / 2.4) - 0.055
        }
    };
    Vec3::new(convert(color.x), convert(color.y), convert(color.z))
}

#[must_use]
pub fn hash21(point: Vec2) -> f32 {
    fract((point.x * 12.9898 + point.y * 78.233).sin() * 43_758.547)
}

#[must_use]
pub fn noise21(point: Vec2) -> f32 {
    let i = Vec2::new(point.x.floor(), point.y.floor());
    let f = Vec2::new(fract(point.x), fract(point.y));
    let u = Vec2::new(f.x * f.x * (3.0 - 2.0 * f.x), f.y * f.y * (3.0 - 2.0 * f.y));
    let a = hash21(i);
    let b = hash21(Vec2::new(i.x + 1.0, i.y));
    let c = hash21(Vec2::new(i.x, i.y + 1.0));
    let d = hash21(Vec2::new(i.x + 1.0, i.y + 1.0));
    mix(mix(a, b, u.x), mix(c, d, u.x), u.y)
}

#[must_use]
pub fn fbm21(point: Vec2, octaves: i32) -> f32 {
    let mut value = 0.0;
    let mut amplitude = 0.5;
    let mut p = point;
    for _ in 0..octaves.clamp(1, 8) {
        value += amplitude * noise21(p);
        p = Vec2::new(p.x * 2.0 + 17.0, p.y * 2.0 + 31.0);
        amplitude *= 0.5;
    }
    value
}

#[must_use]
pub fn rotate(point: Vec2, radians: f32) -> Vec2 {
    let (sin, cos) = radians.sin_cos();
    Vec2::new(point.x * cos - point.y * sin, point.x * sin + point.y * cos)
}

#[must_use]
pub fn remap(x: f32, in_min: f32, in_max: f32, out_min: f32, out_max: f32) -> f32 {
    let span = in_max - in_min;
    if span.abs() < 0.000_001 {
        return out_min;
    }
    out_min + (x - in_min) * (out_max - out_min) / span
}

#[must_use]
pub fn contrast(color: Vec3, amount: f32) -> Vec3 {
    let apply = |value: f32| ((value - 0.5) * amount + 0.5).clamp(0.0, 1.0);
    Vec3::new(apply(color.x), apply(color.y), apply(color.z))
}

#[must_use]
pub fn saturate(color: Vec3, amount: f32) -> Vec3 {
    let l = luminance(color);
    let apply = |value: f32| (l + (value - l) * amount).clamp(0.0, 1.0);
    Vec3::new(apply(color.x), apply(color.y), apply(color.z))
}

#[must_use]
pub fn checker_uv(uv: Vec2, scale: f32) -> f32 {
    let x = (uv.x * scale).floor() as i32;
    let y = (uv.y * scale).floor() as i32;
    if (x + y) % 2 == 0 { 1.0 } else { 0.0 }
}

#[must_use]
pub fn vignette(uv: Vec2, strength: f32) -> f32 {
    let dx = uv.x - 0.5;
    let dy = uv.y - 0.5;
    let distance = (dx * dx + dy * dy).sqrt() * core::f32::consts::SQRT_2;
    (1.0 - distance * strength).clamp(0.0, 1.0)
}
