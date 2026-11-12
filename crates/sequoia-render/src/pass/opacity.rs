//! Offscreen compositing for nested opacity groups.
//!
//! A group is rendered into a transparent layer and then composited once at
//! its group opacity, so overlapping children do not accumulate alpha.

#[allow(unused_imports)]
use std::any::Any;

use crate::gpu::resources::UserShaderEntry;
use crate::scene::grouping::*;
use crate::scene::instances::opacity_composite_instance;
use crate::types::*;
use sequoia_core::{OpacityNode, Rect, RenderItem, RoundedClip, Transform};

/// One prepared opacity-plan node with runs already shifted into the shared
/// instance buffers.
pub(crate) enum OpacityDraw {
    Segment {
        runs: Vec<GuiRun>,
        all_opaque: bool,
        has_content: bool,
        /// Physical-pixel bounds of the segment content, or `None` when the
        /// segment has to be composited over the whole framebuffer.
        scissor: Option<[f32; 4]>,
    },
    Group {
        composite_idx: u32,
        children: Vec<OpacityDraw>,
        scissor: Option<[f32; 4]>,
    },
}

impl OpacityDraw {
    fn scissor(&self) -> Option<[f32; 4]> {
        match *self {
            Self::Segment { scissor, .. } | Self::Group { scissor, .. } => scissor,
        }
    }
}

/// Renderer resources needed while recording opacity passes.
pub(crate) struct OpacityDrawContext<'a> {    pub(crate) group_layers: &'a [GroupLayer],
    pub(crate) quad_pipeline: &'a wgpu::RenderPipeline,
    pub(crate) quad_pipeline_opaque: &'a wgpu::RenderPipeline,
    pub(crate) glyph_pipeline: &'a wgpu::RenderPipeline,
    pub(crate) blit_pipeline: &'a wgpu::RenderPipeline,
    pub(crate) gradient_pipeline: &'a wgpu::RenderPipeline,
    pub(crate) backdrop_pipeline: &'a wgpu::RenderPipeline,
    pub(crate) bind_group: &'a wgpu::BindGroup,
    pub(crate) atlas_bg: Option<&'a wgpu::BindGroup>,
    pub(crate) image_atlas_bg: Option<&'a wgpu::BindGroup>,
    pub(crate) quad_buf: &'a wgpu::Buffer,
    pub(crate) glyph_buf: &'a wgpu::Buffer,
    pub(crate) image_buf: &'a wgpu::Buffer,
    pub(crate) gradient_buf: &'a wgpu::Buffer,
    pub(crate) shader_buf: &'a wgpu::Buffer,
    pub(crate) shader_instances: &'a [GpuShaderInstance],
    pub(crate) shader_bind_group: &'a wgpu::BindGroup,
    pub(crate) shader_textures: &'a [ShaderTextureGroups],
    pub(crate) user_shaders: &'a std::collections::HashMap<sequoia_core::ShaderId, UserShaderEntry>,
    pub(crate) backdrop_buf: &'a wgpu::Buffer,
    pub(crate) fb: (u32, u32),
    pub(crate) format: wgpu::TextureFormat,
}

/// Conservative physical-pixel bounds for one opacity segment or group.
#[derive(Clone, Copy)]
struct Bounds {
    min: [f32; 2],
    max: [f32; 2],
    valid: bool,
    full: bool,
}

impl Bounds {
    fn new() -> Self {
        Self {
            min: [f32::INFINITY; 2],
            max: [f32::NEG_INFINITY; 2],
            valid: false,
            full: false,
        }
    }

    fn full() -> Self {
        Self {
            valid: false,
            full: true,
            ..Self::new()
        }
    }

    fn include_rect(&mut self, rect: [f32; 4]) {
        let x0 = rect[0] - 2.0;
        let y0 = rect[1] - 2.0;
        let x1 = rect[0] + rect[2] + 2.0;
        let y1 = rect[1] + rect[3] + 2.0;
        if !x0.is_finite() || !y0.is_finite() || !x1.is_finite() || !y1.is_finite() {
            self.full = true;
            return;
        }
        if !self.valid {
            self.min = [x0, y0];
            self.max = [x1, y1];
            self.valid = true;
        } else {
            self.min = [self.min[0].min(x0), self.min[1].min(y0)];
            self.max = [self.max[0].max(x1), self.max[1].max(y1)];
        }
    }

    fn scissor(self) -> Option<[f32; 4]> {
        if self.full || !self.valid {
            None
        } else {
            Some([self.min[0], self.min[1], self.max[0], self.max[1]])
        }
    }
}

fn identity_transform(a: [f32; 4], b: [f32; 2]) -> bool {
    (a[0] - 1.0).abs() < 1e-6
        && a[1].abs() < 1e-6
        && a[2].abs() < 1e-6
        && (a[3] - 1.0).abs() < 1e-6
        && b[0].abs() < 1e-6
        && b[1].abs() < 1e-6
}

fn segment_scissor(
    quads: &[GpuQuad],
    glyphs: &[GpuGlyph],
    images: &[BlitQuad],
    gradients: &[GpuGradient],
    shaders: &[GpuShaderInstance],
) -> Option<[f32; 4]> {
    let mut bounds = Bounds::new();
    for quad in quads {
        if !identity_transform(quad.transform_a, quad.transform_b)
            || quad.softness != 0.0
            || quad.spread != 0.0
            || quad.shadow_offset != [0.0, 0.0]
        {
            return None;
        }
        bounds.include_rect(quad.rect);
    }
    for glyph in glyphs {
        if !identity_transform(glyph.transform_a, glyph.transform_b) {
            return None;
        }
        bounds.include_rect(glyph.rect);
    }
    for image in images {
        if !identity_transform(image.transform_a, image.transform_b) {
            return None;
        }
        bounds.include_rect(image.rect);
    }
    for gradient in gradients {
        if !identity_transform(gradient.transform_a, gradient.transform_b) {
            return None;
        }
        bounds.include_rect(gradient.rect);
    }
    for shader in shaders {
        if !identity_transform(shader.transform_a, shader.transform_b) {
            return None;
        }
        bounds.include_rect(shader.rect);
    }
    bounds.scissor()
}

fn scissor_px(bounds: [f32; 4], fb: (u32, u32)) -> [u32; 4] {
    let fw = fb.0.max(1) as f32;
    let fh = fb.1.max(1) as f32;
    let x0 = bounds[0].floor().clamp(0.0, fw) as u32;
    let y0 = bounds[1].floor().clamp(0.0, fh) as u32;
    let x1 = bounds[2].ceil().clamp(0.0, fw) as u32;
    let y1 = bounds[3].ceil().clamp(0.0, fh) as u32;
    [x0, y0, x1.saturating_sub(x0), y1.saturating_sub(y0)]
}

/// Returns the maximum number of nested groups in `nodes`.
pub(crate) fn opacity_plan_depth(nodes: &[OpacityNode]) -> usize {
    nodes
        .iter()
        .map(|node| match node {
            OpacityNode::Group { children, .. } => 1 + opacity_plan_depth(children),
            _ => 0,
        })
        .max()
        .unwrap_or(0)
}

/// Converts an opacity plan into draw plans while filling the shared instance
/// buffers.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_opacity_draws(
    nodes: &[OpacityNode],
    items: &[RenderItem],
    clips: &[Option<RoundedClip>],
    transforms: &[Transform],
    scale: f32,
    viewport: Option<Rect>,
    ceilings: (usize, usize, usize, usize),
    quads: &mut Vec<GpuQuad>,
    glyphs: &mut Vec<GpuGlyph>,
    images: &mut Vec<BlitQuad>,
    image_keys: &mut Vec<u64>,
    gradients: &mut Vec<GpuGradient>,
    shaders: &mut ShaderGroupData,
    stats: &mut RenderStats,
    fb: (u32, u32),
    composites: &mut Vec<GpuBackdrop>,
) -> Vec<OpacityDraw> {
    let mut out = Vec::new();
    for node in nodes {
        match node {
            OpacityNode::Segment { start, end } => {
                let start = (*start).min(items.len());
                let end = (*end).min(items.len());
                if start >= end {
                    continue;
                }

                let mut seg_quads: Vec<GpuQuad> = Vec::new();
                let mut seg_glyphs: Vec<GpuGlyph> = Vec::new();
                let mut seg_images: Vec<BlitQuad> = Vec::new();
                let mut seg_keys: Vec<u64> = Vec::new();
                let mut seg_gradients: Vec<GpuGradient> = Vec::new();
                let mut seg_runs: Vec<GuiRun> = Vec::new();
                let shader_base = shaders.instances.len() as u32;
                let (dq, dg, di, dgrad, culled) = group_runs_with_opacity(
                    &items[start..end],
                    &clips[start..end],
                    &transforms[start..end],
                    &[],
                    scale,
                    viewport,
                    ceilings.0,
                    ceilings.1,
                    ceilings.2,
                    ceilings.3,
                    &mut seg_quads,
                    &mut seg_glyphs,
                    &mut seg_images,
                    &mut seg_keys,
                    &mut seg_gradients,
                    &mut seg_runs,
                    Some(shaders),
                );
                stats.dropped_quads += dq as u64;
                stats.dropped_glyphs += dg as u64;
                stats.dropped_images += di as u64;
                stats.dropped_gradients += dgrad as u64;
                stats.culled += culled as u64;

                let all_opaque = seg_gradients.is_empty() && seg_quads.iter().all(quad_is_opaque);
                let shader_end = shaders.instances.len();
                let scissor = segment_scissor(
                    &seg_quads,
                    &seg_glyphs,
                    &seg_images,
                    &seg_gradients,
                    &shaders.instances[shader_base as usize..shader_end],
                );
                let runs = shift_gui_runs_with_shaders(
                    &seg_runs,
                    quads.len() as u32,
                    glyphs.len() as u32,
                    images.len() as u32,
                    gradients.len() as u32,
                    shader_base,
                );
                quads.extend_from_slice(&seg_quads);
                glyphs.extend_from_slice(&seg_glyphs);
                images.extend_from_slice(&seg_images);
                image_keys.extend_from_slice(&seg_keys);
                gradients.extend_from_slice(&seg_gradients);

                if !runs.is_empty() {
                    out.push(OpacityDraw::Segment {
                        runs,
                        all_opaque,
                        has_content: true,
                        scissor,
                    });
                }
            }
            OpacityNode::Group { opacity, children } => {
                let child_draws = build_opacity_draws(
                    children, items, clips, transforms, scale, viewport, ceilings, quads, glyphs,
                    images, image_keys, gradients, shaders, stats, fb, composites,
                );
                if child_draws.is_empty() {
                    continue;
                }
                let mut bounds = Bounds::new();
                for child in &child_draws {
                    match child.scissor() {
                        Some(rect) => bounds.include_rect([
                            rect[0],
                            rect[1],
                            rect[2] - rect[0],
                            rect[3] - rect[1],
                        ]),
                        None => bounds = Bounds::full(),
                    }
                    if bounds.full {
                        break;
                    }
                }
                let composite_idx = composites.len() as u32;
                composites.push(opacity_composite_instance(fb, *opacity));
                out.push(OpacityDraw::Group {
                    composite_idx,
                    children: child_draws,
                    scissor: bounds.scissor(),
                });
            }
            _ => {}
        }
    }
    out
}

/// Records one opacity segment or group into `target`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_opacity_draws(
    nodes: &[OpacityDraw],
    target: &wgpu::TextureView,
    depth: usize,
    quads: &[GpuQuad],
    image_draws: &[Option<ImageBinding>],
    encoder: &mut wgpu::CommandEncoder,
    passes: &mut u64,
    draws: &mut u64,
    ctx: &OpacityDrawContext<'_>,
) {
    for node in nodes {
        match node {
            OpacityDraw::Segment {
                runs,
                all_opaque,
                has_content,
                scissor,
            } => {
                if !*has_content || runs.is_empty() {
                    continue;
                }
                let px = scissor.map(|bounds| scissor_px(bounds, ctx.fb));
                if let Some([_, _, w, h]) = px {
                    if w == 0 || h == 0 {
                        continue;
                    }
                }
                *passes += 1;
                *draws += gui_run_draws(runs, image_draws, ctx.image_atlas_bg.is_some());
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("sequoia-opacity-segment"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: target,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                if let Some([x, y, w, h]) = px {
                    pass.set_scissor_rect(x, y, w, h);
                }
                crate::renderer::GpuRenderer::draw_gui_runs(
                    &mut pass,
                    ctx.quad_pipeline,
                    ctx.quad_pipeline_opaque,
                    ctx.glyph_pipeline,
                    ctx.blit_pipeline,
                    ctx.gradient_pipeline,
                    ctx.bind_group,
                    ctx.atlas_bg,
                    ctx.image_atlas_bg,
                    ctx.quad_buf,
                    quads,
                    ctx.glyph_buf,
                    ctx.image_buf,
                    ctx.gradient_buf,
                    ctx.shader_buf,
                    ctx.shader_instances,
                    ctx.shader_bind_group,
                    ctx.shader_textures,
                    ctx.user_shaders,
                    runs,
                    image_draws,
                    *all_opaque,
                    ctx.format,
                );
            }
            OpacityDraw::Group {
                composite_idx,
                children,
                scissor,
            } => {
                let Some(layer) = ctx.group_layers.get(depth) else {
                    continue;
                };
                *passes += 1;
                {
                    let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("sequoia-opacity-layer-clear"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &layer.view,
                            depth_slice: None,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                        multiview_mask: None,
                    });
                    drop(pass);
                }
                draw_opacity_draws(
                    children,
                    &layer.view,
                    depth + 1,
                    quads,
                    image_draws,
                    encoder,
                    passes,
                    draws,
                    ctx,
                );

                *passes += 1;
                *draws += 1;
                let px = scissor.map(|bounds| scissor_px(bounds, ctx.fb));
                if let Some([_, _, w, h]) = px {
                    if w == 0 || h == 0 {
                        continue;
                    }
                }
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("sequoia-opacity-composite"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: target,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                if let Some([x, y, w, h]) = px {
                    pass.set_scissor_rect(x, y, w, h);
                }
                pass.set_pipeline(ctx.backdrop_pipeline);
                pass.set_bind_group(0, ctx.bind_group, &[]);
                pass.set_bind_group(1, &layer.bg, &[]);
                pass.set_vertex_buffer(0, ctx.backdrop_buf.slice(..));
                pass.draw(0..4, *composite_idx..*composite_idx + 1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sequoia_core::{Color, RenderList};

    fn quad(list: &mut RenderList, x: f32) {
        list.push_quad(Rect::from_xywh(x, 0.0, 10.0, 10.0), Color::RED, 0.0);
    }

    #[test]
    fn nested_plan_builds_layers_and_segments() {
        let mut list = RenderList::new();
        quad(&mut list, 0.0);
        list.push_opacity(0.5);
        quad(&mut list, 10.0);
        list.push_opacity(0.5);
        quad(&mut list, 20.0);
        list.pop_opacity();
        list.pop_opacity();

        let plan = list.opacity_plan();
        assert_eq!(opacity_plan_depth(&plan), 2);

        let mut quads = Vec::new();
        let mut glyphs = Vec::new();
        let mut images = Vec::new();
        let mut keys = Vec::new();
        let mut gradients = Vec::new();
        let mut shaders = ShaderGroupData::default();
        let mut stats = RenderStats::default();
        let mut composites = Vec::new();
        let draws = build_opacity_draws(
            &plan,
            list.items(),
            list.item_clips(),
            list.item_transforms(),
            1.0,
            None,
            (1024, 4096, 64, 32),
            &mut quads,
            &mut glyphs,
            &mut images,
            &mut keys,
            &mut gradients,
            &mut shaders,
            &mut stats,
            (100, 100),
            &mut composites,
        );

        assert_eq!(draws.len(), 2, "leading segment and outer group");
        assert!(matches!(
            draws[0],
            OpacityDraw::Segment {
                has_content: true,
                ..
            }
        ));
        let OpacityDraw::Group { children, .. } = &draws[1] else {
            panic!("expected outer group");
        };
        assert_eq!(children.len(), 2);
        assert_eq!(composites.len(), 2, "inner and outer group layers");
        assert_eq!(quads.len(), 3);
        assert!(
            draws.iter().all(|draw| draw.scissor().is_some()),
            "untransformed content gets a scissor"
        );
        let leading = draws[0].scissor().expect("leading segment scissor");
        assert!(leading[0] <= 0.0 && leading[2] >= 10.0);
        let outer = draws[1].scissor().expect("outer group scissor");
        assert!(
            outer[0] <= 10.0 && outer[2] >= 30.0,
            "group scissor covers every child"
        );
    }
}
