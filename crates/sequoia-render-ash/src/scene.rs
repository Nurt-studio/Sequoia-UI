//! CPU conversion from a [`RenderList`] into Vulkan instance buffers.
//!
//! The layouts mirror `sequoia-render/src/types.rs` exactly; both backends
//! consume the same `RenderList` and must interpret its items identically.

use sequoia_core::{
    Color, ImageSampling, ImageSource, Rect, RenderItem, RenderList, RenderTargetId, TargetFormat,
    Transform,
};

pub(crate) const TARGET_KEY_BIT: u64 = 1 << 63;
pub(crate) const TARGET_FORMAT_BIT: u64 = 1 << 62;

pub(crate) const QUAD_BYTES: usize = 120;
pub(crate) const GLYPH_BYTES: usize = 96;
pub(crate) const BLIT_BYTES: usize = 84;
pub(crate) const GRADIENT_BYTES: usize = 176;

const MAX_IMAGE_DIM: u32 = 8192;
const MAX_IMAGE_BYTES: usize = 64 * 1024 * 1024;

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct GpuQuad {
    pub rect: [f32; 4],
    pub color: [f32; 4],
    pub radius: f32,
    pub border_color: [f32; 4],
    pub border_width: f32,
    pub clip_rect: [f32; 4],
    pub clip_radius: f32,
    pub softness: f32,
    pub spread: f32,
    pub shadow_offset: [f32; 2],
    pub inset: f32,
    pub transform_a: [f32; 4],
    pub transform_b: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct GpuGlyph {
    pub rect: [f32; 4],
    pub uv: [f32; 4],
    pub color: [f32; 4],
    pub color_glyph: f32,
    pub clip_rect: [f32; 4],
    pub clip_radius: f32,
    pub transform_a: [f32; 4],
    pub transform_b: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct BlitQuad {
    pub rect: [f32; 4],
    pub uv: [f32; 4],
    pub clip_rect: [f32; 4],
    pub clip_radius: f32,
    pub transform_a: [f32; 4],
    pub transform_b: [f32; 2],
    pub opacity: f32,
    pub nearest: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct GpuGradient {
    pub rect: [f32; 4],
    pub stop0: [f32; 4],
    pub stop1: [f32; 4],
    pub stop2: [f32; 4],
    pub stop3: [f32; 4],
    pub stop_pos: [f32; 4],
    pub params: [f32; 4],
    pub clip_rect: [f32; 4],
    pub clip_radius: f32,
    pub _pad: [f32; 5],
    pub transform_a: [f32; 4],
    pub transform_b: [f32; 2],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Run {
    Quads { start: u32, len: u32 },
    Glyphs { start: u32, len: u32 },
    Images { start: u32, len: u32 },
    Gradients { start: u32, len: u32 },
    Surface(u32),
}

#[derive(Default)]
pub(crate) struct Scene {
    pub quads: Vec<GpuQuad>,
    pub glyphs: Vec<GpuGlyph>,
    pub blits: Vec<BlitQuad>,
    pub blit_keys: Vec<u64>,
    pub blit_sources: Vec<ImageSource>,
    pub gradients: Vec<GpuGradient>,
    pub runs: Vec<Run>,
    pub unsupported: usize,
}

impl Scene {
    /// Appends one blit instance with an externally managed texture key.
    pub(crate) fn push_blit(&mut self, blit: BlitQuad, key: u64) {
        self.blits.push(blit);
        self.blit_keys.push(key);
        push_image_run(self);
    }

    /// Appends another scene and returns the range of runs it contributed.
    pub(crate) fn append(&mut self, mut source: Scene) -> std::ops::Range<u32> {
        let quad_base = self.quads.len() as u32;
        let glyph_base = self.glyphs.len() as u32;
        let blit_base = self.blits.len() as u32;
        let gradient_base = self.gradients.len() as u32;
        let run_start = self.runs.len() as u32;

        for run in source.runs.drain(..) {
            let run = match run {
                Run::Quads { start, len } => Run::Quads {
                    start: start + quad_base,
                    len,
                },
                Run::Glyphs { start, len } => Run::Glyphs {
                    start: start + glyph_base,
                    len,
                },
                Run::Images { start, len } => Run::Images {
                    start: start + blit_base,
                    len,
                },
                Run::Gradients { start, len } => Run::Gradients {
                    start: start + gradient_base,
                    len,
                },
                Run::Surface(index) => Run::Surface(index),
            };
            self.runs.push(run);
        }

        self.quads.append(&mut source.quads);
        self.glyphs.append(&mut source.glyphs);
        self.blits.append(&mut source.blits);
        self.blit_keys.append(&mut source.blit_keys);
        self.blit_sources.append(&mut source.blit_sources);
        self.gradients.append(&mut source.gradients);
        self.unsupported += source.unsupported;

        std::ops::Range {
            start: run_start,
            end: self.runs.len() as u32,
        }
    }
}

/// Encodes a render target, including its format, as a blit source key.
pub(crate) fn target_key(target: RenderTargetId) -> u64 {
    let format = if target.format() == TargetFormat::Rgba16Float {
        TARGET_FORMAT_BIT
    } else {
        0
    };
    let raw = target.get() & !(TARGET_KEY_BIT | TARGET_FORMAT_BIT);
    TARGET_KEY_BIT | format | raw
}

pub(crate) struct SceneInput<'a> {
    pub items: &'a [RenderItem],
    pub clips: &'a [Option<sequoia_core::RoundedClip>],
    pub transforms: &'a [Transform],
    pub opacities: &'a [f32],
    pub scale: f32,
    pub viewport: Rect,
    /// `(paint item index, surface job index)` markers, sorted by item index.
    pub surface_marks: &'a [(usize, u32)],
}

pub(crate) fn build_scene(input: SceneInput<'_>, scene: &mut Scene) {
    scene.quads.clear();
    scene.glyphs.clear();
    scene.blits.clear();
    scene.blit_keys.clear();
    scene.blit_sources.clear();
    scene.gradients.clear();
    scene.runs.clear();
    scene.unsupported = 0;

    let scale = input.scale;
    let viewport = input.viewport;
    let opacity = |index: usize| {
        input
            .opacities
            .get(index)
            .copied()
            .unwrap_or(1.0)
            .clamp(0.0, 1.0)
    };
    let clip = |index: usize| -> ([f32; 4], f32) {
        match input.clips.get(index).copied().flatten() {
            Some(clip) if !clip.rect.is_empty() => (
                [
                    clip.rect.left() * scale,
                    clip.rect.top() * scale,
                    clip.rect.size.width * scale,
                    clip.rect.size.height * scale,
                ],
                clip.radius * scale,
            ),
            _ => ([0.0; 4], 0.0),
        }
    };
    let transform = |index: usize| -> ([f32; 4], [f32; 2]) {
        match input.transforms.get(index).copied() {
            Some(transform) => {
                let m = transform.m;
                ([m[0], m[1], m[2], m[3]], [m[4] * scale, m[5] * scale])
            }
            None => ([1.0, 0.0, 0.0, 1.0], [0.0, 0.0]),
        }
    };
    let cull = |rect: Rect, index: usize| -> bool {
        if viewport.is_empty() {
            return false;
        }
        let rect = match input.transforms.get(index).copied() {
            Some(transform) if !transform.is_identity() => transform.apply_rect_aabb(rect),
            _ => rect,
        };
        !rect.intersects(viewport)
    };

    let mut surface_mark = 0usize;
    for (index, item) in input.items.iter().enumerate() {
        while let Some(&(at, surface)) = input.surface_marks.get(surface_mark) {
            if at > index {
                break;
            }
            scene.runs.push(Run::Surface(surface));
            surface_mark += 1;
        }
        match item {
            RenderItem::Quad(quad) => {
                if cull(quad.rect, index) {
                    continue;
                }
                let (clip_rect, clip_radius) = clip(index);
                let (transform_a, transform_b) = transform(index);
                let mut color = linear(quad.color);
                color[3] *= opacity(index);
                let mut border_color = linear(quad.border_color);
                border_color[3] *= opacity(index);
                scene.quads.push(GpuQuad {
                    rect: phys_rect(quad.rect, scale),
                    color,
                    radius: quad.radius * scale,
                    border_color,
                    border_width: quad.border_width * scale,
                    clip_rect,
                    clip_radius,
                    softness: 0.0,
                    spread: 0.0,
                    shadow_offset: [0.0; 2],
                    inset: 0.0,
                    transform_a,
                    transform_b,
                });
                push_quad_run(scene);
            }
            RenderItem::Shadow(shadow) => {
                if cull(shadow.bounds(), index) {
                    continue;
                }
                let (clip_rect, clip_radius) = clip(index);
                let (transform_a, transform_b) = transform(index);
                let mut color = linear(shadow.color);
                color[3] *= opacity(index);
                let hard = shadow.blur <= 0.0 && !shadow.inset;
                let spread = if shadow.spread.is_finite() {
                    shadow.spread
                } else {
                    0.0
                };
                let shadow_rect = if hard {
                    Rect::from_xywh(
                        shadow.rect.left() + shadow.offset.x - spread,
                        shadow.rect.top() + shadow.offset.y - spread,
                        shadow.rect.size.width + spread * 2.0,
                        shadow.rect.size.height + spread * 2.0,
                    )
                } else {
                    shadow.rect
                };
                if shadow_rect.is_empty() {
                    continue;
                }
                scene.quads.push(GpuQuad {
                    rect: phys_rect(shadow_rect, scale),
                    color,
                    radius: (if hard {
                        (shadow.radius + spread).max(0.0)
                    } else {
                        shadow.radius
                    }) * scale,
                    border_color: [0.0; 4],
                    border_width: 0.0,
                    clip_rect,
                    clip_radius,
                    softness: if hard {
                        0.0
                    } else {
                        (shadow.blur * scale).max(0.0)
                    },
                    spread: if hard { 0.0 } else { shadow.spread * scale },
                    shadow_offset: if hard {
                        [0.0; 2]
                    } else {
                        [shadow.offset.x * scale, shadow.offset.y * scale]
                    },
                    inset: if hard {
                        0.0
                    } else if shadow.inset {
                        1.0
                    } else {
                        0.0
                    },
                    transform_a,
                    transform_b,
                });
                push_quad_run(scene);
            }
            RenderItem::Text(text) => {
                let start = scene.glyphs.len() as u32;
                let (clip_rect, clip_radius) = clip(index);
                let (transform_a, transform_b) = transform(index);
                let mut color = linear(text.color);
                color[3] *= opacity(index);
                for glyph in &text.glyphs {
                    if cull(glyph.rect, index) {
                        continue;
                    }
                    scene.glyphs.push(GpuGlyph {
                        rect: phys_rect(glyph.rect, scale),
                        uv: glyph.uv,
                        color,
                        color_glyph: if glyph.color_glyph { 1.0 } else { 0.0 },
                        clip_rect,
                        clip_radius,
                        transform_a,
                        transform_b,
                    });
                }
                let added = scene.glyphs.len() as u32 - start;
                if added == 0 {
                    continue;
                }
                match scene.runs.last_mut() {
                    Some(Run::Glyphs {
                        start: run_start,
                        len,
                    }) if *run_start + *len == start => {
                        *len += added;
                    }
                    _ => scene.runs.push(Run::Glyphs { start, len: added }),
                }
            }
            RenderItem::Image(image) => {
                if matches!(image.source, ImageSource::Cube(_)) || cull(image.rect, index) {
                    scene.unsupported += usize::from(matches!(image.source, ImageSource::Cube(_)));
                    continue;
                }
                let (clip_rect, clip_radius) = clip(index);
                let (transform_a, transform_b) = transform(index);
                scene.blits.push(BlitQuad {
                    rect: phys_rect(image.rect, scale),
                    uv: image.uv,
                    clip_rect,
                    clip_radius,
                    transform_a,
                    transform_b,
                    opacity: opacity(index),
                    nearest: if image.sampling == ImageSampling::Nearest {
                        1.0
                    } else {
                        0.0
                    },
                });
                scene.blit_keys.push(image.source.cache_key());
                scene.blit_sources.push(image.source.clone());
                match scene.runs.last_mut() {
                    Some(Run::Images {
                        start: run_start,
                        len,
                    }) if *run_start + *len == scene.blits.len() as u32 - 1 => {
                        *len += 1;
                    }
                    _ => scene.runs.push(Run::Images {
                        start: scene.blits.len() as u32 - 1,
                        len: 1,
                    }),
                }
            }
            RenderItem::Waveform(waveform) => {
                let (clip_rect, clip_radius) = clip(index);
                let (transform_a, transform_b) = transform(index);
                let mut color = linear(waveform.color);
                color[3] *= opacity(index);
                let start = scene.quads.len() as u32;
                for bar in &waveform.bars {
                    if cull(*bar, index) {
                        continue;
                    }
                    scene.quads.push(GpuQuad {
                        rect: phys_rect(*bar, scale),
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
                let added = scene.quads.len() as u32 - start;
                if added == 0 {
                    continue;
                }
                match scene.runs.last_mut() {
                    Some(Run::Quads {
                        start: run_start,
                        len,
                    }) if *run_start + *len == start => {
                        *len += added;
                    }
                    _ => scene.runs.push(Run::Quads { start, len: added }),
                }
            }
            RenderItem::Gradient(gradient) => {
                if cull(gradient.rect, index) {
                    continue;
                }
                let Some((mut colors, pos, count)) = gradient.stops.for_gpu() else {
                    scene.unsupported += 1;
                    continue;
                };
                let item_opacity = opacity(index);
                if item_opacity < 1.0 {
                    for color in &mut colors {
                        color[3] *= item_opacity;
                    }
                }
                let (clip_rect, clip_radius) = clip(index);
                let (transform_a, transform_b) = transform(index);
                let (kind, extent) = match gradient.kind {
                    sequoia_core::GradientKind::Linear { angle_deg } => {
                        let angle = if angle_deg.is_finite() {
                            angle_deg.to_radians()
                        } else {
                            0.0
                        };
                        (0.0, angle)
                    }
                    sequoia_core::GradientKind::Radial { radius } => {
                        let radius = if radius.is_finite() {
                            radius.max(0.0)
                        } else {
                            0.0
                        };
                        (1.0, radius * scale)
                    }
                };
                scene.gradients.push(GpuGradient {
                    rect: phys_rect(gradient.rect, scale),
                    stop0: colors[0],
                    stop1: colors[1],
                    stop2: colors[2],
                    stop3: colors[3],
                    stop_pos: pos,
                    params: [kind, extent, gradient.radius * scale, count as f32],
                    clip_rect,
                    clip_radius,
                    _pad: [0.0; 5],
                    transform_a,
                    transform_b,
                });
                match scene.runs.last_mut() {
                    Some(Run::Gradients {
                        start: run_start,
                        len,
                    }) if *run_start + *len == scene.gradients.len() as u32 - 1 => {
                        *len += 1;
                    }
                    _ => scene.runs.push(Run::Gradients {
                        start: scene.gradients.len() as u32 - 1,
                        len: 1,
                    }),
                }
            }
            RenderItem::Blit {
                rect, uv, target, ..
            } => {
                if cull(*rect, index) {
                    continue;
                }
                let (clip_rect, clip_radius) = clip(index);
                let (transform_a, transform_b) = transform(index);
                scene.blits.push(BlitQuad {
                    rect: phys_rect(*rect, scale),
                    uv: *uv,
                    clip_rect,
                    clip_radius,
                    transform_a,
                    transform_b,
                    opacity: opacity(index),
                    nearest: 0.0,
                });
                scene.blit_keys.push(target_key(*target));
                push_image_run(scene);
            }
            RenderItem::TargetBegin { .. } | RenderItem::TargetEnd { .. } => {}
            RenderItem::Shader(_) => {
                scene.unsupported += 1;
            }
            _ => {
                scene.unsupported += 1;
            }
        }
    }
    while let Some(&(_, surface)) = input.surface_marks.get(surface_mark) {
        scene.runs.push(Run::Surface(surface));
        surface_mark += 1;
    }
}

fn push_quad_run(scene: &mut Scene) {
    let index = scene.quads.len() as u32 - 1;
    match scene.runs.last_mut() {
        Some(Run::Quads { start, len }) if *start + *len == index => *len += 1,
        _ => scene.runs.push(Run::Quads {
            start: index,
            len: 1,
        }),
    }
}

fn push_image_run(scene: &mut Scene) {
    let index = scene.blits.len() as u32 - 1;
    match scene.runs.last_mut() {
        Some(Run::Images { start, len }) if *start + *len == index => *len += 1,
        _ => scene.runs.push(Run::Images {
            start: index,
            len: 1,
        }),
    }
}

fn phys_rect(rect: Rect, scale: f32) -> [f32; 4] {
    [
        rect.left() * scale,
        rect.top() * scale,
        rect.size.width * scale,
        rect.size.height * scale,
    ]
}

fn linear(color: Color) -> [f32; 4] {
    let color = color.to_linear();
    [color.r, color.g, color.b, color.a]
}

/// Builds the per-item opacity array used for non-isolated opacity groups.
pub(crate) fn item_opacities(list: &RenderList) -> Vec<f32> {
    let mut opacities = vec![1.0; list.items().len()];
    for span in list.opacity_spans() {
        let start = span.start.min(opacities.len());
        let end = span.end.min(opacities.len());
        for opacity in &mut opacities[start..end] {
            *opacity *= span.opacity;
        }
    }
    opacities
}

/// Decodes an image source into tightly packed RGBA8 pixels.
pub(crate) fn decode_image(source: &ImageSource) -> Option<(u32, u32, Vec<u8>)> {
    match source {
        ImageSource::File(path) => {
            if let Ok((width, height)) = image::image_dimensions(path)
                && !image_dims_allowed(width, height)
            {
                return None;
            }
            let image = image::open(path)
                .inspect_err(|error| {
                    log::warn!("sequoia-render-ash: cannot load image `{path}`: {error}");
                })
                .ok()?
                .to_rgba8();
            let (width, height) = image.dimensions();
            if !image_dims_allowed(width, height) {
                return None;
            }
            Some((width, height, image.into_raw()))
        }
        ImageSource::Rgba {
            width,
            height,
            data,
            ..
        }
        | ImageSource::Dynamic {
            width,
            height,
            data,
            ..
        } => {
            let (width, height) = (*width, *height);
            if !image_dims_allowed(width, height) {
                return None;
            }
            let expected = (width as usize)
                .checked_mul(height as usize)?
                .checked_mul(4)?;
            if data.len() < expected {
                return None;
            }
            Some((width, height, data[..expected].to_vec()))
        }
        ImageSource::Cube(_) => None,
        _ => None,
    }
}

fn image_dims_allowed(width: u32, height: u32) -> bool {
    if width == 0 || height == 0 || width > MAX_IMAGE_DIM || height > MAX_IMAGE_DIM {
        return false;
    }
    (width as usize)
        .checked_mul(height as usize)
        .and_then(|pixels| pixels.checked_mul(4))
        .is_some_and(|bytes| bytes <= MAX_IMAGE_BYTES)
}
