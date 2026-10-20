#[allow(unused_imports)]
use std::any::Any;

use crate::types::*;
use crate::utils::*;
use sequoia_core::{Rect, RenderItem};

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
                quads.push(GpuQuad {
                    rect: [
                        q.rect.left() * scale,
                        q.rect.top() * scale,
                        q.rect.size.width * scale,
                        q.rect.size.height * scale,
                    ],
                    color: linear_rgba(q.color),
                    radius: q.radius * scale,
                    border_color: linear_rgba(q.border_color),
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
                quads.push(GpuQuad {
                    rect: [
                        s.rect.left() * scale,
                        s.rect.top() * scale,
                        s.rect.size.width * scale,
                        s.rect.size.height * scale,
                    ],
                    color: linear_rgba(s.color),
                    radius: s.radius * scale,
                    border_color: [0.0; 4],
                    border_width: 0.0,
                    clip_rect,
                    clip_radius,
                    softness: (s.blur * scale).max(0.0),
                    spread: s.spread * scale,
                    shadow_offset: [s.offset.x * scale, s.offset.y * scale],
                    inset: if s.inset { 1.0 } else { 0.0 },
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
                        color: linear_rgba(t.color),
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
                let color = linear_rgba(w.color);
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
                let Some((colors, pos, count)) = g.stops.for_gpu() else {
                    dropped_gradients += 1;
                    continue;
                };
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

pub(crate) fn shift_gui_runs(
    runs: &[GuiRun],
    quad_base: u32,
    glyph_base: u32,
    image_base: u32,
    gradient_base: u32,
) -> Vec<GuiRun> {
    runs.iter()
        .map(|run| match *run {
            GuiRun::Quads(start, len) => GuiRun::Quads(start + quad_base, len),
            GuiRun::Glyphs(start, len) => GuiRun::Glyphs(start + glyph_base, len),
            GuiRun::Images(start, len) => GuiRun::Images(start + image_base, len),
            GuiRun::Gradients(start, len) => GuiRun::Gradients(start + gradient_base, len),
        })
        .collect()
}

pub(crate) fn gui_run_draws(runs: &[GuiRun], image_draws: &[(u32, wgpu::BindGroup)]) -> u64 {
    let mut draws = 0u64;
    for run in runs {
        match *run {
            GuiRun::Quads(_, _) | GuiRun::Glyphs(_, _) | GuiRun::Gradients(_, _) => draws += 1,
            GuiRun::Images(start, len) => {
                let end = start + len;
                draws += image_draws
                    .iter()
                    .filter(|(idx, _)| *idx >= start && *idx < end)
                    .count() as u64;
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
