//! A 3D LUT pass: the photo is graded through a `.cube` table sampled by a
//! `Sampler3D`.
//!
//! The left half is the untouched photo; the right half samples the same pixels
//! through `film_lut`, which looks each colour up in a 33³ teal-orange table.
//! The LUT is bound as `TexInput::Image(ImageSource::cube(..))`, so the renderer
//! uploads it as a `texture_3d` and samples it with trilinear filtering.
//!
//! Run the window preview:
//!     cargo run --example lut_photo -p sequoia-ui
//!
//! Export the same scene headlessly:
//!     cargo run --example lut_photo -p sequoia-ui -- --export lut-photo.png
//!
//! Painting: "The Starry Night" by Vincent van Gogh, public domain,
//! via Wikimedia Commons.

use sequoia_core::{Color, Fill, GradientStops, ImageSource, Point, Rect, TexInput};
use sequoia_shader::{SequoiShader, shader};
use sequoia_ui::export::{CpuShader, ExportOptions, Exporter, ShaderCtx};
use sequoia_ui::prelude::*;
use sequoia_ui::render::lut::{Lut3D, load_cube};
use sequoia_view::{Canvas, Painter};

const PHOTO: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/examples/assets/starry-night.jpg"
);
const LUT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/examples/assets/luts/teal-orange.cube"
);

shader! {
    /// Looks the source colour up in a 3D LUT and keeps the photo's alpha.
    pub fn film_lut(uv: FragUv, content: Sampler2D, lut: Sampler3D) -> Color {
        let src = textureSample(content, content_sampler, uv);
        let coord = clamp(src.rgb, vec3<f32>(0.0), vec3<f32>(1.0));
        let mapped = textureSample(lut, lut_sampler, coord);
        return vec4<f32>(mapped.rgb, src.a);
    }
}

/// CPU counterpart used by `--export`. The GPU path samples the uploaded
/// `texture_3d`; the exporter samples the same table in memory with the same
/// trilinear interpolation, so both outputs match.
struct FilmLutCpu {
    lut: Lut3D,
}

impl FilmLutCpu {
    fn sample(&self, rgb: [f32; 3]) -> [f32; 3] {
        let n = self.lut.size.max(2) as i32;
        let last = (n - 1) as f32;
        let p = rgb.map(|v| v.clamp(0.0, 1.0) * last);
        let lo = p.map(|v| v.floor() as i32);
        let hi = [
            (lo[0] + 1).min(n - 1),
            (lo[1] + 1).min(n - 1),
            (lo[2] + 1).min(n - 1),
        ];
        let f = [p[0] - lo[0] as f32, p[1] - lo[1] as f32, p[2] - lo[2] as f32];

        let texel = |r: i32, g: i32, b: i32| {
            let i = (((b * n + g) * n + r) * 4) as usize;
            [self.lut.data[i], self.lut.data[i + 1], self.lut.data[i + 2]]
        };
        let lerp = |a: [f32; 3], b: [f32; 3], t: f32| {
            [
                a[0] + (b[0] - a[0]) * t,
                a[1] + (b[1] - a[1]) * t,
                a[2] + (b[2] - a[2]) * t,
            ]
        };

        let c00 = lerp(
            texel(lo[0], lo[1], lo[2]),
            texel(hi[0], lo[1], lo[2]),
            f[0],
        );
        let c10 = lerp(
            texel(lo[0], hi[1], lo[2]),
            texel(hi[0], hi[1], lo[2]),
            f[0],
        );
        let c01 = lerp(
            texel(lo[0], lo[1], hi[2]),
            texel(hi[0], lo[1], hi[2]),
            f[0],
        );
        let c11 = lerp(
            texel(lo[0], hi[1], hi[2]),
            texel(hi[0], hi[1], hi[2]),
            f[0],
        );
        lerp(lerp(c00, c10, f[1]), lerp(c01, c11, f[1]), f[2])
    }
}

impl CpuShader for FilmLutCpu {
    fn name(&self) -> &'static str {
        <FilmLut as SequoiShader>::NAME
    }

    fn shade(&self, ctx: &ShaderCtx<'_>) -> [f32; 4] {
        let src = ctx.sample(0, ctx.uv);
        let rgb = self.sample([src[0], src[1], src[2]]);
        [rgb[0], rgb[1], rgb[2], src[3]]
    }
}

fn draw(g: &mut Painter, photo: &ImageSource, lut_path: &str) {
    let canvas = g.rect();
    if canvas.is_empty() {
        return;
    }

    g.image(canvas, [0.0, 0.0, 1.0, 1.0], photo.clone());

    let half = canvas.size.width * 0.5;
    let right = Rect::from_xywh(half, 0.0, half, canvas.size.height);
    g.shader_at(
        FilmLut,
        right,
        [0.5, 0.0, 1.0, 1.0],
        &[
            ("content", TexInput::Image(photo.clone())),
            (
                "lut",
                TexInput::Image(ImageSource::cube(lut_path.to_string())),
            ),
        ],
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
        "film_lut",
        Point::new(pad, 26.0),
        26.0,
        Color::rgba(0.96, 0.97, 1.0, 1.0),
    );
    g.text_tl(
        "Sampler3D · 33³ .cube table · trilinear lookup",
        Point::new(pad, 64.0),
        15.0,
        Color::rgba(0.82, 0.86, 0.93, 0.82),
    );

    let table = "teal-orange.cube";
    let table_w = g.text_width(table, 14.0);
    g.text_tl(
        table,
        Point::new(canvas.size.width - pad - table_w, 34.0),
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
    let graded = "TEAL_ORANGE";
    let graded_w = g.text_width(graded, 13.0);
    g.text_tl(
        graded,
        Point::new(canvas.size.width - pad - graded_w, label_y),
        13.0,
        Color::rgba(0.99, 0.76, 0.52, 0.95),
    );
}

fn main() {
    let photo = load_pixels(PHOTO)
        .expect("load examples/assets/starry-night.jpg")
        .to_source();
    let lut = load_cube(LUT).expect("load examples/assets/luts/teal-orange.cube");

    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("--export") {
        let out = args
            .get(2)
            .cloned()
            .unwrap_or_else(|| "lut-photo.png".to_string());
        let width = args
            .get(3)
            .and_then(|value| value.parse::<f32>().ok())
            .unwrap_or(1280.0);
        let height = args
            .get(4)
            .and_then(|value| value.parse::<f32>().ok())
            .unwrap_or(800.0);
        let opts = ExportOptions::new(width, height).scale(1.0);
        let source = photo.clone();
        let path = LUT.to_string();
        Exporter::new()
            .shader(FilmLutCpu { lut })
            .export_png(
                Canvas::new().on_draw(move |g| draw(g, &source, &path)),
                &opts,
                &out,
            )
            .expect("export png");
        println!("wrote {out}");
        return;
    }

    let path = LUT.to_string();
    App::new()
        .title("Sequoia 3D LUT")
        .size(1280.0, 800.0)
        .shader_type::<FilmLut>()
        .root(Canvas::new().on_draw(move |g| draw(g, &photo, &path)))
        .run()
        .expect("event loop failed");
}
