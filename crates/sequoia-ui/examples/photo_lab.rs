//! An interactive photo lab: a 3D LUT and live adjustments over a photo.
//!
//! Drag the sliders to change the LUT strength, contrast, saturation, exposure
//! and warmth; pick one of two `.cube` tables. The preview redraws through
//! `photo_lab`, which samples the photo (`Sampler2D`) and the LUT
//! (`Sampler3D`) and mixes the graded result over the adjusted source.
//!
//! Run it:
//!     cargo run --example photo_lab -p sequoia-ui
//!
//! Painting: "The Starry Night" by Vincent van Gogh, public domain,
//! via Wikimedia Commons.

use sequoia_core::{Color, Fill, ImageSource, Rect, TexInput};
use sequoia_shader::shader;
use sequoia_ui::prelude::*;
use sequoia_view::{Canvas, Painter};

const PHOTO: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/examples/assets/starry-night.jpg"
);
const LUT_TEAL: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/examples/assets/luts/teal-orange.cube"
);
const LUT_BLEACH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/examples/assets/luts/bleach-bypass.cube"
);

const APP_BG: Color = Color::hex(0x14161B);
const PANEL_BG: Color = Color::hex(0x1D2027);
const TEXT: Color = Color::hex(0xF2F4F8);
const MUTED: Color = Color::hex(0x9299A8);
const ACCENT: Color = Color::hex(0xE8A34D);

shader! {
    /// Applies exposure, contrast, saturation and warmth, then mixes a 3D LUT
    /// over the result by `strength`.
    pub fn photo_lab(
        uv: FragUv,
        content: Sampler2D,
        lut: Sampler3D,
        strength: f32,
        contrast: f32,
        saturation: f32,
        exposure: f32,
        warmth: f32,
    ) -> Color {
        let src = textureSample(content, content_sampler, uv);
        let exposed = clamp(src.rgb * exposure, vec3<f32>(0.0), vec3<f32>(1.0));
        let curved = (exposed - vec3<f32>(0.5)) * (1.0 + contrast) + vec3<f32>(0.5);
        let luma = dot(curved, vec3<f32>(0.2126, 0.7152, 0.0722));
        let saturated = mix(vec3<f32>(luma), curved, 1.0 + saturation);
        let warm = saturated * vec3<f32>(1.0 + warmth * 0.10, 1.0, 1.0 - warmth * 0.10);
        let coord = clamp(warm, vec3<f32>(0.0), vec3<f32>(1.0));

        let graded = textureSample(lut, lut_sampler, coord);
        let mixed = mix(coord, graded.rgb, clamp(strength, 0.0, 1.0));
        return vec4<f32>(mixed, src.a);
    }
}

struct Lab {
    strength: Signal<f32>,
    contrast: Signal<f32>,
    saturation: Signal<f32>,
    exposure: Signal<f32>,
    warmth: Signal<f32>,
    preset: Signal<usize>,
}

impl Lab {
    fn new() -> Self {
        Self {
            strength: Signal::new(1.0),
            contrast: Signal::new(0.0),
            saturation: Signal::new(0.0),
            exposure: Signal::new(1.0),
            warmth: Signal::new(0.0),
            preset: Signal::new(0),
        }
    }

}

fn preview(lab: &Lab, photo: &ImageSource, aspect: (f32, f32)) -> impl Fn(&mut Painter) + 'static {
    let photo = photo.clone();
    let (strength, contrast, saturation) = (
        lab.strength.clone(),
        lab.contrast.clone(),
        lab.saturation.clone(),
    );
    let (exposure, warmth, preset) = (
        lab.exposure.clone(),
        lab.warmth.clone(),
        lab.preset.clone(),
    );

    move |g: &mut Painter| {
        let area = g.rect();
        if area.is_empty() {
            return;
        }
        g.fill(area, Fill::solid(APP_BG));

        // Contain the painting inside the preview so it keeps its aspect.
        let (aw, ah) = aspect;
        let scale = (area.size.width / aw).min(area.size.height / ah);
        let w = aw * scale;
        let h = ah * scale;
        let dst = Rect::from_xywh(
            area.left() + (area.size.width - w) * 0.5,
            area.top() + (area.size.height - h) * 0.5,
            w,
            h,
        );

        let lut = if preset.get() == 1 { LUT_BLEACH } else { LUT_TEAL };
        g.shader_at(
            PhotoLab {
                strength: strength.get(),
                contrast: contrast.get(),
                saturation: saturation.get(),
                exposure: exposure.get(),
                warmth: warmth.get(),
            },
            dst,
            [0.0, 0.0, 1.0, 1.0],
            &[
                ("content", TexInput::Image(photo.clone())),
                (
                    "lut",
                    TexInput::Image(ImageSource::cube(lut.to_string())),
                ),
            ],
        );
    }
}

fn slider_row(label: &'static str, value: Signal<f32>, min: f32, max: f32) -> impl View + 'static {
    let read = value.clone();
    Row::new()
        .fill_width()
        .gap(12.0)
        .child(
            Text::new(label)
                .size(12.0)
                .color(MUTED)
                .width(78.0),
        )
        .child(Slider::new(value).min(min).max(max))
        .child(
            Text::with(move || format!("{:+.2}", read.get()))
                .size(12.0)
                .color(ACCENT)
                .width(46.0),
        )
}

fn main() {
    let pixels = load_pixels(PHOTO).expect("load examples/assets/starry-night.jpg");
    let aspect = (pixels.width() as f32, pixels.height() as f32);
    let photo = pixels.to_source();

    let lab = Lab::new();
    let reset_strength = lab.strength.clone();
    let reset_contrast = lab.contrast.clone();
    let reset_saturation = lab.saturation.clone();
    let reset_exposure = lab.exposure.clone();
    let reset_warmth = lab.warmth.clone();

    let controls = Container::new()
        .width(320.0)
        .padding(22.0)
        .background(PANEL_BG)
        .radius(16.0)
        .child(
            Column::new()
                .fill_width()
                .gap(14.0)
                .child(Text::new("Photo Lab").size(22.0).bold().color(TEXT))
                .child(
                    Text::new("3D LUT + live adjustments")
                        .size(13.0)
                        .color(MUTED),
                )
                .child(
                    RadioGroup::new(lab.preset.clone(), ["Teal Orange", "Bleach Bypass"])
                        .label("Look"),
                )
                .child(slider_row(
                    "LUT mix",
                    lab.strength.clone(),
                    0.0,
                    1.0,
                ))
                .child(slider_row(
                    "Contrast",
                    lab.contrast.clone(),
                    -0.6,
                    0.8,
                ))
                .child(slider_row(
                    "Saturation",
                    lab.saturation.clone(),
                    -0.8,
                    0.8,
                ))
                .child(slider_row(
                    "Exposure",
                    lab.exposure.clone(),
                    0.4,
                    1.8,
                ))
                .child(slider_row("Warmth", lab.warmth.clone(), -1.0, 1.0))
                .child(
                    Button::new("Reset")
                        .style(ButtonStyle::SECONDARY)
                        .on_click(move || {
                            reset_strength.set(1.0);
                            reset_contrast.set(0.0);
                            reset_saturation.set(0.0);
                            reset_exposure.set(1.0);
                            reset_warmth.set(0.0);
                        }),
                )
                .child(
                    Text::new("Pick a .cube table, then drag the sliders.")
                        .size(12.0)
                        .color(MUTED),
                ),
        );

    App::new()
        .title("Sequoia Photo Lab")
        .size(1280.0, 840.0)
        .shader_type::<PhotoLab>()
        .root(
            Stack::new()
                .fill_size()
                .background(APP_BG)
                .padding(20.0)
                .child(
                    Row::new()
                        .fill_size()
                        .gap(20.0)
                        .child(Canvas::new().flex(1.0).on_draw(preview(&lab, &photo, aspect)))
                        .child(controls),
                ),
        )
        .run()
        .expect("event loop failed");
}
