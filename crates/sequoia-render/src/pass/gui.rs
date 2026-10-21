#[allow(unused_imports)]
use std::any::Any;

use crate::scene::grouping::quad_range_is_opaque;
use crate::types::*;

impl crate::renderer::GpuRenderer {
    /// Records all instanced runs in order, selecting a pipeline for each.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn draw_gui_runs(
        pass: &mut wgpu::RenderPass<'_>,
        quad_pipeline: &wgpu::RenderPipeline,
        quad_pipeline_opaque: &wgpu::RenderPipeline,
        glyph_pipeline: &wgpu::RenderPipeline,
        blit_pipeline: &wgpu::RenderPipeline,
        gradient_pipeline: &wgpu::RenderPipeline,
        bind_group: &wgpu::BindGroup,
        atlas_bg: Option<&wgpu::BindGroup>,
        quad_buf: &wgpu::Buffer,
        quad_insts: &[GpuQuad],
        glyph_buf: &wgpu::Buffer,
        image_buf: &wgpu::Buffer,
        gradient_buf: &wgpu::Buffer,
        runs: &[GuiRun],
        image_draws: &[(u32, wgpu::BindGroup)],
        all_opaque: bool,
    ) {
        for run in runs.iter() {
            match *run {
                GuiRun::Quads(start, len) => {
                    let opaque = all_opaque || quad_range_is_opaque(quad_insts, start, len);
                    pass.set_pipeline(if opaque {
                        quad_pipeline_opaque
                    } else {
                        quad_pipeline
                    });
                    pass.set_bind_group(0, bind_group, &[]);
                    pass.set_vertex_buffer(0, quad_buf.slice(..));
                    pass.draw(0..4, start..start + len);
                }
                GuiRun::Gradients(start, len) => {
                    pass.set_pipeline(gradient_pipeline);
                    pass.set_bind_group(0, bind_group, &[]);
                    pass.set_vertex_buffer(0, gradient_buf.slice(..));
                    pass.draw(0..4, start..start + len);
                }
                GuiRun::Glyphs(start, len) => {
                    if let Some(bg) = atlas_bg {
                        pass.set_pipeline(glyph_pipeline);
                        pass.set_bind_group(0, bind_group, &[]);
                        pass.set_bind_group(1, bg, &[]);
                        pass.set_vertex_buffer(0, glyph_buf.slice(..));
                        pass.draw(0..4, start..start + len);
                    }
                }
                GuiRun::Images(start, len) => {
                    if image_draws.is_empty() {
                        continue;
                    }
                    pass.set_pipeline(blit_pipeline);
                    pass.set_bind_group(0, bind_group, &[]);
                    pass.set_vertex_buffer(0, image_buf.slice(..));
                    for (idx, bg) in image_draws.iter() {
                        if *idx < start || *idx >= start + len {
                            continue;
                        }
                        pass.set_bind_group(1, bg, &[]);
                        pass.draw(0..4, *idx..*idx + 1);
                    }
                }
            }
        }
    }
}
