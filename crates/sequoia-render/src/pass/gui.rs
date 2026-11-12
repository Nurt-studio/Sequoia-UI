#[allow(unused_imports)]
use std::any::Any;

use crate::gpu::resources::UserShaderEntry;
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
        image_atlas_bg: Option<&wgpu::BindGroup>,
        quad_buf: &wgpu::Buffer,
        quad_insts: &[GpuQuad],
        glyph_buf: &wgpu::Buffer,
        image_buf: &wgpu::Buffer,
        gradient_buf: &wgpu::Buffer,
        shader_buf: &wgpu::Buffer,
        shader_instances: &[GpuShaderInstance],
        shader_bind_group: &wgpu::BindGroup,
        shader_textures: &[ShaderTextureGroups],
        user_shaders: &std::collections::HashMap<sequoia_core::ShaderId, UserShaderEntry>,
        runs: &[GuiRun],
        image_draws: &[Option<ImageBinding>],
        all_opaque: bool,
        shader_format: wgpu::TextureFormat,
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
                    let mut index = start as usize;
                    let end = (start + len) as usize;
                    let mut atlas_bound = false;
                    while index < end {
                        match image_draws.get(index) {
                            Some(Some(ImageBinding::Atlas)) => {
                                let Some(atlas_bg) = image_atlas_bg else {
                                    index += 1;
                                    atlas_bound = false;
                                    continue;
                                };
                                let run_start = index;
                                while index < end
                                    && matches!(
                                        image_draws.get(index),
                                        Some(Some(ImageBinding::Atlas))
                                    )
                                {
                                    index += 1;
                                }
                                if !atlas_bound {
                                    pass.set_bind_group(1, atlas_bg, &[]);
                                    atlas_bound = true;
                                }
                                pass.draw(0..4, run_start as u32..index as u32);
                            }
                            Some(Some(ImageBinding::Texture(bg))) => {
                                pass.set_bind_group(1, bg, &[]);
                                pass.draw(0..4, index as u32..index as u32 + 1);
                                atlas_bound = false;
                                index += 1;
                            }
                            _ => {
                                atlas_bound = false;
                                index += 1;
                            }
                        }
                    }
                }
                GuiRun::Shader(start, len, shader_id, blend) => {
                    let Some(entry) = user_shaders.get(&shader_id) else {
                        continue;
                    };
                    let Some(pipeline) =
                        entry.pipelines.get(&(blend, shader_format)).or_else(|| {
                            entry
                                .pipelines
                                .get(&(sequoia_core::BlendMode::Normal, shader_format))
                        })
                    else {
                        continue;
                    };
                    pass.set_pipeline(pipeline);
                    pass.set_vertex_buffer(0, shader_buf.slice(..));
                    for index in start..start + len {
                        let Some(instance) = shader_instances.get(index as usize) else {
                            break;
                        };
                        pass.set_bind_group(0, shader_bind_group, &[instance.uniform_offset]);
                        if let Some(groups) = shader_textures.get(index as usize) {
                            for (slot, group) in groups.iter().enumerate() {
                                pass.set_bind_group(slot as u32 + 1, group, &[]);
                            }
                        }
                        pass.draw(0..4, index..index + 1);
                    }
                }
            }
        }
    }
}
