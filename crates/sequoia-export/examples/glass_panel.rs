//! A glassmorphism panel: a frosted, blurred pane over drifting colour.
//!
//! The scene is captured into an offscreen target. The glass shader samples a
//! 12-tap blur of that target, adds a milky tint, a bright rounded border, an
//! inner glow and a diagonal reflection. The coloured lights orbit behind the
//! panel, so the blur keeps shifting through the loop.
//!
//! Run with:
//!     cargo run --release -p sequoia-export --example glass_panel -- glass-panel.gif
//!
//! A path ending in `.png` exports a single still instead of the animation.
//! Debug builds are roughly 10x slower here: each frame is rasterized and
//! shader-shaded on the CPU, then quantized to a GIF palette.

use sequoia_core::{BlendMode, Color, Fill, GradientStops, Point, Rect, TexInput};
use sequoia_export::{AnimationOptions, CpuShader, ExportOptions, Exporter, SCALE_2X, ShaderCtx};
use sequoia_shader::{SequoiShader, shader};
use sequoia_view::{Canvas, Painter};

const PANEL_W: f32 = 208.0;
const PANEL_H: f32 = 144.0;
const PANEL_RADIUS: f32 = 26.0;
const BLUR: f32 = 18.0;
const TINT: f32 = 0.24;

shader! {
    pub fn glass(
        uv: FragUv,
        content: Sampler2D,
        u0: f32,
        v0: f32,
        u1: f32,
        v1: f32,
        width: f32,
        height: f32,
        radius: f32,
        blur: f32,
        tint: f32,
    ) -> Color {
        let size_uv = vec2<f32>(max(u1 - u0, 0.0001), max(v1 - v0, 0.0001));
        let local = (uv - vec2<f32>(u0, v0)) / size_uv;
        let px = local * vec2<f32>(width, height);
        let half_size = vec2<f32>(width, height) * 0.5;

        // Signed distance to a rounded rectangle, in logical pixels.
        let q = abs(px - half_size) - (half_size - vec2<f32>(radius, radius));
        let dist = length(max(q, vec2<f32>(0.0, 0.0)))
            + min(max(q.x, q.y), 0.0)
            - radius;
        let alpha = 1.0 - sq_smoothstep(-0.7, 0.7, dist);
        if (alpha <= 0.001) {
            return vec4<f32>(0.0, 0.0, 0.0, 0.0);
        }

        // Frosted backdrop: a golden-angle spiral of taps, weighted towards
        // the centre and scaled by the panel's logical pixel size.
        let texel = vec2<f32>(
            (u1 - u0) / max(width, 0.001),
            (v1 - v0) / max(height, 0.001),
        );
        var acc = vec4<f32>(0.0, 0.0, 0.0, 0.0);
        var total = 0.0;
        for (var i = 0; i < 12; i = i + 1) {
            let angle = f32(i) * 2.399963 + 0.7;
            let ring = sqrt((f32(i) + 0.5) / 12.0);
            let sample_uv = clamp(
                uv + vec2<f32>(cos(angle), sin(angle)) * ring * blur * texel,
                vec2<f32>(0.0, 0.0),
                vec2<f32>(1.0, 1.0),
            );
            let weight = 1.0 - ring * 0.45;
            acc = acc + textureSample(content, content_sampler, sample_uv) * weight;
            total = total + weight;
        }
        let blurred = acc / max(total, 0.0001);

        // Milky glass body.
        var color = mix(blurred.rgb, vec3<f32>(0.93, 0.95, 1.0), tint);
        color = color + vec3<f32>(0.05, 0.06, 0.08);

        // Soft border: a narrow core, a wider halo, and a top-edge sheen.
        let rim = 1.0 - sq_smoothstep(0.0, 1.1, abs(dist + 0.55));
        let halo = (1.0 - sq_smoothstep(0.0, 6.0, abs(dist + 1.2))) * 0.08;
        let glow = (1.0 - sq_smoothstep(0.0, 12.0, -dist)) * 0.05;
        let sheen = (1.0 - local.y) * 0.030;
        let top_rim = rim * (0.30 + 0.22 * (1.0 - local.y));
        color = color + vec3<f32>(1.0, 1.0, 1.0) * (top_rim + halo + glow + sheen);

        // Soft diagonal reflection.
        let axis = local.x * 0.55 + local.y * 0.45;
        let streak = 1.0 - sq_smoothstep(0.0, 0.075, abs(axis - 0.36));
        color = color + vec3<f32>(0.90, 0.95, 1.0) * streak * 0.045;

        color = clamp(color, vec3<f32>(0.0, 0.0, 0.0), vec3<f32>(1.0, 1.0, 1.0));
        return vec4<f32>(color, alpha);
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
        let size_uv = [
            (self.u1 - self.u0).max(0.0001),
            (self.v1 - self.v0).max(0.0001),
        ];
        let local = [
            (ctx.uv[0] - self.u0) / size_uv[0],
            (ctx.uv[1] - self.v0) / size_uv[1],
        ];
        let px = [local[0] * self.width, local[1] * self.height];
        let half = [self.width * 0.5, self.height * 0.5];
        let q = [
            (px[0] - half[0]).abs() - (half[0] - self.radius),
            (px[1] - half[1]).abs() - (half[1] - self.radius),
        ];
        let dist = (q[0].max(0.0).powi(2) + q[1].max(0.0).powi(2)).sqrt() + q[0].max(q[1]).min(0.0)
            - self.radius;
        let alpha = 1.0 - smoothstep(-0.7, 0.7, dist);
        if alpha <= 0.001 {
            return [0.0, 0.0, 0.0, 0.0];
        }

        let texel = [
            (self.u1 - self.u0) / self.width.max(0.001),
            (self.v1 - self.v0) / self.height.max(0.001),
        ];
        let mut acc = [0.0_f32; 3];
        let mut total = 0.0;
        for i in 0..12 {
            let fi = i as f32;
            let angle = fi * 2.399963 + 0.7;
            let ring = ((fi + 0.5) / 12.0).sqrt();
            let sx = (ctx.uv[0] + angle.cos() * ring * self.blur * texel[0]).clamp(0.0, 1.0);
            let sy = (ctx.uv[1] + angle.sin() * ring * self.blur * texel[1]).clamp(0.0, 1.0);
            let weight = 1.0 - ring * 0.45;
            let sample = ctx.sample(0, [sx, sy]);
            acc[0] += sample[0] * weight;
            acc[1] += sample[1] * weight;
            acc[2] += sample[2] * weight;
            total += weight;
        }
        let inv = 1.0 / total.max(0.0001);
        let blurred = [acc[0] * inv, acc[1] * inv, acc[2] * inv];
        let mix = |a: f32, b: f32| a + (b - a) * self.tint;
        let mut color = [
            mix(blurred[0], 0.93) + 0.05,
            mix(blurred[1], 0.95) + 0.06,
            mix(blurred[2], 1.0) + 0.08,
        ];

        let rim = 1.0 - smoothstep(0.0, 1.1, (dist + 0.55).abs());
        let halo = (1.0 - smoothstep(0.0, 6.0, (dist + 1.2).abs())) * 0.08;
        let glow = (1.0 - smoothstep(0.0, 12.0, -dist)) * 0.05;
        let sheen = (1.0 - local[1]) * 0.030;
        let top_rim = rim * (0.30 + 0.22 * (1.0 - local[1]));
        let add = top_rim + halo + glow + sheen;
        color[0] += add;
        color[1] += add;
        color[2] += add;

        let axis = local[0] * 0.55 + local[1] * 0.45;
        let streak = 1.0 - smoothstep(0.0, 0.075, (axis - 0.36).abs());
        color[0] += 0.90 * streak * 0.045;
        color[1] += 0.95 * streak * 0.045;
        color[2] += streak * 0.045;

        [
            color[0].clamp(0.0, 1.0),
            color[1].clamp(0.0, 1.0),
            color[2].clamp(0.0, 1.0),
            alpha,
        ]
    }
}

fn glass_shader() -> Glass {
    Glass {
        u0: 0.175,
        v0: 0.2,
        u1: 0.825,
        v1: 0.8,
        width: PANEL_W,
        height: PANEL_H,
        radius: PANEL_RADIUS,
        blur: BLUR,
        tint: TINT,
    }
}

fn glass_scene(g: &mut Painter) {
    let rect = g.rect();
    let size = rect.size;
    if size.width <= 0.0 || size.height <= 0.0 {
        return;
    }

    let card = rect.inflate(-20.0, -20.0);
    let card_radius = 26.0;
    let panel = Rect::from_xywh(
        card.center().x - PANEL_W * 0.5,
        card.center().y - PANEL_H * 0.5,
        PANEL_W,
        PANEL_H,
    );
    let t = g.ctx().runtime().elapsed_secs();
    let phase = t * std::f32::consts::TAU / 1.5;

    // Everything behind the panel, including the high-frequency dots that make
    // the frost visible.
    let (content, ()) = g.group(size, BlendMode::Normal, |inner| {
        inner.clipped_rounded(card, card_radius, |c| {
            c.fill(
                card,
                Fill::vertical(GradientStops::two(
                    Color::hex(0x1A1033),
                    Color::hex(0x061A22),
                )),
            );

            let pink_x = card.left() + card.size.width * 0.28 + phase.cos() * 20.0;
            let pink_y = card.top() + card.size.height * 0.32 + phase.sin() * 13.0;
            c.circle(
                Point::new(pink_x, pink_y),
                92.0,
                Fill::radial_radius(
                    GradientStops::two(
                        Color::rgba(1.0, 0.20, 0.55, 0.80),
                        Color::rgba(1.0, 0.20, 0.55, 0.0),
                    ),
                    92.0,
                ),
            );

            let cyan_x = card.right() - card.size.width * 0.22 + (phase + 2.2).cos() * 24.0;
            let cyan_y = card.bottom() - card.size.height * 0.28 + (phase + 2.2).sin() * 15.0;
            c.circle(
                Point::new(cyan_x, cyan_y),
                104.0,
                Fill::radial_radius(
                    GradientStops::two(
                        Color::rgba(0.10, 0.80, 0.95, 0.75),
                        Color::rgba(0.10, 0.80, 0.95, 0.0),
                    ),
                    104.0,
                ),
            );

            let violet_x = card.right() - card.size.width * 0.36 + (phase + 4.1).cos() * 18.0;
            let violet_y = card.top() + card.size.height * 0.24 + (phase + 4.1).sin() * 12.0;
            c.circle(
                Point::new(violet_x, violet_y),
                66.0,
                Fill::radial_radius(
                    GradientStops::two(
                        Color::rgba(0.55, 0.35, 1.0, 0.70),
                        Color::rgba(0.55, 0.35, 1.0, 0.0),
                    ),
                    66.0,
                ),
            );

            // A soft white light behind the top-left of the panel.
            let glow_x = panel.left() + 34.0 + phase.sin() * 10.0;
            let glow_y = panel.top() + 24.0;
            c.circle(
                Point::new(glow_x, glow_y),
                46.0,
                Fill::radial_radius(
                    GradientStops::two(
                        Color::rgba(1.0, 0.96, 0.94, 0.40),
                        Color::rgba(1.0, 0.96, 0.94, 0.0),
                    ),
                    46.0,
                ),
            );

            let step = 17.0;
            let dot = Color::rgba(1.0, 1.0, 1.0, 0.20);
            let mut x = card.left() + step * 0.5;
            while x < card.right() {
                let mut y = card.top() + step * 0.5;
                while y < card.bottom() {
                    c.circle(Point::new(x, y), 1.4, Fill::solid(dot));
                    y += step;
                }
                x += step;
            }

            c.rect_outline(
                card,
                card_radius,
                sequoia_core::BrushStroke::new(1.0),
                Color::rgba(1.0, 1.0, 1.0, 0.12),
            );
        });
    });

    let card_uv = Painter::target_uv(size, card);
    let panel_uv = Painter::target_uv(size, panel);
    let glass = Glass {
        u0: panel_uv[0],
        v0: panel_uv[1],
        u1: panel_uv[2],
        v1: panel_uv[3],
        width: PANEL_W,
        height: PANEL_H,
        radius: PANEL_RADIUS,
        blur: BLUR,
        tint: TINT,
    };

    g.clipped_rounded(card, card_radius, |inner| {
        inner.blit(content, card, card_uv);

        // Soft, layered drop shadow under the pane.
        for i in 0..7 {
            let grow = 1.0 + i as f32 * 2.8;
            let shadow = Rect::from_xywh(
                panel.left() - grow + 3.0,
                panel.top() + 14.0 - grow * 0.35,
                panel.size.width + grow * 2.0,
                panel.size.height + grow * 1.6,
            );
            inner.rounded(
                shadow,
                PANEL_RADIUS + grow,
                Fill::solid(Color::rgba(0.0, 0.01, 0.02, 0.03)),
            );
        }

        inner.shader_at(
            glass,
            panel,
            panel_uv,
            &[("content", TexInput::Target(content))],
        );
    });
}

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().unwrap_or_else(|| "glass-panel.gif".to_string());
    let still = path.to_ascii_lowercase().ends_with(".png");
    let opts = ExportOptions::new(320.0, 240.0)
        .scale(SCALE_2X)
        .background(Color::hex(0x0E0E14));
    let exporter = Exporter::new().shader(glass_shader());

    if still {
        exporter
            .export_png(Canvas::new().on_draw(glass_scene), &opts, &path)
            .expect("export png");
        println!("wrote {path}");
    } else {
        let anim = AnimationOptions::new(12, 1.5);
        exporter
            .export_gif(Canvas::new().on_draw(glass_scene), &opts, &anim, &path)
            .expect("export gif");
        println!("wrote {path} ({} frames)", anim.total_frames());
    }
}
