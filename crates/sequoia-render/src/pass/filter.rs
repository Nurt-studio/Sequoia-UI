#[allow(unused_imports)]
use std::any::Any;

use crate::types::*;

impl crate::renderer::GpuRenderer {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn draw_filter_at(
        filter_pipeline: &wgpu::RenderPipeline,
        bind_group: &wgpu::BindGroup,
        quad_pipeline: &wgpu::RenderPipeline,
        quad_pipeline_opaque: &wgpu::RenderPipeline,
        glyph_pipeline: &wgpu::RenderPipeline,
        blit_pipeline: &wgpu::RenderPipeline,
        gradient_pipeline: &wgpu::RenderPipeline,
        atlas_bg: Option<&wgpu::BindGroup>,
        bufs: &BackdropBuffers,
        encoder: &mut wgpu::CommandEncoder,
        target_a: &wgpu::TextureView,
        _quad_insts: &[GpuQuad],
        backdrop_buf: &wgpu::Buffer,
        instance_idx: u32,
        quad_buf: &wgpu::Buffer,
        glyph_buf: &wgpu::Buffer,
        image_buf: &wgpu::Buffer,
        gradient_buf: &wgpu::Buffer,
        runs: &[GuiRun],
        image_draws: &[(u32, wgpu::BindGroup)],
        all_opaque: bool,
    ) {
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("sequoia-filter-content"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &bufs.b_view,
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
            Self::draw_gui_runs(
                &mut pass,
                quad_pipeline,
                quad_pipeline_opaque,
                glyph_pipeline,
                blit_pipeline,
                gradient_pipeline,
                bind_group,
                atlas_bg,
                quad_buf,
                _quad_insts,
                glyph_buf,
                image_buf,
                gradient_buf,
                runs,
                image_draws,
                all_opaque,
            );
        }

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("sequoia-filter-composite"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target_a,
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
            pass.set_pipeline(filter_pipeline);
            pass.set_bind_group(0, bind_group, &[]);
            pass.set_bind_group(1, &bufs.b_bg, &[]);
            pass.set_vertex_buffer(0, backdrop_buf.slice(..));
            pass.draw(0..4, instance_idx..instance_idx + 1);
        }
    }
}
