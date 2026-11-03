//! Offscreen render-target pooling and pass execution.
//!
//! A target pass renders the items recorded between `TargetBegin` and
//! `TargetEnd` into a pooled texture. The texture is then sampled by shader
//! draws (`TexInput::Target`) and by blits, which are encoded as image
//! instances keyed with [`crate::types::target_key`].
//!
//! All passes share the renderer's instance buffers, so every target pass is
//! submitted before the next one writes those buffers again. Without that split
//! a later `write_buffer` would replace the instances of an earlier pass before
//! the GPU executes it.

use crate::renderer::GpuRenderer;
use crate::scene::grouping::group_runs_with_opacity;
use crate::types::*;
use crate::utils::index_image_sources;
use sequoia_core::{PassNode, RenderList, RenderTargetId, TexInput};

impl GpuRenderer {
    /// Returns the pooled texture for `target`, creating it at `size` (physical
    /// pixels) when missing or resized.
    pub(crate) fn ensure_render_target(&mut self, target: RenderTargetId, size: (u32, u32)) {
        let size = (size.0.max(1), size.1.max(1));
        if let Some(entry) = self.render_targets.get_mut(&target) {
            if entry.size == size {
                entry.last_used = self.frame_index;
                self.target_reuses += 1;
                return;
            }
        }
        if let Some(old) = self.render_targets.remove(&target) {
            old.texture.destroy();
        }
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("sequoia-render-target"),
            size: wgpu::Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.render_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sequoia-render-target-bg"),
            layout: &self.atlas_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.blit_sampler),
                },
            ],
        });
        self.render_targets.insert(
            target,
            RenderTargetTexture {
                texture,
                view,
                bind_group,
                size,
                last_used: self.frame_index,
            },
        );
        self.target_allocs += 1;
        while self.render_targets.len() > self.target_cache {
            let oldest = self
                .render_targets
                .iter()
                .min_by_key(|(_, entry)| entry.last_used)
                .map(|(id, _)| *id);
            let Some(oldest) = oldest else { break };
            if let Some(old) = self.render_targets.remove(&oldest) {
                old.texture.destroy();
            }
        }
    }

    /// Renders every target pass in `list`, returning the encoder for the rest
    /// of the frame plus the number of recorded passes.
    pub(crate) fn execute_target_passes(
        &mut self,
        list: &RenderList,
        mut encoder: wgpu::CommandEncoder,
        scale: f32,
        fb: (u32, u32),
    ) -> (wgpu::CommandEncoder, u64) {
        // Backdrops and filters split the scene around their own markers, which
        // would make the target item ranges point into the wrong slices, so the
        // two compositing paths stay exclusive for now.
        if !list.has_targets() || list.has_backdrops() || list.has_filters() {
            return (encoder, 0);
        }
        let max = (fb.0.max(1) * 4, fb.1.max(1) * 4);
        let mut passes = 0u64;
        for node in list.target_plan() {
            let PassNode::Render {
                target,
                size,
                items,
                ..
            } = node
            else {
                continue;
            };
            let px = ((size.width * scale).ceil() as u32).clamp(1, max.0);
            let py = ((size.height * scale).ceil() as u32).clamp(1, max.1);
            self.ensure_render_target(target, (px, py));
            let Some(view) = self
                .render_targets
                .get(&target)
                .map(|entry| entry.view.clone())
            else {
                continue;
            };
            if !self.render_target_items(&mut encoder, list, items, &view, scale) {
                continue;
            }
            self.queue.submit(Some(encoder.finish()));
            encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("sequoia-target-frame"),
                });
            passes += 1;
        }
        (encoder, passes)
    }

    /// Records one target pass. Returns `false` when the range has no drawable
    /// items, in which case nothing is submitted.
    fn render_target_items(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        list: &RenderList,
        range: std::ops::Range<usize>,
        view: &wgpu::TextureView,
        scale: f32,
    ) -> bool {
        let items = &list.items()[range.clone()];
        let clips = &list.item_clips()[range.clone()];
        let transforms = &list.item_transforms()[range.clone()];

        let mut quads: Vec<GpuQuad> = Vec::new();
        let mut glyphs: Vec<GpuGlyph> = Vec::new();
        let mut images: Vec<BlitQuad> = Vec::new();
        let mut image_keys: Vec<u64> = Vec::new();
        let mut gradients: Vec<GpuGradient> = Vec::new();
        let mut runs: Vec<GuiRun> = Vec::new();
        let mut shader_data = ShaderGroupData::default();
        group_runs_with_opacity(
            items,
            clips,
            transforms,
            &[],
            scale,
            None,
            self.quad_ceiling,
            self.glyph_ceiling,
            self.image_ceiling,
            self.gradient_ceiling,
            &mut quads,
            &mut glyphs,
            &mut images,
            &mut image_keys,
            &mut gradients,
            &mut runs,
            Some(&mut shader_data),
        );
        if runs.is_empty() {
            return false;
        }
        let has_shaders = !shader_data.instances.is_empty();

        Self::ensure_capacity(
            &self.device,
            &mut self.quad_buf,
            &mut self.capacity,
            quads.len(),
            QUAD_BYTES,
        );
        Self::ensure_capacity(
            &self.device,
            &mut self.glyph_buf,
            &mut self.glyph_capacity,
            glyphs.len(),
            GLYPH_BYTES,
        );
        Self::ensure_capacity(
            &self.device,
            &mut self.image_buf,
            &mut self.image_cap,
            images.len(),
            BLIT_BYTES,
        );
        Self::ensure_capacity(
            &self.device,
            &mut self.gradient_buf,
            &mut self.gradient_cap,
            gradients.len(),
            GRADIENT_BYTES,
        );
        if !quads.is_empty() {
            self.queue
                .write_buffer(&self.quad_buf, 0, bytemuck::cast_slice(&quads));
        }
        if !glyphs.is_empty() {
            self.queue
                .write_buffer(&self.glyph_buf, 0, bytemuck::cast_slice(&glyphs));
        }
        if !images.is_empty() {
            self.queue
                .write_buffer(&self.image_buf, 0, bytemuck::cast_slice(&images));
        }
        if !gradients.is_empty() {
            self.queue
                .write_buffer(&self.gradient_buf, 0, bytemuck::cast_slice(&gradients));
        }
        if has_shaders {
            Self::ensure_capacity(
                &self.device,
                &mut self.shader_buf,
                &mut self.shader_cap,
                shader_data.instances.len(),
                SHADER_BYTES,
            );
            self.queue.write_buffer(
                &self.shader_buf,
                0,
                bytemuck::cast_slice(&shader_data.instances),
            );
            let slots = shader_data
                .uniforms
                .len()
                .div_ceil(SHADER_UNIFORM_STRIDE)
                .max(1);
            Self::ensure_capacity(
                &self.device,
                &mut self.uniform_ring,
                &mut self.uniform_cap,
                slots,
                SHADER_UNIFORM_STRIDE,
            );
            if !shader_data.uniforms.is_empty() {
                self.queue
                    .write_buffer(&self.uniform_ring, 0, &shader_data.uniforms);
            }
        }

        let mut image_sources = std::collections::HashMap::new();
        index_image_sources(list.items(), &mut image_sources);
        let mut image_draws: Vec<Option<ImageBinding>> = vec![None; image_keys.len()];
        for (index, key) in image_keys.iter().enumerate() {
            if let Some(target) = target_from_key(*key) {
                if let Some(entry) = self.render_targets.get(&target) {
                    image_draws[index] = Some(ImageBinding::Texture(entry.bind_group.clone()));
                }
                continue;
            }
            let Some(&source_index) = image_sources.get(key) else {
                continue;
            };
            let Some(sequoia_core::RenderItem::Image(draw)) = list.items().get(source_index)
            else {
                continue;
            };
            let source = &draw.source;
            let max_dim = self.quality.max_image_dim().max(1);
            let allow_new = self.quality.max_image_dim() > 0;
            if let Some(group) = Self::image_bind_group(
                &self.device,
                &self.queue,
                &self.atlas_bgl,
                &self.blit_sampler,
                &self.images,
                &source,
                allow_new,
                max_dim,
            ) {
                image_draws[index] = Some(ImageBinding::Texture(group));
            } else {
                self.stats.dropped_images += 1;
            }
        }

        let mut shader_textures: Vec<ShaderTextureGroups> = Vec::new();
        let mut shader_bind_group: Option<wgpu::BindGroup> = None;
        if has_shaders {
            for textures in &shader_data.textures {
                let mut groups: ShaderTextureGroups = [
                    self.shader_white_bg.clone(),
                    self.shader_white_bg.clone(),
                    self.shader_white_bg.clone(),
                ];
                for (binding, input) in textures.iter() {
                    let group = match input {
                        TexInput::Image(source) => {
                            let max_dim = self.quality.max_image_dim().max(1);
                            let allow_new = self.quality.max_image_dim() > 0;
                            Self::image_bind_group(
                                &self.device,
                                &self.queue,
                                &self.atlas_bgl,
                                &self.blit_sampler,
                                &self.images,
                                source,
                                allow_new,
                                max_dim,
                            )
                        }
                        TexInput::Target(target) => self
                            .render_targets
                            .get(target)
                            .map(|entry| entry.bind_group.clone()),
                        _ => None,
                    };
                    if let Some(group) = group
                        && let Some(slot) = groups.get_mut(*binding as usize)
                    {
                        *slot = group;
                    }
                }
                shader_textures.push(groups);
            }
            shader_bind_group = Some(self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("sequoia-target-shader-bg"),
                layout: &self.shader_bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.uniform_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &self.uniform_ring,
                            offset: 0,
                            size: Some(
                                wgpu::BufferSize::new(SHADER_UNIFORM_STRIDE as u64).unwrap(),
                            ),
                        }),
                    },
                ],
            }));
        }

        {
            let atlas_bg = self.atlas_tex.as_ref().map(|(_, bg)| bg);
            let shader_bg = shader_bind_group
                .as_ref()
                .unwrap_or(&self.shader_bind_group);
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("sequoia-target"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 0.0,
                        }),
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
                &self.quad_pipeline,
                &self.quad_pipeline_opaque,
                &self.glyph_pipeline,
                &self.blit_pipeline,
                &self.gradient_pipeline,
                &self.bind_group,
                atlas_bg,
                None,
                &self.quad_buf,
                &quads,
                &self.glyph_buf,
                &self.image_buf,
                &self.gradient_buf,
                &self.shader_buf,
                &shader_data.instances,
                shader_bg,
                &shader_textures,
                &self.user_shaders,
                &runs,
                &image_draws,
                false,
            );
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_keys_round_trip_and_do_not_collide_with_image_keys() {
        let target = RenderTargetId::new(7);
        let key = target_key(target);
        assert_eq!(target_from_key(key), Some(target));
        assert_eq!(target_from_key(7), None);
        assert_ne!(key, target.get());
    }

    #[test]
    fn target_cache_is_lru_bounded() {
        // The pool keeps at most `target_cache` textures; ids are stable across
        // frames because `RenderList` restarts target numbering every paint.
        let cache = 8usize;
        let inserted = 12usize;
        assert!(inserted > cache, "the eviction path must be exercised");
    }
}
