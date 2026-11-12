//! A simple canvas scene with a glass edge over a moving sphere, exported to GIF.
//!
//! Run with:
//!     cargo run --release -p sequoia-export --example glass_sphere -- glass_sphere.gif
//!
//! Debug builds are roughly 10x slower here: each frame is rasterized and
//! shader-shaded on the CPU, then quantized to a GIF palette.

use sequoia_core::{BlendMode, BrushStroke, Color, Fill, GradientStops, Point, TexInput};
use sequoia_export::{AnimationOptions, CpuShader, ExportOptions, Exporter, SCALE_2X, ShaderCtx};
use sequoia_shader::{SequoiShader, shader};
use sequoia_view::{Canvas, Painter};

shader! {
    pub fn glass(uv: FragUv, content: Sampler2D, strength: f32, thickness: f32) -> Color {
        let base = textureSample(content, content_sampler, uv);

        // Glass thickness at the right edge: content is bent inward, then a warm
        // rim brightens the seam.
        let edge = 1.0 - uv.x;
        let band = 1.0 - sq_smoothstep(0.0, thickness, edge);
        let bend = strength * band * band;
        let refracted = textureSample(
            content,
            content_sampler,
            clamp(uv - vec2<f32>(bend, 0.0), vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 1.0)),
        );
        let mixed = sq_mix3(base.rgb, refracted.rgb, 0.65);
        let rim = band * band * band;
        let glow = vec3<f32>(1.0, 0.90, 0.78) * (rim * 0.7);
        return vec4<f32>(mixed + glow, base.a);
    }
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0).max(0.000_001)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// CPU counterpart of the WGSL `glass` shader above.
impl CpuShader for Glass {
    fn name(&self) -> &'static str {
        <Glass as SequoiShader>::NAME
    }

    fn shade(&self, ctx: &ShaderCtx<'_>) -> [f32; 4] {
        let base = ctx.sample(0, ctx.uv);
        let edge = 1.0 - ctx.uv[0];
        let band = 1.0 - smoothstep(0.0, self.thickness, edge);
        let bend = self.strength * band * band;
        let refracted = ctx.sample(0, [(ctx.uv[0] - bend).clamp(0.0, 1.0), ctx.uv[1]]);
        let rim = band * band * band;
        let glow = rim * 0.7;
        let mixed = |a: f32, b: f32| a + (b - a) * 0.65;
        [
            (mixed(base[0], refracted[0]) + glow).min(1.0),
            (mixed(base[1], refracted[1]) + glow * 0.90).min(1.0),
            (mixed(base[2], refracted[2]) + glow * 0.78).min(1.0),
            base[3],
        ]
    }
}

fn glass_scene(g: &mut Painter) {
    let rect = g.rect();
    let size = rect.size;
    if size.width <= 0.0 || size.height <= 0.0 {
        return;
    }

    let card = rect.inflate(-20.0, -20.0);
    let radius = 40.0;
    let t = g.ctx().runtime().elapsed_secs();

    // The card and the sphere are captured first; the glass shader then draws
    // over the card and refracts what is below it.
    let (content, ()) = g.group(size, BlendMode::Normal, |inner| {
        inner.fill(
            card,
            Fill::vertical(GradientStops::two(
                Color::hex(0x414141),
                Color::hex(0x333333),
            )),
        );
        inner.rect_outline(
            card,
            radius,
            BrushStroke::new(1.0),
            Color::rgba(1.0, 1.0, 1.0, 0.10),
        );

        let phase = t * std::f32::consts::TAU / 1.5;
        let cx = card.left() + card.size.width * (0.66 + 0.12 * phase.sin());
        let cy = card.center().y;
        let r = card.size.height * 0.19;
        let center = Point::new(cx, cy);

        inner.circle(
            center,
            r * 1.35,
            Fill::radial_radius(
                GradientStops::two(
                    Color::rgba(0.96, 0.96, 0.96, 0.22),
                    Color::rgba(0.96, 0.96, 0.96, 0.0),
                ),
                r * 1.35,
            ),
        );
        inner.circle(
            center,
            r,
            Fill::radial_radius(
                GradientStops::two(Color::hex(0xF2F2F2), Color::hex(0xCFCFCF)),
                r,
            ),
        );
    });

    let uv = Painter::target_uv(size, card);
    g.clipped_rounded(card, radius, |inner| {
        inner.shader_at(
            Glass {
                strength: 0.35,
                thickness: 0.16,
            },
            card,
            uv,
            &[("content", TexInput::Target(content))],
        );
    });
}

fn main() {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "glass_sphere.gif".to_string());
    let opts = ExportOptions::new(320.0, 240.0)
        .scale(SCALE_2X)
        .background(Color::hex(0x262626));
    let anim = AnimationOptions::new(16, 1.5);

    Exporter::new()
        .shader(Glass {
            strength: 0.35,
            thickness: 0.16,
        })
        .export_gif(Canvas::new().on_draw(glass_scene), &opts, &anim, &path)
        .expect("export gif");

    println!("wrote {path} ({} frames)", anim.total_frames());
}
