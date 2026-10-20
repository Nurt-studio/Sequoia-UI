#[allow(unused_imports)]
use std::any::Any;

use crate::types::*;
use crate::utils::*;
use sequoia_core::Color;

impl crate::renderer::GpuRenderer {
    pub(crate) fn backdrop_kernel_reach(blur_px: f32) -> f32 {
        if blur_px < 0.5 {
            1.0
        } else {
            2.0 * (blur_px * 0.45).clamp(1.0, 10.0) + 1.0
        }
    }

    pub(crate) fn backdrop_copy_region(
        rect: [f32; 4],
        blur_px: f32,
        fb: (u32, u32),
    ) -> Option<(wgpu::Origin3d, wgpu::Extent3d)> {
        let reach = Self::backdrop_kernel_reach(blur_px);
        let fw = fb.0.max(1) as f32;
        let fh = fb.1.max(1) as f32;
        let x0 = (rect[0] - reach).floor().clamp(0.0, fw) as u32;
        let y0 = (rect[1] - reach).floor().clamp(0.0, fh) as u32;
        let x1 = (rect[0] + rect[2] + reach).ceil().clamp(0.0, fw) as u32;
        let y1 = (rect[1] + rect[3] + reach).ceil().clamp(0.0, fh) as u32;
        if x1 <= x0 || y1 <= y0 {
            return None;
        }
        Some((
            wgpu::Origin3d { x: x0, y: y0, z: 0 },
            wgpu::Extent3d {
                width: x1 - x0,
                height: y1 - y0,
                depth_or_array_layers: 1,
            },
        ))
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn draw_backdrop_marker_at(
        backdrop_pipeline: &wgpu::RenderPipeline,
        bind_group: &wgpu::BindGroup,
        bufs: &BackdropBuffers,
        copy_origin: wgpu::Origin3d,
        copy_extent: wgpu::Extent3d,
        encoder: &mut wgpu::CommandEncoder,
        target_a: &wgpu::TextureView,
        backdrop_buf: &wgpu::Buffer,
        instance_idx: u32,
    ) {
        encoder.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &bufs.a_tex,
                mip_level: 0,
                origin: copy_origin,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyTextureInfo {
                texture: &bufs.b_tex,
                mip_level: 0,
                origin: copy_origin,
                aspect: wgpu::TextureAspect::All,
            },
            copy_extent,
        );
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("sequoia-backdrop"),
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
        pass.set_pipeline(backdrop_pipeline);
        pass.set_bind_group(0, bind_group, &[]);
        pass.set_bind_group(1, &bufs.b_bg, &[]);
        pass.set_vertex_buffer(0, backdrop_buf.slice(..));
        pass.draw(0..4, instance_idx..instance_idx + 1);
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn present_backdrop_buffer(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        present_buf: &mut wgpu::Buffer,
        present_cap: &mut usize,
        present_pipeline: &wgpu::RenderPipeline,
        bind_group: &wgpu::BindGroup,
        bufs: &BackdropBuffers,
        clear_color: Color,
        fb: (u32, u32),
        encoder: &mut wgpu::CommandEncoder,
        screen: &wgpu::TextureView,
    ) {
        let quad = BlitQuad {
            rect: [0.0, 0.0, fb.0.max(1) as f32, fb.1.max(1) as f32],
            uv: [0.0, 0.0, 1.0, 1.0],
            clip_rect: [0.0; 4],
            clip_radius: 0.0,
            transform_a: [1.0, 0.0, 0.0, 1.0],
            transform_b: [0.0, 0.0],
        };
        Self::ensure_capacity(device, present_buf, present_cap, 1, BLIT_BYTES);
        queue.write_buffer(present_buf, 0, bytemuck::cast_slice(&[quad]));
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("sequoia-present"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: screen,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(clear_color_linear(clear_color)),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(present_pipeline);
        pass.set_bind_group(0, bind_group, &[]);
        pass.set_bind_group(1, &bufs.a_bg, &[]);
        pass.set_vertex_buffer(0, present_buf.slice(..));
        pass.draw(0..4, 0..1);
    }
}
