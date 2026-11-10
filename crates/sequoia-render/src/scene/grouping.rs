#[allow(unused_imports)]
use std::any::Any;

use crate::types::*;
use crate::utils::*;
use sequoia_core::{ImageSampling, Rect, RenderItem};

/// Converts paint items into GPU instances and contiguous draw runs.
///
/// Items outside the viewport are culled. Items that exceed the per-buffer
/// ceilings are dropped and counted in the returned tuple.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn group_runs(
    items: &[RenderItem],
    clips: &[Option<sequoia_core::RoundedClip>],
    transforms: &[sequoia_core::Transform],
    scale: f32,
    viewport: Option<Rect>,
    quad_ceiling: usize,
    glyph_ceiling: usize,
    image_ceiling: usize,
    gradient_ceiling: usize,
    quads: &mut Vec<GpuQuad>,
    glyphs: &mut Vec<GpuGlyph>,
    images: &mut Vec<BlitQuad>,
    image_keys: &mut Vec<u64>,
    gradients: &mut Vec<GpuGradient>,
    runs: &mut Vec<GuiRun>,
) -> (usize, usize, usize, usize, usize) {
    group_runs_with_opacity(
        items,
        clips,
        transforms,
        &[],
        scale,
        viewport,
        quad_ceiling,
        glyph_ceiling,
        image_ceiling,
        gradient_ceiling,
        quads,
        glyphs,
        images,
        image_keys,
        gradients,
        runs,
        None,
    )
}

/// Converts paint items with per-item alpha from an opacity span fallback.
///
/// `opacities[i]` scales the alpha of item `i`; missing entries mean `1.0`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn group_runs_with_opacity(
    items: &[RenderItem],
    clips: &[Option<sequoia_core::RoundedClip>],
    transforms: &[sequoia_core::Transform],
    opacities: &[f32],
    scale: f32,
    viewport: Option<Rect>,
    quad_ceiling: usize,
    glyph_ceiling: usize,
    image_ceiling: usize,
    gradient_ceiling: usize,
    quads: &mut Vec<GpuQuad>,
    glyphs: &mut Vec<GpuGlyph>,
    images: &mut Vec<BlitQuad>,
    image_keys: &mut Vec<u64>,
    gradients: &mut Vec<GpuGradient>,
    runs: &mut Vec<GuiRun>,
    shaders: Option<&mut ShaderGroupData>,
) -> (usize, usize, usize, usize, usize) {
    let opacity = |index: usize| opacities.get(index).copied().unwrap_or(1.0).clamp(0.0, 1.0);
    let mut shader_data = shaders;
    quads.clear();
    glyphs.clear();
    images.clear();
    image_keys.clear();
    gradients.clear();
    runs.clear();
    let mut dropped_quads = 0usize;
    let mut dropped_glyphs = 0usize;
    let mut dropped_images = 0usize;
    let mut dropped_gradients = 0usize;
    let mut culled = 0usize;

    let cull_rect = |r: Rect| -> bool {
        match viewport {
            Some(vp) if !vp.is_empty() => !r.intersects(vp),
            _ => false,
        }
    };

    let gpu_clip = |index: usize| -> ([f32; 4], f32) {
        match clips.get(index).copied().flatten() {
            Some(c) if !c.rect.is_empty() => (
                [
                    c.rect.left() * scale,
                    c.rect.top() * scale,
                    c.rect.size.width * scale,
                    c.rect.size.height * scale,
                ],
                c.radius * scale,
            ),
            _ => ([0.0; 4], 0.0),
        }
    };

    let gpu_transform = |index: usize| -> ([f32; 4], [f32; 2]) {
        match transforms.get(index).copied() {
            Some(tm) => {
                let m = tm.m;
                ([m[0], m[1], m[2], m[3]], [m[4] * scale, m[5] * scale])
            }
            None => ([1.0, 0.0, 0.0, 1.0], [0.0, 0.0]),
        }
    };

    let cull_rect_t = |r: Rect, index: usize| -> bool {
        let r = match transforms.get(index).copied() {
            Some(tm) if !tm.is_identity() => tm.apply_rect_aabb(r),
            _ => r,
        };
        cull_rect(r)
    };
    for (index, item) in items.iter().enumerate() {
        match item {
            RenderItem::Quad(q) => {
                if cull_rect_t(q.rect, index) {
                    culled += 1;
                    continue;
                }
                if quads.len() >= quad_ceiling {
                    dropped_quads += 1;
                    continue;
                }
                let (clip_rect, clip_radius) = gpu_clip(index);
                let (transform_a, transform_b) = gpu_transform(index);
                let o = opacity(index);
                let mut color = linear_rgba(q.color);
                color[3] *= o;
                let mut border_color = linear_rgba(q.border_color);
                border_color[3] *= o;
                quads.push(GpuQuad {
                    rect: [
                        q.rect.left() * scale,
                        q.rect.top() * scale,
                        q.rect.size.width * scale,
                        q.rect.size.height * scale,
                    ],
                    color,
                    radius: q.radius * scale,
                    border_color,
                    border_width: q.border_width * scale,
                    clip_rect,
                    clip_radius,
                    softness: 0.0,
                    spread: 0.0,
                    shadow_offset: [0.0; 2],
                    inset: 0.0,
                    transform_a,
                    transform_b,
                });
                match runs.last_mut() {
                    Some(GuiRun::Quads(_, len)) => *len += 1,
                    _ => runs.push(GuiRun::Quads(quads.len() as u32 - 1, 1)),
                }
            }

            RenderItem::Shadow(s) => {
                if cull_rect_t(s.bounds(), index) {
                    culled += 1;
                    continue;
                }
                if quads.len() >= quad_ceiling {
                    dropped_quads += 1;
                    continue;
                }
                let (clip_rect, clip_radius) = gpu_clip(index);
                let (transform_a, transform_b) = gpu_transform(index);
                let mut color = linear_rgba(s.color);
                color[3] *= opacity(index);
                let hard = s.blur <= 0.0 && !s.inset;
                let spread = if s.spread.is_finite() {
                    s.spread
                } else {
                    0.0
                };
                let shadow_rect = if hard {
                    Rect::from_xywh(
                        s.rect.left() + s.offset.x - spread,
                        s.rect.top() + s.offset.y - spread,
                        s.rect.size.width + spread * 2.0,
                        s.rect.size.height + spread * 2.0,
                    )
                } else {
                    s.rect
                };
                if shadow_rect.is_empty() {
                    continue;
                }
                quads.push(GpuQuad {
                    rect: [
                        shadow_rect.left() * scale,
                        shadow_rect.top() * scale,
                        shadow_rect.size.width * scale,
                        shadow_rect.size.height * scale,
                    ],
                    color,
                    radius: (if hard {
                        (s.radius + spread).max(0.0)
                    } else {
                        s.radius
                    }) * scale,
                    border_color: [0.0; 4],
                    border_width: 0.0,
                    clip_rect,
                    clip_radius,
                    softness: if hard { 0.0 } else { (s.blur * scale).max(0.0) },
                    spread: if hard { 0.0 } else { s.spread * scale },
                    shadow_offset: if hard {
                        [0.0; 2]
                    } else {
                        [s.offset.x * scale, s.offset.y * scale]
                    },
                    inset: if hard {
                        0.0
                    } else if s.inset {
                        1.0
                    } else {
                        0.0
                    },
                    transform_a,
                    transform_b,
                });
                match runs.last_mut() {
                    Some(GuiRun::Quads(_, len)) => *len += 1,
                    _ => runs.push(GuiRun::Quads(quads.len() as u32 - 1, 1)),
                }
            }
            RenderItem::Text(t) => {
                let start = glyphs.len() as u32;
                let (clip_rect, clip_radius) = gpu_clip(index);
                let (transform_a, transform_b) = gpu_transform(index);
                let mut color = linear_rgba(t.color);
                color[3] *= opacity(index);
                for g in &t.glyphs {
                    if cull_rect_t(g.rect, index) {
                        culled += 1;
                        continue;
                    }
                    if glyphs.len() >= glyph_ceiling {
                        dropped_glyphs += 1;
                        continue;
                    }
                    glyphs.push(GpuGlyph {
                        rect: [
                            g.rect.left() * scale,
                            g.rect.top() * scale,
                            g.rect.size.width * scale,
                            g.rect.size.height * scale,
                        ],
                        uv: g.uv,
                        color,
                        color_glyph: if g.color_glyph { 1.0 } else { 0.0 },
                        clip_rect,
                        clip_radius,
                        transform_a,
                        transform_b,
                    });
                }
                let added = glyphs.len() as u32 - start;
                if added == 0 {
                    continue;
                }
                match runs.last_mut() {
                    Some(GuiRun::Glyphs(s, len)) if *s + *len == start => *len += added,
                    _ => runs.push(GuiRun::Glyphs(start, added)),
                }
            }
            RenderItem::Image(d) => {
                if cull_rect_t(d.rect, index) {
                    culled += 1;
                    continue;
                }
                if images.len() >= image_ceiling {
                    dropped_images += 1;
                    continue;
                }
                let (clip_rect, clip_radius) = gpu_clip(index);
                let (transform_a, transform_b) = gpu_transform(index);
                images.push(BlitQuad {
                    rect: [
                        d.rect.left() * scale,
                        d.rect.top() * scale,
                        d.rect.size.width * scale,
                        d.rect.size.height * scale,
                    ],
                    uv: d.uv,
                    clip_rect,
                    clip_radius,
                    transform_a,
                    transform_b,
                    opacity: opacity(index),
                    nearest: if d.sampling == ImageSampling::Nearest {
                        1.0
                    } else {
                        0.0
                    },
                });
                image_keys.push(d.source.cache_key());
                match runs.last_mut() {
                    Some(GuiRun::Images(_, len)) => *len += 1,
                    _ => runs.push(GuiRun::Images(images.len() as u32 - 1, 1)),
                }
            }
            RenderItem::Waveform(w) => {
                let (clip_rect, clip_radius) = gpu_clip(index);
                let (transform_a, transform_b) = gpu_transform(index);
                let mut color = linear_rgba(w.color);
                color[3] *= opacity(index);
                let start = quads.len() as u32;
                for bar in &w.bars {
                    if cull_rect_t(*bar, index) {
                        culled += 1;
                        continue;
                    }
                    if quads.len() >= quad_ceiling {
                        dropped_quads += 1;
                        continue;
                    }
                    quads.push(GpuQuad {
                        rect: [
                            bar.left() * scale,
                            bar.top() * scale,
                            bar.size.width * scale,
                            bar.size.height * scale,
                        ],
                        color,
                        radius: 0.0,
                        border_color: [0.0; 4],
                        border_width: 0.0,
                        clip_rect,
                        clip_radius,
                        softness: 0.0,
                        spread: 0.0,
                        shadow_offset: [0.0; 2],
                        inset: 0.0,
                        transform_a,
                        transform_b,
                    });
                }
                let added = quads.len() as u32 - start;
                if added == 0 {
                    continue;
                }
                match runs.last_mut() {
                    Some(GuiRun::Quads(s, len)) if *s + *len == start => *len += added,
                    _ => runs.push(GuiRun::Quads(start, added)),
                }
            }
            RenderItem::Gradient(g) => {
                if cull_rect_t(g.rect, index) {
                    culled += 1;
                    continue;
                }
                let Some((mut colors, pos, count)) = g.stops.for_gpu() else {
                    dropped_gradients += 1;
                    continue;
                };
                let o = opacity(index);
                if o < 1.0 {
                    for c in &mut colors {
                        c[3] *= o;
                    }
                }
                if gradients.len() >= gradient_ceiling {
                    dropped_gradients += 1;
                    continue;
                }
                let (clip_rect, clip_radius) = gpu_clip(index);
                let (transform_a, transform_b) = gpu_transform(index);
                let (kind, extent) = match g.kind {
                    sequoia_core::GradientKind::Linear { angle_deg } => {
                        let a = if angle_deg.is_finite() {
                            angle_deg.to_radians()
                        } else {
                            0.0
                        };
                        (0.0, a)
                    }
                    sequoia_core::GradientKind::Radial { radius } => {
                        let r = if radius.is_finite() {
                            radius.max(0.0)
                        } else {
                            0.0
                        };
                        (1.0, r * scale)
                    }
                };
                gradients.push(GpuGradient {
                    rect: [
                        g.rect.left() * scale,
                        g.rect.top() * scale,
                        g.rect.size.width * scale,
                        g.rect.size.height * scale,
                    ],
                    stop0: colors[0],
                    stop1: colors[1],
                    stop2: colors[2],
                    stop3: colors[3],
                    stop_pos: pos,
                    params: [kind, extent, g.radius * scale, count as f32],
                    clip_rect,
                    clip_radius,
                    _pad: [0.0; 5],
                    transform_a,
                    transform_b,
                });
                match runs.last_mut() {
                    Some(GuiRun::Gradients(_, len)) => *len += 1,
                    _ => runs.push(GuiRun::Gradients(gradients.len() as u32 - 1, 1)),
                }
            }

            RenderItem::Shader(s) => {
                let Some(data) = shader_data.as_deref_mut() else {
                    culled += 1;
                    continue;
                };
                if cull_rect_t(s.rect, index) {
                    culled += 1;
                    continue;
                }
                let (clip_rect, clip_radius) = gpu_clip(index);
                let (transform_a, transform_b) = gpu_transform(index);
                let opacity = (s.opacity * opacity(index)).clamp(0.0, 1.0);
                if opacity <= 0.0 {
                    culled += 1;
                    continue;
                }
                let start = data.instances.len() as u32;
                let uniform_offset = data.uniforms.len() as u32;
                data.uniforms.extend_from_slice(&s.uniforms);
                data.uniforms
                    .resize(uniform_offset as usize + SHADER_UNIFORM_STRIDE, 0);
                data.instances.push(GpuShaderInstance {
                    rect: [
                        s.rect.left() * scale,
                        s.rect.top() * scale,
                        s.rect.size.width * scale,
                        s.rect.size.height * scale,
                    ],
                    uv: s.uv,
                    clip_rect,
                    clip_radius,
                    opacity,
                    transform_a,
                    transform_b,
                    shader_id: s.shader.get(),
                    uniform_offset,
                    _pad: [0.0; 2],
                });
                data.textures.push(s.textures.clone());
                let shader_id = s.shader;
                let blend = s.blend;
                match runs.last_mut() {
                    Some(GuiRun::Shader(run_start, len, id, mode))
                        if *run_start + *len == start && *id == shader_id && *mode == blend =>
                    {
                        *len += 1;
                    }
                    _ => runs.push(GuiRun::Shader(start, 1, shader_id, blend)),
                }
            }

            RenderItem::Blit {
                rect, uv, target, ..
            } => {
                if cull_rect_t(*rect, index) {
                    culled += 1;
                    continue;
                }
                if images.len() >= image_ceiling {
                    dropped_images += 1;
                    continue;
                }
                let (clip_rect, clip_radius) = gpu_clip(index);
                let (transform_a, transform_b) = gpu_transform(index);
                images.push(BlitQuad {
                    rect: [
                        rect.left() * scale,
                        rect.top() * scale,
                        rect.size.width * scale,
                        rect.size.height * scale,
                    ],
                    uv: *uv,
                    clip_rect,
                    clip_radius,
                    transform_a,
                    transform_b,
                    opacity: opacity(index),
                    nearest: 0.0,
                });
                image_keys.push(target_key(*target));
                match runs.last_mut() {
                    Some(GuiRun::Images(_, len)) => *len += 1,
                    _ => runs.push(GuiRun::Images(images.len() as u32 - 1, 1)),
                }
            }
            RenderItem::TargetBegin { .. } | RenderItem::TargetEnd { .. } => {}
            _ => {}
        }
    }
    (
        dropped_quads,
        dropped_glyphs,
        dropped_images,
        dropped_gradients,
        culled,
    )
}

/// Offsets run ranges by the instance counts already present in the buffers.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn shift_gui_runs(
    runs: &[GuiRun],
    quad_base: u32,
    glyph_base: u32,
    image_base: u32,
    gradient_base: u32,
) -> Vec<GuiRun> {
    shift_gui_runs_with_shaders(runs, quad_base, glyph_base, image_base, gradient_base, 0)
}

pub(crate) fn shift_gui_runs_with_shaders(
    runs: &[GuiRun],
    quad_base: u32,
    glyph_base: u32,
    image_base: u32,
    gradient_base: u32,
    shader_base: u32,
) -> Vec<GuiRun> {
    runs.iter()
        .map(|run| match *run {
            GuiRun::Quads(start, len) => GuiRun::Quads(start + quad_base, len),
            GuiRun::Glyphs(start, len) => GuiRun::Glyphs(start + glyph_base, len),
            GuiRun::Images(start, len) => GuiRun::Images(start + image_base, len),
            GuiRun::Gradients(start, len) => GuiRun::Gradients(start + gradient_base, len),
            GuiRun::Shader(start, len, shader, blend) => {
                GuiRun::Shader(start + shader_base, len, shader, blend)
            }
        })
        .collect()
}

pub(crate) fn gui_run_draws(
    runs: &[GuiRun],
    image_draws: &[Option<ImageBinding>],
    has_image_atlas: bool,
) -> u64 {
    let mut draws = 0u64;
    for run in runs {
        match *run {
            GuiRun::Quads(_, _) | GuiRun::Glyphs(_, _) | GuiRun::Gradients(_, _) => draws += 1,
            GuiRun::Shader(_, _, _, _) => draws += 1,
            GuiRun::Images(start, len) => {
                let end = (start + len) as usize;
                let mut index = start as usize;
                while index < end {
                    match image_draws.get(index) {
                        Some(Some(ImageBinding::Atlas)) => {
                            if has_image_atlas {
                                draws += 1;
                            }
                            while index < end
                                && matches!(image_draws.get(index), Some(Some(ImageBinding::Atlas)))
                            {
                                index += 1;
                            }
                        }
                        Some(Some(ImageBinding::Texture(_))) => {
                            draws += 1;
                            index += 1;
                        }
                        _ => index += 1,
                    }
                }
            }
        }
    }
    draws
}

#[inline]
pub(crate) fn quad_is_opaque(q: &GpuQuad) -> bool {
    q.softness <= 0.0
        && q.color[3] >= 0.999
        && q.radius <= 0.0
        && q.clip_radius <= 0.0
        && (q.border_width <= 0.0 || q.border_color[3] >= 0.999)
}

pub(crate) fn quad_range_is_opaque(quads: &[GpuQuad], start: u32, len: u32) -> bool {
    let start = start as usize;
    let len = len as usize;
    let end = start.saturating_add(len).min(quads.len());
    if start >= end {
        return true;
    }
    quads[start..end].iter().all(quad_is_opaque)
}
