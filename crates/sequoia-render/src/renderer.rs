#[allow(unused_imports)]
use std::any::Any;
use std::cell::Cell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

use sequoia_text::GlyphAtlas;
use winit::window::Window;

use crate::gpu::pipelines::*;
use crate::gpu::resources::vertex_buffer;
use crate::scene::grouping::*;
use crate::scene::instances::*;
use crate::scene::planning::*;
use crate::surface::*;
use crate::types::*;
use crate::utils::*;
use sequoia_core::{Color, Rect, RenderList};
use sequoia_view::SurfaceDraw;

pub struct GpuRenderer {
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
    pub(crate) adapter: wgpu::AdapterInfo,
    pub(crate) backend: GpuBackend,
    pub(crate) surface: wgpu::Surface<'static>,
    pub(crate) config: wgpu::SurfaceConfiguration,
    pub(crate) quad_pipeline: wgpu::RenderPipeline,

    pub(crate) quad_pipeline_opaque: wgpu::RenderPipeline,
    pub(crate) glyph_pipeline: wgpu::RenderPipeline,
    pub(crate) atlas_bgl: wgpu::BindGroupLayout,
    pub(crate) bind_group: wgpu::BindGroup,
    pub(crate) uniform_buf: wgpu::Buffer,
    pub(crate) quad_buf: wgpu::Buffer,
    pub(crate) glyph_buf: wgpu::Buffer,
    pub(crate) frame_buf: wgpu::Buffer,
    pub(crate) frame_cap: usize,
    pub(crate) blit_buf: wgpu::Buffer,
    pub(crate) blit_cap: usize,
    pub(crate) blit_pipeline: wgpu::RenderPipeline,
    pub(crate) blit_sampler: wgpu::Sampler,

    pub(crate) present_buf: wgpu::Buffer,
    pub(crate) present_cap: usize,
    pub(crate) backdrop_buf: wgpu::Buffer,
    pub(crate) backdrop_cap: usize,
    pub(crate) backdrop_pipeline: wgpu::RenderPipeline,
    pub(crate) gradient_pipeline: wgpu::RenderPipeline,
    pub(crate) gradient_buf: wgpu::Buffer,
    pub(crate) gradient_cap: usize,
    pub(crate) present_pipeline: wgpu::RenderPipeline,
    pub(crate) backdrop_tex: Option<BackdropBuffers>,
    pub(crate) capacity: usize,
    pub(crate) glyph_capacity: usize,

    pub(crate) quad_ceiling: usize,

    pub(crate) glyph_ceiling: usize,

    pub(crate) image_ceiling: usize,

    pub(crate) gradient_ceiling: usize,
    pub(crate) atlas_tex: Option<(wgpu::Texture, wgpu::BindGroup)>,
    pub(crate) atlas_synced: Option<(u64, u32, u32)>,
    pub(crate) surface_tex: HashMap<usize, CachedSurface>,

    pub(crate) depth_tex: Option<(wgpu::Texture, wgpu::TextureView, (u32, u32))>,

    pub(crate) clear_color: Color,

    pub(crate) images: HashMap<u64, GpuImage>,
    pub(crate) image_order: VecDeque<u64>,
    pub(crate) image_failures: HashSet<u64>,
    pub(crate) image_buf: wgpu::Buffer,
    pub(crate) image_cap: usize,
    pub(crate) last_frame: Option<Instant>,
    pub(crate) frame_index: u64,
    pub(crate) redraw: Cell<bool>,

    pub(crate) skipped: Cell<Option<FrameSkip>>,

    pub(crate) stats: RenderStats,

    pub(crate) warned_filter_combo: bool,

    pub(crate) scratch_quads: Vec<GpuQuad>,
    pub(crate) scratch_glyphs: Vec<GpuGlyph>,
    pub(crate) scratch_runs: Vec<GuiRun>,
    pub(crate) scratch_frame: Vec<GpuQuad>,
    pub(crate) scratch_blits: Vec<BlitQuad>,
    pub(crate) scratch_blit_keys: Vec<usize>,
    pub(crate) scratch_images: Vec<BlitQuad>,
    pub(crate) scratch_image_keys: Vec<u64>,
    pub(crate) scratch_gradients: Vec<GpuGradient>,

    pub(crate) scratch_image_draws: Vec<(u32, wgpu::BindGroup)>,
    pub(crate) scratch_seen: Vec<usize>,
    pub(crate) scratch_cached: Vec<CachedJob>,
    pub(crate) scratch_regions: Vec<(Region, Rc<SurfaceBinding>)>,
    pub(crate) atlas_scratch: Vec<u8>,
}

impl GpuRenderer {
    pub fn new(window: Arc<Window>, width: u32, height: u32) -> Result<Self, RenderError> {
        Self::new_with_options(window, width, height, GpuOptions::default())
    }

    pub fn new_with_options(
        window: Arc<Window>,
        width: u32,
        height: u32,
        options: GpuOptions,
    ) -> Result<Self, RenderError> {
        let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
        if options.backend != GpuBackend::Auto {
            desc.backends = options.backend.wgpu_backends();
        }
        let instance = wgpu::Instance::new(desc);
        let surface = instance
            .create_surface(window)
            .map_err(RenderError::CreateSurface)?;

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: options.power_preference.wgpu(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .map_err(|_| RenderError::NoAdapter)?;
        let adapter_info = adapter.get_info();

        log::info!(
            "sequoia-render: backend {:?}, adapter '{}' ({:?}, {} {})",
            adapter_info.backend,
            adapter_info.name,
            adapter_info.device_type,
            adapter_info.driver,
            adapter_info.driver_info
        );

        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("sequoia-device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),

            memory_hints: wgpu::MemoryHints::MemoryUsage,
            trace: wgpu::Trace::Off,
        }))
        .map_err(RenderError::Device)?;

        device.on_uncaptured_error(Arc::new(|error| {
            log::error!("sequoia-render: uncaptured GPU error: {error}");
        }));

        let caps = surface.get_capabilities(&adapter);
        let format = pick_format(&caps.formats).ok_or(RenderError::NoAdapter)?;
        let present_mode = resolve_present_mode(&caps.present_modes, options.present_mode);
        if present_mode != options.present_mode.wgpu() {
            log::warn!(
                "sequoia-render: present mode `{}` unavailable on this surface - using vsync",
                options.present_mode.name()
            );
        }

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: width.max(1),
            height: height.max(1),
            present_mode,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let screen_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sequoia-screen-bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,

                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(16),
                },
                count: None,
            }],
        });

        let atlas_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sequoia-atlas-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sequoia-uniform"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sequoia-bind-group"),
            layout: &screen_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buf.as_entire_binding(),
            }],
        });

        let quad_buf = vertex_buffer(
            &device,
            "sequoia-quads",
            (INITIAL_QUAD_CAPACITY * QUAD_BYTES) as u64,
        );

        let glyph_buf = vertex_buffer(
            &device,
            "sequoia-glyphs",
            (INITIAL_GLYPH_CAPACITY * GLYPH_BYTES) as u64,
        );

        let frame_buf = vertex_buffer(
            &device,
            "sequoia-frame-quads",
            (INITIAL_FRAME_CAPACITY * QUAD_BYTES) as u64,
        );

        let blit_buf = vertex_buffer(
            &device,
            "sequoia-blits",
            (INITIAL_BLIT_CAPACITY * BLIT_BYTES) as u64,
        );

        let image_buf = vertex_buffer(
            &device,
            "sequoia-images",
            (INITIAL_IMAGE_CAPACITY * BLIT_BYTES) as u64,
        );

        let gradient_buf = vertex_buffer(
            &device,
            "sequoia-gradients",
            (INITIAL_GRADIENT_CAPACITY * GRADIENT_BYTES) as u64,
        );

        let present_buf = vertex_buffer(
            &device,
            "sequoia-present",
            (INITIAL_BLIT_CAPACITY * BLIT_BYTES) as u64,
        );
        let blit_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("sequoia-blit-sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let shader = create_shader_module(&device);

        let (layout, glyph_layout) = create_pipeline_layouts(&device, &screen_bgl, &atlas_bgl);

        let blend = premultiplied_blend();

        let quad_pipeline = create_quad_pipeline(&device, &shader, &layout, format, blend);

        let quad_pipeline_opaque = create_quad_opaque_pipeline(&device, &shader, &layout, format);

        let glyph_pipeline = create_glyph_pipeline(&device, &shader, &glyph_layout, format, blend);

        let blit_pipeline = create_blit_pipeline(&device, &shader, &glyph_layout, format, blend);

        let backdrop_buf = vertex_buffer(
            &device,
            "sequoia-backdrops",
            (INITIAL_BACKDROP_CAPACITY * BACKDROP_BYTES) as u64,
        );

        let backdrop_pipeline =
            create_backdrop_pipeline(&device, &shader, &glyph_layout, format, blend);

        let present_pipeline = create_present_pipeline(&device, &shader, &glyph_layout, format);

        let quad_ceiling = instance_ceiling(&device, QUAD_BYTES);
        let glyph_ceiling = instance_ceiling(&device, GLYPH_BYTES);
        let image_ceiling = instance_ceiling(&device, BLIT_BYTES);
        let gradient_ceiling = instance_ceiling(&device, GRADIENT_BYTES);

        let gradient_pipeline = create_gradient_pipeline(&device, &shader, &layout, format, blend);

        Ok(Self {
            device,
            queue,
            adapter: adapter_info,
            backend: options.backend,
            surface,
            config,
            quad_pipeline,
            quad_pipeline_opaque,
            glyph_pipeline,
            blit_pipeline,
            blit_sampler,
            backdrop_buf,
            backdrop_cap: INITIAL_BACKDROP_CAPACITY,
            backdrop_pipeline,
            gradient_pipeline,
            gradient_buf,
            gradient_cap: INITIAL_GRADIENT_CAPACITY,
            present_pipeline,
            backdrop_tex: None,
            atlas_bgl,
            bind_group,
            uniform_buf,
            quad_buf,
            glyph_buf,
            frame_buf,
            frame_cap: INITIAL_FRAME_CAPACITY,
            blit_buf,
            blit_cap: INITIAL_BLIT_CAPACITY,
            present_buf,
            present_cap: INITIAL_BLIT_CAPACITY,
            capacity: INITIAL_QUAD_CAPACITY,
            glyph_capacity: INITIAL_GLYPH_CAPACITY,
            quad_ceiling,
            glyph_ceiling,
            image_ceiling,
            gradient_ceiling,
            atlas_tex: None,
            atlas_synced: None,
            surface_tex: HashMap::new(),
            depth_tex: None,
            clear_color: CLEAR_COLOR,
            images: HashMap::new(),
            image_order: VecDeque::new(),
            image_failures: HashSet::new(),
            image_buf,
            image_cap: INITIAL_IMAGE_CAPACITY,
            last_frame: None,
            frame_index: 0,
            redraw: Cell::new(false),
            skipped: Cell::new(None),
            stats: RenderStats::default(),
            warned_filter_combo: false,
            scratch_quads: Vec::with_capacity(INITIAL_QUAD_CAPACITY),
            scratch_glyphs: Vec::with_capacity(INITIAL_GLYPH_CAPACITY),
            scratch_runs: Vec::new(),
            scratch_frame: Vec::with_capacity(INITIAL_FRAME_CAPACITY),
            scratch_blits: Vec::with_capacity(INITIAL_BLIT_CAPACITY),
            scratch_blit_keys: Vec::with_capacity(INITIAL_BLIT_CAPACITY),
            scratch_images: Vec::with_capacity(INITIAL_IMAGE_CAPACITY),
            scratch_image_keys: Vec::with_capacity(INITIAL_IMAGE_CAPACITY),
            scratch_gradients: Vec::with_capacity(INITIAL_GRADIENT_CAPACITY),
            scratch_image_draws: Vec::with_capacity(INITIAL_IMAGE_CAPACITY),
            scratch_seen: Vec::new(),
            scratch_cached: Vec::new(),
            scratch_regions: Vec::new(),
            atlas_scratch: Vec::new(),
        })
    }

    pub fn adapter_info(&self) -> wgpu::AdapterInfo {
        self.adapter.clone()
    }

    pub fn backend(&self) -> GpuBackend {
        self.backend
    }

    pub fn take_redraw_request(&self) -> bool {
        self.redraw.take()
    }

    pub fn take_frame_skip(&self) -> Option<FrameSkip> {
        self.skipped.take()
    }

    pub fn stats(&self) -> RenderStats {
        self.stats
    }

    pub fn take_stats(&mut self) -> RenderStats {
        std::mem::take(&mut self.stats)
    }

    pub fn set_clear_color(&mut self, color: Color) {
        self.clear_color = color;
    }

    pub fn clear_color(&self) -> Color {
        self.clear_color
    }

    pub fn clear_image_cache(&mut self) {
        for (_, img) in self.images.drain() {
            img.texture.destroy();
        }
        self.image_order.clear();
        self.image_failures.clear();
    }

    pub fn remove_image(&mut self, key: u64) {
        if let Some(img) = self.images.remove(&key) {
            img.texture.destroy();
            self.image_order.retain(|k| *k != key);
        }
        self.image_failures.remove(&key);
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.config.width = width.max(1);
        self.config.height = height.max(1);
        self.surface.configure(&self.device, &self.config);
    }

    pub fn render(
        &mut self,
        list: &RenderList,
        atlas: &mut GlyphAtlas,
        scale: f32,
        surfaces: &[SurfaceDraw],
    ) -> Result<(), RenderError> {
        if self.config.width == 0 || self.config.height == 0 {
            return Ok(());
        }
        let s = if scale.is_finite() && scale > 0.0 {
            scale
        } else {
            1.0
        };
        self.sync_atlas(atlas);

        let mut reconfigured = false;
        let frame = loop {
            match self.surface.get_current_texture() {
                wgpu::CurrentSurfaceTexture::Success(frame) => break frame,
                wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                    if reconfigured {
                        break frame;
                    }
                    drop(frame);
                    self.surface.configure(&self.device, &self.config);
                    reconfigured = true;
                }
                wgpu::CurrentSurfaceTexture::Outdated => {
                    self.surface.configure(&self.device, &self.config);
                }
                wgpu::CurrentSurfaceTexture::Lost => {
                    self.surface.configure(&self.device, &self.config);
                }
                wgpu::CurrentSurfaceTexture::Timeout => {
                    self.skipped.set(Some(FrameSkip::Timeout));
                    return Ok(());
                }
                wgpu::CurrentSurfaceTexture::Occluded => {
                    self.skipped.set(Some(FrameSkip::Occluded));
                    return Ok(());
                }
                wgpu::CurrentSurfaceTexture::Validation => {
                    return Err(RenderError::Surface);
                }
            }
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("sequoia-frame"),
            });

        let mut passes: u64 = 0;
        let mut draws: u64 = 0;

        let fb = (self.config.width, self.config.height);

        let has_backdrops = list.has_backdrops();

        let has_filters = list.has_filters();
        if has_backdrops || has_filters {
            self.ensure_backdrop_buffers(fb);
        }
        let backdrop_a_view: Option<wgpu::TextureView> = (has_backdrops || has_filters)
            .then(|| self.backdrop_tex.as_ref().map(|b| b.a_view.clone()))
            .flatten();
        let scene_target: &wgpu::TextureView = backdrop_a_view.as_ref().unwrap_or(&view);

        let frame_quads = &mut self.scratch_frame;
        let regions = &mut self.scratch_regions;
        let cached = &mut self.scratch_cached;
        let seen = &mut self.scratch_seen;
        frame_quads.clear();
        regions.clear();
        cached.clear();
        seen.clear();
        for draw in surfaces {
            let Ok(binding) = draw.target.clone().downcast::<SurfaceBinding>() else {
                continue;
            };
            let before = frame_quads.len();
            let region = surface_frame(draw.rect, &binding.frame, s, fb, frame_quads);
            if frame_quads.len() > before && frame_quads.len() >= MAX_FRAME_QUADS {
                self.stats.dropped_frame_quads +=
                    (frame_quads.len() - before.max(MAX_FRAME_QUADS)) as u64;
                log::debug!("sequoia-render: frame quad budget exceeded (max {MAX_FRAME_QUADS})");
            }
            if binding.cached {
                if !seen.contains(&binding.key) {
                    seen.push(binding.key);
                }

                let (Some(full), Some(screen)) = (
                    phys_rect(draw.full, s, fb.0, fb.1),
                    phys_rect(draw.rect, s, fb.0, fb.1),
                ) else {
                    continue;
                };
                let Some(size) = Region::from_phys(full).map(|r| (r.width, r.height)) else {
                    continue;
                };
                if cached.len() >= MAX_BLITS {
                    self.stats.dropped_surfaces += 1;
                    log::debug!("sequoia-render: cached surface budget exceeded (max {MAX_BLITS})");
                    continue;
                }
                cached.push(CachedJob {
                    binding,
                    size,
                    full,
                    screen,
                });
            } else {
                let Some(region) = region.and_then(Region::from_phys) else {
                    continue;
                };
                regions.push((region, binding));
            }
        }

        Self::ensure_capacity(
            &self.device,
            &mut self.frame_buf,
            &mut self.frame_cap,
            frame_quads.len(),
            QUAD_BYTES,
        );
        if !frame_quads.is_empty() {
            self.queue
                .write_buffer(&self.frame_buf, 0, bytemuck::cast_slice(frame_quads));
        }

        let size = [
            self.config.width as f32,
            self.config.height as f32,
            0.0,
            0.0,
        ];
        self.queue
            .write_buffer(&self.uniform_buf, 0, bytemuck::cast_slice(&size));

        let now = Instant::now();
        let delta = self
            .last_frame
            .map(|t| now.duration_since(t))
            .unwrap_or_default();
        self.last_frame = Some(now);
        let frame_index = self.frame_index;
        self.frame_index += 1;

        let blit_quads = &mut self.scratch_blits;
        let blit_keys = &mut self.scratch_blit_keys;
        blit_quads.clear();
        blit_keys.clear();
        for job in cached.iter() {
            let samples = job.binding.samples;
            let recreate = match self.surface_tex.get(&job.binding.key) {
                Some(e) => e.size != job.size || e.samples != samples,
                None => true,
            };
            if recreate {
                if let Some(old) = self.surface_tex.remove(&job.binding.key) {
                    old.texture.destroy();
                    if let Some((msaa, _)) = old.msaa {
                        msaa.destroy();
                    }
                }
                self.surface_tex.insert(
                    job.binding.key,
                    Self::create_cached_surface(
                        &self.device,
                        &self.atlas_bgl,
                        &self.blit_sampler,
                        job.size,
                        samples,
                    ),
                );
            }
            if recreate || (job.binding.check_dirty)() {
                let entry = &self.surface_tex[&job.binding.key];

                let clear = clear_color_authored(self.clear_color);
                let bg = job.binding.frame.background.map_or(clear, |c| wgpu::Color {
                    r: c[0] as f64,
                    g: c[1] as f64,
                    b: c[2] as f64,
                    a: c[3] as f64,
                });

                let (target_view, resolve) = match &entry.msaa {
                    Some((_, msaa_view)) => (msaa_view, Some(&entry.view)),
                    None => (&entry.view, None),
                };
                {
                    passes += 1;
                    let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("sequoia-surface-tex-clear"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: target_view,
                            depth_slice: None,
                            resolve_target: resolve,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(bg),
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
                let entry = &self.surface_tex[&job.binding.key];

                let app_target = match &entry.msaa {
                    Some((_, v)) => v,
                    None => &entry.view,
                };
                let mut frame = WgpuFrame {
                    device: &self.device,
                    queue: &self.queue,
                    encoder: &mut encoder,
                    target: app_target,
                    depth: None,
                    format: CACHE_FORMAT,
                    size: entry.size,
                    scale: s,
                    region: Region {
                        x: 0,
                        y: 0,
                        width: entry.size.0,
                        height: entry.size.1,
                    },
                    adapter: self.adapter.clone(),
                    backend: self.backend,
                    index: frame_index,
                    delta,
                    redraw: &self.redraw,
                };
                (job.binding.draw)(&mut frame);

                if let Some((_, msaa_view)) = &entry.msaa {
                    passes += 1;
                    let resolve_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("sequoia-surface-msaa-resolve"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: msaa_view,
                            depth_slice: None,
                            resolve_target: Some(&entry.view),
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
                    drop(resolve_pass);
                }
                job.binding.mark_clean.borrow_mut()();
            }
            if self.surface_tex.contains_key(&job.binding.key) {
                blit_quads.push(blit_instance(job.screen, job.full));
                blit_keys.push(job.binding.key);
            }
        }
        self.surface_tex.retain(|k, v| {
            let keep = seen.contains(k);
            if !keep {
                v.texture.destroy();
                if let Some((msaa, _)) = &v.msaa {
                    msaa.destroy();
                }
            }
            keep
        });

        Self::ensure_capacity(
            &self.device,
            &mut self.blit_buf,
            &mut self.blit_cap,
            blit_quads.len(),
            BLIT_BYTES,
        );
        if !blit_quads.is_empty() {
            self.queue
                .write_buffer(&self.blit_buf, 0, bytemuck::cast_slice(blit_quads));
        }

        let viewport = Rect::from_xywh(0.0, 0.0, fb.0 as f32 / s, fb.1 as f32 / s);
        let quads = &mut self.scratch_quads;
        let glyphs = &mut self.scratch_glyphs;
        let images = &mut self.scratch_images;
        let image_keys = &mut self.scratch_image_keys;
        let gradients = &mut self.scratch_gradients;
        let runs = &mut self.scratch_runs;
        let mut backdrop_plans: Vec<BackdropSegPlan> = Vec::new();
        let mut backdrop_insts: Vec<GpuBackdrop> = Vec::new();
        let mut comp_plans: Vec<CompPlan> = Vec::new();
        if !has_backdrops && !has_filters {
            let (dq, dg, di, dgrad, culled) = group_runs(
                list.items(),
                list.item_clips(),
                list.item_transforms(),
                s,
                Some(viewport),
                self.quad_ceiling,
                self.glyph_ceiling,
                self.image_ceiling,
                self.gradient_ceiling,
                quads,
                glyphs,
                images,
                image_keys,
                gradients,
                runs,
            );
            self.stats.dropped_quads += dq as u64;
            self.stats.dropped_glyphs += dg as u64;
            self.stats.dropped_images += di as u64;
            self.stats.dropped_gradients += dgrad as u64;
            self.stats.culled += culled as u64;
            self.stats.frames += 1;
            if dq + dg + di + dgrad > 0 {
                log::error!(
                    "sequoia-render: frame exceeds the device vertex-buffer limit — \
                 dropped quads={dq} glyphs={dg} images={di} gradients={dgrad} \
                 (paint items={}, quad ceiling={}, glyph ceiling={})",
                    list.items().len(),
                    self.quad_ceiling,
                    self.glyph_ceiling,
                );
            }
        } else {
            quads.clear();
            glyphs.clear();
            images.clear();
            image_keys.clear();
            gradients.clear();

            let mut seg_quads: Vec<GpuQuad> = Vec::new();
            let mut seg_glyphs: Vec<GpuGlyph> = Vec::new();
            let mut seg_images: Vec<BlitQuad> = Vec::new();
            let mut seg_keys: Vec<u64> = Vec::new();
            let mut seg_gradients: Vec<GpuGradient> = Vec::new();
            let mut seg_runs: Vec<GuiRun> = Vec::new();
            let ceil = (
                self.quad_ceiling,
                self.glyph_ceiling,
                self.image_ceiling,
                self.gradient_ceiling,
            );
            let items = list.items();
            let clips = list.item_clips();
            let transforms = list.item_transforms();

            let mut group_segment =
                |from: usize,
                 to: usize,
                 quads: &mut Vec<GpuQuad>,
                 glyphs: &mut Vec<GpuGlyph>,
                 images: &mut Vec<BlitQuad>,
                 image_keys: &mut Vec<u64>,
                 gradients: &mut Vec<GpuGradient>,
                 stats: &mut RenderStats,
                 plan: &mut BackdropSegPlan| {
                    let (dq, dg, di, dgrad, culled) = group_runs(
                        &items[from..to],
                        &clips[from..to],
                        &transforms[from..to],
                        s,
                        Some(viewport),
                        ceil.0,
                        ceil.1,
                        ceil.2,
                        ceil.3,
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

                    plan.all_opaque =
                        seg_gradients.is_empty() && seg_quads.iter().all(quad_is_opaque);
                    plan.runs = shift_gui_runs(
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
                    plan.has_content = true;
                };
            let mut prev = 0usize;
            if !has_filters {
                for span in list.backdrops() {
                    let end = span.at.min(items.len());
                    let mut plan = BackdropSegPlan {
                        runs: Vec::new(),
                        all_opaque: true,
                        has_content: false,
                        backdrop_idx: None,
                    };
                    if end > prev {
                        group_segment(
                            prev,
                            end,
                            quads,
                            glyphs,
                            images,
                            image_keys,
                            gradients,
                            &mut self.stats,
                            &mut plan,
                        );
                        prev = end;
                    }
                    if let Some(inst) = Self::backdrop_instance(span, s) {
                        plan.backdrop_idx = Some(backdrop_insts.len() as u32);
                        backdrop_insts.push(inst);
                    }
                    backdrop_plans.push(plan);
                }
                if prev < items.len() {
                    let mut plan = BackdropSegPlan {
                        runs: Vec::new(),
                        all_opaque: true,
                        has_content: false,
                        backdrop_idx: None,
                    };
                    group_segment(
                        prev,
                        items.len(),
                        quads,
                        glyphs,
                        images,
                        image_keys,
                        gradients,
                        &mut self.stats,
                        &mut plan,
                    );
                    backdrop_plans.push(plan);
                }
            } else {
                let ops = plan_composite_ops(list.backdrops(), list.filter_spans(), items.len());
                let skipped = list.backdrops().len() + list.filter_spans().len() - ops.len();
                if skipped > 0 {
                    self.stats.filter_skipped += skipped as u64;
                    if !self.warned_filter_combo {
                        self.warned_filter_combo = true;
                        log::warn!(
                            "sequoia-render: {skipped} composite op(s) skipped \
                             (backdrop marker inside a filter range or nested filter — \
                             the outer filter wins)"
                        );
                    }
                }
                for op in ops {
                    match op {
                        CompOp::Marker { at, span_idx } => {
                            let mut plan = BackdropSegPlan {
                                runs: Vec::new(),
                                all_opaque: true,
                                has_content: false,
                                backdrop_idx: None,
                            };
                            if at > prev {
                                group_segment(
                                    prev,
                                    at,
                                    quads,
                                    glyphs,
                                    images,
                                    image_keys,
                                    gradients,
                                    &mut self.stats,
                                    &mut plan,
                                );
                                prev = at;
                            }
                            if let Some(inst) =
                                Self::backdrop_instance(&list.backdrops()[span_idx], s)
                            {
                                plan.backdrop_idx = Some(backdrop_insts.len() as u32);
                                backdrop_insts.push(inst);
                            }
                            comp_plans.push(CompPlan::Seg(plan));
                        }
                        CompOp::FilterRange {
                            start,
                            end,
                            span_idx,
                        } => {
                            if start > prev {
                                let mut plan = BackdropSegPlan {
                                    runs: Vec::new(),
                                    all_opaque: true,
                                    has_content: false,
                                    backdrop_idx: None,
                                };
                                group_segment(
                                    prev,
                                    start,
                                    quads,
                                    glyphs,
                                    images,
                                    image_keys,
                                    gradients,
                                    &mut self.stats,
                                    &mut plan,
                                );

                                comp_plans.push(CompPlan::Seg(plan));
                            }
                            let mut tmp = BackdropSegPlan {
                                runs: Vec::new(),
                                all_opaque: true,
                                has_content: false,
                                backdrop_idx: None,
                            };
                            group_segment(
                                start,
                                end,
                                quads,
                                glyphs,
                                images,
                                image_keys,
                                gradients,
                                &mut self.stats,
                                &mut tmp,
                            );
                            prev = end;
                            let filter_idx =
                                Self::filter_instance(&list.filter_spans()[span_idx], s).map(
                                    |inst| {
                                        backdrop_insts.push(inst);
                                        backdrop_insts.len() as u32 - 1
                                    },
                                );
                            comp_plans.push(CompPlan::Filter(FilterSegPlan {
                                runs: tmp.runs,
                                all_opaque: tmp.all_opaque,
                                has_content: tmp.has_content,
                                filter_idx,
                            }));
                        }
                    }
                }
                if prev < items.len() {
                    let mut plan = BackdropSegPlan {
                        runs: Vec::new(),
                        all_opaque: true,
                        has_content: false,
                        backdrop_idx: None,
                    };
                    group_segment(
                        prev,
                        items.len(),
                        quads,
                        glyphs,
                        images,
                        image_keys,
                        gradients,
                        &mut self.stats,
                        &mut plan,
                    );
                    comp_plans.push(CompPlan::Seg(plan));
                }
            }
        }

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
                .write_buffer(&self.quad_buf, 0, bytemuck::cast_slice(quads));
        }
        if !glyphs.is_empty() {
            self.queue
                .write_buffer(&self.glyph_buf, 0, bytemuck::cast_slice(glyphs));
        }
        if !images.is_empty() {
            self.queue
                .write_buffer(&self.image_buf, 0, bytemuck::cast_slice(images));
        }
        if !gradients.is_empty() {
            self.queue
                .write_buffer(&self.gradient_buf, 0, bytemuck::cast_slice(gradients));
        }

        let image_draws = &mut self.scratch_image_draws;
        image_draws.clear();
        if !images.is_empty() {
            for key in image_keys.iter() {
                if !self.images.contains_key(key)
                    && !self.image_failures.contains(key)
                    && let Some(src) = find_image_source(list.items(), *key)
                {
                    Self::image_bind_group(
                        &self.device,
                        &self.queue,
                        &self.atlas_bgl,
                        &self.blit_sampler,
                        &mut self.images,
                        &mut self.image_order,
                        &mut self.image_failures,
                        &src,
                    );
                }
            }
            for (i, key) in image_keys.iter().enumerate() {
                if let Some(entry) = self.images.get(key) {
                    image_draws.push((i as u32, entry.bind_group.clone()));
                }
            }
        }

        if has_backdrops || has_filters {
            Self::ensure_capacity(
                &self.device,
                &mut self.backdrop_buf,
                &mut self.backdrop_cap,
                backdrop_insts.len(),
                BACKDROP_BYTES,
            );
            if !backdrop_insts.is_empty() {
                self.queue.write_buffer(
                    &self.backdrop_buf,
                    0,
                    bytemuck::cast_slice(&backdrop_insts),
                );
            }
        }

        let fb_drawn = !regions.is_empty()
            || !frame_quads.is_empty()
            || !blit_quads.is_empty()
            || !images.is_empty();

        let shared_depth = !regions.is_empty();
        if shared_depth {
            Self::ensure_depth_slot(&self.device, &mut self.depth_tex, fb.0, fb.1);
        }

        if fb_drawn {
            let clear = clear_color_linear(self.clear_color);

            let split_depth = shared_depth && !frame_quads.is_empty();
            let depth_attachment = if shared_depth && !split_depth {
                self.depth_tex
                    .as_ref()
                    .map(|(_, view, _)| wgpu::RenderPassDepthStencilAttachment {
                        view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    })
            } else {
                None
            };
            passes += 1;
            if !frame_quads.is_empty() {
                draws += 1;
            }
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("sequoia-surface-clear"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: scene_target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(clear),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: depth_attachment,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            if !frame_quads.is_empty() {
                pass.set_pipeline(&self.quad_pipeline);
                pass.set_bind_group(0, &self.bind_group, &[]);
                pass.set_vertex_buffer(0, self.frame_buf.slice(..));
                pass.draw(0..4, 0..frame_quads.len() as u32);
            }
            drop(pass);
            if split_depth {
                if let Some((_, view, _)) = self.depth_tex.as_ref() {
                    passes += 1;
                    let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("sequoia-surface-depth-clear"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: scene_target,
                            depth_slice: None,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Load,
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                            view,
                            depth_ops: Some(wgpu::Operations {
                                load: wgpu::LoadOp::Clear(1.0),
                                store: wgpu::StoreOp::Store,
                            }),
                            stencil_ops: None,
                        }),
                        timestamp_writes: None,
                        occlusion_query_set: None,
                        multiview_mask: None,
                    });
                    drop(pass);
                }
            }
        }

        if !regions.is_empty() {
            let depth_view = self.depth_tex.as_ref().map(|(_, v, _)| v);
            let mut frame = WgpuFrame {
                device: &self.device,
                queue: &self.queue,
                encoder: &mut encoder,
                target: scene_target,
                depth: depth_view,
                format: self.config.format,
                size: fb,
                scale: s,
                region: regions[0].0,
                adapter: self.adapter.clone(),
                backend: self.backend,
                index: frame_index,
                delta,
                redraw: &self.redraw,
            };
            for (region, binding) in regions.iter() {
                frame.region = *region;
                (binding.draw)(&mut frame);
            }
        }

        if has_backdrops {
            let mut need_clear = !fb_drawn;
            if !blit_quads.is_empty() {
                let load = if need_clear {
                    wgpu::LoadOp::Clear(clear_color_linear(self.clear_color))
                } else {
                    wgpu::LoadOp::Load
                };
                need_clear = false;
                passes += 1;
                draws += blit_quads.len() as u64;
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("sequoia-backdrop-blits"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: scene_target,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                pass.set_pipeline(&self.blit_pipeline);
                pass.set_bind_group(0, &self.bind_group, &[]);
                pass.set_vertex_buffer(0, self.blit_buf.slice(..));
                for (i, key) in blit_keys.iter().enumerate() {
                    let entry = &self.surface_tex[key];
                    pass.set_bind_group(1, &entry.bind_group, &[]);
                    let i = i as u32;
                    pass.draw(0..4, i..i + 1);
                }
            }
            self.stats.frames += 1;

            let has_buffers = self.backdrop_tex.is_some();

            if has_filters {
                for plan in &comp_plans {
                    match plan {
                        CompPlan::Seg(seg) => {
                            if seg.has_content {
                                let load = if need_clear {
                                    wgpu::LoadOp::Clear(clear_color_linear(self.clear_color))
                                } else {
                                    wgpu::LoadOp::Load
                                };
                                need_clear = false;
                                passes += 1;
                                draws += gui_run_draws(&seg.runs, image_draws);
                                let mut pass =
                                    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                        label: Some("sequoia-backdrop-segment"),
                                        color_attachments: &[Some(
                                            wgpu::RenderPassColorAttachment {
                                                view: scene_target,
                                                depth_slice: None,
                                                resolve_target: None,
                                                ops: wgpu::Operations {
                                                    load,
                                                    store: wgpu::StoreOp::Store,
                                                },
                                            },
                                        )],
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
                                    self.atlas_tex.as_ref().map(|(_, bg)| bg),
                                    &self.quad_buf,
                                    quads,
                                    &self.glyph_buf,
                                    &self.image_buf,
                                    &self.gradient_buf,
                                    &seg.runs,
                                    image_draws,
                                    seg.all_opaque,
                                );
                            }
                            if has_buffers && let Some(idx) = seg.backdrop_idx {
                                let inst = backdrop_insts[idx as usize];
                                if let (Some((origin, extent)), Some(bufs)) = (
                                    Self::backdrop_copy_region(inst.rect, inst.params[0], fb),
                                    self.backdrop_tex.as_ref(),
                                ) {
                                    Self::draw_backdrop_marker_at(
                                        &self.backdrop_pipeline,
                                        &self.bind_group,
                                        bufs,
                                        origin,
                                        extent,
                                        &mut encoder,
                                        scene_target,
                                        &self.backdrop_buf,
                                        idx,
                                    );
                                    passes += 1;
                                    draws += 1;
                                }
                            }
                        }
                        CompPlan::Filter(fplan) => {
                            if need_clear {
                                passes += 1;
                                let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                    label: Some("sequoia-filter-clear"),
                                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                        view: scene_target,
                                        depth_slice: None,
                                        resolve_target: None,
                                        ops: wgpu::Operations {
                                            load: wgpu::LoadOp::Clear(clear_color_linear(
                                                self.clear_color,
                                            )),
                                            store: wgpu::StoreOp::Store,
                                        },
                                    })],
                                    depth_stencil_attachment: None,
                                    timestamp_writes: None,
                                    occlusion_query_set: None,
                                    multiview_mask: None,
                                });
                                drop(pass);
                                need_clear = false;
                            }
                            let mut composited = false;
                            if has_buffers
                                && let (Some(idx), Some(bufs)) =
                                    (fplan.filter_idx, self.backdrop_tex.as_ref())
                            {
                                Self::draw_filter_at(
                                    &self.backdrop_pipeline,
                                    &self.bind_group,
                                    &self.quad_pipeline,
                                    &self.quad_pipeline_opaque,
                                    &self.glyph_pipeline,
                                    &self.blit_pipeline,
                                    &self.gradient_pipeline,
                                    self.atlas_tex.as_ref().map(|(_, bg)| bg),
                                    bufs,
                                    &mut encoder,
                                    scene_target,
                                    quads,
                                    &self.backdrop_buf,
                                    idx,
                                    &self.quad_buf,
                                    &self.glyph_buf,
                                    &self.image_buf,
                                    &self.gradient_buf,
                                    &fplan.runs,
                                    image_draws,
                                    fplan.all_opaque,
                                );
                                passes += 2;
                                draws += gui_run_draws(&fplan.runs, image_draws) + 1;
                                self.stats.filter_spans += 1;
                                composited = true;
                            }
                            if !composited && fplan.has_content {
                                let load = if need_clear {
                                    wgpu::LoadOp::Clear(clear_color_linear(self.clear_color))
                                } else {
                                    wgpu::LoadOp::Load
                                };
                                need_clear = false;
                                passes += 1;
                                draws += gui_run_draws(&fplan.runs, image_draws);
                                let mut pass =
                                    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                        label: Some("sequoia-filter-fallback"),
                                        color_attachments: &[Some(
                                            wgpu::RenderPassColorAttachment {
                                                view: scene_target,
                                                depth_slice: None,
                                                resolve_target: None,
                                                ops: wgpu::Operations {
                                                    load,
                                                    store: wgpu::StoreOp::Store,
                                                },
                                            },
                                        )],
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
                                    self.atlas_tex.as_ref().map(|(_, bg)| bg),
                                    &self.quad_buf,
                                    quads,
                                    &self.glyph_buf,
                                    &self.image_buf,
                                    &self.gradient_buf,
                                    &fplan.runs,
                                    image_draws,
                                    fplan.all_opaque,
                                );
                            }
                        }
                    }
                }
            }
            for plan in &backdrop_plans {
                if plan.has_content {
                    let load = if need_clear {
                        wgpu::LoadOp::Clear(clear_color_linear(self.clear_color))
                    } else {
                        wgpu::LoadOp::Load
                    };
                    need_clear = false;
                    passes += 1;
                    draws += gui_run_draws(&plan.runs, image_draws);
                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("sequoia-backdrop-segment"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: scene_target,
                            depth_slice: None,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load,
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
                        self.atlas_tex.as_ref().map(|(_, bg)| bg),
                        &self.quad_buf,
                        quads,
                        &self.glyph_buf,
                        &self.image_buf,
                        &self.gradient_buf,
                        &plan.runs,
                        image_draws,
                        plan.all_opaque,
                    );
                }
                if has_buffers && let Some(idx) = plan.backdrop_idx {
                    let inst = backdrop_insts[idx as usize];

                    if let (Some((origin, extent)), Some(bufs)) = (
                        Self::backdrop_copy_region(inst.rect, inst.params[0], fb),
                        self.backdrop_tex.as_ref(),
                    ) {
                        Self::draw_backdrop_marker_at(
                            &self.backdrop_pipeline,
                            &self.bind_group,
                            bufs,
                            origin,
                            extent,
                            &mut encoder,
                            scene_target,
                            &self.backdrop_buf,
                            idx,
                        );
                        passes += 1;
                        draws += 1;
                    }
                }
            }
            if need_clear {
                passes += 1;
                let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("sequoia-backdrop-clear"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: scene_target,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(clear_color_linear(self.clear_color)),
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
            if has_buffers && let Some(bufs) = self.backdrop_tex.as_ref() {
                passes += 1;
                draws += 1;
                Self::present_backdrop_buffer(
                    &self.device,
                    &self.queue,
                    &mut self.present_buf,
                    &mut self.present_cap,
                    &self.present_pipeline,
                    &self.bind_group,
                    bufs,
                    self.clear_color,
                    fb,
                    &mut encoder,
                    &view,
                );
            }
        } else {
            let all_opaque = gradients.is_empty() && quads.iter().all(quad_is_opaque);

            {
                let load = if fb_drawn {
                    wgpu::LoadOp::Load
                } else {
                    wgpu::LoadOp::Clear(clear_color_linear(self.clear_color))
                };
                passes += 1;
                draws += blit_quads.len() as u64 + gui_run_draws(runs, image_draws);
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("sequoia-composite"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });

                if !blit_quads.is_empty() {
                    pass.set_pipeline(&self.blit_pipeline);
                    pass.set_bind_group(0, &self.bind_group, &[]);
                    pass.set_vertex_buffer(0, self.blit_buf.slice(..));
                    for (i, key) in blit_keys.iter().enumerate() {
                        let entry = &self.surface_tex[key];
                        pass.set_bind_group(1, &entry.bind_group, &[]);
                        let i = i as u32;
                        pass.draw(0..4, i..i + 1);
                    }
                }

                for run in runs.iter() {
                    match *run {
                        GuiRun::Quads(start, len) => {
                            let opaque = all_opaque || quad_range_is_opaque(quads, start, len);
                            pass.set_pipeline(if opaque {
                                &self.quad_pipeline_opaque
                            } else {
                                &self.quad_pipeline
                            });
                            pass.set_bind_group(0, &self.bind_group, &[]);
                            pass.set_vertex_buffer(0, self.quad_buf.slice(..));
                            pass.draw(0..4, start..start + len);
                        }
                        GuiRun::Glyphs(start, len) => {
                            if let Some((_, bg)) = &self.atlas_tex {
                                pass.set_pipeline(&self.glyph_pipeline);
                                pass.set_bind_group(0, &self.bind_group, &[]);
                                pass.set_bind_group(1, bg, &[]);
                                pass.set_vertex_buffer(0, self.glyph_buf.slice(..));
                                pass.draw(0..4, start..start + len);
                            }
                        }
                        GuiRun::Images(start, len) => {
                            if image_draws.is_empty() {
                                continue;
                            }
                            pass.set_pipeline(&self.blit_pipeline);
                            pass.set_bind_group(0, &self.bind_group, &[]);
                            pass.set_vertex_buffer(0, self.image_buf.slice(..));
                            for (idx, bg) in image_draws.iter() {
                                if *idx < start || *idx >= start + len {
                                    continue;
                                }
                                pass.set_bind_group(1, bg, &[]);
                                pass.draw(0..4, *idx..*idx + 1);
                            }
                        }
                        GuiRun::Gradients(start, len) => {
                            pass.set_pipeline(&self.gradient_pipeline);
                            pass.set_bind_group(0, &self.bind_group, &[]);
                            pass.set_vertex_buffer(0, self.gradient_buf.slice(..));
                            pass.draw(0..4, start..start + len);
                        }
                    }
                }
            }
        }
        self.stats.quad_instances = quads.len() as u64;
        self.stats.glyph_instances = glyphs.len() as u64;
        self.stats.image_instances = images.len() as u64;
        self.stats.gradient_instances = gradients.len() as u64;
        self.stats.draw_calls = draws;
        self.stats.passes = passes;
        self.queue.submit([encoder.finish()]);
        frame.present();
        Ok(())
    }
}
