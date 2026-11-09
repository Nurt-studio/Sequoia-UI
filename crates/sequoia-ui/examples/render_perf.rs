//! Renderer stress benchmark: opacity groups, backdrop blur, and shader draws.
//!
//! Run with:
//!
//! ```text
//! cargo run --release -p sequoia-ui --example render_perf -- --scene opacity --frames 240
//! ```
//!
//! Scenes isolate the renderer paths that are candidates for optimisation:
//!
//! * `quads`   - background quad grid only (baseline overhead),
//! * `opacity` - 16 opacity cards, nested up to four levels (opacity layers),
//! * `blur`    - 6 backdrop-blur panels with radii from 8 to 64 px,
//! * `shader`  - 256 instances of one user shader,
//! * `mixed`   - everything at once.
//!
//! The `[stress]` line printed by the platform benchmark reports the per-frame
//! renderer cost, draw calls, and passes. Save the output before and after a
//! renderer change to compare the percentage improvement:
//!
//! ```text
//! cargo run --release -p sequoia-ui --example render_perf -- --scene opacity --frames 240 > before.txt
//! # apply the optimisation
//! cargo run --release -p sequoia-ui --example render_perf -- --scene opacity --frames 240 > after.txt
//! ```

use std::sync::atomic::{AtomicU32, Ordering};

use sequoia_core::{Color, Fill, Length, Rect};
use sequoia_shader::shader;
use sequoia_ui::prelude::*;
use sequoia_ui::render::PresentMode;
use sequoia_view::{Canvas, Painter};

const CARDS: usize = 16;
const CARD_W: f32 = 250.0;
const CARD_H: f32 = 150.0;
const CARD_GAP: f32 = 24.0;
const CARD_COLS: usize = 4;

const BLUR_PANELS: usize = 6;
const BLUR_RADII: [f32; BLUR_PANELS] = [8.0, 16.0, 24.0, 32.0, 48.0, 64.0];
const BLUR_W: f32 = 260.0;
const BLUR_H: f32 = 150.0;

const SHADER_COLS: usize = 32;
const SHADER_ROWS: usize = 8;
const SHADER_W: f32 = 32.0;
const SHADER_H: f32 = 24.0;

static FRAME: AtomicU32 = AtomicU32::new(0);

shader! {
    pub fn tint(color: Color, gain: f32) -> Color {
        return vec4<f32>(color.rgb * gain, color.a);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SceneMode {
    Quads,
    Opacity,
    Blur,
    Shader,
    Mixed,
}

impl SceneMode {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "quads" => Some(Self::Quads),
            "opacity" => Some(Self::Opacity),
            "blur" => Some(Self::Blur),
            "shader" => Some(Self::Shader),
            "mixed" => Some(Self::Mixed),
            _ => None,
        }
    }

    fn has_opacity(self) -> bool {
        matches!(self, Self::Opacity | Self::Mixed)
    }

    fn has_blur(self) -> bool {
        matches!(self, Self::Blur | Self::Mixed)
    }

    fn has_shader(self) -> bool {
        matches!(self, Self::Shader | Self::Mixed)
    }
}

/// Advances the benchmark clock and keeps requesting frames so every sample is
/// recorded from a full, animated repaint.
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

/// One card or blur panel: a small stack of overlapping quads so the opacity
/// group has real content to composite.
#[derive(Clone)]
struct Block {
    w: f32,
    h: f32,
    seed: u32,
    layers: u32,
}

impl Block {
    fn new(w: f32, h: f32, seed: u32) -> Self {
        Self {
            w,
            h,
            seed,
            layers: 4,
        }
    }

    fn color(&self, index: u32) -> Color {
        let frame = FRAME.load(Ordering::Relaxed) as f32;
        let phase = self.seed as f32 * 0.37 + index as f32 * 0.71 + frame * 0.03;
        Color::rgba(
            0.35 + 0.45 * phase.sin(),
            0.35 + 0.45 * (phase * 1.3).cos(),
            0.35 + 0.45 * (phase * 0.7).sin(),
            0.55,
        )
    }
}

impl Leaf for Block {
    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle::default()
            .width(Length::Px(self.w))
            .height(Length::Px(self.h))
    }

    fn paint(&self, ctx: &mut PaintCtx) {
        let rect = ctx.rect;
        for index in 0..self.layers {
            let inset = 6.0 + index as f32 * 10.0;
            let layer = Rect::from_xywh(
                rect.origin.x + inset,
                rect.origin.y + inset,
                (rect.size.width - inset * 2.0).max(1.0),
                (rect.size.height - inset * 2.0).max(1.0),
            );
            ctx.quad(layer, self.color(index), 8.0 + index as f32 * 3.0);
        }
    }
}

impl_view_for_leaf!(Block);

fn card(index: usize) -> AnyView {
    let block = Block::new(CARD_W, CARD_H, index as u32);
    let levels = if index < 4 { 1 + index } else { 1 };
    let mut view = AnyView::new(block);
    for level in 0..levels {
        let opacity = 0.78 + level as f32 * 0.04;
        view = AnyView::new(Opacity::new(view, opacity));
    }
    view
}

fn blur_panel(index: usize) -> impl View {
    let block = Block::new(BLUR_W, BLUR_H, 100 + index as u32);
    BackdropBlur::blurred(block, BLUR_RADII[index])
}

fn card_position(index: usize) -> (f32, f32) {
    let col = index % CARD_COLS;
    let row = index / CARD_COLS;
    let x = 32.0 + col as f32 * (CARD_W + CARD_GAP);
    let y = 32.0 + row as f32 * (CARD_H + CARD_GAP);
    (x, y)
}

fn blur_position(index: usize) -> (f32, f32) {
    let col = index % 3;
    let row = index / 3;
    let x = 140.0 + col as f32 * 380.0;
    let y = 690.0 + row as f32 * 90.0;
    (x, y)
}

fn background_canvas() -> Canvas {
    Canvas::new()
        .on_draw(|painter: &mut Painter| {
            let canvas = painter.rect();
            painter.fill(canvas, Fill::solid(Color::rgb8(0x18, 0x20, 0x2c)));

            let cols = 24;
            let rows = 14;
            let cell_w = canvas.size.width / cols as f32;
            let cell_h = canvas.size.height / rows as f32;
            for row in 0..rows {
                for col in 0..cols {
                    let shade = ((row * cols + col) % 7) as f32 / 7.0;
                    let color =
                        Color::rgba(0.12 + shade * 0.2, 0.16 + shade * 0.25, 0.28, 1.0);
                    painter.fill(
                        Rect::from_xywh(
                            col as f32 * cell_w,
                            row as f32 * cell_h,
                            cell_w + 1.0,
                            cell_h + 1.0,
                        ),
                        Fill::solid(color),
                    );
                }
            }
        })
        .width(Length::Fill(1.0))
        .height(Length::Fill(1.0))
}

fn shader_canvas() -> Canvas {
    Canvas::new()
        .on_draw(|painter: &mut Painter| {
            let frame = FRAME.load(Ordering::Relaxed) as f32;
            for row in 0..SHADER_ROWS {
                for col in 0..SHADER_COLS {
                    let index = row * SHADER_COLS + col;
                    let phase = index as f32 * 0.11 + frame * 0.04;
                    let color = sequoia_shader::Color::rgba(
                        0.4 + 0.5 * phase.sin(),
                        0.4 + 0.5 * (phase * 1.2).cos(),
                        0.4 + 0.5 * (phase * 0.8).sin(),
                        0.8,
                    );
                    painter.shader(
                        Tint { color, gain: 1.0 },
                        Rect::from_xywh(
                            28.0 + col as f32 * (SHADER_W + 4.0),
                            28.0 + row as f32 * (SHADER_H + 4.0),
                            SHADER_W,
                            SHADER_H,
                        ),
                        &[],
                    );
                }
            }
        })
        .width(Length::Fill(1.0))
        .height(Length::Fill(1.0))
}

fn scene(mode: SceneMode) -> impl View {
    let mut stack = Stack::new().fill_size().fill(background_canvas());

    if mode.has_blur() {
        for index in 0..BLUR_PANELS {
            let (x, y) = blur_position(index);
            stack = stack.child_at(blur_panel(index), StackPos::top_left().offset(x, y));
        }
    }

    if mode.has_opacity() {
        for index in 0..CARDS {
            let (x, y) = card_position(index);
            stack = stack.child_at(card(index), StackPos::top_left().offset(x, y));
        }
    }

    if mode.has_shader() {
        stack = stack.fill(shader_canvas());
    }

    stack.child(Driver)
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
    let mode = arg_value(&args, "--scene")
        .as_deref()
        .and_then(SceneMode::parse)
        .unwrap_or(SceneMode::Mixed);

    println!("[perf] scene={mode:?} frames={frames}");

    App::new()
        .title("sequoia render perf")
        .size(1440.0, 900.0)
        .present_mode(PresentMode::Immediate)
        .shader_type::<Tint>()
        .root(scene(mode))
        .benchmark(frames)
        .run()
        .expect("render_perf event loop failed");
}
