//! Offscreen compositing for nested opacity groups.
//!
//! A group is rendered into a transparent layer and then composited once at
//! its group opacity, so overlapping children do not accumulate alpha.

#[allow(unused_imports)]
use std::any::Any;

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
    },
    Group {
        composite_idx: u32,
        children: Vec<OpacityDraw>,
    },
}

/// Renderer resources needed while recording opacity passes.
pub(crate) struct OpacityDrawContext<'a> {
    pub(crate) group_layers: &'a [GroupLayer],
    pub(crate) quad_pipeline: &'a wgpu::RenderPipeline,
    pub(crate) quad_pipeline_opaque: &'a wgpu::RenderPipeline,
    pub(crate) glyph_pipeline: &'a wgpu::RenderPipeline,
    pub(crate) blit_pipeline: &'a wgpu::RenderPipeline,
    pub(crate) gradient_pipeline: &'a wgpu::RenderPipeline,
    pub(crate) backdrop_pipeline: &'a wgpu::RenderPipeline,
    pub(crate) bind_group: &'a wgpu::BindGroup,
    pub(crate) atlas_bg: Option<&'a wgpu::BindGroup>,
    pub(crate) quad_buf: &'a wgpu::Buffer,
    pub(crate) glyph_buf: &'a wgpu::Buffer,
    pub(crate) image_buf: &'a wgpu::Buffer,
    pub(crate) gradient_buf: &'a wgpu::Buffer,
    pub(crate) backdrop_buf: &'a wgpu::Buffer,
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
                let (dq, dg, di, dgrad, culled) = group_runs(
                    &items[start..end],
                    &clips[start..end],
                    &transforms[start..end],
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
                );
                stats.dropped_quads += dq as u64;
                stats.dropped_glyphs += dg as u64;
                stats.dropped_images += di as u64;
                stats.dropped_gradients += dgrad as u64;
                stats.culled += culled as u64;

                let all_opaque = seg_gradients.is_empty() && seg_quads.iter().all(quad_is_opaque);
                let runs = shift_gui_runs(
                    &seg_runs,
                    quads.len() as u32,
                    glyphs.len() as u32,
                    images.len() as u32,
                    gradients.len() as u32,
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
                    });
                }
            }
            OpacityNode::Group { opacity, children } => {
                let child_draws = build_opacity_draws(
                    children, items, clips, transforms, scale, viewport, ceilings, quads, glyphs,
                    images, image_keys, gradients, stats, fb, composites,
                );
                if child_draws.is_empty() {
                    continue;
                }
                let composite_idx = composites.len() as u32;
                composites.push(opacity_composite_instance(fb, *opacity));
                out.push(OpacityDraw::Group {
                    composite_idx,
                    children: child_draws,
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
    image_draws: &[(u32, wgpu::BindGroup)],
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
            } => {
                if !*has_content || runs.is_empty() {
                    continue;
                }
                *passes += 1;
                *draws += gui_run_draws(runs, image_draws);
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
                crate::renderer::GpuRenderer::draw_gui_runs(
                    &mut pass,
                    ctx.quad_pipeline,
                    ctx.quad_pipeline_opaque,
                    ctx.glyph_pipeline,
                    ctx.blit_pipeline,
                    ctx.gradient_pipeline,
                    ctx.bind_group,
                    ctx.atlas_bg,
                    ctx.quad_buf,
                    quads,
                    ctx.glyph_buf,
                    ctx.image_buf,
                    ctx.gradient_buf,
                    runs,
                    image_draws,
                    *all_opaque,
                );
            }
            OpacityDraw::Group {
                composite_idx,
                children,
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
    }
}
