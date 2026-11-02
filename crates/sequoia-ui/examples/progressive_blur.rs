//! Progressive blur demo: one square, blur 0 px at the top, 400 px at the bottom.
//!
//! Build & run with:
//!     cargo run --example progressive_blur -p sequoia-ui
//!
//! The effect is a two-pass separable blur. `BlurX` samples the captured
//! content horizontally into a second target, `BlurY` samples that target
//! vertically and draws the result into the window. Both passes share the same
//! ramp, so the square is sharp along its top edge and reaches `MAX_BLUR` at its
//! bottom edge.
//!
//! Each pass takes `2 * TAPS + 1` jittered samples per pixel. Jittering the tap
//! positions with a per-pixel hash avoids the banding a regular tap grid would
//! show at a 400 px radius.

use sequoia_core::{BlendMode, Color as Rgba, Fill, GradientStops, Rect, TexInput};
use sequoia_shader::shader;
use sequoia_ui::prelude::*;
use sequoia_view::{Canvas, Painter};

/// Blur radius at the bottom edge of the square, in logical pixels.
const MAX_BLUR: f32 = 400.0;

/// Margin between the square and the window edge.
const SQUARE_INSET: f32 = 32.0;

shader! {
    pub fn blur_x(uv: FragUv, content: Sampler2D, max_blur: f32, from_y: f32, to_y: f32) -> Color {
        let span = max(to_y - from_y, 0.0001);
        let progress = clamp((uv.y - from_y) / span, 0.0, 1.0);
        let radius = max_blur * progress * progress;

        let taps = 24;
        let jitter = sq_hash21(uv * 977.0);
        var acc = vec4<f32>(0.0, 0.0, 0.0, 0.0);
        var total = 0.0;
        for (var i = -taps; i <= taps; i = i + 1) {
            let k = (f32(i) + jitter) / f32(taps);
            let w = 1.0 - abs(k);
            let offset = vec2<f32>(k * radius, 0.0) / screen.size.xy;
            let suv = clamp(uv + offset, vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 1.0));
            acc = acc + textureSample(content, content_sampler, suv) * w;
            total = total + w;
        }
        return acc / max(total, 0.0001);
    }
}

shader! {
    pub fn blur_y(uv: FragUv, content: Sampler2D, max_blur: f32, from_y: f32, to_y: f32) -> Color {
        let span = max(to_y - from_y, 0.0001);
        let progress = clamp((uv.y - from_y) / span, 0.0, 1.0);
        let radius = max_blur * progress * progress;

        let taps = 24;
        let jitter = sq_hash21(uv * 1237.0);
        var acc = vec4<f32>(0.0, 0.0, 0.0, 0.0);
        var total = 0.0;
        for (var i = -taps; i <= taps; i = i + 1) {
            let k = (f32(i) + jitter) / f32(taps);
            let w = 1.0 - abs(k);
            let offset = vec2<f32>(0.0, k * radius) / screen.size.xy;
            let suv = clamp(uv + offset, vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 1.0));
            acc = acc + textureSample(content, content_sampler, suv) * w;
            total = total + w;
        }
        return acc / max(total, 0.0001);
    }
}

fn demo_content(g: &mut Painter) {
    let canvas = g.rect();
    g.fill(canvas, Fill::solid(Rgba::rgba(0.05, 0.05, 0.08, 1.0)));

    // The one square the whole demo is about.
    let square = Rect::from_xywh(
        SQUARE_INSET,
        SQUARE_INSET,
        canvas.size.width - SQUARE_INSET * 2.0,
        canvas.size.height - SQUARE_INSET * 2.0,
    );
    g.fill(
        square,
        Fill::vertical(GradientStops::two(
            Rgba::rgba(0.30, 0.70, 1.00, 1.0),
            Rgba::rgba(0.92, 0.34, 0.58, 1.0),
        )),
    );

    // Vertical stripes make the blur radius readable: they stay crisp at the
    // top of the square and melt together at the bottom.
    let mut x = square.left() + 28.0;
    while x < square.right() - 12.0 {
        g.fill(
            Rect::from_xywh(x, square.top() + 18.0, 5.0, square.size.height - 36.0),
            Fill::solid(Rgba::rgba(1.0, 1.0, 1.0, 0.6)),
        );
        x += 44.0;
    }
}

fn draw(g: &mut Painter) {
    let rect = g.rect();
    let size = rect.size;
    if size.width <= 0.0 || size.height <= 0.0 {
        return;
    }

    // The ramp runs from the square's top edge (0 px) to its bottom edge
    // (`MAX_BLUR`), expressed in the 0..1 texture space the shader sees.
    let from_y = SQUARE_INSET / size.height;
    let to_y = (size.height - SQUARE_INSET) / size.height;

    // 1. Paint the square into a target.
    let (sharp, _) = g.group(size, BlendMode::Normal, demo_content);

    // 2. Horizontal pass into a second target.
    let (horizontal, _) = g.group(size, BlendMode::Normal, |inner| {
        inner.shader(
            BlurX {
                max_blur: MAX_BLUR,
                from_y,
                to_y,
            },
            rect,
            &[("content", TexInput::Target(sharp))],
        );
    });

    // 3. Vertical pass straight into the window.
    g.shader(
        BlurY {
            max_blur: MAX_BLUR,
            from_y,
            to_y,
        },
        rect,
        &[("content", TexInput::Target(horizontal))],
    );
}

fn main() {
    let view = Canvas::new().on_draw(draw);

    App::new()
        .title("Progressive blur 0 -> 400")
        .size(760.0, 460.0)
        .shader_type::<BlurX>()
        .shader_type::<BlurY>()
        .root(view)
        .run()
        .expect("event loop failed");
}
