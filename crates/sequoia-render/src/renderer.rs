//! Frame orchestration for the main window surface.
//!
//! [`GpuRenderer`] owns the wgpu device, window surface, pipelines, caches, and
//! reusable instance buffers. Each call to [`GpuRenderer::render`] prepares the
//! glyph atlas and cached surfaces, groups the scene, and records the passes.

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
use crate::gpu::resources::{UserShaderEntry, vertex_buffer};
use crate::gpu::shaders::{passthrough_wgsl, validate_user_shader};
use crate::pass::opacity::*;
use crate::scene::grouping::*;
use crate::scene::instances::*;
use crate::scene::planning::*;
use crate::surface::*;
use crate::types::*;
use crate::utils::*;
use sequoia_core::{BlendMode, Color, Rect, RenderList, ShaderId, TexInput};
use sequoia_view::SurfaceDraw;

/// Owns the wgpu device and window surface and renders frames into it.
pub struct GpuRenderer {
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
    pub(crate) adapter: wgpu::AdapterInfo,
    pub(crate) backend: GpuBackend,
    pub(crate) surface: wgpu::Surface<'static>,
    pub(crate) config: wgpu::SurfaceConfiguration,
    pub(crate) render_format: wgpu::TextureFormat,
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
    pub(crate) surface_blit_pipeline: wgpu::RenderPipeline,
    pub(crate) blit_sampler: wgpu::Sampler,
    pub(crate) present_buf: wgpu::Buffer,
    pub(crate) present_cap: usize,
    pub(crate) backdrop_buf: wgpu::Buffer,
    pub(crate) backdrop_cap: usize,
    pub(crate) backdrop_pipeline: wgpu::RenderPipeline,
    pub(crate) backdrop_blur_pipeline: wgpu::RenderPipeline,
    pub(crate) gradient_pipeline: wgpu::RenderPipeline,
    pub(crate) gradient_buf: wgpu::Buffer,
    pub(crate) gradient_cap: usize,
    pub(crate) present_pipeline: wgpu::RenderPipeline,
    pub(crate) backdrop_tex: Option<BackdropBuffers>,
    pub(crate) frame_target: Option<FrameTarget>,
    pub(crate) frame_target_valid: bool,
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
    pub(crate) trace_acquires: u64,
    pub(crate) trace_atlas_recreates: u64,
    pub(crate) redraw: Cell<bool>,
    pub(crate) skipped: Cell<Option<FrameSkip>>,
    pub(crate) stats: RenderStats,
    pub(crate) warned_filter_combo: bool,
    pub(crate) warned_cached_formats: Vec<wgpu::TextureFormat>,
    pub(crate) scratch_quads: Vec<GpuQuad>,
    pub(crate) scratch_glyphs: Vec<GpuGlyph>,
    pub(crate) scratch_runs: Vec<GuiRun>,
    pub(crate) scratch_frame: Vec<GpuQuad>,
    pub(crate) scratch_blits: Vec<BlitQuad>,
    pub(crate) scratch_blit_keys: Vec<usize>,
    pub(crate) scratch_images: Vec<BlitQuad>,
    pub(crate) scratch_image_keys: Vec<u64>,
    pub(crate) scratch_gradients: Vec<GpuGradient>,
    pub(crate) scratch_opacities: Vec<f32>,
    pub(crate) group_layers: Vec<GroupLayer>,
    pub(crate) scratch_image_draws: Vec<(u32, wgpu::BindGroup)>,
    pub(crate) scratch_seen: Vec<usize>,
    pub(crate) scratch_cached: Vec<CachedJob>,
    pub(crate) scratch_regions: Vec<(Region, Rc<SurfaceBinding>)>,
    pub(crate) atlas_scratch: Vec<u8>,
    pub(crate) user_shaders: HashMap<ShaderId, UserShaderEntry>,
    pub(crate) by_name: HashMap<String, ShaderId>,
    pub(crate) next_shader_id: u32,
    pub(crate) shader_data: ShaderGroupData,
    pub(crate) shader_buf: wgpu::Buffer,
    pub(crate) shader_cap: usize,
    pub(crate) uniform_ring: wgpu::Buffer,
    pub(crate) uniform_cap: usize,
    pub(crate) shader_bgl: wgpu::BindGroupLayout,
    pub(crate) shader_bind_group: wgpu::BindGroup,
    pub(crate) shader_white_bg: wgpu::BindGroup,
    pub(crate) _shader_white_tex: wgpu::Texture,
    pub(crate) scratch_shader_textures: Vec<ShaderTextureGroups>,
    pub(crate) render_targets: HashMap<sequoia_core::RenderTargetId, RenderTargetTexture>,
    pub(crate) target_cache: usize,
    pub(crate) target_allocs: u64,
    pub(crate) target_reuses: u64,
    #[cfg(debug_assertions)]
    pub(crate) pipeline_builds: u64,
}

fn create_white_bind_group(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
) -> (wgpu::Texture, wgpu::BindGroup) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("sequoia-shader-white"),
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &[255u8; 4],
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4),
            rows_per_image: None,
        },
        wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("sequoia-shader-white-bg"),
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
    (texture, bind_group)
}

fn create_instance(backend: GpuBackend) -> wgpu::Instance {
    let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
    desc.backends = backend.wgpu_backends();
    desc.flags = wgpu::InstanceFlags::VALIDATION_INDIRECT_CALL.with_env();
    wgpu::Instance::new(desc)
}

/// Returns backend candidates in preference order for the current platform.
pub(crate) fn auto_backend_candidates() -> &'static [GpuBackend] {
    #[cfg(target_os = "macos")]
    {
        &[GpuBackend::Metal]
    }
    #[cfg(target_os = "ios")]
    {
        &[GpuBackend::Metal]
    }
    #[cfg(target_os = "windows")]
    {
        &[GpuBackend::Vulkan, GpuBackend::Dx12, GpuBackend::Gl]
    }
    #[cfg(target_os = "android")]
    {
        &[GpuBackend::Vulkan, GpuBackend::Gl]
    }
    #[cfg(target_arch = "wasm32")]
    {
        &[GpuBackend::WebGpu]
    }
    #[cfg(not(any(
        target_os = "macos",
        target_os = "ios",
        target_os = "windows",
        target_os = "android",
        target_arch = "wasm32"
    )))]
    {
        &[GpuBackend::Vulkan, GpuBackend::Gl]
    }
}

impl GpuRenderer {
    /// Creates a renderer for `window` using default options.
    pub fn new(window: Arc<Window>, width: u32, height: u32) -> Result<Self, RenderError> {
        Self::new_with_options(window, width, height, GpuOptions::default())
    }

    /// Creates a renderer using `options`.
    ///
    /// When the backend is [`GpuBackend::Auto`], platform candidates are tried
    /// in order until an adapter and surface can be created.
    pub fn new_with_options(
        window: Arc<Window>,
        width: u32,
        height: u32,
        options: GpuOptions,
    ) -> Result<Self, RenderError> {
        let candidates: Vec<GpuBackend> = match options.backend {
            GpuBackend::Auto => auto_backend_candidates().to_vec(),
            backend => vec![backend],
        };

        let mut selected = None;
        for backend in candidates {
            let instance = create_instance(backend);
            let Ok(surface) = instance.create_surface(window.clone()) else {
                continue;
            };
            let Ok(adapter) =
                pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: options.power_preference.wgpu(),
                    compatible_surface: Some(&surface),
                    force_fallback_adapter: false,
                }))
            else {
                continue;
            };
            selected = Some((instance, surface, adapter));
            break;
        }
        let (_instance, surface, adapter) = selected.ok_or(RenderError::NoAdapter)?;
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
            memory_hints: wgpu::MemoryHints::Manual {
                suballocated_device_memory_block_size: (4 << 20)..(32 << 20),
            },
            trace: wgpu::Trace::Off,
        }))
        .map_err(RenderError::Device)?;

        device.on_uncaptured_error(Arc::new(|error| {
            log::error!("sequoia-render: uncaptured GPU error: {error}");
        }));

        let caps = surface.get_capabilities(&adapter);
        let mut resolved = resolve_surface_format(&caps.formats, options.surface_format)
            .ok_or(RenderError::NoAdapter)?;
        if !resolved.view_formats.is_empty()
            && !adapter
                .get_downlevel_capabilities()
                .flags
                .contains(wgpu::DownlevelFlags::SURFACE_VIEW_FORMATS)
        {
            log::warn!(
                "sequoia-render: surface view formats are unavailable - using the default sRGB format"
            );
            resolved = resolve_surface_format(&caps.formats, None).ok_or(RenderError::NoAdapter)?;
        }
        let format = resolved.render;
        let present_mode = resolve_present_mode(&caps.present_modes, options.present_mode);
        if present_mode != options.present_mode.wgpu() {
            log::warn!(
                "sequoia-render: present mode `{}` unavailable on this surface - using vsync",
                options.present_mode.name()
            );
        }

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: resolved.config,
            width: width.max(1),
            height: height.max(1),
            present_mode,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: resolved.view_formats,
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
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });

        let (shader_white_tex, shader_white_bg) =
            create_white_bind_group(&device, &queue, &atlas_bgl, &blit_sampler);

        let shader = create_shader_module(&device);

        let (layout, glyph_layout) = create_pipeline_layouts(&device, &screen_bgl, &atlas_bgl);

        let blend = premultiplied_blend();

        let quad_pipeline = create_quad_pipeline(&device, &shader, &layout, format, blend);

        let quad_pipeline_opaque = create_quad_opaque_pipeline(&device, &shader, &layout, format);

        let glyph_pipeline = create_glyph_pipeline(&device, &shader, &glyph_layout, format, blend);

        let blit_pipeline = create_blit_pipeline(&device, &shader, &glyph_layout, format, blend);

        let surface_blit_pipeline =
            create_surface_blit_pipeline(&device, &shader, &glyph_layout, format, blend);

        let backdrop_buf = vertex_buffer(
            &device,
            "sequoia-backdrops",
            (INITIAL_BACKDROP_CAPACITY * BACKDROP_BYTES) as u64,
        );

        let backdrop_pipeline =
            create_backdrop_pipeline(&device, &shader, &glyph_layout, format, blend);

        let backdrop_blur_pipeline =
            create_backdrop_blur_pipeline(&device, &shader, &glyph_layout, format);

        let present_pipeline = create_present_pipeline(&device, &shader, &glyph_layout, format);

        let quad_ceiling = instance_ceiling(&device, QUAD_BYTES);
        let glyph_ceiling = instance_ceiling(&device, GLYPH_BYTES);
        let image_ceiling = instance_ceiling(&device, BLIT_BYTES);
        let gradient_ceiling = instance_ceiling(&device, GRADIENT_BYTES);

        let gradient_pipeline = create_gradient_pipeline(&device, &shader, &layout, format, blend);

        let shader_bgl = create_user_shader_bind_group_layout(&device);
        let shader_buf = vertex_buffer(
            &device,
            "sequoia-shaders",
            (INITIAL_SHADER_CAPACITY * SHADER_BYTES) as u64,
        );
        let uniform_ring = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sequoia-shader-uniforms"),
            size: (INITIAL_SHADER_CAPACITY * SHADER_UNIFORM_STRIDE) as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let shader_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sequoia-shader-bg"),
            layout: &shader_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &uniform_ring,
                        offset: 0,
                        size: Some(wgpu::BufferSize::new(SHADER_UNIFORM_STRIDE as u64).unwrap()),
                    }),
                },
            ],
        });

        // Built-in passthrough shader (id 0).
        let passthrough_src = passthrough_wgsl();
        let passthrough_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("passthrough"),
            source: wgpu::ShaderSource::Wgsl(passthrough_src.into()),
        });
        let passthrough_layout =
            create_user_shader_pipeline_layout(&device, &shader_bgl, &atlas_bgl);
        let passthrough_pipeline = create_user_shader_pipeline(
            &device,
            &passthrough_module,
            &passthrough_layout,
            format,
            blend_component(sequoia_core::BlendMode::Normal),
        );
        let mut passthrough_pipelines = HashMap::new();
        passthrough_pipelines.insert(sequoia_core::BlendMode::Normal, passthrough_pipeline);
        let passthrough_entry = UserShaderEntry {
            pipelines: passthrough_pipelines,
            bgl: shader_bgl.clone(),
            module: passthrough_module,
        };
        let mut passthrough_shaders = HashMap::new();
        passthrough_shaders.insert(ShaderId::new(0), passthrough_entry);
        let mut passthrough_names = HashMap::new();
        passthrough_names.insert("passthrough".to_string(), ShaderId::new(0));

        Ok(Self {
            device,
            queue,
            adapter: adapter_info,
            backend: options.backend,
            surface,
            config,
            render_format: format,
            quad_pipeline,
            quad_pipeline_opaque,
            glyph_pipeline,
            blit_pipeline,
            surface_blit_pipeline,
            blit_sampler,
            backdrop_buf,
            backdrop_cap: INITIAL_BACKDROP_CAPACITY,
            backdrop_pipeline,
            backdrop_blur_pipeline,
            gradient_pipeline,
            gradient_buf,
            gradient_cap: INITIAL_GRADIENT_CAPACITY,
            present_pipeline,
            backdrop_tex: None,
            frame_target: None,
            frame_target_valid: false,
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
            trace_acquires: 0,
            trace_atlas_recreates: 0,
            redraw: Cell::new(false),
            skipped: Cell::new(None),
            stats: RenderStats::default(),
            warned_filter_combo: false,
            warned_cached_formats: Vec::new(),
            scratch_quads: Vec::with_capacity(INITIAL_QUAD_CAPACITY),
            scratch_glyphs: Vec::with_capacity(INITIAL_GLYPH_CAPACITY),
            scratch_runs: Vec::new(),
            scratch_frame: Vec::with_capacity(INITIAL_FRAME_CAPACITY),
            scratch_blits: Vec::with_capacity(INITIAL_BLIT_CAPACITY),
            scratch_blit_keys: Vec::with_capacity(INITIAL_BLIT_CAPACITY),
            scratch_images: Vec::with_capacity(INITIAL_IMAGE_CAPACITY),
            scratch_image_keys: Vec::with_capacity(INITIAL_IMAGE_CAPACITY),
            scratch_gradients: Vec::with_capacity(INITIAL_GRADIENT_CAPACITY),
            scratch_opacities: Vec::new(),
            group_layers: Vec::new(),
            scratch_image_draws: Vec::with_capacity(INITIAL_IMAGE_CAPACITY),
            scratch_seen: Vec::new(),
            scratch_cached: Vec::new(),
            scratch_regions: Vec::new(),
            atlas_scratch: Vec::new(),
            user_shaders: passthrough_shaders,
            by_name: passthrough_names,
            next_shader_id: 1,
            shader_data: ShaderGroupData::default(),
            shader_buf,
            shader_cap: INITIAL_SHADER_CAPACITY,
            uniform_ring,
            uniform_cap: INITIAL_SHADER_CAPACITY,
            shader_bgl,
            shader_bind_group,
            shader_white_bg,
            _shader_white_tex: shader_white_tex,
            scratch_shader_textures: Vec::new(),
            render_targets: HashMap::new(),
            target_cache: 8,
            target_allocs: 0,
            target_reuses: 0,
            #[cfg(debug_assertions)]
            pipeline_builds: 1,
        })
    }

    /// Returns information about the selected adapter.
    pub fn adapter_info(&self) -> wgpu::AdapterInfo {
        self.adapter.clone()
    }

    /// Returns the backend selected at creation time.
    pub fn backend(&self) -> GpuBackend {
        self.backend
    }

    /// Returns the swapchain format used by the window.
    pub fn surface_format(&self) -> wgpu::TextureFormat {
        self.config.format
    }

    /// Returns the format used by render pipelines and offscreen targets.
    pub fn render_format(&self) -> wgpu::TextureFormat {
        self.render_format
    }

    /// Consumes a pending redraw request made through [`WgpuFrame::request_frame`].
    pub fn take_redraw_request(&self) -> bool {
        self.redraw.take()
    }

    /// Consumes the reason for the last skipped frame, if any.
    pub fn take_frame_skip(&self) -> Option<FrameSkip> {
        self.skipped.take()
    }

    /// Returns the accumulated render statistics without resetting them.
    pub fn stats(&self) -> RenderStats {
        self.stats
    }

    /// Human-readable summary of the GPU allocations this renderer owns.
    ///
    /// Covers our own textures and instance buffers, not driver-private
    /// staging memory, so a report that keeps growing points at resources being
    /// recreated every frame.
    pub fn memory_report(&self) -> String {
        fn mb(bytes: f64) -> f64 {
            bytes / 1_048_576.0
        }
        fn tex(size: (u32, u32)) -> f64 {
            mb(size.0 as f64 * size.1 as f64 * 4.0)
        }
        let mut surfaces = 0.0;
        for s in self.surface_tex.values() {
            surfaces += tex(s.size) * if s.msaa.is_some() { 2.0 } else { 1.0 };
        }
        let mut images = 0.0;
        for i in self.images.values() {
            images += tex((i.width, i.height));
        }
        let mut groups = 0.0;
        for g in &self.group_layers {
            groups += tex(g.size);
        }
        let frame = self
            .frame_target
            .as_ref()
            .map(|t| tex(t.size))
            .unwrap_or(0.0);
        let backdrop = self
            .backdrop_tex
            .as_ref()
            .map(|b| tex(b.size) * 3.0)
            .unwrap_or(0.0);
        let depth = self
            .depth_tex
            .as_ref()
            .map(|(_, _, size)| tex(*size))
            .unwrap_or(0.0);
        let atlas = self
            .atlas_synced
            .map(|(_, w, h)| tex((w, h)))
            .unwrap_or(0.0);
        let bufs = mb((self.capacity * QUAD_BYTES
            + self.glyph_capacity * GLYPH_BYTES
            + self.image_cap * BLIT_BYTES
            + self.frame_cap * QUAD_BYTES
            + self.blit_cap * BLIT_BYTES
            + self.present_cap * BLIT_BYTES
            + self.gradient_cap * GRADIENT_BYTES
            + self.backdrop_cap * BACKDROP_BYTES) as f64);
        format!(
            "gpu: target={frame:.1} backdrop={backdrop:.1} depth={depth:.1} atlas={atlas:.1} \
             groups={groups:.1}(n={}) surfaces={surfaces:.1}(n={}) images={images:.1}(n={}) \
             bufs={bufs:.1}MB acq={} atlas_new={}",
            self.group_layers.len(),
            self.surface_tex.len(),
            self.images.len(),
            self.trace_acquires,
            self.trace_atlas_recreates,
        )
    }

    /// Returns the accumulated render statistics and resets the counters.
    pub fn take_stats(&mut self) -> RenderStats {
        std::mem::take(&mut self.stats)
    }

    fn cached_surface_format(
        device: &wgpu::Device,
        warned: &mut Vec<wgpu::TextureFormat>,
        requested: wgpu::TextureFormat,
    ) -> wgpu::TextureFormat {
        if texture_format_usable(device, requested) {
            return requested;
        }
        if !warned.contains(&requested) {
            warned.push(requested);
            log::warn!(
                "sequoia-render: surface format {:?} cannot be used as an offscreen target - using the default",
                requested
            );
        }
        default_surface_format(device)
    }

    /// Sets the color used to clear the window and cached surfaces.
    pub fn set_clear_color(&mut self, color: Color) {
        self.clear_color = color;
    }

    /// Returns the current clear color.
    pub fn clear_color(&self) -> Color {
        self.clear_color
    }

    /// Destroys all cached image textures.
    pub fn clear_image_cache(&mut self) {
        for (_, img) in self.images.drain() {
            img.texture.destroy();
        }
        self.image_order.clear();
        self.image_failures.clear();
    }

    /// Destroys the cached texture for one image key.
    pub fn remove_image(&mut self, key: u64) {
        if let Some(img) = self.images.remove(&key) {
            img.texture.destroy();
            self.image_order.retain(|k| *k != key);
        }
        self.image_failures.remove(&key);
    }

    /// Resizes the window surface; zero dimensions are clamped to one.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.config.width = width.max(1);
        self.config.height = height.max(1);
        self.surface.configure(&self.device, &self.config);
    }

    /// Compiles and registers a user shader from a complete WGSL module.
    ///
    /// The source must define `vs_main` and `fs_main` entry points matching the
    /// engine ABI. [`crate::compose_user_shader`] can wrap a bare `fs_user`
    /// function when the shader does not need a custom vertex stage.
    ///
    /// Registering an existing name replaces that shader in place, keeping its
    /// [`ShaderId`] stable. Pipelines for blend modes other than
    /// [`BlendMode::Normal`] are built lazily the first time they are drawn.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError::ShaderCompile`] with the naga or wgpu diagnostic
    /// when parsing, validation, or pipeline creation fails.
    pub fn register_shader(
        &mut self,
        name: impl Into<String>,
        wgsl: impl AsRef<str>,
    ) -> Result<ShaderId, RenderError> {
        let name = name.into();
        let wgsl = wgsl.as_ref();
        validate_user_shader(wgsl)?;

        let pipeline_layout =
            create_user_shader_pipeline_layout(&self.device, &self.shader_bgl, &self.atlas_bgl);
        let error_scope = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let module = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(name.as_str()),
                source: wgpu::ShaderSource::Wgsl(wgsl.into()),
            });
        let pipeline = create_user_shader_pipeline(
            &self.device,
            &module,
            &pipeline_layout,
            self.render_format,
            blend_component(BlendMode::Normal),
        );
        if let Some(error) = pollster::block_on(error_scope.pop()) {
            return Err(RenderError::ShaderCompile(error.to_string()));
        }

        let mut pipelines = HashMap::new();
        pipelines.insert(BlendMode::Normal, pipeline);
        let id = if let Some(id) = self.by_name.get(&name).copied() {
            id
        } else {
            let id = ShaderId::new(self.next_shader_id);
            self.next_shader_id += 1;
            id
        };
        self.user_shaders.insert(
            id,
            UserShaderEntry {
                pipelines,
                bgl: self.shader_bgl.clone(),
                module,
            },
        );
        self.by_name.insert(name, id);
        #[cfg(debug_assertions)]
        {
            self.pipeline_builds += 1;
        }
        Ok(id)
    }

    /// Looks up a previously registered shader by name.
    #[must_use]
    pub fn shader_id(&self, name: &str) -> Option<ShaderId> {
        self.by_name.get(name).copied()
    }

    /// Returns how many user-shader pipelines have been built.
    ///
    /// Debug builds only; useful for asserting that blend modes get separate
    /// pipelines without inspecting GPU objects.
    #[cfg(debug_assertions)]
    #[must_use]
    pub fn pipeline_builds(&self) -> u64 {
        self.pipeline_builds
    }

    /// Builds the pipeline for one shader and blend mode if it does not exist.
    ///
    /// Returns `true` when a new pipeline was created.
    pub(crate) fn ensure_shader_pipeline(&mut self, shader: ShaderId, blend: BlendMode) -> bool {
        let Some(entry) = self.user_shaders.get(&shader) else {
            return false;
        };
        if entry.pipelines.contains_key(&blend) {
            return false;
        }
        let module = entry.module.clone();
        let bgl = entry.bgl.clone();
        let atlas_bgl = self.atlas_bgl.clone();
        if blend_mode_is_advanced(blend) {
            log::warn!("sequoia-render: blend mode {blend:?} falls back to Normal until stage 4");
        }
        let layout = create_user_shader_pipeline_layout(&self.device, &bgl, &atlas_bgl);
        let pipeline = create_user_shader_pipeline(
            &self.device,
            &module,
            &layout,
            self.render_format,
            blend_component(blend),
        );
        if let Some(entry) = self.user_shaders.get_mut(&shader) {
            entry.pipelines.insert(blend, pipeline);
        }
        #[cfg(debug_assertions)]
        {
            self.pipeline_builds += 1;
        }
        true
    }

    /// Pre-builds every user-shader pipeline needed by this frame.
    ///
    /// Doing this before recording passes keeps pipeline creation out of the
    /// hot drawing loop and makes the per-blend-mode cache observable in debug.
    fn prepare_shader_pipelines(&mut self, list: &RenderList) {
        for item in list.items() {
            if let sequoia_core::RenderItem::Shader(draw) = item {
                self.ensure_shader_pipeline(draw.shader, draw.blend);
            }
        }
    }

    /// Renders `list` into the window surface.
    ///
    /// `scale` converts logical coordinates to physical pixels and must be
    /// positive; invalid values fall back to `1.0`. `surfaces` contains the
    /// embedded surface callbacks produced by the view system.
    pub fn render(
        &mut self,
        list: &RenderList,
        atlas: &mut GlyphAtlas,
        scale: f32,
        surfaces: &[SurfaceDraw],
    ) -> Result<(), RenderError> {
        self.render_with_damage(list, atlas, scale, surfaces, None)
    }

    /// Renders `list`, limiting the colour writes to `damage` when possible.
    ///
    /// `damage` is in logical window coordinates. `None` means a full repaint.
    /// Composited paths (backdrops, filters, opacity groups, embedded surfaces,
    /// and cached surface blits) currently fall back to a full repaint.
    pub fn render_with_damage(
        &mut self,
        list: &RenderList,
        atlas: &mut GlyphAtlas,
        scale: f32,
        surfaces: &[SurfaceDraw],
        damage: Option<Rect>,
    ) -> Result<(), RenderError> {
        self.render_with_damage_regions(
            list,
            atlas,
            scale,
            surfaces,
            damage.as_ref().map(std::slice::from_ref),
        )
    }

    /// Renders `list`, limiting colour writes to multiple disjoint `damage`
    /// regions when possible.
    ///
    /// Regions are in logical window coordinates. `None` or an empty slice
    /// means a full repaint. Each region gets its own scissored clear/draw
    /// pass. At most eight regions are used; extra inputs collapse into one
    /// bounding box.
    ///
    /// The same fallbacks as [`render_with_damage`](Self::render_with_damage)
    /// apply: composited paths and embedded surfaces repaint the full frame.
    pub fn render_with_damage_regions(
        &mut self,
        list: &RenderList,
        atlas: &mut GlyphAtlas,
        scale: f32,
        surfaces: &[SurfaceDraw],
        damage: Option<&[Rect]>,
    ) -> Result<(), RenderError> {
        const MAX_SURFACE_ACQUIRE_ATTEMPTS: u32 = 4;

        if self.config.width == 0 || self.config.height == 0 {
            return Ok(());
        }
        let s = if scale.is_finite() && scale > 0.0 {
            scale
        } else {
            1.0
        };
        self.sync_atlas(atlas);
        self.prepare_shader_pipelines(list);

        let mut reconfigured = false;
        let mut attempts = 0u32;
        let frame = loop {
            self.trace_acquires = self.trace_acquires.wrapping_add(1);
            match self.surface.get_current_texture() {
                wgpu::CurrentSurfaceTexture::Success(frame) => break frame,
                wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                    if reconfigured {
                        break frame;
                    }
                    attempts += 1;
                    drop(frame);
                    self.surface.configure(&self.device, &self.config);
                    reconfigured = true;
                }
                wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                    // A configure allocates a fresh set of swapchain images, so
                    // retrying without a bound would churn memory every frame
                    // when the surface never reports a usable image.
                    attempts += 1;
                    if attempts > MAX_SURFACE_ACQUIRE_ATTEMPTS {
                        self.skipped.set(Some(FrameSkip::Timeout));
                        return Ok(());
                    }
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
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor {
            format: (self.render_format != self.config.format).then_some(self.render_format),
            ..Default::default()
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("sequoia-frame"),
            });

        let mut passes: u64 = 0;
        let mut draws: u64 = 0;

        let fb = (self.config.width, self.config.height);

        // Target passes read the screen-size uniform in their vertex stage, so
        // it has to be current before those passes are recorded and submitted.
        let screen = [
            self.config.width as f32,
            self.config.height as f32,
            0.0,
            0.0,
        ];
        self.queue
            .write_buffer(&self.uniform_buf, 0, bytemuck::cast_slice(&screen));

        // Offscreen targets are rendered before the scene that samples them.
        // Every target pass submits on its own because all passes share the
        // instance buffers: one more `write_buffer` would otherwise replace the
        // instances of an earlier pass before the GPU executes it.
        let (target_encoder, target_passes) = self.execute_target_passes(list, encoder, s, fb);
        encoder = target_encoder;
        passes += target_passes;

        // Backdrops and filters render into an intermediate target first.
        let has_backdrops = list.has_backdrops();

        let has_filters = list.has_filters();
        if has_backdrops || has_filters {
            self.ensure_backdrop_buffers(fb);
        }
        let backdrop_a_view: Option<wgpu::TextureView> = (has_backdrops || has_filters)
            .then(|| self.backdrop_tex.as_ref().map(|b| b.a_view.clone()))
            .flatten();

        // Partial repaint needs a persistent colour target. Composited paths
        // (backdrops, filters, opacity groups) and embedded surface callbacks
        // keep the old full-repaint path for now.
        let use_offscreen = !has_backdrops && !has_filters && !list.has_opacity_groups();
        if use_offscreen {
            if self.ensure_frame_target(fb) {
                self.frame_target_valid = false;
            }
        } else {
            self.frame_target_valid = false;
        }
        let frame_view: Option<wgpu::TextureView> = use_offscreen
            .then(|| self.frame_target.as_ref().map(|t| t.view.clone()))
            .flatten();
        let scene_target: &wgpu::TextureView = frame_view
            .as_ref()
            .or(backdrop_a_view.as_ref())
            .unwrap_or(&view);
        let clear_color = self.clear_color;

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
            let offscreen =
                binding.cached || binding.format.is_some_and(|f| f != self.render_format);
            if offscreen {
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

        // Shadows and antialiasing reach beyond the painted rect, so the
        // scissor is deliberately more generous than the tracker's 2 px.
        const DAMAGE_MARGIN: f32 = 32.0;

        // Embedded surface callbacks and cached blits are not scissored, so
        // they stay on the full-repaint path even though the colour target is
        // still offscreen.
        let partial_regions = if use_offscreen
            && self.frame_target_valid
            && regions.is_empty()
            && cached.is_empty()
        {
            damage
                .map(|regions| damage_scissors(regions, DAMAGE_MARGIN, s, fb))
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let partial = !partial_regions.is_empty();

        for (index, (damage, _)) in partial_regions.iter().enumerate() {
            // Leading instances replace old pixels in each damage region,
            // then surface decorations and content are drawn over them.
            frame_quads.insert(
                index,
                quad_instance(*damage, linear_rgba(clear_color), 0.0, [0.0; 4], 0.0),
            );
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
            let requested = job.binding.format.unwrap_or(DEFAULT_SURFACE_FORMAT);
            let format = Self::cached_surface_format(
                &self.device,
                &mut self.warned_cached_formats,
                requested,
            );
            let samples = supported_samples(format, job.binding.samples);
            let recreate = match self.surface_tex.get(&job.binding.key) {
                Some(e) => e.size != job.size || e.samples != samples || e.format != format,
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
                        format,
                    ),
                );
            }
            if recreate || (job.binding.check_dirty)() {
                let entry = &self.surface_tex[&job.binding.key];

                let clear = clear_color_linear(self.clear_color);
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
                    format: entry.format,
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

        // Convert paint items into instanced runs, splitting the scene around
        // backdrop markers and filter ranges.
        let viewport = Rect::from_xywh(0.0, 0.0, fb.0 as f32 / s, fb.1 as f32 / s);
        let opacity_plan = list.has_opacity_groups().then(|| list.opacity_plan());
        let opacity_depth = opacity_plan
            .as_ref()
            .map(|plan| opacity_plan_depth(plan))
            .unwrap_or(0);
        let target_ranges: Vec<std::ops::Range<usize>> = list
            .target_plan()
            .into_iter()
            .filter_map(|node| match node {
                sequoia_core::PassNode::Render { items, .. } => Some(items),
                _ => None,
            })
            .collect();
        let has_targets = !target_ranges.is_empty() && !has_backdrops && !has_filters;
        if !target_ranges.is_empty() && !has_targets {
            log::warn!(
                "sequoia-render: offscreen targets are ignored while backdrops or filters are present"
            );
        }
        let use_opacity_layers = !has_backdrops
            && !has_filters
            && !has_targets
            && opacity_depth > 0
            && opacity_depth <= MAX_GROUP_DEPTH
            && blit_quads.is_empty()
            && frame_quads.is_empty()
            && regions.is_empty();
        if use_opacity_layers {
            Self::ensure_group_layers(
                &self.device,
                &self.atlas_bgl,
                &self.blit_sampler,
                self.render_format,
                &mut self.group_layers,
                fb,
                opacity_depth,
            );
        }

        let item_opacities = &mut self.scratch_opacities;
        item_opacities.clear();
        if list.has_opacity_groups() {
            let len = list.items().len();
            item_opacities.resize(len, 1.0);
            for span in list.opacity_spans() {
                let start = span.start.min(len);
                let end = span.end.min(len);
                for o in &mut item_opacities[start..end] {
                    *o *= span.opacity;
                }
            }
        }

        // Items that belong to an offscreen target are drawn by the target pass,
        // not by the scene pass, so they are filtered out here together with the
        // target markers. Opacities stay aligned with the filtered item order.
        let mut main_items: Vec<sequoia_core::RenderItem> = Vec::new();
        let mut main_clips: Vec<Option<sequoia_core::RoundedClip>> = Vec::new();
        let mut main_transforms: Vec<sequoia_core::Transform> = Vec::new();
        let mut main_opacities: Vec<f32> = Vec::new();
        if has_targets {
            for (index, item) in list.items().iter().enumerate() {
                if matches!(
                    item,
                    sequoia_core::RenderItem::TargetBegin { .. }
                        | sequoia_core::RenderItem::TargetEnd { .. }
                ) || target_ranges.iter().any(|range| range.contains(&index))
                {
                    continue;
                }
                main_items.push(item.clone());
                main_clips.push(list.item_clips()[index]);
                main_transforms.push(list.item_transforms()[index]);
                main_opacities.push(item_opacities.get(index).copied().unwrap_or(1.0));
            }
        }
        let (scene_items, scene_clips, scene_transforms, scene_opacities) = if has_targets {
            (
                main_items.as_slice(),
                main_clips.as_slice(),
                main_transforms.as_slice(),
                main_opacities.as_slice(),
            )
        } else {
            (
                list.items(),
                list.item_clips(),
                list.item_transforms(),
                item_opacities.as_slice(),
            )
        };

        let quads = &mut self.scratch_quads;
        let glyphs = &mut self.scratch_glyphs;
        let images = &mut self.scratch_images;
        let image_keys = &mut self.scratch_image_keys;
        let gradients = &mut self.scratch_gradients;
        let runs = &mut self.scratch_runs;
        self.shader_data = ShaderGroupData::default();
        let mut backdrop_plans: Vec<BackdropSegPlan> = Vec::new();
        let mut backdrop_insts: Vec<GpuBackdrop> = Vec::new();
        let mut comp_plans: Vec<CompPlan> = Vec::new();
        let mut opacity_draws: Option<Vec<OpacityDraw>> = None;
        let mut group_composites: Vec<GpuBackdrop> = Vec::new();
        if !has_backdrops && !has_filters {
            if use_opacity_layers {
                let draws = build_opacity_draws(
                    opacity_plan.as_ref().expect("opacity plan"),
                    list.items(),
                    list.item_clips(),
                    list.item_transforms(),
                    s,
                    Some(viewport),
                    (
                        self.quad_ceiling,
                        self.glyph_ceiling,
                        self.image_ceiling,
                        self.gradient_ceiling,
                    ),
                    quads,
                    glyphs,
                    images,
                    image_keys,
                    gradients,
                    &mut self.shader_data,
                    &mut self.stats,
                    fb,
                    &mut group_composites,
                );
                if !draws.is_empty() {
                    opacity_draws = Some(draws);
                } else {
                    group_composites.clear();
                }
            }
            if opacity_draws.is_none() {
                let (dq, dg, di, dgrad, culled) = group_runs_with_opacity(
                    scene_items,
                    scene_clips,
                    scene_transforms,
                    scene_opacities,
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
                    Some(&mut self.shader_data),
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
                self.stats.frames += 1;
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
            let items = scene_items;
            let clips = scene_clips;
            let transforms = scene_transforms;

            let mut group_segment =
                |from: usize,
                 to: usize,
                 quads: &mut Vec<GpuQuad>,
                 glyphs: &mut Vec<GpuGlyph>,
                 images: &mut Vec<BlitQuad>,
                 image_keys: &mut Vec<u64>,
                 gradients: &mut Vec<GpuGradient>,
                 shaders: &mut ShaderGroupData,
                 stats: &mut RenderStats,
                 plan: &mut BackdropSegPlan| {
                    let shader_base = shaders.instances.len() as u32;
                    let (dq, dg, di, dgrad, culled) = group_runs_with_opacity(
                        &items[from..to],
                        &clips[from..to],
                        &transforms[from..to],
                        item_opacities.get(from..to).unwrap_or(&[]),
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
                        Some(shaders),
                    );
                    stats.dropped_quads += dq as u64;
                    stats.dropped_glyphs += dg as u64;
                    stats.dropped_images += di as u64;
                    stats.dropped_gradients += dgrad as u64;
                    stats.culled += culled as u64;

                    plan.all_opaque =
                        seg_gradients.is_empty() && seg_quads.iter().all(quad_is_opaque);
                    plan.runs = shift_gui_runs_with_shaders(
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
                            &mut self.shader_data,
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
                        &mut self.shader_data,
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
                                    &mut self.shader_data,
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
                                    &mut self.shader_data,
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
                                &mut self.shader_data,
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
                        &mut self.shader_data,
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
        // Upload shader instances + uniforms.
        let shader_instances = &self.shader_data.instances;
        let shader_uniforms = &self.shader_data.uniforms;
        Self::ensure_capacity(
            &self.device,
            &mut self.shader_buf,
            &mut self.shader_cap,
            shader_instances.len(),
            SHADER_BYTES,
        );
        if shader_instances.len() > self.uniform_cap {
            let new_cap = shader_instances
                .len()
                .max(self.uniform_cap.saturating_mul(2).max(1));
            self.uniform_ring = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("sequoia-shader-uniforms"),
                size: (new_cap * SHADER_UNIFORM_STRIDE) as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.uniform_cap = new_cap;
            // Recreate bind group with new buffer.
            self.shader_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("sequoia-shader-bg"),
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
            });
        }
        if !shader_instances.is_empty() {
            self.queue
                .write_buffer(&self.shader_buf, 0, bytemuck::cast_slice(shader_instances));
        }
        if !shader_uniforms.is_empty() {
            self.queue
                .write_buffer(&self.uniform_ring, 0, shader_uniforms);
        }

        let image_draws = &mut self.scratch_image_draws;
        image_draws.clear();
        if !images.is_empty() {
            for key in image_keys.iter() {
                if self.image_failures.contains(key) {
                    continue;
                }
                let Some(src) = find_image_source(list.items(), *key) else {
                    continue;
                };
                let needs_upload = match self.images.get(key) {
                    None => true,
                    Some(entry) => {
                        let version_changed = src
                            .dynamic_version()
                            .is_some_and(|version| Some(version) != entry.version);
                        let extent_changed = src
                            .extent()
                            .is_some_and(|extent| extent != (entry.width, entry.height));
                        version_changed || extent_changed
                    }
                };
                if needs_upload {
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
                if let Some(target) = target_from_key(*key) {
                    if let Some(entry) = self.render_targets.get(&target) {
                        image_draws.push((i as u32, entry.bind_group.clone()));
                    }
                    continue;
                }
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

        self.scratch_shader_textures.clear();
        let shader_texture_inputs = std::mem::take(&mut self.shader_data.textures);
        for textures in &shader_texture_inputs {
            let mut groups: ShaderTextureGroups = [
                self.shader_white_bg.clone(),
                self.shader_white_bg.clone(),
                self.shader_white_bg.clone(),
            ];
            for (binding, input) in textures.iter() {
                let group = match input {
                    TexInput::Image(source) => Self::image_bind_group(
                        &self.device,
                        &self.queue,
                        &self.atlas_bgl,
                        &self.blit_sampler,
                        &mut self.images,
                        &mut self.image_order,
                        &mut self.image_failures,
                        source,
                    ),
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
            self.scratch_shader_textures.push(groups);
        }
        self.shader_data.textures = shader_texture_inputs;

        if !group_composites.is_empty() {
            Self::ensure_capacity(
                &self.device,
                &mut self.backdrop_buf,
                &mut self.backdrop_cap,
                group_composites.len(),
                BACKDROP_BYTES,
            );
            self.queue.write_buffer(
                &self.backdrop_buf,
                0,
                bytemuck::cast_slice(&group_composites),
            );
        }

        // Record the concrete passes.
        let fb_drawn = !regions.is_empty()
            || !frame_quads.is_empty()
            || !blit_quads.is_empty()
            || !images.is_empty();

        let shared_depth = !regions.is_empty();
        if shared_depth {
            Self::ensure_depth_slot(&self.device, &mut self.depth_tex, fb.0, fb.1);
        }

        if fb_drawn || use_offscreen {
            let clear = clear_color_linear(self.clear_color);
            let load = if partial {
                wgpu::LoadOp::Load
            } else {
                wgpu::LoadOp::Clear(clear)
            };

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
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("sequoia-surface-clear"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: scene_target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: depth_attachment,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            if !frame_quads.is_empty() {
                pass.set_bind_group(0, &self.bind_group, &[]);
                pass.set_vertex_buffer(0, self.frame_buf.slice(..));
                if partial {
                    // Leading instances are damage-rect clears; REPLACE avoids
                    // blending them with stale contents. Content is drawn once
                    // per region so every scissor sees the full scene.
                    let clear_count = partial_regions.len() as u32;
                    pass.set_pipeline(&self.quad_pipeline_opaque);
                    for (index, (_, scissor)) in partial_regions.iter().enumerate() {
                        pass.set_scissor_rect(scissor.0, scissor.1, scissor.2, scissor.3);
                        let index = index as u32;
                        pass.draw(0..4, index..index + 1);
                        draws += 1;
                    }
                    if frame_quads.len() > clear_count as usize {
                        pass.set_pipeline(&self.quad_pipeline);
                        for (_, scissor) in partial_regions.iter() {
                            pass.set_scissor_rect(scissor.0, scissor.1, scissor.2, scissor.3);
                            pass.draw(0..4, clear_count..frame_quads.len() as u32);
                            draws += 1;
                        }
                    }
                } else {
                    pass.set_pipeline(&self.quad_pipeline);
                    pass.draw(0..4, 0..frame_quads.len() as u32);
                    draws += 1;
                }
            }
            drop(pass);
            if split_depth && let Some((_, view, _)) = self.depth_tex.as_ref() {
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

        if !regions.is_empty() {
            let depth_view = self.depth_tex.as_ref().map(|(_, v, _)| v);
            let mut frame = WgpuFrame {
                device: &self.device,
                queue: &self.queue,
                encoder: &mut encoder,
                target: scene_target,
                depth: depth_view,
                format: self.render_format,
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
                pass.set_pipeline(&self.surface_blit_pipeline);
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
                                    &self.shader_buf,
                                    &self.shader_data.instances,
                                    &self.shader_bind_group,
                                    &self.scratch_shader_textures,
                                    &self.user_shaders,
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
                                        &self.backdrop_blur_pipeline,
                                        &self.bind_group,
                                        bufs,
                                        origin,
                                        extent,
                                        &mut encoder,
                                        scene_target,
                                        &self.backdrop_buf,
                                        idx,
                                        inst.params[0],
                                    );
                                    passes += 1;
                                    draws += 1;
                                    if inst.params[0] >= 0.5 {
                                        passes += 1;
                                        draws += 1;
                                    }
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
                                let filter_blur = backdrop_insts[idx as usize].params[0];
                                Self::draw_filter_at(
                                    &self.backdrop_pipeline,
                                    &self.backdrop_blur_pipeline,
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
                                    &self.shader_buf,
                                    &self.shader_data.instances,
                                    &self.shader_bind_group,
                                    &self.scratch_shader_textures,
                                    &self.user_shaders,
                                    &fplan.runs,
                                    image_draws,
                                    fplan.all_opaque,
                                    filter_blur,
                                );
                                passes += if filter_blur >= 0.5 { 3 } else { 2 };
                                draws += gui_run_draws(&fplan.runs, image_draws) + 1;
                                if filter_blur >= 0.5 {
                                    draws += 1;
                                }
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
                                    &self.shader_buf,
                                    &self.shader_data.instances,
                                    &self.shader_bind_group,
                                    &self.scratch_shader_textures,
                                    &self.user_shaders,
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
                        &self.shader_buf,
                        &self.shader_data.instances,
                        &self.shader_bind_group,
                        &self.scratch_shader_textures,
                        &self.user_shaders,
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
                            &self.backdrop_blur_pipeline,
                            &self.bind_group,
                            bufs,
                            origin,
                            extent,
                            &mut encoder,
                            scene_target,
                            &self.backdrop_buf,
                            idx,
                            inst.params[0],
                        );
                        passes += 1;
                        draws += 1;
                        if inst.params[0] >= 0.5 {
                            passes += 1;
                            draws += 1;
                        }
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
        } else if let Some(nodes) = &opacity_draws {
            passes += 1;
            {
                let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("sequoia-opacity-clear"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
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
            let ctx = OpacityDrawContext {
                group_layers: &self.group_layers,
                quad_pipeline: &self.quad_pipeline,
                quad_pipeline_opaque: &self.quad_pipeline_opaque,
                glyph_pipeline: &self.glyph_pipeline,
                blit_pipeline: &self.blit_pipeline,
                gradient_pipeline: &self.gradient_pipeline,
                backdrop_pipeline: &self.backdrop_pipeline,
                bind_group: &self.bind_group,
                atlas_bg: self.atlas_tex.as_ref().map(|(_, bg)| bg),
                quad_buf: &self.quad_buf,
                glyph_buf: &self.glyph_buf,
                image_buf: &self.image_buf,
                gradient_buf: &self.gradient_buf,
                shader_buf: &self.shader_buf,
                shader_instances: &self.shader_data.instances,
                shader_bind_group: &self.shader_bind_group,
                shader_textures: &self.scratch_shader_textures,
                user_shaders: &self.user_shaders,
                backdrop_buf: &self.backdrop_buf,
            };
            draw_opacity_draws(
                nodes,
                &view,
                0,
                quads,
                image_draws,
                &mut encoder,
                &mut passes,
                &mut draws,
                &ctx,
            );
        } else {
            let all_opaque = gradients.is_empty() && quads.iter().all(quad_is_opaque);

            {
                let load = if use_offscreen || fb_drawn {
                    wgpu::LoadOp::Load
                } else {
                    wgpu::LoadOp::Clear(clear_color_linear(self.clear_color))
                };
                passes += 1;
                draws += blit_quads.len() as u64 + gui_run_draws(runs, image_draws);
                let target = if use_offscreen { scene_target } else { &view };
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("sequoia-composite"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: target,
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
                    pass.set_pipeline(&self.surface_blit_pipeline);
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
                        GuiRun::Shader(start, len, shader_id, blend) => {
                            let Some(entry) = self.user_shaders.get(&shader_id) else {
                                continue;
                            };
                            let Some(pipeline) = entry
                                .pipelines
                                .get(&blend)
                                .or_else(|| entry.pipelines.get(&sequoia_core::BlendMode::Normal))
                            else {
                                continue;
                            };
                            pass.set_pipeline(pipeline);
                            pass.set_vertex_buffer(0, self.shader_buf.slice(..));
                            for index in start..start + len {
                                let Some(instance) = self.shader_data.instances.get(index as usize)
                                else {
                                    break;
                                };
                                pass.set_bind_group(
                                    0,
                                    &self.shader_bind_group,
                                    &[instance.uniform_offset],
                                );
                                if let Some(groups) =
                                    self.scratch_shader_textures.get(index as usize)
                                {
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

            if use_offscreen {
                let frame_bg = self
                    .frame_target
                    .as_ref()
                    .map(|t| &t.bg)
                    .expect("frame target");
                Self::present_frame_target(
                    &self.device,
                    &self.queue,
                    &mut self.present_buf,
                    &mut self.present_cap,
                    &self.present_pipeline,
                    &self.bind_group,
                    frame_bg,
                    clear_color,
                    fb,
                    &mut encoder,
                    &view,
                );
                passes += 1;
                draws += 1;
            }
        }
        self.frame_target_valid = use_offscreen;
        // Publish statistics and present the finished frame.
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
