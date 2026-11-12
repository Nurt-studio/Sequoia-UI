//! A photo pass: filmic S-curve, shadow lift, saturation and warmth.
//!
//! The left half is the untouched photo; the right half samples the same pixels
//! through `photo_curve`. The shader is registered with `App::shader_type`, so
//! the window and the headless exporter use the same uniforms.
//!
//! Run the window preview:
//!     cargo run --example photo_shader -p sequoia-ui
//!
//! Export the same scene headlessly:
//!     cargo run --example photo_shader -p sequoia-ui -- --export photo-shader.png
//!
//! Photo: "Mountain lake" by Aleksandra Boguslawska, CC0 (Unsplash).

use sequoia_core::{Color, Fill, GradientStops, ImageSource, Point, Rect, TexInput};
use sequoia_shader::{SequoiShader, shader};
use sequoia_ui::export::{CpuShader, ExportOptions, Exporter, ShaderCtx};
use sequoia_ui::prelude::*;
use sequoia_view::{Canvas, Painter};

const PHOTO: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/examples/assets/mountain-lake.jpg"
);

const CONTRAST: f32 = 0.78;
const LIFT: f32 = 0.035;
const SATURATION: f32 = 1.15;
const WARMTH: f32 = 0.18;

shader! {
    /// Photographic contrast curve: pulls tones apart around mid-grey, lifts
    /// the blacks slightly, then restores some colour and a warm highlight.
    pub fn photo_curve(
        uv: FragUv,
        content: Sampler2D,
        contrast: f32,
        lift: f32,
        saturation: f32,
        warmth: f32,
    ) -> Color {
        let src = textureSample(content, content_sampler, uv);
        let x = clamp(src.rgb, vec3<f32>(0.0), vec3<f32>(1.0));

        // Smoothstep is an S-curve: shadows down, highlights up.
        let s = x * x * (vec3<f32>(3.0) - 2.0 * x);
        let curved = x + (s - x) * contrast;
        let lifted = vec3<f32>(lift) + curved * (1.0 - lift);

        let luma = dot(lifted, vec3<f32>(0.2126, 0.7152, 0.0722));
        let saturated = mix(vec3<f32>(luma), lifted, saturation);
        let tint = vec3<f32>(1.0 + warmth * 0.08, 1.0, 1.0 - warmth * 0.08);

        return vec4<f32>(
            clamp(saturated * tint, vec3<f32>(0.0), vec3<f32>(1.0)),
            src.a,
        );
    }
}

/// CPU counterpart used by `--export`.
impl CpuShader for PhotoCurve {
    fn name(&self) -> &'static str {
        <PhotoCurve as SequoiShader>::NAME
    }

    fn shade(&self, ctx: &ShaderCtx<'_>) -> [f32; 4] {
        let src = ctx.sample(0, ctx.uv);
        let x = [
            src[0].clamp(0.0, 1.0),
            src[1].clamp(0.0, 1.0),
            src[2].clamp(0.0, 1.0),
        ];

        let mut lifted = [0.0; 3];
        for i in 0..3 {
            let s = x[i] * x[i] * (3.0 - 2.0 * x[i]);
            let curved = x[i] + (s - x[i]) * self.contrast;
            lifted[i] = self.lift + curved * (1.0 - self.lift);
        }

        let luma = lifted[0] * 0.2126 + lifted[1] * 0.7152 + lifted[2] * 0.0722;
        let tint = [1.0 + self.warmth * 0.08, 1.0, 1.0 - self.warmth * 0.08];

        [
            ((luma + (lifted[0] - luma) * self.saturation) * tint[0]).clamp(0.0, 1.0),
            ((luma + (lifted[1] - luma) * self.saturation) * tint[1]).clamp(0.0, 1.0),
            ((luma + (lifted[2] - luma) * self.saturation) * tint[2]).clamp(0.0, 1.0),
            src[3],
        ]
    }
}

fn curve() -> PhotoCurve {
    PhotoCurve {
        contrast: CONTRAST,
        lift: LIFT,
        saturation: SATURATION,
        warmth: WARMTH,
    }
}

fn draw(g: &mut Painter, photo: &ImageSource) {
    let canvas = g.rect();
    if canvas.is_empty() {
        return;
    }

    g.image(canvas, [0.0, 0.0, 1.0, 1.0], photo.clone());

    let half = canvas.size.width * 0.5;
    let right = Rect::from_xywh(half, 0.0, half, canvas.size.height);
    g.shader_at(
        curve(),
        right,
        [0.5, 0.0, 1.0, 1.0],
        &[("content", TexInput::Image(photo.clone()))],
    );

    g.fill(
        Rect::from_xywh(half - 1.0, 0.0, 2.0, canvas.size.height),
        Fill::solid(Color::rgba(1.0, 1.0, 1.0, 0.75)),
    );

    // Scrims keep the labels readable over a bright sky and a dark foreground.
    g.fill(
        Rect::from_xywh(0.0, 0.0, canvas.size.width, 140.0),
        Fill::vertical(GradientStops::two(
            Color::rgba(0.02, 0.03, 0.05, 0.72),
            Color::rgba(0.02, 0.03, 0.05, 0.0),
        )),
    );
    let scrim = 120.0;
    g.fill(
        Rect::from_xywh(0.0, canvas.size.height - scrim, canvas.size.width, scrim),
        Fill::vertical(GradientStops::two(
            Color::rgba(0.02, 0.03, 0.05, 0.0),
            Color::rgba(0.02, 0.03, 0.05, 0.78),
        )),
    );

    let pad = 40.0;
    g.text_tl(
        "photo_curve",
        Point::new(pad, 26.0),
        26.0,
        Color::rgba(0.96, 0.97, 1.0, 1.0),
    );
    g.text_tl(
        "S-curve contrast · shadow lift · saturation",
        Point::new(pad, 64.0),
        15.0,
        Color::rgba(0.82, 0.86, 0.93, 0.82),
    );

    let params = format!(
        "contrast {CONTRAST:.2}   lift {LIFT:.3}   saturation {SATURATION:.2}   warmth {WARMTH:.2}"
    );
    let params_w = g.text_width(&params, 14.0);
    g.text_tl(
        &params,
        Point::new(canvas.size.width - pad - params_w, 34.0),
        14.0,
        Color::rgba(0.82, 0.86, 0.93, 0.72),
    );

    let label_y = canvas.size.height - 46.0;
    g.text_tl(
        "ORIGINAL",
        Point::new(pad, label_y),
        13.0,
        Color::rgba(0.92, 0.94, 0.99, 0.9),
    );
    let corrected = "PHOTO_CURVE";
    let corrected_w = g.text_width(corrected, 13.0);
    g.text_tl(
        corrected,
        Point::new(canvas.size.width - pad - corrected_w, label_y),
        13.0,
        Color::rgba(0.58, 0.92, 0.70, 0.95),
    );
}

fn main() {
    let photo = load_pixels(PHOTO)
        .expect("load examples/assets/mountain-lake.jpg")
        .to_source();

    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("--export") {
        let out = args
            .get(2)
            .cloned()
            .unwrap_or_else(|| "photo-shader.png".to_string());
        let opts = ExportOptions::new(1280.0, 800.0).scale(1.0);
        let source = photo.clone();
        Exporter::new()
            .shader(curve())
            .export_png(
                Canvas::new().on_draw(move |g| draw(g, &source)),
                &opts,
                &out,
            )
            .expect("export png");
        println!("wrote {out}");
        return;
    }

    App::new()
        .title("Sequoia photo shader")
        .size(1280.0, 800.0)
        .shader_type::<PhotoCurve>()
        .root(Canvas::new().on_draw(move |g| draw(g, &photo)))
        .run()
        .expect("event loop failed");
}
