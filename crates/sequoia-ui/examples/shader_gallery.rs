//! Simple progressive blur: #1C1C1E background, #D9D9D9 card melting into it.
//! No hard clipping at the bottom -> smooth fade out. No light streaks.
//!
//! Build & run with:
//!     cargo run --example simple_blur -p sequoia-ui

use sequoia_core::{BlendMode, Color as Rgba, Fill, Rect, TexInput};
use sequoia_shader::shader;
use sequoia_ui::prelude::*;
use sequoia_view::{Canvas, Painter};

/// Blur radius at the bottom edge of the card.
const MAX_BLUR: f32 = 400.0;
/// Corner radius of the card.
const RADIUS: f32 = 40.0;
/// Vertical gap between the title and the card.
const GAP: f32 = 24.0;
/// Title text height.
const TITLE_HEIGHT: f32 = 64.0;
/// Margin from window edges.
const MARGIN: f32 = 64.0;

shader! {
    pub fn blur_x(uv: FragUv, content: Sampler2D, max_blur: f32, from_y: f32, to_y: f32) -> Color {
        let span = max(to_y - from_y, 0.0001);
        // Clamp progress to 1.0 so blur doesn't keep growing below the card
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
        // Clamp progress to 1.0 so blur doesn't keep growing below the card
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

fn draw(g: &mut Painter) {
    let canvas = g.rect();
    let size = canvas.size;
    if size.width <= 0.0 || size.height <= 0.0 {
        return;
    }

    // 1. Background: #1C1C1E (Dark Gray)
    g.fill(canvas, Fill::solid(Rgba::rgba(0.110, 0.110, 0.118, 1.0)));

    // 2. Title text on top
    let title_rect = Rect::from_xywh(0.0, MARGIN, size.width, TITLE_HEIGHT);
    g.text_center(
        title_rect,
        "Sequoia Shaders",
        32.0,
        Rgba::rgba(0.95, 0.95, 0.96, 1.0),
    );

    // 3. Card rectangle: starts 24px below the title text
    let card_top = MARGIN + TITLE_HEIGHT + GAP;
    let card_w = size.width * 0.5;
    let card_h = card_w; // Square aspect ratio
    let card = Rect::from_xywh((size.width - card_w) * 0.5, card_top, card_w, card_h);

    // 4. Paint the card content (#D9D9D9) into an offscreen target
    // We use clipped_rounded HERE so the source shape has nice corners.
    let (sharp, _) = g.group(size, BlendMode::Normal, |inner| {
        inner.clipped_rounded(card, RADIUS, |c| {
            c.fill(card, Fill::solid(Rgba::rgba(0.851, 0.851, 0.851, 1.0)));
        });
    });

    // Calculate global UV coordinates for the top and bottom of the card.
    // This ensures the blur ramp stops exactly at the bottom edge of the card.
    let from_y = card_top / size.height;
    let to_y = (card_top + card_h) / size.height;

    // 5. Horizontal blur pass (into another target)
    // Drawn on full canvas so it can bleed outwards.
    let (horizontal, _) = g.group(size, BlendMode::Normal, |inner| {
        inner.shader(
            BlurX {
                max_blur: MAX_BLUR,
                from_y,
                to_y,
            },
            canvas,
            &[("content", TexInput::Target(sharp))],
        );
    });

    // 6. Vertical blur pass (straight into the window)
    // NO clipped_rounded here! Let the blur fade smoothly into the background.
    // Because `progress` is clamped to 1.0 in the shader, the blur radius
    // stays constant below the card, preventing the infinite "light streak".
    g.shader(
        BlurY {
            max_blur: MAX_BLUR,
            from_y,
            to_y,
        },
        canvas,
        &[("content", TexInput::Target(horizontal))],
    );
}

fn main() {
    let view = Canvas::new().on_draw(draw);

    App::new()
        .title("Sequoia Shaders")
        .size(800.0, 800.0)
        .shader_type::<BlurX>()
        .shader_type::<BlurY>()
        .root(view)
        .run()
        .expect("event loop failed");
}
