#[allow(unused_imports)]
use std::any::Any;

use crate::surface::SurfaceFrame;
use crate::types::*;
use crate::utils::*;
use sequoia_core::{BackdropSpan, FilterSpan, Rect};

/// Builds a blit instance that maps a visible screen area to a cached surface.
pub(crate) fn blit_instance(screen: Rect, full: Rect) -> BlitQuad {
    let u0 = (screen.left() - full.left()) / full.size.width;
    let v0 = (screen.top() - full.top()) / full.size.height;
    let u1 = (screen.right() - full.left()) / full.size.width;
    let v1 = (screen.bottom() - full.top()) / full.size.height;
    BlitQuad {
        rect: [
            screen.left(),
            screen.top(),
            screen.size.width,
            screen.size.height,
        ],
        uv: [u0, v0, u1, v1],
        clip_rect: [0.0; 4],
        clip_radius: 0.0,
        transform_a: [1.0, 0.0, 0.0, 1.0],
        transform_b: [0.0, 0.0],
        opacity: 1.0,
    }
}

/// Builds a full-screen backdrop instance that samples a group layer and
/// multiplies its premultiplied alpha by `opacity`.
pub(crate) fn opacity_composite_instance(fb: (u32, u32), opacity: f32) -> GpuBackdrop {
    GpuBackdrop {
        rect: [0.0, 0.0, fb.0.max(1) as f32, fb.1.max(1) as f32],
        dim: [0.0; 4],
        params: [0.0; 4],
        clip_rect: [0.0; 4],
        mask: [2.0, opacity.clamp(0.0, 1.0), 0.0, 0.0],
    }
}

pub(crate) fn quad_instance(
    rect: Rect,
    color: [f32; 4],
    radius: f32,
    border_color: [f32; 4],
    border_width: f32,
) -> GpuQuad {
    GpuQuad {
        rect: [rect.left(), rect.top(), rect.size.width, rect.size.height],
        color,
        radius,
        border_color,
        border_width,
        clip_rect: [0.0; 4],
        clip_radius: 0.0,
        softness: 0.0,
        spread: 0.0,
        shadow_offset: [0.0; 2],
        inset: 0.0,
        transform_a: [1.0, 0.0, 0.0, 1.0],
        transform_b: [0.0, 0.0],
    }
}

/// Emits decoration quads for a surface and returns its content rectangle.
pub(crate) fn surface_frame(
    outer: Rect,
    frame: &SurfaceFrame,
    scale: f32,
    fb: (u32, u32),
    out: &mut Vec<GpuQuad>,
) -> Option<Rect> {
    let phys = phys_rect(outer, scale, fb.0, fb.1)?;
    let mut push = |r: Rect, color: [f32; 4], radius: f32, border: Option<([f32; 4], f32)>| {
        if r.is_empty() {
            return;
        }
        if out.len() >= MAX_FRAME_QUADS {
            return;
        }
        let (border_color, border_width) = border.unwrap_or(([0.0; 4], 0.0));
        out.push(quad_instance(r, color, radius, border_color, border_width));
    };
    let s = scale;

    let outer = phys;
    match (frame.background, frame.border) {
        (Some(bg), Some((bc, bw))) if bw > 0.0 => {
            push(outer, bg, frame.radius * s, Some((bc, bw * s)));
        }
        (Some(bg), _) => push(outer, bg, frame.radius * s, None),
        (None, Some((bc, bw))) if bw > 0.0 => {
            push(outer, [0.0; 4], frame.radius * s, Some((bc, bw * s)));
        }
        _ => {}
    }

    phys_rect(
        inset_rect(outer, scale_inset(frame.inset, s)),
        1.0,
        fb.0,
        fb.1,
    )
}

impl crate::renderer::GpuRenderer {
    pub(crate) fn backdrop_instance(span: &BackdropSpan, scale: f32) -> Option<GpuBackdrop> {
        let r = span.draw.rect;
        if r.is_empty() {
            return None;
        }
        let s = if scale.is_finite() && scale > 0.0 {
            scale
        } else {
            1.0
        };
        let rect = [
            r.left() * s,
            r.top() * s,
            r.size.width * s,
            r.size.height * s,
        ];
        if rect[2] <= 0.0 || rect[3] <= 0.0 {
            return None;
        }
        let blur = (span.draw.blur * s).clamp(0.0, MAX_BACKDROP_BLUR_PX);
        let (clip_rect, clip_radius) = match span.clip {
            Some(c) if !c.rect.is_empty() => (
                [
                    c.rect.left() * s,
                    c.rect.top() * s,
                    c.rect.size.width * s,
                    c.rect.size.height * s,
                ],
                c.radius * s,
            ),
            _ => ([0.0; 4], 0.0),
        };
        Some(GpuBackdrop {
            rect,
            dim: linear_rgba(span.draw.dim),
            params: [blur, span.draw.noise, span.draw.radius * s, clip_radius],
            clip_rect,
            mask: span.draw.mask.mask_vec(),
        })
    }

    pub(crate) fn filter_instance(span: &FilterSpan, scale: f32) -> Option<GpuBackdrop> {
        let r = span.draw.rect;
        if r.is_empty() {
            return None;
        }
        let s = if scale.is_finite() && scale > 0.0 {
            scale
        } else {
            1.0
        };
        let rect = [
            r.left() * s,
            r.top() * s,
            r.size.width * s,
            r.size.height * s,
        ];
        if rect[2] <= 0.0 || rect[3] <= 0.0 {
            return None;
        }
        let blur = (span.draw.blur * s).clamp(0.0, MAX_BACKDROP_BLUR_PX);
        let (clip_rect, clip_radius) = match span.clip {
            Some(c) if !c.rect.is_empty() => (
                [
                    c.rect.left() * s,
                    c.rect.top() * s,
                    c.rect.size.width * s,
                    c.rect.size.height * s,
                ],
                c.radius * s,
            ),
            _ => ([0.0; 4], 0.0),
        };
        Some(GpuBackdrop {
            rect,
            dim: [0.0; 4],
            params: [blur, 0.0, span.draw.radius * s, clip_radius],
            clip_rect,
            mask: span.draw.mask.mask_vec(),
        })
    }
}
