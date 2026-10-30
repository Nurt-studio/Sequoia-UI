//! Unit tests for scene grouping, planning, options, and the embedded shader.
//!
//! Rendering itself requires a GPU adapter, so most tests exercise the pure
//! conversion and planning paths.

use crate::color::{ColorSpace, linear_to_srgb, srgb_to_linear};
use crate::gpu::shaders::SHADER;
use crate::renderer::GpuRenderer;
use crate::scene::grouping::*;
use crate::scene::instances::*;
use crate::scene::planning::*;
use crate::surface::SurfaceFrame;
use crate::surface::*;
use crate::types::*;
use crate::utils::*;
use sequoia_core::{BackdropSpan, Color, FilterSpan, ImageSource, Rect, RenderItem, RenderList};
use sequoia_view::Runtime;
use sequoia_view::{PointerEvent, SurfaceDraw, Unit, View};
#[allow(unused_imports)]
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

#[test]
fn embedded_shader_parses_and_validates() {
    let module = wgpu::naga::front::wgsl::parse_str(SHADER).expect("Sequoia shader fails to parse");
    let mut validator = wgpu::naga::valid::Validator::new(
        wgpu::naga::valid::ValidationFlags::all(),
        wgpu::naga::valid::Capabilities::all(),
    );
    validator
        .validate(&module)
        .expect("Sequoia shader fails to validate");
}

fn backdrop_span(rect: Rect, blur: f32) -> BackdropSpan {
    BackdropSpan {
        at: 2,
        draw: sequoia_core::BackdropDraw {
            rect,
            radius: 8.0,
            blur,
            dim: Color::TRANSPARENT,
            noise: 0.0,
            mask: sequoia_core::SpanMask::None,
        },
        clip: None,
    }
}

#[test]
fn backdrop_instance_scales_to_physical_pixels() {
    let inst = super::GpuRenderer::backdrop_instance(
        &backdrop_span(Rect::from_xywh(10.0, 20.0, 200.0, 100.0), 16.0),
        2.0,
    )
    .expect("widoczny marker");
    assert_eq!(inst.rect, [20.0, 40.0, 400.0, 200.0]);
    assert_eq!(inst.params[0], 32.0, "blur 16 * scale 2");
    assert_eq!(inst.params[2], 16.0, "radius 8 * scale 2");
    assert_eq!(inst.clip_rect, [0.0; 4], "brak clipu");
}

#[test]
fn backdrop_instance_clamps_and_rejects_empty() {
    assert!(super::GpuRenderer::backdrop_instance(&backdrop_span(Rect::ZERO, 16.0), 1.0).is_none());
    let inst = super::GpuRenderer::backdrop_instance(
        &backdrop_span(Rect::from_xywh(0.0, 0.0, 10.0, 10.0), 1000.0),
        2.0,
    )
    .unwrap();
    assert_eq!(inst.params[0], MAX_BACKDROP_BLUR_PX);

    let inst = super::GpuRenderer::backdrop_instance(
        &backdrop_span(Rect::from_xywh(0.0, 0.0, 10.0, 10.0), 8.0),
        f32::NAN,
    )
    .unwrap();
    assert_eq!(inst.params[0], 8.0, "scale NAN → 1.0");
}

#[test]
fn backend_maps_to_wgpu_flags() {
    assert_eq!(GpuBackend::Vulkan.wgpu_backends(), wgpu::Backends::VULKAN);
    assert_eq!(GpuBackend::Metal.wgpu_backends(), wgpu::Backends::METAL);
    assert_eq!(GpuBackend::Dx12.wgpu_backends(), wgpu::Backends::DX12);
    assert_eq!(GpuBackend::Gl.wgpu_backends(), wgpu::Backends::GL);
    assert_eq!(
        GpuBackend::WebGpu.wgpu_backends(),
        wgpu::Backends::BROWSER_WEBGPU
    );
    assert_eq!(GpuBackend::Auto.wgpu_backends(), wgpu::Backends::all());
}

#[test]
fn options_default_to_auto_backend() {
    assert_eq!(GpuOptions::default().backend, GpuBackend::Auto);
    assert_eq!(
        GpuOptions::with_backend(GpuBackend::Dx12).backend,
        GpuBackend::Dx12
    );
}

#[test]
fn gpu_options_carry_present_mode_and_power() {
    let opts = GpuOptions::with_backend(GpuBackend::Vulkan)
        .present_mode(PresentMode::Immediate)
        .power_preference(PowerPreference::LowPower);
    assert_eq!(opts.backend, GpuBackend::Vulkan);
    assert_eq!(opts.present_mode, PresentMode::Immediate);
    assert_eq!(opts.power_preference, PowerPreference::LowPower);

    let default = GpuOptions::default();
    assert_eq!(default.present_mode, PresentMode::Vsync);
    assert_eq!(default.power_preference, PowerPreference::HighPerformance);
    assert_eq!(default.surface_format, None);
}

#[test]
fn gpu_options_carry_surface_format() {
    let opts = GpuOptions::with_backend(GpuBackend::Vulkan)
        .surface_format(wgpu::TextureFormat::Bgra8Unorm);
    assert_eq!(opts.surface_format, Some(wgpu::TextureFormat::Bgra8Unorm));
}

#[test]
fn present_mode_falls_back_to_vsync_when_unsupported() {
    assert_eq!(
        resolve_present_mode(&[wgpu::PresentMode::Immediate], PresentMode::Immediate),
        wgpu::PresentMode::Immediate
    );

    assert_eq!(
        resolve_present_mode(&[wgpu::PresentMode::Fifo], PresentMode::Immediate),
        wgpu::PresentMode::Fifo
    );
    assert_eq!(
        resolve_present_mode(&[], PresentMode::Mailbox),
        wgpu::PresentMode::Fifo
    );
}

#[test]
fn backend_names_are_stable() {
    for (backend, name) in [
        (GpuBackend::Auto, "auto"),
        (GpuBackend::Vulkan, "vulkan"),
        (GpuBackend::Metal, "metal"),
        (GpuBackend::Dx12, "dx12"),
        (GpuBackend::Gl, "gl"),
        (GpuBackend::WebGpu, "webgpu"),
    ] {
        assert_eq!(backend.name(), name);
    }
}

#[test]
fn auto_backend_candidates_single_native_backend_first() {
    let candidates = crate::renderer::auto_backend_candidates();
    assert!(!candidates.is_empty());
    assert!(!candidates.contains(&GpuBackend::Auto));
    for (i, a) in candidates.iter().enumerate() {
        assert!(
            !candidates[i + 1..].contains(a),
            "duplicate backend in auto order: {a:?}"
        );
    }

    #[cfg(target_os = "windows")]
    assert_eq!(candidates[0], GpuBackend::Vulkan);
    #[cfg(target_os = "macos")]
    assert_eq!(candidates[0], GpuBackend::Metal);
    #[cfg(target_os = "ios")]
    assert_eq!(candidates[0], GpuBackend::Metal);
    #[cfg(not(any(
        target_os = "windows",
        target_os = "macos",
        target_os = "ios",
        target_arch = "wasm32"
    )))]
    assert_eq!(candidates[0], GpuBackend::Vulkan);
}

#[test]
fn decode_image_rejects_oversize_and_mismatched_buffers() {
    assert!(
        decode_image(&ImageSource::Rgba {
            id: 1,
            width: u32::MAX,
            height: u32::MAX,
            data: Arc::new(Vec::new()),
        })
        .is_none()
    );
    assert!(
        decode_image(&ImageSource::Rgba {
            id: 2,
            width: MAX_IMAGE_DIM + 1,
            height: 16,
            data: Arc::new(vec![0; 64]),
        })
        .is_none()
    );

    assert!(
        decode_image(&ImageSource::Rgba {
            id: 3,
            width: 4,
            height: 4,
            data: Arc::new(vec![0; 10]),
        })
        .is_none()
    );

    assert!(
        decode_image(&ImageSource::Rgba {
            id: 4,
            width: 0,
            height: 4,
            data: Arc::new(Vec::new()),
        })
        .is_none()
    );

    assert!(
        decode_image(&ImageSource::Rgba {
            id: 5,
            width: 2,
            height: 1,
            data: Arc::new(vec![128; 2 * 4]),
        })
        .is_some()
    );

    assert!(decode_image(&ImageSource::File("no/such/file.png".into())).is_none());
}

#[test]
fn mip_level_count_covers_the_full_chain() {
    assert_eq!(mip_level_count(1, 1), 1);
    assert_eq!(mip_level_count(2, 2), 2);
    assert_eq!(mip_level_count(4, 4), 3);
    assert_eq!(mip_level_count(3, 3), 2);
    assert_eq!(mip_level_count(100, 50), 7);
}

#[test]
fn downsample_averages_in_linear_light() {
    let data = [
        255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
    ];
    let (w, h, out) = downsample_rgba(2, 2, &data);
    assert_eq!((w, h), (1, 1));
    // Linear red + green + blue + white averages to mid-grey, not sRGB 128.
    for channel in &out[..3] {
        assert!(
            channel.abs_diff(188) <= 1,
            "expected linear-light mid-grey, got {out:?}"
        );
    }
    assert_eq!(out[3], 255);
}

#[test]
fn downsample_weights_color_by_alpha() {
    let data = [255, 0, 0, 255, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    let (_, _, out) = downsample_rgba(2, 2, &data);
    // Transparent texels must not pull the color toward black.
    assert_eq!(&out[..3], &[255, 0, 0]);
    assert_eq!(out[3], 64);
}

struct Stub;
impl WgpuRenderer for Stub {
    fn render(&mut self, _frame: &mut WgpuFrame) {}
}

fn painted_draws(root: impl View + 'static, w: f32, h: f32) -> Vec<SurfaceDraw> {
    use sequoia_text::TextSystem;
    use sequoia_view::Runtime;
    let rt = Runtime::new();
    rt.mount(root);
    let mut text = TextSystem::new();
    rt.layout(Rect::from_xywh(0.0, 0.0, w, h), &mut text);
    let mut list = RenderList::new();
    rt.paint(&mut list, &mut text);
    assert!(list.is_empty(), "surface produces no RenderItem");
    rt.take_surface_draws()
}

#[test]
fn surface_draw_collected_with_layout_rect() {
    use sequoia_view::Column;
    let draws = painted_draws(
        Column::new()
            .child(WgpuSurface::new(Stub).height(100.0))
            .child(Unit),
        400.0,
        400.0,
    );
    assert_eq!(draws.len(), 1);
    assert_eq!(draws[0].rect, Rect::from_xywh(0.0, 0.0, 400.0, 100.0));

    let binding = draws[0].target.clone().downcast::<SurfaceBinding>();
    assert!(binding.is_ok());
}

#[test]
fn empty_surface_rect_is_culled() {
    let draws = painted_draws(
        WgpuSurface::new(Stub).width(100.0).height(0.0),
        400.0,
        400.0,
    );
    assert!(draws.is_empty(), "empty rect is not reported");
}

#[test]
fn plain_views_emit_no_surface_draws() {
    let draws = painted_draws(Unit, 400.0, 400.0);
    assert!(draws.is_empty());
}

fn frame_with(bg: Option<[f32; 4]>, border: Option<([f32; 4], f32)>) -> SurfaceFrame {
    let bw = border.map(|(_, w)| w).unwrap_or(0.0);
    SurfaceFrame {
        background: bg,
        border,
        radius: 0.0,
        inset: [bw, bw, bw, bw],
    }
}

#[test]
fn frame_ring_over_background() {
    let white = [1.0, 1.0, 1.0, 1.0];
    let blue = [0.0, 0.0, 1.0, 1.0];
    let frame = SurfaceFrame {
        background: Some(blue),
        border: Some((white, 2.0)),
        radius: 8.0,
        inset: [2.0, 2.0, 2.0, 2.0],
    };

    let mut quads = Vec::new();
    let region = surface_frame(
        Rect::from_xywh(0.0, 0.0, 100.0, 60.0),
        &frame,
        1.0,
        (400, 400),
        &mut quads,
    );
    assert_eq!(quads.len(), 1, "background + frame is one bordered quad");
    assert_eq!(quads[0].rect, [0.0, 0.0, 100.0, 60.0]);
    assert_eq!(quads[0].color, blue);
    assert_eq!(quads[0].radius, 8.0);
    assert_eq!(quads[0].border_color, white);
    assert_eq!(quads[0].border_width, 2.0);

    assert_eq!(region, Some(Rect::from_xywh(2.0, 2.0, 96.0, 56.0)));

    quads.clear();
    let region = surface_frame(
        Rect::from_xywh(0.0, 0.0, 100.0, 60.0),
        &frame,
        2.0,
        (800, 800),
        &mut quads,
    );
    assert_eq!(quads.len(), 1);
    assert_eq!(quads[0].rect, [0.0, 0.0, 200.0, 120.0]);
    assert_eq!(quads[0].border_width, 4.0);
    assert_eq!(region, Some(Rect::from_xywh(4.0, 4.0, 192.0, 112.0)));
}

#[test]
fn frame_strips_for_transparent_background() {
    let green = [0.0, 1.0, 0.0, 1.0];
    let frame = frame_with(None, Some((green, 2.0)));
    let mut quads = Vec::new();
    let region = surface_frame(
        Rect::from_xywh(0.0, 0.0, 100.0, 60.0),
        &frame,
        1.0,
        (400, 400),
        &mut quads,
    );
    assert_eq!(quads.len(), 1, "ring alone in one quad");
    assert_eq!(quads[0].rect, [0.0, 0.0, 100.0, 60.0]);
    assert_eq!(quads[0].border_color, green);
    assert_eq!(quads[0].border_width, 2.0);
    assert_eq!(region, Some(Rect::from_xywh(2.0, 2.0, 96.0, 56.0)));
}

#[test]
fn frame_empty_when_fully_offscreen() {
    let frame = frame_with(Some([1.0, 0.0, 0.0, 1.0]), None);
    let mut quads = Vec::new();
    let region = surface_frame(
        Rect::from_xywh(500.0, 500.0, 100.0, 100.0),
        &frame,
        1.0,
        (400, 400),
        &mut quads,
    );
    assert!(quads.is_empty());
    assert_eq!(region, None);
}

#[test]
fn frame_without_style_has_no_quads_and_full_region() {
    let mut quads = Vec::new();
    let region = surface_frame(
        Rect::from_xywh(10.0, 10.0, 100.0, 60.0),
        &SurfaceFrame::default(),
        1.0,
        (400, 400),
        &mut quads,
    );
    assert!(quads.is_empty());
    assert_eq!(region, Some(Rect::from_xywh(10.0, 10.0, 100.0, 60.0)));
}

#[test]
fn shadow_becomes_a_quad_instance_with_softness() {
    use sequoia_core::{RenderList, Shadow, Vec2};
    let r = Rect::from_xywh(10.0, 20.0, 100.0, 60.0);
    let mut list = RenderList::new();
    list.push_clip(Rect::from_xywh(0.0, 0.0, 60.0, 200.0));
    list.push_shadow(Shadow {
        rect: r,
        color: Color::BLACK,
        radius: 6.0,
        blur: 8.0,
        spread: 2.0,
        offset: Vec2::new(1.0, 3.0),
        inset: false,
    });
    let items: Vec<RenderItem> = list.items().to_vec();
    let clips = list.item_clips().to_vec();
    let mut quads = Vec::new();
    let (mut glyphs, mut images, mut keys, mut gradients, mut runs) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
    group_runs(
        &items,
        &clips,
        &[],
        2.0,
        None,
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
        &mut quads,
        &mut glyphs,
        &mut images,
        &mut keys,
        &mut gradients,
        &mut runs,
    );
    assert_eq!(
        runs,
        vec![GuiRun::Quads(0, 1)],
        "shadow goes into quad batch"
    );
    let q = quads[0];
    assert_eq!(q.rect, [20.0, 40.0, 200.0, 120.0], "shape in physical px");
    assert_eq!(q.softness, 16.0, "blur × scale");
    assert_eq!(q.spread, 4.0);
    assert_eq!(q.shadow_offset, [2.0, 6.0]);
    assert_eq!(q.inset, 0.0);
    assert_eq!(q.border_width, 0.0, "shadow has no border");
    assert_eq!(q.clip_rect, [0.0, 0.0, 120.0, 400.0], "rect clip");
    assert_eq!(q.clip_radius, 0.0);
    assert!(!quad_is_opaque(&q), "shadow never takes the REPLACE path");
}

#[test]
fn inset_shadow_maps_to_the_inset_flag() {
    use sequoia_core::{RenderList, Shadow};
    let mut list = RenderList::new();
    list.push_shadow(Shadow {
        rect: Rect::from_xywh(0.0, 0.0, 40.0, 20.0),
        color: Color::BLACK,
        radius: 3.0,
        blur: 6.0,
        spread: 0.0,
        offset: sequoia_core::Vec2::ZERO,
        inset: true,
    });
    let items: Vec<RenderItem> = list.items().to_vec();
    let clips = list.item_clips().to_vec();
    let mut quads = Vec::new();
    let (mut glyphs, mut images, mut keys, mut gradients, mut runs) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
    group_runs(
        &items,
        &clips,
        &[],
        1.0,
        None,
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
        &mut quads,
        &mut glyphs,
        &mut images,
        &mut keys,
        &mut gradients,
        &mut runs,
    );
    assert_eq!(quads[0].inset, 1.0);
    assert_eq!(quads[0].clip_rect, [0.0; 4], "no ancestor clip");
}

fn sample_items() -> Vec<RenderItem> {
    use sequoia_core::{GlyphQuad, Quad, Rect, TextDraw};
    let q = |x: f32| {
        RenderItem::Quad(Quad {
            rect: Rect::from_xywh(x, 0.0, 10.0, 10.0),
            color: Color::RED,
            radius: 0.0,
            border_color: Color::TRANSPARENT,
            border_width: 0.0,
        })
    };
    let t = |n: usize| {
        RenderItem::Text(TextDraw {
            color: Color::WHITE,
            glyphs: (0..n)
                .map(|i| GlyphQuad {
                    rect: Rect::from_xywh(i as f32, 0.0, 5.0, 5.0),
                    uv: [0.0; 4],
                    color_glyph: false,
                })
                .collect(),
        })
    };
    vec![q(0.0), q(10.0), t(2), q(20.0), t(0), t(1)]
}

#[test]
fn runs_preserve_paint_order_and_merge_neighbours() {
    let items = sample_items();
    let clips: Vec<Option<sequoia_core::RoundedClip>> = vec![None; items.len()];
    let mut quads = Vec::new();
    let mut glyphs = Vec::new();
    let mut images = Vec::new();
    let mut image_keys = Vec::new();
    let mut gradients = Vec::new();
    let mut runs = Vec::new();
    group_runs(
        &items,
        &clips,
        &[],
        1.0,
        None,
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
        &mut quads,
        &mut glyphs,
        &mut images,
        &mut image_keys,
        &mut gradients,
        &mut runs,
    );
    assert_eq!(quads.len(), 3);
    assert_eq!(glyphs.len(), 3);
    assert_eq!(
        runs,
        vec![
            GuiRun::Quads(0, 2),
            GuiRun::Glyphs(0, 2),
            GuiRun::Quads(2, 1),
            GuiRun::Glyphs(2, 1),
        ]
    );
}

#[test]
fn runs_scale_instances() {
    let items = sample_items();
    let clips: Vec<Option<sequoia_core::RoundedClip>> = vec![None; items.len()];
    let mut quads = Vec::new();
    let mut glyphs = Vec::new();
    let mut images = Vec::new();
    let mut image_keys = Vec::new();
    let mut gradients = Vec::new();
    let mut runs = Vec::new();
    group_runs(
        &items,
        &clips,
        &[],
        2.0,
        None,
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
        &mut quads,
        &mut glyphs,
        &mut images,
        &mut image_keys,
        &mut gradients,
        &mut runs,
    );
    let _ = runs;
    assert_eq!(quads[0].rect, [0.0, 0.0, 20.0, 20.0]);
    assert_eq!(glyphs[0].rect, [0.0, 0.0, 10.0, 10.0]);
}

#[test]
fn runs_collect_images_in_paint_order() {
    use sequoia_core::{ImageDraw, ImageSampling, ImageSource};
    let img = |x: f32, id: u64| {
        RenderItem::Image(ImageDraw {
            rect: Rect::from_xywh(x, 0.0, 10.0, 10.0),
            uv: [0.0, 0.0, 1.0, 1.0],
            source: ImageSource::Rgba {
                id,
                width: 1,
                height: 1,
                data: std::sync::Arc::new(vec![0u8, 0, 0, 255]),
            },
            sampling: ImageSampling::Linear,
        })
    };
    let items = vec![img(0.0, 7), img(10.0, 9)];
    let clips: Vec<Option<sequoia_core::RoundedClip>> = vec![None; items.len()];
    let mut quads = Vec::new();
    let mut glyphs = Vec::new();
    let mut images = Vec::new();
    let mut image_keys = Vec::new();
    let mut gradients = Vec::new();
    let mut runs = Vec::new();
    group_runs(
        &items,
        &clips,
        &[],
        2.0,
        None,
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
        &mut quads,
        &mut glyphs,
        &mut images,
        &mut image_keys,
        &mut gradients,
        &mut runs,
    );
    assert_eq!(
        runs,
        vec![GuiRun::Images(0, 2)],
        "adjacent images in one batch"
    );
    assert_eq!(image_keys, vec![7, 9]);
    assert_eq!(images[0].rect, [0.0, 0.0, 20.0, 20.0], "skala fizyczna");
    assert_eq!(images[0].uv, [0.0, 0.0, 1.0, 1.0]);
}

#[test]
fn a_large_scene_loses_nothing() {
    use sequoia_core::Quad;
    use sequoia_core::{GlyphQuad, TextDraw};
    let mut items = Vec::new();

    for i in 0..100_000u32 {
        items.push(RenderItem::Quad(Quad {
            rect: Rect::from_xywh(i as f32, 0.0, 10.0, 10.0),
            color: Color::RED,
            radius: 0.0,
            border_color: Color::TRANSPARENT,
            border_width: 0.0,
        }));
    }
    for i in 0..200_000u32 {
        items.push(RenderItem::Text(TextDraw {
            color: Color::WHITE,
            glyphs: vec![GlyphQuad {
                rect: Rect::from_xywh(i as f32, 0.0, 5.0, 5.0),
                uv: [0.0; 4],
                color_glyph: false,
            }],
        }));
    }
    let clips: Vec<Option<sequoia_core::RoundedClip>> = vec![None; items.len()];
    let mut quads = Vec::new();
    let mut glyphs = Vec::new();
    let mut images = Vec::new();
    let mut image_keys = Vec::new();
    let mut gradients = Vec::new();
    let mut runs = Vec::new();
    let (dq, dg, di, _dgrad, culled) = group_runs(
        &items,
        &clips,
        &[],
        1.0,
        None,
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
        &mut quads,
        &mut glyphs,
        &mut images,
        &mut image_keys,
        &mut gradients,
        &mut runs,
    );
    assert_eq!(
        (dq, dg, di, _dgrad, culled),
        (0, 0, 0, 0, 0),
        "nothing was rejected"
    );
    assert_eq!(quads.len(), 100_000);
    assert_eq!(glyphs.len(), 200_000);

    let covered = |want_glyphs: bool| -> u32 {
        runs.iter()
            .map(|r| match (r, want_glyphs) {
                (GuiRun::Quads(_, len), false) => *len,
                (GuiRun::Glyphs(_, len), true) => *len,
                _ => 0,
            })
            .sum()
    };
    assert_eq!(covered(false), 100_000, "no quad dropped from batches");
    assert_eq!(covered(true), 200_000, "no glyph dropped from batches");
}

fn plan_quad(x: f32) -> RenderItem {
    use sequoia_core::Quad;
    RenderItem::Quad(Quad {
        rect: Rect::from_xywh(x, 0.0, 10.0, 10.0),
        color: Color::RED,
        radius: 0.0,
        border_color: Color::TRANSPARENT,
        border_width: 0.0,
    })
}

fn plan_text(n: usize) -> RenderItem {
    use sequoia_core::{GlyphQuad, TextDraw};
    RenderItem::Text(TextDraw {
        color: Color::WHITE,
        glyphs: (0..n)
            .map(|i| GlyphQuad {
                rect: Rect::from_xywh(i as f32, 0.0, 5.0, 5.0),
                uv: [0.0; 4],
                color_glyph: false,
            })
            .collect(),
    })
}

#[test]
fn shift_gui_runs_offsets_every_kind() {
    let runs = vec![
        GuiRun::Quads(0, 2),
        GuiRun::Glyphs(0, 3),
        GuiRun::Images(0, 1),
        GuiRun::Gradients(0, 1),
    ];
    assert_eq!(
        shift_gui_runs(&runs, 10, 20, 30, 40),
        vec![
            GuiRun::Quads(10, 2),
            GuiRun::Glyphs(20, 3),
            GuiRun::Images(30, 1),
            GuiRun::Gradients(40, 1),
        ]
    );
    assert_eq!(shift_gui_runs(&runs, 0, 0, 0, 0), runs, "zero is identity");
    assert!(shift_gui_runs(&[], 5, 5, 5, 5).is_empty());
}

#[test]
fn backdrop_segments_accumulate_with_absolute_ranges() {
    let seg1 = vec![plan_quad(0.0), plan_quad(10.0)];
    let seg2 = vec![plan_quad(20.0), plan_text(1)];
    let clips1: Vec<Option<sequoia_core::RoundedClip>> = vec![None; seg1.len()];
    let clips2: Vec<Option<sequoia_core::RoundedClip>> = vec![None; seg2.len()];

    let mut all_quads: Vec<GpuQuad> = Vec::new();
    let mut all_glyphs: Vec<GpuGlyph> = Vec::new();
    let mut all_images: Vec<BlitQuad> = Vec::new();
    let mut all_keys: Vec<u64> = Vec::new();
    let mut all_gradients: Vec<GpuGradient> = Vec::new();
    let mut plans: Vec<BackdropSegPlan> = Vec::new();

    for (seg, clips) in [(&seg1, &clips1), (&seg2, &clips2)] {
        let (mut sq, mut sg, mut si, mut sk, mut sgrad, mut sr) = (
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );
        let (dq, dg, di, _dgrad, culled) = group_runs(
            seg,
            clips,
            &[],
            1.0,
            None,
            usize::MAX,
            usize::MAX,
            usize::MAX,
            usize::MAX,
            &mut sq,
            &mut sg,
            &mut si,
            &mut sk,
            &mut sgrad,
            &mut sr,
        );
        assert_eq!((dq, dg, di, _dgrad, culled), (0, 0, 0, 0, 0));
        plans.push(BackdropSegPlan {
            runs: shift_gui_runs(
                &sr,
                all_quads.len() as u32,
                all_glyphs.len() as u32,
                all_images.len() as u32,
                all_gradients.len() as u32,
            ),
            all_opaque: sq.iter().all(quad_is_opaque),
            has_content: true,
            backdrop_idx: None,
        });
        all_quads.extend_from_slice(&sq);
        all_glyphs.extend_from_slice(&sg);
        all_images.extend_from_slice(&si);
        all_keys.extend_from_slice(&sk);
        all_gradients.extend_from_slice(&sgrad);
    }

    assert_eq!(all_quads.len(), 3);
    assert_eq!(all_glyphs.len(), 1);
    assert_eq!(
        plans[0].runs,
        vec![GuiRun::Quads(0, 2)],
        "pierwszy segment od zera"
    );
    assert_eq!(
        plans[1].runs,
        vec![GuiRun::Quads(2, 1), GuiRun::Glyphs(0, 1)],
        "second segment: quads from 2, runes not cobbled together by the border"
    );

    for plan in &plans {
        for run in &plan.runs {
            match *run {
                GuiRun::Quads(s, l) => assert!((s + l) as usize <= all_quads.len()),
                GuiRun::Glyphs(s, l) => assert!((s + l) as usize <= all_glyphs.len()),
                GuiRun::Images(s, l) => assert!((s + l) as usize <= all_images.len()),
                GuiRun::Gradients(s, l) => assert!((s + l) as usize <= all_gradients.len()),
            }
        }
    }
}

#[test]
fn a_device_ceiling_counts_overflow_instead_of_hiding_it() {
    let items = sample_items();
    let clips: Vec<Option<sequoia_core::RoundedClip>> = vec![None; items.len()];
    let mut quads = Vec::new();
    let mut glyphs = Vec::new();
    let mut images = Vec::new();
    let mut image_keys = Vec::new();
    let mut gradients = Vec::new();
    let mut runs = Vec::new();

    let (dq, dg, di, dgrad, _culled) = group_runs(
        &items,
        &clips,
        &[],
        1.0,
        None,
        2,
        2,
        usize::MAX,
        usize::MAX,
        &mut quads,
        &mut glyphs,
        &mut images,
        &mut image_keys,
        &mut gradients,
        &mut runs,
    );
    assert_eq!((quads.len(), glyphs.len()), (2, 2));
    assert_eq!((dq, dg, di, dgrad), (1, 1, 0, 0), "overflow bez cullingu");
}

#[test]
fn viewport_culls_offscreen_instances() {
    use sequoia_core::{
        GlyphQuad, ImageDraw, ImageSampling, ImageSource, Quad, Shadow, TextDraw, Vec2,
    };
    let vp = Rect::from_xywh(0.0, 0.0, 100.0, 100.0);
    let quad = |x: f32, y: f32| {
        RenderItem::Quad(Quad {
            rect: Rect::from_xywh(x, y, 10.0, 10.0),
            color: Color::RED,
            radius: 0.0,
            border_color: Color::TRANSPARENT,
            border_width: 0.0,
        })
    };
    let items = vec![
        quad(0.0, 0.0),
        quad(500.0, 500.0),
        RenderItem::Text(TextDraw {
            color: Color::WHITE,
            glyphs: vec![
                GlyphQuad {
                    rect: Rect::from_xywh(10.0, 10.0, 5.0, 5.0),
                    uv: [0.0; 4],
                    color_glyph: false,
                },
                GlyphQuad {
                    rect: Rect::from_xywh(900.0, 900.0, 5.0, 5.0),
                    uv: [0.0; 4],
                    color_glyph: false,
                },
            ],
        }),
        RenderItem::Image(ImageDraw {
            rect: Rect::from_xywh(200.0, 200.0, 10.0, 10.0),
            uv: [0.0, 0.0, 1.0, 1.0],
            source: ImageSource::Rgba {
                id: 11,
                width: 1,
                height: 1,
                data: std::sync::Arc::new(vec![0u8, 0, 0, 255]),
            },
            sampling: ImageSampling::Linear,
        }),
        RenderItem::Shadow(Shadow {
            rect: Rect::from_xywh(500.0, 500.0, 100.0, 60.0),
            color: Color::BLACK,
            radius: 6.0,
            blur: 8.0,
            spread: 0.0,
            offset: Vec2::new(0.0, 0.0),
            inset: false,
        }),
        RenderItem::Waveform(sequoia_core::WaveformDraw {
            bars: vec![
                Rect::from_xywh(5.0, 5.0, 2.0, 2.0),
                Rect::from_xywh(300.0, 300.0, 2.0, 2.0),
            ],
            color: Color::WHITE,
        }),
    ];
    let clips: Vec<Option<sequoia_core::RoundedClip>> = vec![None; items.len()];
    let mut quads = Vec::new();
    let mut glyphs = Vec::new();
    let mut images = Vec::new();
    let mut image_keys = Vec::new();
    let mut gradients = Vec::new();
    let mut runs = Vec::new();
    let (dq, dg, di, dgrad, culled) = group_runs(
        &items,
        &clips,
        &[],
        1.0,
        Some(vp),
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
        &mut quads,
        &mut glyphs,
        &mut images,
        &mut image_keys,
        &mut gradients,
        &mut runs,
    );
    assert_eq!((dq, dg, di, dgrad), (0, 0, 0, 0), "culling to nie drop");
    assert_eq!(culled, 5, "quad + glyph + image + shadow + offscreen bar");
    assert_eq!(quads.len(), 2, "widoczny quad + widoczny pasek");
    assert_eq!(glyphs.len(), 1);
    assert!(images.is_empty() && image_keys.is_empty());
    assert_eq!(
        runs,
        vec![
            GuiRun::Quads(0, 1),
            GuiRun::Glyphs(0, 1),
            GuiRun::Quads(1, 1),
        ],
        "runes retain painting order"
    );
}

fn opaque_quad() -> GpuQuad {
    GpuQuad {
        rect: [0.0, 0.0, 10.0, 10.0],
        color: [1.0, 0.0, 0.0, 1.0],
        radius: 0.0,
        border_color: [0.0; 4],
        border_width: 0.0,
        clip_rect: [0.0; 4],
        clip_radius: 0.0,
        softness: 0.0,
        spread: 0.0,
        shadow_offset: [0.0; 2],
        inset: 0.0,
        transform_a: [1.0, 0.0, 0.0, 1.0],
        transform_b: [0.0, 0.0],
    }
}

#[test]
fn opaque_range_picks_replace_pipeline() {
    let opaque = opaque_quad();
    let mut transparent = opaque_quad();
    transparent.color[3] = 0.5;
    let mut rounded = opaque_quad();
    rounded.radius = 4.0;
    let quads = vec![opaque, transparent, opaque, rounded];
    assert!(quad_range_is_opaque(&quads, 0, 1));
    assert!(
        !quad_range_is_opaque(&quads, 0, 2),
        "jeden transparentny psuje zakres"
    );
    assert!(
        quad_range_is_opaque(&quads, 2, 1),
        "ogon bez domieszki znowu REPLACE"
    );
    assert!(!quad_range_is_opaque(&quads, 0, 4));
    assert!(!quad_range_is_opaque(&quads, 3, 1), "radius is also blend");
    assert!(
        quad_range_is_opaque(&quads, 9, 3),
        "poza buforem = harmless true"
    );
    assert!(quad_range_is_opaque(&[], 0, 0));
}

#[test]
fn backdrop_copy_covers_marker_plus_kernel() {
    let fb = (800, 600);

    let (origin, extent) =
        GpuRenderer::backdrop_copy_region([100.0, 100.0, 200.0, 120.0], 16.0, fb)
            .expect("marker na ekranie");

    assert_eq!((origin.x, origin.y), (53, 53));
    assert_eq!((extent.width, extent.height), (294, 214));
    assert!(extent.width < fb.0 && extent.height < fb.1);

    let (origin, extent) = GpuRenderer::backdrop_copy_region([100.0, 100.0, 200.0, 120.0], 0.0, fb)
        .expect("dim copies too");
    assert_eq!((origin.x, origin.y), (99, 99));
    assert_eq!((extent.width, extent.height), (202, 122));
}

#[test]
fn backdrop_copy_clamps_and_skips_offscreen() {
    let fb = (800, 600);

    let (origin, extent) =
        GpuRenderer::backdrop_copy_region([700.0, 500.0, 200.0, 200.0], 16.0, fb)
            .expect("partially visible");
    assert_eq!((origin.x, origin.y), (653, 453));
    assert_eq!((extent.width, extent.height), (800 - 653, 600 - 453));

    assert!(GpuRenderer::backdrop_copy_region([2000.0, 2000.0, 100.0, 100.0], 16.0, fb).is_none());
    assert!(
        GpuRenderer::backdrop_copy_region([-500.0, -500.0, 100.0, 100.0], 64.0, fb).is_none(),
        "even max blur does not reach the screen"
    );
}

#[test]
fn backdrop_kernel_grows_with_blur_radius() {
    assert_eq!(GpuRenderer::backdrop_kernel_reach(0.0), 1.0);
    assert!((GpuRenderer::backdrop_kernel_reach(16.0) - 46.8128).abs() < 0.001);
    assert!((GpuRenderer::backdrop_kernel_reach(64.0) - 181.2512).abs() < 0.001);
    assert!(
        (GpuRenderer::backdrop_kernel_reach(256.0) - 181.2512).abs() < 0.001,
        "cap keeps the kernel bounded"
    );
}

#[test]
fn empty_viewport_disables_culling() {
    use sequoia_core::Quad;
    let items = vec![RenderItem::Quad(Quad {
        rect: Rect::from_xywh(500.0, 500.0, 10.0, 10.0),
        color: Color::RED,
        radius: 0.0,
        border_color: Color::TRANSPARENT,
        border_width: 0.0,
    })];
    let clips: Vec<Option<sequoia_core::RoundedClip>> = vec![None];
    let mut quads = Vec::new();
    let mut glyphs = Vec::new();
    let mut images = Vec::new();
    let mut image_keys = Vec::new();
    let mut gradients = Vec::new();
    let mut runs = Vec::new();
    let (_, _, _, _, culled) = group_runs(
        &items,
        &clips,
        &[],
        1.0,
        Some(Rect::ZERO),
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
        &mut quads,
        &mut glyphs,
        &mut images,
        &mut image_keys,
        &mut gradients,
        &mut runs,
    );
    assert_eq!(culled, 0, "degenerate viewport does not lose content");
    assert_eq!(quads.len(), 1);
}

#[test]
fn rounded_clip_reaches_gpu_instances_scaled() {
    use sequoia_core::{Quad, Rect, RoundedClip};
    let card = Rect::from_xywh(10.0, 20.0, 240.0, 280.0);
    let items = vec![RenderItem::Quad(Quad {
        rect: Rect::from_xywh(10.0, 20.0, 240.0, 20.0),
        color: Color::GREEN,
        radius: 0.0,
        border_color: Color::TRANSPARENT,
        border_width: 0.0,
    })];
    let clips = vec![Some(RoundedClip::new(card, 8.0))];
    let mut quads = Vec::new();
    let mut glyphs = Vec::new();
    let mut images = Vec::new();
    let mut image_keys = Vec::new();
    let mut gradients = Vec::new();
    let mut runs = Vec::new();
    group_runs(
        &items,
        &clips,
        &[],
        2.0,
        None,
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
        &mut quads,
        &mut glyphs,
        &mut images,
        &mut image_keys,
        &mut gradients,
        &mut runs,
    );
    assert_eq!(quads.len(), 1);
    assert_eq!(quads[0].clip_rect, [20.0, 40.0, 480.0, 560.0]);
    assert_eq!(quads[0].clip_radius, 16.0);
}

#[test]
fn instance_ceiling_follows_the_device_buffer_limit() {
    assert_eq!(ceiling_for(256 << 20, QUAD_BYTES), (256 << 20) / 120);
    assert_eq!(ceiling_for(0, QUAD_BYTES), 1, "always at least one");
    assert_eq!(
        ceiling_for(1024, 0),
        1024,
        "zero size does not divide by zero"
    );
}

#[test]
fn sanitize_samples_maps_to_supported_counts() {
    assert_eq!(sanitize_samples(0), 1);
    assert_eq!(sanitize_samples(1), 1);
    assert_eq!(sanitize_samples(2), 2);
    assert_eq!(sanitize_samples(3), 4);
    assert_eq!(sanitize_samples(8), 4);
}

#[test]
fn decode_image_validates_rgba() {
    use sequoia_core::ImageSource;
    let good = ImageSource::Rgba {
        id: 1,
        width: 2,
        height: 1,
        data: std::sync::Arc::new(vec![0u8; 8]),
    };
    let (w, h, data) = decode_image(&good).expect("poprawne RGBA");
    assert_eq!((w, h), (2, 1));
    assert_eq!(data.len(), 8);
    let bad_len = ImageSource::Rgba {
        id: 2,
        width: 2,
        height: 2,
        data: std::sync::Arc::new(vec![0u8; 8]),
    };
    assert!(decode_image(&bad_len).is_none(), "bad data size");
    let missing = ImageSource::File("no/such/file.png".into());
    assert!(decode_image(&missing).is_none(), "missing file");
}

#[test]
fn surface_respects_ancestor_clip() {
    use sequoia_core::{Point, Vec2};
    use sequoia_text::TextSystem;
    use sequoia_view::{Column, Modifiers, Panel, Runtime, ScrollView};
    let rt = Runtime::new();
    rt.mount(ScrollView::new(
        Column::new()
            .child(WgpuSurface::new(Stub).height(100.0))
            .child(Panel::new().height(800.0)),
    ));
    let mut text = TextSystem::new();
    let mut list = RenderList::new();
    let bounds = Rect::from_xywh(0.0, 0.0, 300.0, 200.0);

    rt.layout(bounds, &mut text);
    rt.paint(&mut list, &mut text);
    let draws = rt.take_surface_draws();
    assert_eq!(draws.len(), 1);
    assert_eq!(draws[0].rect, Rect::from_xywh(0.0, 0.0, 300.0, 100.0));

    rt.wheel(
        Point::new(150.0, 100.0),
        Vec2::new(0.0, -50.0),
        Modifiers::NONE,
        &mut text,
    );
    rt.layout(bounds, &mut text);
    rt.paint(&mut list, &mut text);
    let draws = rt.take_surface_draws();
    assert_eq!(draws.len(), 1);
    assert_eq!(draws[0].rect, Rect::from_xywh(0.0, 0.0, 300.0, 50.0));

    rt.wheel(
        Point::new(150.0, 100.0),
        Vec2::new(0.0, -100.0),
        Modifiers::NONE,
        &mut text,
    );
    rt.layout(bounds, &mut text);
    rt.paint(&mut list, &mut text);
    assert!(rt.take_surface_draws().is_empty());
}

#[test]
fn blit_maps_screen_subrect_to_uv() {
    let full = Rect::from_xywh(10.0, 20.0, 200.0, 100.0);

    let q = blit_instance(full, full);
    assert_eq!(q.rect, [10.0, 20.0, 200.0, 100.0]);
    assert_eq!(q.uv, [0.0, 0.0, 1.0, 1.0]);

    let q = blit_instance(Rect::from_xywh(10.0, 20.0, 200.0, 50.0), full);
    assert_eq!(q.uv, [0.0, 0.0, 1.0, 0.5]);

    let q = blit_instance(Rect::from_xywh(60.0, 45.0, 100.0, 50.0), full);
    assert_eq!(q.uv, [0.25, 0.25, 0.75, 0.75]);
}

#[test]
fn dirty_protocol_defaults_to_always() {
    let s = Stub;
    assert!(s.is_dirty());
    let mut s = s;
    s.mark_clean();
    assert!(s.is_dirty(), "default still dirty");
}

#[test]
fn pending_frames_counts_down_and_requests() {
    let mut p = PendingFrames::new();
    assert_eq!(p.pending(), 0);

    let mut calls = 0;
    assert_eq!(p.tick(|| calls += 1), 0);
    assert_eq!(calls, 0);

    p.request(3);
    assert_eq!(p.pending(), 3);
    assert_eq!(p.tick(|| calls += 1), 2);
    assert_eq!(p.tick(|| calls += 1), 1);

    p.request(5);
    assert_eq!(p.pending(), 5);
    p.request(2);
    assert_eq!(p.pending(), 5);
    while p.pending() > 0 {
        p.tick(|| calls += 1);
    }
    assert_eq!(calls, 2 + 5, "2 ticks before request(5) + 5 after");

    assert_eq!(p.tick(|| calls += 1), 0);
}

#[test]
fn pending_frames_request_if_only_on_change() {
    let mut p = PendingFrames::new();
    p.request_if(false, 16);
    assert_eq!(p.pending(), 0);
    p.request_if(true, 16);
    assert_eq!(p.pending(), 16);
}

#[test]
fn revision_first_hash_always_changed() {
    let mut r = Revision::new();
    assert!(r.changed(42), "first hash is a change");
    assert!(!r.changed(42), "same hash is silence");
    assert!(r.changed(43), "other hash is a change");
    assert!(!r.changed(43));
}

#[test]
fn cached_binding_keeps_stable_key_across_paints() {
    use sequoia_text::TextSystem;
    use sequoia_view::Runtime;
    let rt = Runtime::new();
    rt.mount(
        WgpuSurface::new(Stub)
            .cached(true)
            .width(100.0)
            .height(60.0),
    );
    let mut text = TextSystem::new();
    let mut list = RenderList::new();
    rt.layout(Rect::from_xywh(0.0, 0.0, 400.0, 400.0), &mut text);

    rt.paint(&mut list, &mut text);
    let draws = rt.take_surface_draws();
    assert_eq!(draws.len(), 1);
    let key1 = draws[0]
        .target
        .clone()
        .downcast::<SurfaceBinding>()
        .map(|b| (b.key, b.cached))
        .unwrap_or((0, false));

    rt.paint(&mut list, &mut text);
    let draws = rt.take_surface_draws();
    let key2 = draws[0]
        .target
        .clone()
        .downcast::<SurfaceBinding>()
        .map(|b| (b.key, b.cached))
        .unwrap_or((0, false));

    assert!(key1.1, "cached flag in binding");
    assert_eq!(key1, key2, "same renderer = same cache key");
}

#[test]
fn samples_plumbing_reaches_binding() {
    use sequoia_text::TextSystem;
    use sequoia_view::Runtime;
    fn samples_of(surface: WgpuSurface<Stub>) -> u32 {
        let rt = Runtime::new();
        rt.mount(surface);
        let mut text = TextSystem::new();
        let mut list = RenderList::new();
        rt.layout(Rect::from_xywh(0.0, 0.0, 400.0, 400.0), &mut text);
        rt.paint(&mut list, &mut text);
        let draws = rt.take_surface_draws();
        assert_eq!(draws.len(), 1);
        draws[0]
            .target
            .clone()
            .downcast::<SurfaceBinding>()
            .map(|b| b.samples)
            .unwrap_or(0)
    }
    assert_eq!(
        samples_of(WgpuSurface::new(Stub).width(100.0).height(60.0)),
        1,
        "default: no MSAA"
    );
    assert_eq!(
        samples_of(
            WgpuSurface::new(Stub)
                .cached(true)
                .samples(4)
                .width(100.0)
                .height(60.0)
        ),
        4,
        "builder dociera do bindingu"
    );
    assert_eq!(
        samples_of(WgpuSurface::new(Stub).samples(99).width(100.0).height(60.0)),
        4,
        "sanitize in builder"
    );
}

#[test]
fn surface_format_plumbing_reaches_binding() {
    use sequoia_text::TextSystem;
    use sequoia_view::Runtime;
    fn format_of(surface: WgpuSurface<Stub>) -> Option<wgpu::TextureFormat> {
        let rt = Runtime::new();
        rt.mount(surface);
        let mut text = TextSystem::new();
        let mut list = RenderList::new();
        rt.layout(Rect::from_xywh(0.0, 0.0, 400.0, 400.0), &mut text);
        rt.paint(&mut list, &mut text);
        let draws = rt.take_surface_draws();
        assert_eq!(draws.len(), 1);
        draws[0]
            .target
            .clone()
            .downcast::<SurfaceBinding>()
            .map(|b| b.format)
            .unwrap_or(None)
    }
    let surface = WgpuSurface::new(Stub).width(100.0).height(60.0);
    assert_eq!(
        format_of(surface),
        None,
        "default follows the window format"
    );
    let surface = WgpuSurface::new(Stub)
        .format(wgpu::TextureFormat::Rgba16Float)
        .width(100.0)
        .height(60.0);
    assert_eq!(
        format_of(surface),
        Some(wgpu::TextureFormat::Rgba16Float),
        "builder reaches the binding"
    );
}

#[test]
fn resolves_requested_surface_formats() {
    use wgpu::TextureFormat as F;
    let srgb = resolve_surface_format(&[F::Bgra8Unorm, F::Bgra8UnormSrgb], Some(F::Bgra8UnormSrgb))
        .unwrap();
    assert_eq!(srgb.config, F::Bgra8UnormSrgb);
    assert_eq!(srgb.render, F::Bgra8UnormSrgb);
    assert!(srgb.view_formats.is_empty());

    let unorm = resolve_surface_format(&[F::Bgra8Unorm], Some(F::Bgra8Unorm)).unwrap();
    assert_eq!(unorm.config, F::Bgra8Unorm);
    assert_eq!(unorm.render, F::Bgra8UnormSrgb);
    assert_eq!(unorm.view_formats, vec![F::Bgra8UnormSrgb]);

    let float = resolve_surface_format(&[F::Rgba16Float], Some(F::Rgba16Float)).unwrap();
    assert_eq!(float.config, F::Rgba16Float);
    assert_eq!(float.render, F::Rgba16Float);
    assert!(float.view_formats.is_empty());

    let fallback = resolve_surface_format(&[F::Rgba8UnormSrgb], Some(F::Rgba16Float)).unwrap();
    assert_eq!(fallback.config, F::Rgba8UnormSrgb);
    assert_eq!(fallback.render, F::Rgba8UnormSrgb);

    let no_view =
        resolve_surface_format(&[F::Rgb10a2Unorm, F::Bgra8UnormSrgb], Some(F::Rgb10a2Unorm))
            .unwrap();
    assert_eq!(no_view.config, F::Bgra8UnormSrgb);
    assert_eq!(no_view.render, F::Bgra8UnormSrgb);
    assert!(no_view.view_formats.is_empty());

    let default = resolve_surface_format(&[F::Bgra8Unorm, F::Bgra8UnormSrgb], None).unwrap();
    assert_eq!(default.config, F::Bgra8UnormSrgb);
    assert_eq!(default.render, F::Bgra8UnormSrgb);
}

#[test]
fn color_space_matches_format() {
    use wgpu::TextureFormat as F;
    assert_eq!(ColorSpace::of(F::Rgba8UnormSrgb), ColorSpace::Srgb);
    assert_eq!(ColorSpace::of(F::Bgra8Unorm), ColorSpace::Linear);
    assert_eq!(ColorSpace::of(F::Rgba16Float), ColorSpace::Linear);
    assert!((srgb_to_linear(0.5) - 0.214).abs() < 0.005);
    assert!((linear_to_srgb(0.214) - 0.5).abs() < 0.005);
}

fn pointer_tag(ev: &PointerEvent) -> &'static str {
    match ev {
        PointerEvent::Move { .. } => "move",
        PointerEvent::Down { .. } => "down",
        PointerEvent::Up { .. } => "up",
        PointerEvent::Cancel => "cancel",
        PointerEvent::Wheel { .. } => "wheel",
        _ => "unknown",
    }
}

fn input_surface(log: Rc<RefCell<Vec<String>>>) -> (Runtime, Rc<RefCell<Vec<String>>>) {
    use sequoia_text::TextSystem;
    use sequoia_view::Runtime;
    let lp = Rc::clone(&log);
    let lk = Rc::clone(&log);
    let rt = Runtime::new();
    rt.mount(
        WgpuSurface::new(Stub)
            .width(200.0)
            .height(100.0)
            .on_pointer(move |ev, area| {
                lp.borrow_mut()
                    .push(format!("{}@{}", pointer_tag(ev), area.left() as i32));
            })
            .on_key(move |ev| {
                let _ = ev;
                lk.borrow_mut().push("key".to_string());
            }),
    );
    let mut text = TextSystem::new();
    rt.layout(Rect::from_xywh(0.0, 0.0, 400.0, 400.0), &mut text);
    (rt, log)
}

#[test]
fn surface_input_forwarded_on_hit() {
    use sequoia_core::{Point, Vec2};
    use sequoia_text::TextSystem;
    use sequoia_view::{Key, Modifiers, PointerButton};
    let log: Rc<RefCell<Vec<String>>> = Rc::default();
    let (rt, log) = input_surface(log);
    let mut text = TextSystem::new();

    rt.pointer_move(Point::new(50.0, 50.0), &mut text);
    rt.pointer_down(
        Point::new(50.0, 50.0),
        PointerButton::Left,
        Modifiers::NONE,
        &mut text,
    );
    rt.pointer_up(
        Point::new(50.0, 50.0),
        PointerButton::Left,
        Modifiers::NONE,
        &mut text,
    );
    rt.wheel(
        Point::new(50.0, 50.0),
        Vec2::new(0.0, 10.0),
        Modifiers::NONE,
        &mut text,
    );
    assert_eq!(*log.borrow(), vec!["move@0", "down@0", "up@0", "wheel@0"]);

    rt.pointer_move(Point::new(300.0, 300.0), &mut text);
    assert_eq!(log.borrow().len(), 4);

    rt.key(Key::from_char('a'), Modifiers::NONE, &mut text);
    assert_eq!(log.borrow().len(), 4);
    rt.set_focus(rt.root());
    rt.key(Key::from_char('a'), Modifiers::NONE, &mut text);
    assert_eq!(
        *log.borrow(),
        vec!["move@0", "down@0", "up@0", "wheel@0", "key"]
    );
}

#[test]
fn surface_capture_and_focus_flags() {
    use sequoia_core::Point;
    use sequoia_text::TextSystem;
    use sequoia_view::{Modifiers, PointerButton, Runtime};
    fn harness(surface: WgpuSurface<Stub>) -> Runtime {
        let rt = Runtime::new();
        rt.mount(surface);
        let mut text = TextSystem::new();
        rt.layout(Rect::from_xywh(0.0, 0.0, 400.0, 400.0), &mut text);
        rt
    }
    let mut text = TextSystem::new();

    let rt = harness(WgpuSurface::new(Stub).width(200.0).height(100.0));
    rt.pointer_down(
        Point::new(50.0, 50.0),
        PointerButton::Left,
        Modifiers::NONE,
        &mut text,
    );
    assert_eq!(rt.interact().captured, rt.root());
    assert_eq!(rt.interact().focus, None);

    let rt = harness(
        WgpuSurface::new(Stub)
            .width(200.0)
            .height(100.0)
            .click_through(true),
    );
    rt.pointer_down(
        Point::new(50.0, 50.0),
        PointerButton::Left,
        Modifiers::NONE,
        &mut text,
    );
    assert_eq!(rt.interact().captured, None);

    let rt = harness(
        WgpuSurface::new(Stub)
            .width(200.0)
            .height(100.0)
            .focusable(true),
    );
    rt.pointer_down(
        Point::new(50.0, 50.0),
        PointerButton::Left,
        Modifiers::NONE,
        &mut text,
    );
    assert_eq!(rt.interact().focus, rt.root());
}

#[test]
fn prefers_srgb_format_for_linear_workflow() {
    use wgpu::TextureFormat as F;
    assert_eq!(
        pick_format(&[F::Bgra8Unorm, F::Bgra8UnormSrgb]),
        Some(F::Bgra8UnormSrgb),
        "sRGB: blending in linear light (instance colors are linear)"
    );
    assert_eq!(pick_format(&[F::Bgra8UnormSrgb]), Some(F::Bgra8UnormSrgb));
    assert_eq!(
        pick_format(&[F::Bgra8Unorm]),
        Some(F::Bgra8Unorm),
        "fallback when sRGB missing"
    );
    assert_eq!(pick_format(&[]), None);
}

#[test]
fn instance_colors_are_linear() {
    let g = linear_rgba(Color::rgb(0.5, 0.5, 0.5));
    assert!(
        (g[0] - 0.214).abs() < 0.005,
        "0.5 sRGB ≈ 0.214 liniowo: {g:?}"
    );
    assert_eq!(linear_rgba(Color::WHITE), [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(linear_rgba(Color::BLACK), [0.0, 0.0, 0.0, 1.0]);

    let half = linear_rgba(Color::rgba(1.0, 1.0, 1.0, 0.25));
    assert_eq!(half[3], 0.25);
}

#[test]
fn clear_color_roundtrips_authored_value() {
    let authored = Color::rgb(0.5, 0.5, 0.5);
    let lin = clear_color_linear(authored);
    assert!((lin.r - 0.214).abs() < 0.005, "liniowo: {lin:?}");
    let same = clear_color_authored(authored);
    assert_eq!(same.r, 0.5, "UNORM (surfaces) without conversion");
    assert_eq!(clear_color_linear(Color::WHITE).r, 1.0);
    assert_eq!(clear_color_linear(Color::BLACK).r, 0.0);
}

#[test]
fn surface_pointer_drag_pattern_updates_shared_signal() {
    use sequoia_core::{Point, Vec2};
    use sequoia_reactive::Signal;
    use sequoia_view::{Modifiers, PointerButton, Runtime};
    let yaw = Signal::new(0.0f32);
    let drag = Rc::new(Cell::new(None::<(f32, f32)>));
    let drag_in = Rc::clone(&drag);
    let yaw_in = yaw.clone();
    let rt = Runtime::new();
    rt.mount(
        WgpuSurface::new(Stub)
            .width(200.0)
            .height(100.0)
            .on_pointer(move |ev, _area| match ev {
                PointerEvent::Down {
                    button: PointerButton::Left,
                    pos,
                    ..
                } => {
                    drag_in.set(Some((pos.x, pos.y)));
                }
                PointerEvent::Move { pos, .. } => {
                    if let Some((lx, _)) = drag_in.get() {
                        yaw_in.update(|y| *y += (pos.x - lx) * 0.01);
                        drag_in.set(Some((pos.x, pos.y)));
                    }
                }
                PointerEvent::Up { .. } => drag_in.set(None),
                _ => {}
            }),
    );
    let mut text = sequoia_text::TextSystem::new();
    rt.layout(Rect::from_xywh(0.0, 0.0, 400.0, 400.0), &mut text);

    rt.pointer_down(
        Point::new(50.0, 50.0),
        PointerButton::Left,
        Modifiers::NONE,
        &mut text,
    );
    rt.pointer_move(Point::new(150.0, 50.0), &mut text);
    assert!(
        (yaw.get_untracked() - 1.0).abs() < 1e-4,
        "yaw = {}",
        yaw.get_untracked()
    );

    rt.pointer_up(
        Point::new(150.0, 50.0),
        PointerButton::Left,
        Modifiers::NONE,
        &mut text,
    );
    rt.pointer_move(Point::new(190.0, 50.0), &mut text);
    assert!((yaw.get_untracked() - 1.0).abs() < 1e-4, "gesture finished");

    let _ = Vec2::new(0.0, 0.0);
    rt.wheel(
        Point::new(50.0, 50.0),
        Vec2::new(0.0, 10.0),
        Modifiers::NONE,
        &mut text,
    );
}

#[test]
fn waveform_batch_becomes_one_quad_run() {
    use sequoia_core::Rect;
    let bars: Vec<Rect> = (0..64)
        .map(|i| Rect::from_xywh(i as f32 * 3.0, 10.0, 2.0, 40.0))
        .collect();
    let items = vec![RenderItem::Waveform(sequoia_core::WaveformDraw {
        bars,
        color: Color::WHITE,
    })];
    let clips: Vec<Option<sequoia_core::RoundedClip>> = vec![None];
    let mut quads = Vec::new();
    let mut glyphs = Vec::new();
    let mut images = Vec::new();
    let mut keys = Vec::new();
    let mut gradients = Vec::new();
    let mut runs = Vec::new();
    let (dq, _, _, _, _) = group_runs(
        &items,
        &clips,
        &[],
        1.0,
        None,
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
        &mut quads,
        &mut glyphs,
        &mut images,
        &mut keys,
        &mut gradients,
        &mut runs,
    );
    assert_eq!(dq, 0);
    assert_eq!(quads.len(), 64);
    assert_eq!(runs, vec![GuiRun::Quads(0, 64)], "1 item → 1 run");
}

#[test]
fn gradient_layout_matches_shader_offsets() {
    assert_eq!(
        std::mem::size_of::<GpuGradient>(),
        GRADIENT_BYTES,
        "instance must match vs_gradient attributes down to the byte"
    );

    let g = GpuGradient {
        rect: [1.0, 2.0, 3.0, 4.0],
        stop0: [0.0; 4],
        stop1: [0.0; 4],
        stop2: [0.0; 4],
        stop3: [0.0; 4],
        stop_pos: [0.0, 0.5, 1.0, 1.0],
        params: [0.0, 0.0, 8.0, 3.0],
        clip_rect: [0.0; 4],
        clip_radius: 0.0,
        _pad: [0.0; 5],
        transform_a: [1.0, 0.0, 0.0, 1.0],
        transform_b: [0.0, 0.0],
    };
    let bytes = bytemuck::bytes_of(&g);
    let f = |off: usize| f32::from_le_bytes(bytes[off..off + 4].try_into().unwrap());
    assert_eq!(f(0), 1.0, "rect.x @ 0");
    assert_eq!(f(80), 0.0, "stop_pos[0] @ 80");
    assert_eq!(f(84), 0.5, "stop_pos[1] @ 84");
    assert_eq!(f(96), 0.0, "kind @ 96");
    assert_eq!(f(104), 8.0, "radius @ 104");
    assert_eq!(f(128), 0.0, "clip_radius @ 128");
    assert_eq!(f(152), 1.0, "transform_a @ 152");
}

#[test]
fn gradient_becomes_one_gpu_instance() {
    use sequoia_core::{Color, GradientDraw, GradientKind, GradientStops, Rect};
    let stops = GradientStops::two(Color::RED, Color::BLUE);
    let items = vec![RenderItem::Gradient(GradientDraw {
        rect: Rect::from_xywh(0.0, 0.0, 100.0, 40.0),
        radius: 8.0,
        kind: GradientKind::horizontal(),
        stops,
    })];
    let clips: Vec<Option<sequoia_core::RoundedClip>> = vec![None];
    let mut quads = Vec::new();
    let mut glyphs = Vec::new();
    let mut images = Vec::new();
    let mut keys = Vec::new();
    let mut gradients = Vec::new();
    let mut runs = Vec::new();
    let (dq, dg, di, dgrad, culled) = group_runs(
        &items,
        &clips,
        &[],
        2.0,
        None,
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
        &mut quads,
        &mut glyphs,
        &mut images,
        &mut keys,
        &mut gradients,
        &mut runs,
    );
    assert_eq!((dq, dg, di, dgrad, culled), (0, 0, 0, 0, 0));
    assert!(quads.is_empty(), "gradient nie idzie paskami");
    assert_eq!(gradients.len(), 1, "1 gradient = 1 instancja");
    assert_eq!(runs, vec![GuiRun::Gradients(0, 1)]);
    let g = gradients[0];
    assert_eq!(g.rect, [0.0, 0.0, 200.0, 80.0], "rectangle × scale");
    assert_eq!(g.params[0], 0.0, "liniowy");
    assert_eq!(g.params[2], 16.0, "radius × scale");
    assert_eq!(g.params[3], 2.0, "2 stopy");

    assert_eq!(g.stop0, [1.0, 0.0, 0.0, 1.0]);
    assert_eq!(g.stop1, [0.0, 0.0, 1.0, 1.0]);
}

#[test]
fn gradient_culls_offscreen_and_rejects_too_many_stops() {
    use sequoia_core::{Color, GradientDraw, GradientKind, GradientStops, Rect};
    let two = GradientStops::two(Color::RED, Color::BLUE);
    let five = GradientStops::new(vec![
        sequoia_core::BrushStop::new(0.0, Color::RED),
        sequoia_core::BrushStop::new(0.25, Color::GREEN),
        sequoia_core::BrushStop::new(0.5, Color::BLUE),
        sequoia_core::BrushStop::new(0.75, Color::WHITE),
        sequoia_core::BrushStop::new(1.0, Color::BLACK),
    ]);
    let items = vec![
        RenderItem::Gradient(GradientDraw {
            rect: Rect::from_xywh(500.0, 500.0, 10.0, 10.0),
            radius: 0.0,
            kind: GradientKind::horizontal(),
            stops: two,
        }),
        RenderItem::Gradient(GradientDraw {
            rect: Rect::from_xywh(0.0, 0.0, 10.0, 10.0),
            radius: 0.0,
            kind: GradientKind::cover(),
            stops: five,
        }),
    ];
    let clips: Vec<Option<sequoia_core::RoundedClip>> = vec![None; 2];
    let mut quads = Vec::new();
    let mut glyphs = Vec::new();
    let mut images = Vec::new();
    let mut keys = Vec::new();
    let mut gradients = Vec::new();
    let mut runs = Vec::new();
    let vp = Rect::from_xywh(0.0, 0.0, 100.0, 100.0);
    let (_, _, _, dgrad, culled) = group_runs(
        &items,
        &clips,
        &[],
        1.0,
        Some(vp),
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
        &mut quads,
        &mut glyphs,
        &mut images,
        &mut keys,
        &mut gradients,
        &mut runs,
    );
    assert_eq!(culled, 1, "gradient poza ekranem");
    assert_eq!(dgrad, 1, ">4 stopy = drop (Painter dzieli na pasy)");
    assert!(gradients.is_empty() && runs.is_empty());
}

#[test]
fn waveform_merges_with_neighbour_quads() {
    use sequoia_core::{Quad, Rect};
    let items = vec![
        RenderItem::Quad(Quad {
            rect: Rect::from_xywh(0.0, 0.0, 10.0, 10.0),
            color: Color::RED,
            radius: 0.0,
            border_color: Color::TRANSPARENT,
            border_width: 0.0,
        }),
        RenderItem::Waveform(sequoia_core::WaveformDraw {
            bars: vec![Rect::from_xywh(20.0, 0.0, 5.0, 5.0)],
            color: Color::WHITE,
        }),
    ];
    let clips: Vec<Option<sequoia_core::RoundedClip>> = vec![None, None];
    let mut quads = Vec::new();
    let mut glyphs = Vec::new();
    let mut images = Vec::new();
    let mut keys = Vec::new();
    let mut gradients = Vec::new();
    let mut runs = Vec::new();
    group_runs(
        &items,
        &clips,
        &[],
        1.0,
        None,
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
        &mut quads,
        &mut glyphs,
        &mut images,
        &mut keys,
        &mut gradients,
        &mut runs,
    );
    assert_eq!(quads.len(), 2);
    assert_eq!(runs, vec![GuiRun::Quads(0, 2)], "adjacent runs merge");
}

#[test]
fn decode_dynamic_validates_like_rgba() {
    use sequoia_core::ImageSource;
    let good = ImageSource::Dynamic {
        id: 9,
        width: 4,
        height: 1,
        data: std::sync::Arc::new(vec![0u8; 16]),
        version: 3,
    };
    assert!(decode_image(&good).is_some());
    assert_eq!(good.dynamic_version(), Some(3));
}

fn marker_at(at: usize) -> BackdropSpan {
    use sequoia_core::{BackdropDraw, Color, Rect, SpanMask};
    BackdropSpan {
        at,
        draw: BackdropDraw {
            rect: Rect::from_xywh(0.0, 0.0, 100.0, 100.0),
            radius: 0.0,
            blur: 8.0,
            dim: Color::TRANSPARENT,
            noise: 0.0,
            mask: SpanMask::None,
        },
        clip: None,
    }
}

fn filter_span(start: usize, end: usize) -> FilterSpan {
    use sequoia_core::{FilterDraw, Rect, SpanMask};
    FilterSpan {
        start,
        end,
        draw: FilterDraw {
            rect: Rect::from_xywh(0.0, 0.0, 100.0, 100.0),
            radius: 0.0,
            blur: 8.0,
            mask: SpanMask::None,
        },
        clip: None,
    }
}

#[test]
fn composite_ops_keep_paint_order() {
    let ops = plan_composite_ops(&[marker_at(2), marker_at(7)], &[filter_span(3, 5)], 10);
    assert_eq!(
        ops,
        vec![
            CompOp::Marker { at: 2, span_idx: 0 },
            CompOp::FilterRange {
                start: 3,
                end: 5,
                span_idx: 0
            },
            CompOp::Marker { at: 7, span_idx: 1 },
        ]
    );
}

#[test]
fn composite_ops_marker_before_filter_on_tie() {
    let ops = plan_composite_ops(&[marker_at(3)], &[filter_span(3, 5)], 10);
    assert_eq!(
        ops,
        vec![
            CompOp::Marker { at: 3, span_idx: 0 },
            CompOp::FilterRange {
                start: 3,
                end: 5,
                span_idx: 0
            },
        ]
    );
}

#[test]
fn composite_ops_skip_marker_inside_filter() {
    let ops = plan_composite_ops(&[marker_at(4)], &[filter_span(3, 5)], 10);
    assert_eq!(
        ops,
        vec![CompOp::FilterRange {
            start: 3,
            end: 5,
            span_idx: 0
        }],
        "marker w filtrze pomijany"
    );
}

#[test]
fn composite_ops_outer_filter_wins_nesting() {
    let ops = plan_composite_ops(&[], &[filter_span(0, 4), filter_span(1, 3)], 10);
    assert_eq!(
        ops,
        vec![CompOp::FilterRange {
            start: 0,
            end: 4,
            span_idx: 0
        }],
        "nested covers outer composite"
    );
}

#[test]
fn composite_ops_clamp_to_stream() {
    let ops = plan_composite_ops(&[marker_at(999)], &[filter_span(8, 999)], 10);
    assert_eq!(
        ops,
        vec![
            CompOp::FilterRange {
                start: 8,
                end: 10,
                span_idx: 0
            },
            CompOp::Marker {
                at: 10,
                span_idx: 0
            },
        ],
        "cut filter + trailing marker at the end of the stream"
    );
}

#[test]
fn filter_instance_is_pure_blur() {
    let inst = GpuRenderer::filter_instance(&filter_span(0, 2), 2.0).expect("widoczny");
    assert_eq!(inst.dim, [0.0; 4], "bez przyciemnienia");
    assert_eq!(inst.params[0], 16.0, "blur skalowany");
    assert_eq!(inst.params[1], 0.0, "bez ziarna");
    assert_eq!(inst.rect, [0.0, 0.0, 200.0, 200.0]);
    assert_eq!(inst.mask, [0.0; 4], "no mask, full strength");
}

#[test]
fn filter_instance_carries_mask_unscaled() {
    use sequoia_core::SpanMask;
    let mut span = filter_span(0, 2);
    span.draw.mask = SpanMask::Linear {
        from: 1.0,
        to: 0.0,
        angle_rad: std::f32::consts::FRAC_PI_2,
    };
    let inst = GpuRenderer::filter_instance(&span, 2.0).expect("widoczny");

    assert_eq!(inst.mask, [1.0, 1.0, 0.0, std::f32::consts::FRAC_PI_2]);
    assert_eq!(inst.params[0], 16.0);
}

#[test]
fn backdrop_instance_carries_mask_unscaled() {
    use sequoia_core::{Rect, SpanMask};
    let mut span = backdrop_span(Rect::from_xywh(0.0, 0.0, 10.0, 10.0), 8.0);
    span.draw.mask = SpanMask::Uniform(0.5);
    let inst = super::GpuRenderer::backdrop_instance(&span, 3.0).expect("widoczny");
    assert_eq!(inst.mask, [2.0, 0.5, 0.0, 0.0]);
    assert_eq!(inst.params[0], 24.0, "blur skalowany, maska nie");
}

#[test]
fn draws_reset_every_paint() {
    use sequoia_text::TextSystem;
    use sequoia_view::Runtime;
    let rt = Runtime::new();
    rt.mount(WgpuSurface::new(Stub).width(50.0).height(50.0));
    let mut text = TextSystem::new();
    let mut list = RenderList::new();
    rt.layout(Rect::from_xywh(0.0, 0.0, 400.0, 400.0), &mut text);
    rt.paint(&mut list, &mut text);
    assert_eq!(rt.take_surface_draws().len(), 1);
    assert!(rt.take_surface_draws().is_empty(), "second take empty");
    rt.paint(&mut list, &mut text);
    assert_eq!(rt.take_surface_draws().len(), 1, "paint does not duplicate");
}

#[test]
fn rotation_reaches_gpu_instances() {
    use sequoia_core::{Quad, Rect, Transform};
    let items = vec![RenderItem::Quad(Quad {
        rect: Rect::from_xywh(0.0, 0.0, 10.0, 10.0),
        color: Color::RED,
        radius: 0.0,
        border_color: Color::TRANSPARENT,
        border_width: 0.0,
    })];
    let clips: Vec<Option<sequoia_core::RoundedClip>> = vec![None];
    let rot = Transform::rotate_around_degrees(90.0, sequoia_core::Point::new(5.0, 5.0));
    let transforms = vec![rot];
    let mut quads = Vec::new();
    let mut glyphs = Vec::new();
    let mut images = Vec::new();
    let mut keys = Vec::new();
    let mut gradients = Vec::new();
    let mut runs = Vec::new();
    group_runs(
        &items,
        &clips,
        &transforms,
        2.0,
        None,
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
        &mut quads,
        &mut glyphs,
        &mut images,
        &mut keys,
        &mut gradients,
        &mut runs,
    );
    assert_eq!(quads.len(), 1);
    let q = quads[0];

    assert!((q.transform_a[0] - 0.0).abs() < 1e-4, "{:?}", q.transform_a);
    assert!((q.transform_a[1] - 1.0).abs() < 1e-4, "{:?}", q.transform_a);
    assert_eq!(q.rect, [0.0, 0.0, 20.0, 20.0], "rect × skala");
}

#[test]
fn identity_transform_is_default() {
    use sequoia_core::{Quad, Rect};
    let items = vec![RenderItem::Quad(Quad {
        rect: Rect::from_xywh(0.0, 0.0, 10.0, 10.0),
        color: Color::RED,
        radius: 0.0,
        border_color: Color::TRANSPARENT,
        border_width: 0.0,
    })];
    let clips: Vec<Option<sequoia_core::RoundedClip>> = vec![None];
    let mut quads = Vec::new();
    let mut glyphs = Vec::new();
    let mut images = Vec::new();
    let mut keys = Vec::new();
    let mut gradients = Vec::new();
    let mut runs = Vec::new();
    group_runs(
        &items,
        &clips,
        &[],
        1.0,
        None,
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
        &mut quads,
        &mut glyphs,
        &mut images,
        &mut keys,
        &mut gradients,
        &mut runs,
    );
    assert_eq!(quads[0].transform_a, [1.0, 0.0, 0.0, 1.0]);
    assert_eq!(quads[0].transform_b, [0.0, 0.0]);
}

#[test]
fn damage_scissors_clip_at_framebuffer_edges() {
    let regions = [
        Rect::from_xywh(-10.0, -10.0, 20.0, 20.0),
        Rect::from_xywh(95.0, 90.0, 20.0, 20.0),
    ];
    let scissors = damage_scissors(&regions, 0.0, 1.0, (100, 100));
    assert_eq!(scissors.len(), 2);
    assert_eq!(scissors[0].1, (0, 0, 10, 10), "left/top clipped");
    assert_eq!(scissors[1].1, (95, 90, 5, 10), "right/bottom clipped");
}

#[test]
fn damage_scissors_cover_fractional_edges() {
    let regions = [Rect::from_xywh(10.2, 10.2, 10.0, 10.0)];
    let scissors = damage_scissors(&regions, 0.0, 1.5, (100, 100));
    assert_eq!(
        scissors[0].1,
        (15, 15, 16, 16),
        "floor/ceil keeps anti-aliased edges inside the scissor"
    );
}

#[test]
fn damage_scissors_apply_margin_in_logical_pixels() {
    let regions = [Rect::from_xywh(40.0, 40.0, 20.0, 20.0)];
    let scissors = damage_scissors(&regions, 32.0, 1.0, (200, 200));
    assert_eq!(scissors[0].1, (8, 8, 84, 84));
    assert!(
        scissors[0]
            .0
            .contains_rect(Rect::from_xywh(40.0, 40.0, 20.0, 20.0))
    );
}

#[test]
fn damage_scissors_drop_offscreen_regions() {
    let regions = [
        Rect::from_xywh(200.0, 200.0, 10.0, 10.0),
        Rect::from_xywh(10.0, 10.0, 5.0, 5.0),
    ];
    let scissors = damage_scissors(&regions, 0.0, 1.0, (100, 100));
    assert_eq!(scissors.len(), 1);
    assert_eq!(scissors[0].1, (10, 10, 5, 5));
    assert!(damage_scissors(&[], 0.0, 1.0, (100, 100)).is_empty());
}

#[test]
fn damage_scissors_bound_region_count() {
    let regions: Vec<Rect> = (0..=MAX_DAMAGE_REGIONS)
        .map(|i| Rect::from_xywh(i as f32 * 20.0, 0.0, 4.0, 4.0))
        .collect();
    let scissors = damage_scissors(&regions, 0.0, 1.0, (1000, 1000));
    assert_eq!(scissors.len(), 1, "extra regions collapse into a bbox");
    assert_eq!(scissors[0].1.0, 0);
    assert!(scissors[0].1.2 >= MAX_DAMAGE_REGIONS as u32 * 20);
}
