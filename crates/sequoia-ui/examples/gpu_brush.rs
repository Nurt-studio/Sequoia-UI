//! GPU brush prototype: soft round dabs accumulated in a persistent layer.
//!
//! Run both modes and compare the `[stress]` line the benchmark prints:
//!
//! ```text
//! cargo run --release -p sequoia-ui --example gpu_brush -- --mode gpu --frames 240
//! cargo run --release -p sequoia-ui --example gpu_brush -- --mode cpu --frames 240
//! ```
//!
//! Both modes paint the same dab sequence. `gpu` draws every dab with the
//! [`Dab`] shader into one persistent render target, so each frame only submits
//! the new dabs and the target keeps the earlier ones. `cpu` stamps the same
//! dabs into a `Pixels` buffer with the loop `example/paint` uses, which
//! re-uploads the whole document every frame. The difference between the two
//! runs is the cost of the paint path, not of uploading UI chrome.

use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};

use sequoia_core::{Color, Fill, Length, Pixels, Rect, RenderTargetId, Size, TargetFormat};
use sequoia_shader::shader;
use sequoia_ui::prelude::*;
use sequoia_ui::render::PresentMode;
use sequoia_view::{Canvas, Painter};

const DOC_W: u32 = 900;
const DOC_H: u32 = 600;
const DABS_PER_FRAME: usize = 12;
const DAB_RADIUS: f32 = 18.0;
/// Handle of the persistent layer, stable for the whole run.
const INK_LAYER: RenderTargetId = RenderTargetId::new(1);

static FRAME: AtomicU32 = AtomicU32::new(0);
static CPU_INK: Mutex<Option<Pixels>> = Mutex::new(None);

// Soft round dab. The draw rect is exactly the dab's bounding box, so the
// fragment uv maps the circle without any extra scale uniform.
shader! {
    pub fn dab(uv: FragUv, color: Color, hardness: f32) -> Color {
        let d = length(uv - vec2<f32>(0.5, 0.5)) * 2.0;
        let soft = 1.0 - smoothstep(clamp(hardness, 0.0, 1.0), 1.0, d);
        return vec4<f32>(color.rgb, color.a * soft);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Gpu,
    Cpu,
}

impl Mode {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "gpu" => Some(Self::Gpu),
            "cpu" => Some(Self::Cpu),
            _ => None,
        }
    }
}

/// Advances the benchmark clock and keeps requesting frames.
struct Driver;

impl Leaf for Driver {
    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle::default()
            .width(Length::Px(0.0))
            .height(Length::Px(0.0))
    }

    fn paint(&self, ctx: &mut PaintCtx) {
        FRAME.fetch_add(1, Ordering::Relaxed);
        ctx.request_frame();
    }
}

impl_view_for_leaf!(Driver);

/// Dab positions and colours for `frame`, on a path that stays in the document.
fn dabs(frame: u32) -> impl Iterator<Item = (f32, f32, Color)> {
    let first = frame as usize * DABS_PER_FRAME;
    (0..DABS_PER_FRAME).map(move |i| {
        let t = (first + i) as f32 * 0.035;
        let x = DOC_W as f32 * (0.5 + 0.40 * (t * 1.7).cos() * (t * 0.31).sin());
        let y = DOC_H as f32 * (0.5 + 0.38 * (t * 1.3).sin());
        let color = Color::rgba(
            0.30 + 0.45 * (t * 0.9).sin(),
            0.30 + 0.45 * (t * 1.6).cos(),
            0.40 + 0.35 * (t * 2.3).sin(),
            0.35,
        );
        (x, y, color)
    })
}

fn dab_rect(x: f32, y: f32) -> Rect {
    Rect::from_xywh(x - DAB_RADIUS, y - DAB_RADIUS, DAB_RADIUS * 2.0, DAB_RADIUS * 2.0)
}

fn paint_gpu(painter: &mut Painter, reset_every: u32) {
    let canvas = painter.rect();
    painter.fill(canvas, Fill::solid(Color::WHITE));
    let frame = FRAME.load(Ordering::Relaxed);
    let doc = Size::new(DOC_W as f32, DOC_H as f32);
    if reset_every > 0 && frame > 0 && frame % reset_every == 0 {
        painter.clear_layer(INK_LAYER, doc, TargetFormat::Rgba8);
    }
    painter.persistent_group(INK_LAYER, doc, TargetFormat::Rgba8, |painter| {
        let mut damage: Option<Rect> = None;
        for (x, y, color) in dabs(frame) {
            let rect = dab_rect(x, y);
            damage = Some(damage.map_or(rect, |acc| acc.union(rect)));
            painter.shader(
                Dab {
                    color: sequoia_shader::Color::rgba(color.r, color.g, color.b, color.a),
                    hardness: 0.3,
                },
                rect,
                &[],
            );
        }
        if let Some(rect) = damage {
            painter.damage(rect);
        }
    });
    painter.blit(INK_LAYER, canvas, [0.0, 0.0, 1.0, 1.0]);
}

fn paint_cpu(painter: &mut Painter) {
    let canvas = painter.rect();
    painter.fill(canvas, Fill::solid(Color::WHITE));
    let frame = FRAME.load(Ordering::Relaxed);
    let mut guard = CPU_INK.lock().expect("ink buffer");
    let pixels = guard.get_or_insert_with(|| Pixels::filled(DOC_W, DOC_H, Color::WHITE));
    for (x, y, color) in dabs(frame) {
        stamp_cpu(pixels, x, y, DAB_RADIUS, color);
    }
    painter.pixels_nearest(canvas, pixels);
}

/// CPU twin of the [`Dab`] shader, with the same coverage curve.
fn stamp_cpu(pixels: &mut Pixels, cx: f32, cy: f32, radius: f32, color: Color) {
    let r = radius.max(0.5);
    let x0 = (cx - r).floor().max(0.0) as i32;
    let y0 = (cy - r).floor().max(0.0) as i32;
    let x1 = (cx + r).ceil().min(DOC_W as f32 - 1.0) as i32;
    let y1 = (cy + r).ceil().min(DOC_H as f32 - 1.0) as i32;
    for y in y0..=y1 {
        for x in x0..=x1 {
            let dx = x as f32 + 0.5 - cx;
            let dy = y as f32 + 0.5 - cy;
            let d = (dx * dx + dy * dy).sqrt() / r;
            let coverage = 1.0 - smoothstep(0.3, 1.0, d);
            if coverage > 0.0 {
                pixels.blend(x, y, color.with_alpha(color.a * coverage));
            }
        }
    }
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn ink_canvas(mode: Mode, reset_every: u32) -> Canvas {
    Canvas::new()
        .on_draw(move |painter: &mut Painter| match mode {
            Mode::Gpu => paint_gpu(painter, reset_every),
            Mode::Cpu => paint_cpu(painter),
        })
        .width(Length::Fill(1.0))
        .height(Length::Fill(1.0))
}

fn arg_value(args: &[String], flag: &str) -> Option<String> {
    let index = args.iter().position(|arg| arg == flag)?;
    args.get(index + 1).cloned()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let frames = arg_value(&args, "--frames")
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(240)
        .max(1);
    let mode = arg_value(&args, "--mode")
        .as_deref()
        .and_then(Mode::parse)
        .unwrap_or(Mode::Gpu);
    // `--reset-every N` clears the persistent layer every N frames, which
    // exercises the reset path instead of letting ink accumulate forever.
    let reset_every = arg_value(&args, "--reset-every")
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(0);

    println!(
        "[gpu-brush] mode={mode:?} frames={frames} dabs_per_frame={DABS_PER_FRAME} reset_every={reset_every}"
    );

    App::new()
        .title("sequoia gpu brush")
        .size(960.0, 660.0)
        .present_mode(PresentMode::Immediate)
        .shader_type::<Dab>()
        .root(
            Stack::new()
                .fill_size()
                .fill(ink_canvas(mode, reset_every))
                .child(Driver),
        )
        .benchmark(frames)
        .run()
        .expect("gpu_brush event loop failed");
}
