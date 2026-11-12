//! 20 translucent gradient layers composited into an 8-bit and a 16-bit float
//! intermediate, then tone-mapped side by side.
//!
//! In the 8-bit target every blend step rounds to 1/255, so the shallow ramps
//! develop visible steps. The `Rgba16Float` target keeps the intermediate values
//! at full precision and only quantizes once, in the final present pass.
//!
//! Build & run with:
//!     cargo run --example hdr_layers -p sequoia-ui

use sequoia_core::{BlendMode, Color, Fill, GradientStops, Rect, TargetFormat, TexInput};
use sequoia_shader::shader;
use sequoia_ui::prelude::*;
use sequoia_view::{Canvas, Painter};

/// Number of translucent gradients in the document.
const LAYERS: usize = 20;

shader! {
    /// ACES-style filmic curve applied on the final present pass.
    pub fn tone_map(uv: FragUv, content: Sampler2D, exposure: f32) -> Color {
        let hdr = textureSample(content, content_sampler, uv);
        let x = max(hdr.rgb * exposure, vec3<f32>(0.0));
        let a = x * (2.51 * x + vec3<f32>(0.03));
        let b = x * (2.43 * x + vec3<f32>(0.59)) + vec3<f32>(0.14);
        let mapped = clamp(a / max(b, vec3<f32>(0.0001)), vec3<f32>(0.0), vec3<f32>(1.0));
        return vec4<f32>(mapped, hdr.a);
    }
}

/// Paints the same 20-layer gradient document into whichever target `g` is in.
fn paint_layers(g: &mut Painter, canvas: Rect) {
    g.fill(canvas, Fill::solid(Color::rgba(0.012, 0.014, 0.022, 1.0)));

    for i in 0..LAYERS {
        let t = i as f32 / (LAYERS - 1) as f32;
        let shift = t * 0.08;
        // One shallow horizontal ramp. Every layer adds at most 0.11 alpha, so an
        // 8-bit intermediate rounds the result at each of the 20 blend steps.
        let stops = GradientStops::two(
            Color::rgba(0.04 + shift, 0.05 + shift, 0.09 + shift, 0.0),
            Color::rgba(0.52 + shift, 0.60 + shift * 0.5, 0.94, 0.11),
        );
        g.fill(canvas, Fill::linear(stops, 0.0));
    }
}

fn draw(g: &mut Painter) {
    let canvas = g.rect();
    if canvas.is_empty() {
        return;
    }

    let (ldr, _) = g.group_with_format(
        canvas.size,
        BlendMode::Normal,
        TargetFormat::Rgba8,
        |inner| paint_layers(inner, canvas),
    );
    let (hdr, _) = g.group_with_format(
        canvas.size,
        BlendMode::Normal,
        TargetFormat::Rgba16Float,
        |inner| paint_layers(inner, canvas),
    );

    let half = canvas.size.width * 0.5;
    let left = Rect::from_xywh(0.0, 0.0, half, canvas.size.height);
    let right = Rect::from_xywh(half, 0.0, half, canvas.size.height);

    // Same tone-map shader for both halves, so the only difference is the
    // precision of the intermediate they sample.
    let tone = || ToneMap { exposure: 1.6 };
    g.shader_at(
        tone(),
        left,
        [0.0, 0.0, 1.0, 1.0],
        &[("content", TexInput::Target(ldr))],
    );
    g.shader_at(
        tone(),
        right,
        [0.0, 0.0, 1.0, 1.0],
        &[("content", TexInput::Target(hdr))],
    );

    let label_h = 38.0;
    g.fill(
        Rect::from_xywh(
            0.0,
            canvas.size.height - label_h,
            canvas.size.width,
            label_h,
        ),
        Fill::solid(Color::rgba(0.0, 0.0, 0.0, 0.55)),
    );
    g.text_center(
        Rect::from_xywh(0.0, canvas.size.height - label_h, half, label_h),
        "RGBA8 intermediate",
        15.0,
        Color::rgba(0.95, 0.56, 0.46, 1.0),
    );
    g.text_center(
        Rect::from_xywh(half, canvas.size.height - label_h, half, label_h),
        "RGBA16F intermediate",
        15.0,
        Color::rgba(0.55, 0.86, 0.70, 1.0),
    );
    g.text_center(
        Rect::from_xywh(0.0, 12.0, canvas.size.width, 30.0),
        "20 gradient layers, one composite",
        17.0,
        Color::rgba(0.92, 0.93, 0.96, 1.0),
    );
}

fn main() {
    let view = Canvas::new().on_draw(draw);

    App::new()
        .title("Sequoia 16-bit intermediates")
        .size(1000.0, 640.0)
        .shader_type::<ToneMap>()
        .root(view)
        .run()
        .expect("event loop failed");
}
