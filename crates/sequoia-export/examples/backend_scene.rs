//! Exports the 3D comparison scenes headlessly with `sequoia-export`.
//!
//! The GPU demo records raymarching on the GPU; this example uses the very
//! same scene description as a CPU shader, so the website images are produced
//! by Sequoia's own exporter rather than by a screen grab.
//!
//!     cargo run -p sequoia-export --example backend_scene -- light out.png
//!     cargo run -p sequoia-export --example backend_scene -- heavy out.png

use sequoia_core::{Color, Fill, Point, Rect, Vec2};
use sequoia_export::{CpuShader, ExportOptions, Exporter, SCALE_1X, ShaderCtx};
use sequoia_shader::{SequoiShader, shader};
use sequoia_view::{Canvas, Painter};

const BG: Color = Color::hex(0x0B0D12);
const PANEL: Color = Color::hex(0x151922);
const TEXT: Color = Color::hex(0xF2F4F8);
const MUTED: Color = Color::hex(0x8A93A6);

shader! {
    pub fn raymarch_light(uv: FragUv) -> Color {
        return vec4<f32>(1.0, 1.0, 1.0, 1.0);
    }
}

shader! {
    pub fn raymarch_heavy(uv: FragUv) -> Color {
        return vec4<f32>(1.0, 1.0, 1.0, 1.0);
    }
}

type V3 = [f32; 3];

fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn scale(a: V3, k: f32) -> V3 {
    [a[0] * k, a[1] * k, a[2] * k]
}
fn dot(a: V3, b: V3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn length(a: V3) -> f32 {
    dot(a, a).sqrt()
}
fn normalize(a: V3) -> V3 {
    let len = length(a).max(0.000_001);
    scale(a, 1.0 / len)
}
fn reflect(dir: V3, normal: V3) -> V3 {
    sub(dir, scale(normal, 2.0 * dot(dir, normal)))
}

fn map_scene(p: V3, heavy: bool) -> f32 {
    let sphere = length(sub(p, [0.0, 0.05, 0.0])) - 0.85;
    let plane = p[1] + 0.9;
    if !heavy {
        return sphere.min(plane);
    }
    let ball_a = length(sub(p, [-0.95, -0.3, 0.1])) - 0.58;
    let ball_b = length(sub(p, [0.85, -0.45, -0.5])) - 0.45;
    let q = [length([p[0], p[2], 0.0]) - 0.95, p[1] - 0.15];
    let torus = (q[0] * q[0] + q[1] * q[1]).sqrt() - 0.2;
    sphere.min(plane).min(ball_a).min(ball_b).min(torus)
}

fn normal_at(p: V3, heavy: bool) -> V3 {
    let e = 0.0015;
    normalize([
        map_scene(add(p, [e, 0.0, 0.0]), heavy) - map_scene(sub(p, [e, 0.0, 0.0]), heavy),
        map_scene(add(p, [0.0, e, 0.0]), heavy) - map_scene(sub(p, [0.0, e, 0.0]), heavy),
        map_scene(add(p, [0.0, 0.0, e]), heavy) - map_scene(sub(p, [0.0, 0.0, e]), heavy),
    ])
}

fn march(origin: V3, dir: V3, steps: usize, heavy: bool) -> f32 {
    let mut travel = 0.05;
    for _ in 0..steps {
        let distance = map_scene(add(origin, scale(dir, travel)), heavy);
        if distance < 0.0008 || travel > 14.0 {
            break;
        }
        travel += distance * 0.85;
    }
    travel
}

fn soft_shadow(point: V3, light: V3, heavy: bool) -> f32 {
    let mut shade: f32 = 1.0;
    let mut travel = 0.06;
    let steps = if heavy { 12 } else { 0 };
    for _ in 0..steps {
        let distance = map_scene(add(point, scale(light, travel)), heavy);
        shade = shade.min(10.0 * distance / travel);
        travel += distance.clamp(0.03, 0.4);
        if distance < 0.001 || travel > 7.0 {
            break;
        }
    }
    shade.clamp(0.0, 1.0)
}

fn ambient_occlusion(point: V3, normal: V3, heavy: bool) -> f32 {
    let mut occ = 0.0;
    let mut factor = 1.0;
    let samples = if heavy { 3 } else { 0 };
    for i in 0..samples {
        let height = 0.02 + 0.11 * i as f32;
        let distance = map_scene(add(point, scale(normal, height)), heavy);
        occ += (height - distance) * factor;
        factor *= 0.72;
    }
    (1.0 - 2.2 * occ).clamp(0.0, 1.0)
}

fn surface_color(point: V3, dir: V3, light: V3, heavy: bool) -> V3 {
    let normal = normal_at(point, heavy);
    let diffuse = dot(normal, light).max(0.0);
    let shade = soft_shadow(point, normal, heavy);
    let ao = ambient_occlusion(point, normal, heavy);
    let mut base: V3 = [0.24, 0.56, 0.96];
    if point[1] < -0.85 {
        let checker = ((point[0] * 2.0).floor() + (point[2] * 2.0).floor()) % 2.0;
        base = if checker == 0.0 {
            [0.10, 0.11, 0.16]
        } else {
            [0.24, 0.26, 0.34]
        };
    }
    let spec_dir = reflect(scale(light, -1.0), normal);
    let specular = dot(spec_dir, scale(dir, -1.0)).max(0.0).powf(42.0);
    let rim = (1.0 - dot(normal, scale(dir, -1.0)).max(0.0)).powf(3.0);
    let light_amount = (0.12 + 0.88 * diffuse * shade) * ao;
    [
        (base[0] * light_amount + specular * shade * 0.7 + rim * 0.15).clamp(0.0, 1.0),
        (base[1] * light_amount + specular * shade * 0.7 + rim * 0.15).clamp(0.0, 1.0),
        (base[2] * light_amount + specular * shade * 0.7 + rim * 0.15).clamp(0.0, 1.0),
    ]
}

fn render_pixel(uv: [f32; 2], heavy: bool) -> [f32; 4] {
    let x = (uv[0] - 0.5) * 1.6;
    let y = 0.5 - uv[1];
    let origin: V3 = [0.0, 0.55, 3.1];
    let dir = normalize([x, y, -1.5]);
    let light = normalize([0.55, 0.85, 0.45]);
    let steps = if heavy { 96 } else { 56 };

    let mut color: V3 = [0.04, 0.05, 0.09];
    let travel = march(origin, dir, steps, heavy);
    if travel < 14.0 {
        let point = add(origin, scale(dir, travel));
        color = surface_color(point, dir, light, heavy);
        if heavy {
            let normal = normal_at(point, heavy);
            let bounce_origin = add(point, scale(normal, 0.02));
            let reflected = reflect(dir, normal);
            let bounce = march(bounce_origin, reflected, 32, heavy);
            if bounce < 14.0 {
                let bounce_point = add(bounce_origin, scale(reflected, bounce));
                let reflected_color = surface_color(bounce_point, reflected, light, heavy);
                let fresnel =
                    0.04 + 0.55 * (1.0 - dot(normal, scale(dir, -1.0)).max(0.0)).powf(5.0);
                color = [
                    color[0] * (1.0 - fresnel) + reflected_color[0] * fresnel,
                    color[1] * (1.0 - fresnel) + reflected_color[1] * fresnel,
                    color[2] * (1.0 - fresnel) + reflected_color[2] * fresnel,
                ];
            }
        }
    }
    [color[0], color[1], color[2], 1.0]
}

impl CpuShader for RaymarchLight {
    fn name(&self) -> &'static str {
        <RaymarchLight as SequoiShader>::NAME
    }

    fn shade(&self, ctx: &ShaderCtx<'_>) -> [f32; 4] {
        render_pixel(ctx.uv, false)
    }
}

impl CpuShader for RaymarchHeavy {
    fn name(&self) -> &'static str {
        <RaymarchHeavy as SequoiShader>::NAME
    }

    fn shade(&self, ctx: &ShaderCtx<'_>) -> [f32; 4] {
        render_pixel(ctx.uv, true)
    }
}

fn scene(heavy: bool) -> impl Fn(&mut Painter) + 'static {
    move |g: &mut Painter| {
        let rect = g.rect();
        g.fill(rect, Fill::solid(BG));
        g.text_tl(
            "Embedded 3D surface",
            Point::new(rect.left() + 18.0, rect.top() + 14.0),
            22.0,
            TEXT,
        );
        g.text_tl(
            if heavy {
                "Raymarched scene — heavy — sequoia-export"
            } else {
                "Raymarched scene — light — sequoia-export"
            },
            Point::new(rect.left() + 18.0, rect.top() + 46.0),
            12.0,
            MUTED,
        );
        let panel = Rect::from_xywh(
            rect.left() + 18.0,
            rect.top() + 72.0,
            rect.size.width - 36.0,
            rect.size.height - 90.0,
        );
        g.shadow(
            panel,
            16.0,
            22.0,
            0.0,
            Vec2::new(0.0, 8.0),
            Color::rgba(0.0, 0.0, 0.0, 0.35),
        );
        g.rounded(panel, 12.0, Fill::solid(PANEL));
        let uv = [0.0, 0.0, 1.0, 1.0];
        g.clipped_rounded(panel, 12.0, |c| {
            if heavy {
                c.shader_at(RaymarchHeavy, panel, uv, &[]);
            } else {
                c.shader_at(RaymarchLight, panel, uv, &[]);
            }
        });
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let variant = args.next().unwrap_or_else(|| "light".to_string());
    let path = args
        .next()
        .unwrap_or_else(|| format!("sequoia-{variant}-scene.png"));
    let heavy = variant.eq_ignore_ascii_case("heavy");

    let options = ExportOptions::new(760.0, 480.0)
        .scale(SCALE_1X)
        .background(BG);
    let exporter = Exporter::new().shader(RaymarchLight).shader(RaymarchHeavy);
    exporter
        .export_png(Canvas::new().on_draw(scene(heavy)), &options, &path)
        .expect("export scene png");
    println!("wrote {path}");
}
