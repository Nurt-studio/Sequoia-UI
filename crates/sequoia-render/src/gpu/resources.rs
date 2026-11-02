//! Buffer, texture, atlas, and image-cache management.
//!
//! Resources are created lazily and reused across frames; instance buffers grow
//! geometrically up to the device's maximum buffer size.

#[allow(unused_imports)]
use std::any::Any;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex};

use sequoia_text::GlyphAtlas;

use crate::surface::sanitize_samples;
use crate::types::*;
use crate::utils::*;
use sequoia_core::{BlendMode, ImageSource};

/// Byte- and count-bounded LRU cache of decoded image textures.
///
/// The cache can be shared by several renderers that use the same wgpu device,
/// which lets multi-window applications upload an image once. Textures are
/// destroyed when the last reference to the cache is dropped.
pub(crate) struct ImageCache {
    entries: HashMap<u64, GpuImage>,
    order: VecDeque<u64>,
    failures: HashSet<u64>,
    bytes: u64,
    max_bytes: u64,
    max_textures: usize,
    evictions: u64,
    rejected: u64,
}

impl ImageCache {
    pub(crate) fn new(max_bytes: u64, max_textures: usize) -> Self {
        Self {
            entries: HashMap::new(),
            order: VecDeque::new(),
            failures: HashSet::new(),
            bytes: 0,
            max_bytes,
            max_textures,
            evictions: 0,
            rejected: 0,
        }
    }

    pub(crate) fn set_limits(&mut self, max_bytes: u64, max_textures: usize) {
        self.max_bytes = max_bytes;
        self.max_textures = max_textures.max(1);
        self.enforce_limits();
    }

    pub(crate) fn bytes(&self) -> u64 {
        self.bytes
    }

    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }

    pub(crate) fn evictions(&self) -> u64 {
        self.evictions
    }

    pub(crate) fn get(&self, key: u64) -> Option<&GpuImage> {
        self.entries.get(&key)
    }

    pub(crate) fn failed(&self, key: u64) -> bool {
        self.failures.contains(&key)
    }

    /// Removes one cached texture and reports whether it existed.
    pub(crate) fn remove(&mut self, key: u64) -> bool {
        self.failures.remove(&key);
        if let Some(image) = self.entries.remove(&key) {
            self.bytes = self.bytes.saturating_sub(image.bytes);
            self.order.retain(|k| *k != key);
            image.texture.destroy();
            true
        } else {
            false
        }
    }

    /// Destroys every cached texture and clears failure records.
    pub(crate) fn clear(&mut self) {
        for (_, image) in self.entries.drain() {
            image.texture.destroy();
        }
        self.order.clear();
        self.failures.clear();
        self.bytes = 0;
    }

    /// Enforces the byte and texture-count limits, evicting LRU entries.
    ///
    /// Call this after the frame has been submitted and the GPU no longer
    /// reads the textures recorded in its command buffers.
    pub(crate) fn trim(&mut self) {
        self.enforce_limits();
    }

    fn enforce_limits(&mut self) {
        if self.max_bytes == 0 {
            return;
        }
        while self.entries.len() > self.max_textures || self.bytes > self.max_bytes {
            let Some(oldest) = self.order.pop_front() else {
                break;
            };
            if let Some(image) = self.entries.remove(&oldest) {
                self.bytes = self.bytes.saturating_sub(image.bytes);
                image.texture.destroy();
                self.evictions += 1;
            }
        }
    }

    /// Returns a bind group for `source`, uploading it when needed.
    ///
    /// `allow_new` is `false` when memory pressure has disabled new uploads;
    /// already-cached images keep working. Images larger than `max_dim` or the
    /// byte limit are rejected instead of being allocated.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn bind_group(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
        source: &ImageSource,
        allow_new: bool,
        max_dim: u32,
    ) -> Option<wgpu::BindGroup> {
        let key = source.cache_key();
        if let Some(entry) = self.entries.get_mut(&key) {
            if let Some(pos) = self.order.iter().position(|k| *k == key) {
                self.order.remove(pos);
            }
            self.order.push_back(key);
            let want_version = source.dynamic_version();
            let want_extent = source.extent();
            let entry_version = entry.version;
            let entry_extent = (entry.width, entry.height);
            let same_extent = want_extent.is_none_or(|e| e == entry_extent);
            if want_version.is_some() && same_extent && want_version != entry_version {
                if let Some((w, h, data)) = decode_image(source)
                    && w == entry.width
                    && h == entry.height
                {
                    upload_rgba_levels(queue, &entry.texture, w, h, &data);
                    entry.version = want_version;
                    self.failures.remove(&key);
                    return Some(entry.bind_group.clone());
                }
                self.remove(key);
            } else {
                return Some(entry.bind_group.clone());
            }
        }
        if self.failures.contains(&key) {
            return None;
        }
        if !allow_new {
            self.rejected += 1;
            return None;
        }
        if let Some((w, h)) = source_dimensions(source)
            && (w > max_dim || h > max_dim)
        {
            // Reject from the header without decoding the pixels; the image is
            // retried when the quality level allows a larger texture again.
            self.rejected += 1;
            return None;
        }
        let Some((w, h, data)) = decode_image(source) else {
            if self.failures.len() >= IMAGE_FAILURES_MAX {
                self.failures.clear();
            }
            self.failures.insert(key);
            return None;
        };
        let bytes = rgba8_bytes(w, h);
        if w > max_dim || h > max_dim || bytes > self.max_bytes {
            self.rejected += 1;
            return None;
        }
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("sequoia-image"),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: mip_level_count(w, h),
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        upload_rgba_levels(queue, &texture, w, h, &data);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sequoia-image-bg"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        });
        let bg = bind_group.clone();
        self.bytes = self.bytes.saturating_add(bytes);
        self.entries.insert(
            key,
            GpuImage {
                texture,
                bind_group,
                width: w,
                height: h,
                version: source.dynamic_version(),
                bytes,
            },
        );
        self.order.push_back(key);
        // Limits are enforced after the frame so an image uploaded late in the
        // scene cannot evict a texture that earlier draws still reference.
        Some(bg)
    }
}

impl Drop for ImageCache {
    fn drop(&mut self) {
        for (_, image) in self.entries.drain() {
            image.texture.destroy();
        }
    }
}

/// Fast dimension lookup that avoids decoding pixels when possible.
fn source_dimensions(source: &ImageSource) -> Option<(u32, u32)> {
    if let Some(extent) = source.extent() {
        return Some(extent);
    }
    match source {
        ImageSource::File(path) => image::image_dimensions(path).ok(),
        _ => None,
    }
}

/// Shared handle to an [`ImageCache`].
pub(crate) type SharedImageCache = Arc<Mutex<ImageCache>>;

/// Locks a shared image cache, recovering from a poisoned mutex.
pub(crate) fn lock_images(cache: &SharedImageCache) -> std::sync::MutexGuard<'_, ImageCache> {
    cache.lock().unwrap_or_else(|e| e.into_inner())
}

/// Render pipelines and binding layout for one registered user shader.
pub(crate) struct UserShaderEntry {
    pub(crate) pipelines: HashMap<BlendMode, wgpu::RenderPipeline>,
    pub(crate) bgl: wgpu::BindGroupLayout,
    pub(crate) module: wgpu::ShaderModule,
}

impl crate::renderer::GpuRenderer {
    /// Returns a bind group for `source`, uploading or refreshing it as needed.
    ///
    /// Dynamic images are re-uploaded when their version changes. Decoding
    /// failures are cached to avoid retrying broken sources every frame.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn image_bind_group(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
        images: &SharedImageCache,
        source: &ImageSource,
        allow_new: bool,
        max_dim: u32,
    ) -> Option<wgpu::BindGroup> {
        lock_images(images).bind_group(device, queue, layout, sampler, source, allow_new, max_dim)
    }

    /// Ensures at least `count` full-frame group layers for `size`.
    pub(crate) fn ensure_group_layers(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
        format: wgpu::TextureFormat,
        layers: &mut Vec<GroupLayer>,
        size: (u32, u32),
        count: usize,
    ) {
        let size = (size.0.max(1), size.1.max(1));
        let count = count.max(1);
        if layers.len() >= count && layers.iter().take(count).all(|l| l.size == size) {
            return;
        }
        for layer in layers.drain(..) {
            layer.texture.destroy();
        }
        for i in 0..count {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("sequoia-opacity-layer"),
                size: wgpu::Extent3d {
                    width: size.0,
                    height: size.1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("sequoia-opacity-layer-bg"),
                layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(sampler),
                    },
                ],
            });
            log::trace!("sequoia-render: opacity layer {i} {}x{}", size.0, size.1);
            layers.push(GroupLayer {
                texture,
                view,
                bg,
                size,
            });
        }
    }

    /// Recreates the backdrop ping-pong textures when the size changes.
    pub(crate) fn ensure_backdrop_buffers(&mut self, size: (u32, u32)) {
        let size = (size.0.max(1), size.1.max(1));
        if self.backdrop_tex.as_ref().is_some_and(|b| b.size == size) {
            return;
        }
        if let Some(old) = self.backdrop_tex.take() {
            old.a_tex.destroy();
            old.b_tex.destroy();
            old.c_tex.destroy();
        }
        let format = self.render_format;
        let mk = |label: &'static str| {
            let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width: size.0,
                    height: size.1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::COPY_SRC
                    | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let bg = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("sequoia-backdrop-bg"),
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
            (texture, view, bg)
        };
        let (a_tex, a_view, a_bg) = mk("sequoia-backdrop-a");
        let (b_tex, b_view, b_bg) = mk("sequoia-backdrop-b");
        let (c_tex, c_view, c_bg) = mk("sequoia-backdrop-c");
        self.backdrop_tex = Some(BackdropBuffers {
            a_tex,
            a_view,
            a_bg,
            b_tex,
            b_view,
            b_bg,
            c_tex,
            c_view,
            c_bg,
            size,
        });
    }

    /// Recreates the persistent partial-repaint target when the size changes.
    ///
    /// Returns `true` when the target was created or resized, which means its
    /// contents must be treated as undefined.
    pub(crate) fn ensure_frame_target(&mut self, size: (u32, u32)) -> bool {
        let size = (size.0.max(1), size.1.max(1));
        if self.frame_target.as_ref().is_some_and(|t| t.size == size) {
            return false;
        }
        if let Some(old) = self.frame_target.take() {
            old.texture.destroy();
        }
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("sequoia-frame-target"),
            size: wgpu::Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.render_format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bg = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sequoia-frame-target-bg"),
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
        self.frame_target = Some(FrameTarget {
            texture,
            view,
            bg,
            size,
        });
        true
    }

    /// Grows `buf` to hold at least `len` instances of `item_bytes`.
    pub(crate) fn ensure_capacity(
        device: &wgpu::Device,
        buf: &mut wgpu::Buffer,
        cap: &mut usize,
        len: usize,
        item_bytes: usize,
    ) {
        if len <= *cap {
            return;
        }

        if item_bytes == 0 {
            return;
        }
        let want = len.max(cap.saturating_mul(2).max(1));

        let max_bytes = device.limits().max_buffer_size;
        let allowed = (max_bytes / item_bytes as u64).min(usize::MAX as u64) as usize;
        if len > allowed {
            log::error!(
                "sequoia-render: instance buffer {len} x {item_bytes}B \
                 exceeds max_buffer_size ({max_bytes}B); dropping resize"
            );
            return;
        }
        let new_cap = want.min(allowed);

        let Some(bytes) = new_cap.checked_mul(item_bytes) else {
            log::error!("sequoia-render: instance buffer overflow (len={len}, item={item_bytes})");
            return;
        };
        *buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sequoia-instances"),
            size: bytes as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        *cap = new_cap;
    }

    pub(crate) fn create_cached_surface(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
        size: (u32, u32),
        samples: u32,
        format: wgpu::TextureFormat,
    ) -> CachedSurface {
        let samples = sanitize_samples(samples);
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("sequoia-surface-tex"),
            size: wgpu::Extent3d {
                width: size.0.max(1),
                height: size.1.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sequoia-blit-bg"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        });
        let msaa = if samples > 1 {
            let tex = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("sequoia-surface-msaa"),
                size: wgpu::Extent3d {
                    width: size.0.max(1),
                    height: size.1.max(1),
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: samples,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
            Some((tex, view))
        } else {
            None
        };
        CachedSurface {
            texture,
            view,
            bind_group,
            size,
            samples,
            format,
            msaa,
        }
    }

    pub(crate) fn ensure_depth_slot<'a>(
        device: &wgpu::Device,
        slot: &'a mut Option<(wgpu::Texture, wgpu::TextureView, (u32, u32))>,
        width: u32,
        height: u32,
    ) -> Option<&'a wgpu::TextureView> {
        let size = (width.max(1), height.max(1));
        let stale = match slot {
            Some((_, _, s)) => *s != size,
            None => true,
        };
        if stale {
            if let Some((old, _, _)) = slot.take() {
                old.destroy();
            }
            let tex = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("sequoia-shared-depth"),
                size: wgpu::Extent3d {
                    width: size.0,
                    height: size.1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: SHARED_DEPTH_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
            *slot = Some((tex, view, size));
        }
        slot.as_ref().map(|(_, view, _)| view)
    }

    pub(crate) fn sync_atlas(&mut self, atlas: &mut GlyphAtlas) {
        let (w, h) = atlas.size();
        let ver = atlas.version();

        if self.atlas_tex.is_some() && self.atlas_synced == Some((ver, w, h)) {
            let _ = atlas.take_dirty();
            return;
        }

        let needs_recreate = match &self.atlas_tex {
            None => true,
            Some(_) => self
                .atlas_synced
                .map(|(_, sw, sh)| sw != w || sh != h)
                .unwrap_or(true),
        };

        if needs_recreate {
            // Release the previous atlas before allocating the new one: glyph
            // atlases reach 2048² (16 MB) and a plain drop only frees the GPU
            // allocation after the queue drains, so growth would briefly hold
            // two full atlases.
            self.trace_atlas_recreates = self.trace_atlas_recreates.wrapping_add(1);
            if let Some((old, _)) = self.atlas_tex.take() {
                old.destroy();
            }
            let tex = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("sequoia-atlas"),
                size: wgpu::Extent3d {
                    width: w,
                    height: h,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
            let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("sequoia-atlas-sampler"),
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            });
            let bg = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("sequoia-atlas-bg"),
                layout: &self.atlas_bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                ],
            });
            self.atlas_tex = Some((tex, bg));
        }

        let dirty = atlas.take_dirty();
        if let Some((tex, _)) = &self.atlas_tex {
            if needs_recreate || dirty.is_some_and(|d| d.full) {
                self.queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: tex,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    atlas.pixels(),
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(w * 4),
                        rows_per_image: None,
                    },
                    wgpu::Extent3d {
                        width: w,
                        height: h,
                        depth_or_array_layers: 1,
                    },
                );
            } else if let Some(d) = dirty
                && d.w > 0
                && d.h > 0
            {
                self.atlas_scratch.clear();
                atlas.copy_rect(d.x, d.y, d.w, d.h, &mut self.atlas_scratch);
                if !self.atlas_scratch.is_empty() {
                    self.queue.write_texture(
                        wgpu::TexelCopyTextureInfo {
                            texture: tex,
                            mip_level: 0,
                            origin: wgpu::Origin3d {
                                x: d.x,
                                y: d.y,
                                z: 0,
                            },
                            aspect: wgpu::TextureAspect::All,
                        },
                        &self.atlas_scratch,
                        wgpu::TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(d.w * 4),
                            rows_per_image: None,
                        },
                        wgpu::Extent3d {
                            width: d.w,
                            height: d.h,
                            depth_or_array_layers: 1,
                        },
                    );
                }
            }
        }
        self.atlas_synced = Some((ver, w, h));
    }
}

pub(crate) fn vertex_buffer(device: &wgpu::Device, label: &'static str, size: u64) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}
