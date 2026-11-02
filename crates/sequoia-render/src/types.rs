//! Shared GPU-side types, renderer options, statistics, and errors.
//!
//! The `Gpu*` structs mirror the layout expected by the WGSL shader and are
//! uploaded directly as vertex instance data.

use bytemuck::{Pod, Zeroable};
use std::sync::Arc;

use sequoia_core::{BlendMode, Color, Rect, RenderTargetId, TexInput};

/// Instance data for a rounded rectangle, border, or shadow.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct GpuQuad {
    pub(crate) rect: [f32; 4],
    pub(crate) color: [f32; 4],
    pub(crate) radius: f32,
    pub(crate) border_color: [f32; 4],
    pub(crate) border_width: f32,
    pub(crate) clip_rect: [f32; 4],
    pub(crate) clip_radius: f32,
    pub(crate) softness: f32,
    pub(crate) spread: f32,
    pub(crate) shadow_offset: [f32; 2],
    pub(crate) inset: f32,
    pub(crate) transform_a: [f32; 4],
    pub(crate) transform_b: [f32; 2],
}

/// Instance data for one glyph from the glyph atlas.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct GpuGlyph {
    pub(crate) rect: [f32; 4],
    pub(crate) uv: [f32; 4],
    pub(crate) color: [f32; 4],
    pub(crate) color_glyph: f32,
    pub(crate) clip_rect: [f32; 4],
    pub(crate) clip_radius: f32,
    pub(crate) transform_a: [f32; 4],
    pub(crate) transform_b: [f32; 2],
}

/// Instance data for bitmap blits: images and cached sub-surfaces.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct BlitQuad {
    pub(crate) rect: [f32; 4],
    pub(crate) uv: [f32; 4],
    pub(crate) clip_rect: [f32; 4],
    pub(crate) clip_radius: f32,
    pub(crate) transform_a: [f32; 4],
    pub(crate) transform_b: [f32; 2],
    pub(crate) opacity: f32,
    pub(crate) nearest: f32,
}

/// Instance data for a custom shader draw.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub(crate) struct GpuShaderInstance {
    pub(crate) rect: [f32; 4],
    pub(crate) uv: [f32; 4],
    pub(crate) clip_rect: [f32; 4],
    pub(crate) clip_radius: f32,
    pub(crate) opacity: f32,
    pub(crate) transform_a: [f32; 4],
    pub(crate) transform_b: [f32; 2],
    pub(crate) shader_id: u32,
    pub(crate) uniform_offset: u32,
    pub(crate) _pad: [f32; 2],
}

/// Per-draw texture bind groups in slot order (group 1, 2, 3).
pub(crate) type ShaderTextureGroups = [wgpu::BindGroup; sequoia_shader::MAX_TEXTURES];

/// Cached offscreen texture backing one `RenderTargetId`.
pub(crate) struct RenderTargetTexture {
    pub(crate) texture: wgpu::Texture,
    pub(crate) view: wgpu::TextureView,
    pub(crate) bind_group: wgpu::BindGroup,
    pub(crate) size: (u32, u32),
    pub(crate) last_used: u64,
}

/// Image-bind-group key marking a texture that comes from an offscreen target.
pub(crate) const TARGET_KEY_BIT: u64 = 1 << 63;

/// Encodes a render target as an image bind-group key.
#[must_use]
pub(crate) fn target_key(target: RenderTargetId) -> u64 {
    TARGET_KEY_BIT | target.get()
}

/// Decodes a key produced by [`target_key`].
#[must_use]
pub(crate) fn target_from_key(key: u64) -> Option<RenderTargetId> {
    (key & TARGET_KEY_BIT != 0).then(|| RenderTargetId::new(key & !TARGET_KEY_BIT))
}

/// CPU-side shader instances and their dynamic uniform payloads.
#[derive(Default)]
pub(crate) struct ShaderGroupData {
    pub(crate) instances: Vec<GpuShaderInstance>,
    pub(crate) uniforms: Vec<u8>,
    pub(crate) textures: Vec<Arc<[(u32, TexInput)]>>,
}

/// Returns the fixed-function blend state used by a user-shader pipeline.
///
/// Stage 1 supports the five blend modes expressible with fixed-function
/// hardware. Advanced Photoshop modes are built as separate pipelines with the
/// normal blend state and are replaced by read-back compositing in stage 4.
pub(crate) fn blend_component(mode: BlendMode) -> wgpu::BlendComponent {
    use wgpu::{BlendFactor as Factor, BlendOperation as Op};

    let normal = wgpu::BlendComponent {
        src_factor: Factor::One,
        dst_factor: Factor::OneMinusSrcAlpha,
        operation: Op::Add,
    };
    match mode {
        BlendMode::Normal => normal,
        BlendMode::Add => wgpu::BlendComponent {
            src_factor: Factor::One,
            dst_factor: Factor::One,
            operation: Op::Add,
        },
        BlendMode::Multiply => wgpu::BlendComponent {
            src_factor: Factor::Dst,
            dst_factor: Factor::OneMinusSrcAlpha,
            operation: Op::Add,
        },
        BlendMode::Screen => wgpu::BlendComponent {
            src_factor: Factor::OneMinusDst,
            dst_factor: Factor::One,
            operation: Op::Add,
        },
        BlendMode::Subtract => wgpu::BlendComponent {
            src_factor: Factor::One,
            dst_factor: Factor::One,
            operation: Op::Subtract,
        },
        _ => normal,
    }
}

/// Returns `true` when `mode` needs destination read-back compositing.
pub(crate) fn blend_mode_is_advanced(mode: BlendMode) -> bool {
    !matches!(
        mode,
        BlendMode::Normal
            | BlendMode::Add
            | BlendMode::Multiply
            | BlendMode::Screen
            | BlendMode::Subtract
    )
}

/// Instance data for a gradient fill with up to four color stops.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct GpuGradient {
    pub(crate) rect: [f32; 4],
    pub(crate) stop0: [f32; 4],
    pub(crate) stop1: [f32; 4],
    pub(crate) stop2: [f32; 4],
    pub(crate) stop3: [f32; 4],
    pub(crate) stop_pos: [f32; 4],
    pub(crate) params: [f32; 4],
    pub(crate) clip_rect: [f32; 4],
    pub(crate) clip_radius: f32,
    pub(crate) _pad: [f32; 5],
    pub(crate) transform_a: [f32; 4],
    pub(crate) transform_b: [f32; 2],
}

/// Instance data for a backdrop blur or filter area.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct GpuBackdrop {
    pub(crate) rect: [f32; 4],
    pub(crate) dim: [f32; 4],
    pub(crate) params: [f32; 4],
    pub(crate) clip_rect: [f32; 4],
    pub(crate) mask: [f32; 4],
}

/// Offscreen texture used while compositing one opacity group.
pub(crate) struct GroupLayer {
    pub(crate) texture: wgpu::Texture,
    pub(crate) view: wgpu::TextureView,
    pub(crate) bg: wgpu::BindGroup,
    pub(crate) size: (u32, u32),
}

/// Ping-pong textures used while sampling and compositing the backdrop.
pub(crate) struct BackdropBuffers {
    pub(crate) a_tex: wgpu::Texture,
    pub(crate) a_view: wgpu::TextureView,
    pub(crate) a_bg: wgpu::BindGroup,
    pub(crate) b_tex: wgpu::Texture,
    pub(crate) b_view: wgpu::TextureView,
    pub(crate) b_bg: wgpu::BindGroup,
    pub(crate) c_tex: wgpu::Texture,
    pub(crate) c_view: wgpu::TextureView,
    pub(crate) c_bg: wgpu::BindGroup,
    pub(crate) size: (u32, u32),
}

/// Persistent offscreen target used for partial repaint.
pub(crate) struct FrameTarget {
    pub(crate) texture: wgpu::Texture,
    pub(crate) view: wgpu::TextureView,
    pub(crate) bg: wgpu::BindGroup,
    pub(crate) size: (u32, u32),
}

/// Drawing plan for a filter span and the content it applies to.
pub(crate) struct FilterSegPlan {
    pub(crate) runs: Vec<GuiRun>,
    pub(crate) all_opaque: bool,
    pub(crate) has_content: bool,
    pub(crate) filter_idx: Option<u32>,
}

/// A compositing step: a regular segment or a filter range.
pub(crate) enum CompPlan {
    Seg(BackdropSegPlan),
    Filter(FilterSegPlan),
}

/// A compositing event ordered by paint-item position.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CompOp {
    Marker {
        at: usize,
        span_idx: usize,
    },
    FilterRange {
        start: usize,
        end: usize,
        span_idx: usize,
    },
}

/// Offscreen texture used to cache the output of a [`crate::WgpuRenderer`].
pub(crate) struct CachedSurface {
    pub(crate) texture: wgpu::Texture,
    pub(crate) view: wgpu::TextureView,
    pub(crate) bind_group: wgpu::BindGroup,
    pub(crate) size: (u32, u32),
    pub(crate) samples: u32,
    pub(crate) format: wgpu::TextureFormat,
    pub(crate) msaa: Option<(wgpu::Texture, wgpu::TextureView)>,
}

/// Cached GPU texture for an image source.
pub(crate) struct GpuImage {
    pub(crate) texture: wgpu::Texture,
    pub(crate) bind_group: wgpu::BindGroup,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) version: Option<u64>,
}

pub(crate) const MAX_IMAGE_DIM: u32 = 4096;

pub(crate) const MAX_IMAGE_BYTES: usize = 64 << 20;

pub(crate) const QUAD_BYTES: usize = 30 * 4;

pub(crate) const GLYPH_BYTES: usize = 24 * 4;

pub(crate) const BLIT_BYTES: usize = 21 * 4;

pub(crate) const BACKDROP_BYTES: usize = 20 * 4;

pub(crate) const GRADIENT_BYTES: usize = 44 * 4;

pub(crate) const SHADER_BYTES: usize = 96;

pub(crate) const SHADER_UNIFORM_STRIDE: usize = 256;

pub(crate) const INITIAL_SHADER_CAPACITY: usize = 64;

pub(crate) const MAX_BACKDROP_BLUR_PX: f32 = 64.0;

/// Spacing between adjacent Gaussian taps, as a fraction of the blur radius.
///
/// Two separable passes with 25 taps each must stay in sync with the WGSL
/// `BLUR_STEP_SCALE` constant.
pub(crate) const BACKDROP_BLUR_STEP_SCALE: f32 = 0.1167;

pub(crate) const INITIAL_QUAD_CAPACITY: usize = 1024;

pub(crate) const INITIAL_GLYPH_CAPACITY: usize = 4096;

pub(crate) const INITIAL_FRAME_CAPACITY: usize = 64;

pub(crate) const INITIAL_BLIT_CAPACITY: usize = 16;

pub(crate) const INITIAL_IMAGE_CAPACITY: usize = 64;

pub(crate) const INITIAL_BACKDROP_CAPACITY: usize = 8;

pub(crate) const INITIAL_GRADIENT_CAPACITY: usize = 32;

pub(crate) const IMAGE_CACHE_MAX: usize = 64;

pub(crate) const IMAGE_FAILURES_MAX: usize = 1024;

pub(crate) const MAX_FRAME_QUADS: usize = 512;

pub(crate) const MAX_BLITS: usize = 64;

/// Maximum nesting depth of opacity groups rendered with offscreen layers.
/// Deeper plans fall back to per-item alpha modulation.
pub(crate) const MAX_GROUP_DEPTH: usize = 4;

pub(crate) const DEFAULT_SURFACE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

pub(crate) const SHARED_DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24Plus;

/// Surface formats selected for a window swapchain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ResolvedSurfaceFormat {
    /// Format reported by [`wgpu::SurfaceConfiguration::format`].
    pub(crate) config: wgpu::TextureFormat,
    /// Format used by render pipelines and texture views.
    ///
    /// This is usually the same as `config`, except for non-sRGB 8-bit formats
    /// where an sRGB view is used to keep blending in linear light.
    pub(crate) render: wgpu::TextureFormat,
    /// Additional view formats declared on the surface configuration.
    pub(crate) view_formats: Vec<wgpu::TextureFormat>,
}

/// A contiguous range of instances sharing one pipeline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GuiRun {
    Quads(u32, u32),
    Glyphs(u32, u32),
    Images(u32, u32),
    Gradients(u32, u32),
    /// Shader instance range followed by shader handle and blend mode.
    Shader(u32, u32, sequoia_core::ShaderId, sequoia_core::BlendMode),
}

/// Drawing plan for a scene segment followed by an optional backdrop marker.
pub(crate) struct BackdropSegPlan {
    pub(crate) runs: Vec<GuiRun>,
    pub(crate) all_opaque: bool,
    pub(crate) has_content: bool,
    pub(crate) backdrop_idx: Option<u32>,
}

/// Per-frame rendering statistics.
///
/// Counters accumulate until [`crate::GpuRenderer::take_stats`] is called.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RenderStats {
    pub dropped_quads: u64,
    pub dropped_glyphs: u64,
    pub dropped_images: u64,
    pub dropped_frame_quads: u64,
    pub dropped_surfaces: u64,
    pub culled: u64,
    pub quad_instances: u64,
    pub glyph_instances: u64,
    pub image_instances: u64,
    pub gradient_instances: u64,
    pub dropped_gradients: u64,
    pub draw_calls: u64,
    pub passes: u64,
    pub frames: u64,
    pub filter_spans: u64,
    pub filter_skipped: u64,
}

impl RenderStats {
    /// Returns `true` if any content was dropped in this frame.
    pub fn has_drops(self) -> bool {
        self.dropped_quads > 0
            || self.dropped_glyphs > 0
            || self.dropped_images > 0
            || self.dropped_frame_quads > 0
            || self.dropped_surfaces > 0
    }

    /// Returns the total number of instances submitted in the last frame.
    pub fn instances(self) -> u64 {
        self.quad_instances + self.glyph_instances + self.image_instances + self.gradient_instances
    }
}

/// Reason why a frame was skipped without rendering.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum FrameSkip {
    Timeout,
    Occluded,
}

pub(crate) const CLEAR_COLOR: Color = Color::rgb(0.08, 0.08, 0.09);

impl Region {
    /// Converts a physical-pixel rectangle into a region, or `None` if empty.
    pub(crate) fn from_phys(r: Rect) -> Option<Region> {
        if !r.left().is_finite()
            || !r.top().is_finite()
            || !r.size.width.is_finite()
            || !r.size.height.is_finite()
        {
            return None;
        }

        const MAX_REGION: f32 = 16384.0;
        let x = r.left().floor().clamp(0.0, MAX_REGION) as u32;
        let y = r.top().floor().clamp(0.0, MAX_REGION) as u32;
        let w = (r.right().ceil() - r.left().floor()).clamp(0.0, MAX_REGION) as u32;
        let h = (r.bottom().ceil() - r.top().floor()).clamp(0.0, MAX_REGION) as u32;
        if w == 0 || h == 0 {
            return None;
        }
        Some(Region {
            x,
            y,
            width: w,
            height: h,
        })
    }
}

/// Graphics backend requested for the renderer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum GpuBackend {
    #[default]
    Auto,
    Vulkan,
    Metal,
    Dx12,
    Gl,
    WebGpu,
}

impl GpuBackend {
    pub(crate) fn wgpu_backends(self) -> wgpu::Backends {
        match self {
            Self::Auto => wgpu::Backends::all(),
            Self::Vulkan => wgpu::Backends::VULKAN,
            Self::Metal => wgpu::Backends::METAL,
            Self::Dx12 => wgpu::Backends::DX12,
            Self::Gl => wgpu::Backends::GL,
            Self::WebGpu => wgpu::Backends::BROWSER_WEBGPU,
        }
    }

    /// Returns the user-facing backend name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Vulkan => "vulkan",
            Self::Metal => "metal",
            Self::Dx12 => "dx12",
            Self::Gl => "gl",
            Self::WebGpu => "webgpu",
        }
    }
}

/// Presentation mode for the window surface.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum PresentMode {
    #[default]
    Vsync,
    Mailbox,
    Immediate,
}

impl PresentMode {
    pub(crate) fn wgpu(self) -> wgpu::PresentMode {
        match self {
            Self::Vsync => wgpu::PresentMode::Fifo,
            Self::Mailbox => wgpu::PresentMode::Mailbox,
            Self::Immediate => wgpu::PresentMode::Immediate,
        }
    }

    /// Returns the user-facing presentation mode name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Vsync => "vsync",
            Self::Mailbox => "mailbox",
            Self::Immediate => "immediate",
        }
    }
}

/// GPU power preference used when selecting an adapter.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum PowerPreference {
    #[default]
    HighPerformance,
    LowPower,
}

impl PowerPreference {
    pub(crate) fn wgpu(self) -> wgpu::PowerPreference {
        match self {
            Self::HighPerformance => wgpu::PowerPreference::HighPerformance,
            Self::LowPower => wgpu::PowerPreference::LowPower,
        }
    }
}

/// Options passed to [`crate::GpuRenderer::new_with_options`].
#[derive(Clone, Copy, Debug, Default)]
pub struct GpuOptions {
    pub backend: GpuBackend,
    pub present_mode: PresentMode,
    pub power_preference: PowerPreference,
    /// Preferred swapchain format. `None` selects an sRGB format when available.
    pub surface_format: Option<wgpu::TextureFormat>,
}

impl GpuOptions {
    /// Creates options for a specific backend with all other fields defaulted.
    pub fn with_backend(backend: GpuBackend) -> Self {
        Self {
            backend,
            ..Self::default()
        }
    }

    /// Sets the presentation mode.
    pub fn present_mode(mut self, mode: PresentMode) -> Self {
        self.present_mode = mode;
        self
    }

    /// Sets the adapter power preference.
    pub fn power_preference(mut self, power: PowerPreference) -> Self {
        self.power_preference = power;
        self
    }

    /// Requests a specific swapchain format.
    ///
    /// Unsupported formats fall back to the default sRGB selection. Non-sRGB
    /// 8-bit formats are rendered through an sRGB view so blending stays linear.
    pub fn surface_format(mut self, format: wgpu::TextureFormat) -> Self {
        self.surface_format = Some(format);
        self
    }
}

/// Selects a supported presentation mode, falling back to FIFO (vsync).
pub(crate) fn resolve_present_mode(
    caps: &[wgpu::PresentMode],
    wanted: PresentMode,
) -> wgpu::PresentMode {
    let mode = wanted.wgpu();
    if caps.contains(&mode) {
        mode
    } else {
        wgpu::PresentMode::Fifo
    }
}

/// Errors reported while creating or using the renderer.
#[derive(Debug)]
#[non_exhaustive]
pub enum RenderError {
    NoAdapter,
    CreateSurface(wgpu::CreateSurfaceError),
    Device(wgpu::RequestDeviceError),
    Surface,
    ShaderCompile(String),
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoAdapter => write!(f, "no compatible GPU adapter"),
            Self::CreateSurface(e) => write!(f, "surface creation failed: {e}"),
            Self::Device(e) => write!(f, "device creation failed: {e}"),
            Self::Surface => write!(f, "surface state error"),
            Self::ShaderCompile(e) => write!(f, "shader compilation failed: {e}"),
        }
    }
}

impl std::error::Error for RenderError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::CreateSurface(e) => Some(e),
            Self::Device(e) => Some(e),
            _ => None,
        }
    }
}

/// An integer rectangle in physical pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Region {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}
