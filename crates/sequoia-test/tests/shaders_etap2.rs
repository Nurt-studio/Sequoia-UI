use sequoia_core::{ImageSource, Rect, TexInput};
#[allow(unused_imports)]
use sequoia_shader::{Color, FragUv, Sampler2D, SequoiShader, shader};
use sequoia_test::Harness;
use sequoia_view::prelude::*;
use sequoia_view::{PaintCtx, Painter, View};
use std::sync::Arc;

shader! {
    pub fn solid(color: Color, gain: f32, invert: bool) -> Color {
        return vec4<f32>(color.rgb * gain, select(color.a, 1.0 - color.a, invert));
    }
}

shader! {
    pub fn textured(uv: FragUv, tint: Color, tex: Sampler2D) -> Color {
        let sample = textureSample(tex, tex_sampler, uv);
        return vec4<f32>(sample.rgb * tint.rgb, sample.a);
    }
}

shader! {
    pub fn std_luminance(uv: FragUv) -> Color {
        let v = sq_luminance(uv.xyx);
        return vec4<f32>(v, v, v, 1.0);
    }
}

shader! {
    pub fn std_srgb(uv: FragUv) -> Color {
        let v = sq_srgb_to_linear(uv.xyx);
        return vec4<f32>(v, 1.0);
    }
}

shader! {
    pub fn std_linear(uv: FragUv) -> Color {
        let v = sq_linear_to_srgb(uv.xyx);
        return vec4<f32>(v, 1.0);
    }
}

shader! {
    pub fn std_hash(uv: FragUv) -> Color {
        let v = sq_hash21(uv);
        return vec4<f32>(v, v, v, 1.0);
    }
}

shader! {
    pub fn std_noise(uv: FragUv) -> Color {
        let v = sq_noise21(uv * 8.0);
        return vec4<f32>(v, v, v, 1.0);
    }
}

shader! {
    pub fn std_fbm(uv: FragUv) -> Color {
        let v = sq_fbm21(uv, 3);
        return vec4<f32>(v, v, v, 1.0);
    }
}

shader! {
    pub fn std_rotate(uv: FragUv) -> Color {
        let p = sq_rotate(uv, 1.0);
        return vec4<f32>(p, 0.0, 1.0);
    }
}

shader! {
    pub fn std_remap(uv: FragUv) -> Color {
        let v = sq_remap(uv.x, 0.0, 1.0, 0.0, 1.0);
        return vec4<f32>(v, v, v, 1.0);
    }
}

shader! {
    pub fn std_contrast(uv: FragUv) -> Color {
        let v = sq_contrast(uv.xyx, 1.2);
        return vec4<f32>(v, 1.0);
    }
}

shader! {
    pub fn std_saturate(uv: FragUv) -> Color {
        let v = sq_saturate(uv.xyx, 0.5);
        return vec4<f32>(v, 1.0);
    }
}

shader! {
    pub fn std_checker(uv: FragUv) -> Color {
        let v = sq_checker_uv(uv, 8.0);
        return vec4<f32>(v, v, v, 1.0);
    }
}

shader! {
    pub fn std_vignette(uv: FragUv) -> Color {
        let v = sq_vignette(uv, 1.0);
        return vec4<f32>(v, v, v, 1.0);
    }
}

struct SolidView;

impl View for SolidView {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn is_leaf(&self) -> bool {
        true
    }

    fn paint_self(&self, ctx: &mut PaintCtx) {
        let mut g = Painter::new(ctx);
        g.shader(
            Solid {
                color: Color::rgba(0.25, 0.5, 0.75, 1.0),
                gain: 2.0,
                invert: true,
            },
            Rect::from_xywh(10.0, 20.0, 30.0, 40.0),
            &[],
        );
    }
}

struct TexturedView;

impl View for TexturedView {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn is_leaf(&self) -> bool {
        true
    }

    fn paint_self(&self, ctx: &mut PaintCtx) {
        let mut g = Painter::new(ctx);
        let input = TexInput::Image(ImageSource::Rgba {
            id: 77,
            width: 2,
            height: 2,
            data: Arc::new(vec![255; 16]),
        });
        g.shader(
            Textured { tint: Color::WHITE },
            Rect::from_xywh(0.0, 0.0, 50.0, 50.0),
            &[("tex", input)],
        );
    }
}

#[test]
fn typed_solid_shader_produces_uniforms_and_stable_id() {
    let harness = Harness::new(SolidView);
    let id = harness.runtime().register_shader_id(Solid::NAME);
    harness.layout(400.0, 300.0);
    let list = harness.paint();
    let draw = list
        .shaders()
        .next()
        .expect("typed shader must emit a draw");
    assert_eq!(draw.shader, id);
    assert_eq!(draw.rect, Rect::from_xywh(10.0, 20.0, 30.0, 40.0));
    assert_eq!(draw.uniforms.len(), Solid::UNIFORM_SIZE);
    assert!(draw.textures.is_empty());
    assert_eq!(
        f32::from_le_bytes(draw.uniforms[0..4].try_into().unwrap()),
        0.25
    );
    assert_eq!(
        u32::from_le_bytes(draw.uniforms[20..24].try_into().unwrap()),
        1
    );
}

#[test]
fn typed_texture_shader_maps_named_inputs_to_slots() {
    let harness = Harness::new(TexturedView);
    harness.runtime().register_shader_id(Textured::NAME);
    harness.layout(400.0, 300.0);
    let list = harness.paint();
    let draw = list
        .shaders()
        .next()
        .expect("textured shader must emit a draw");
    assert_eq!(draw.textures.len(), 1);
    let (slot, input) = &draw.textures[0];
    assert_eq!(*slot, 0);
    let TexInput::Image(ImageSource::Rgba { id, .. }) = input else {
        panic!("expected image texture");
    };
    assert_eq!(*id, 77);
    assert_eq!(Textured::TEXTURES, &["tex"]);
}

#[test]
fn unregistered_typed_shader_draws_nothing() {
    let harness = Harness::new(SolidView);
    harness.layout(400.0, 300.0);
    let list = harness.paint();
    assert!(list.shaders().next().is_none());
}

#[test]
fn app_registers_typed_shader_by_name() {
    let app = sequoia_platform::App::new().shader_type::<Solid>();
    let entry = app
        .shaders()
        .iter()
        .find(|(name, _)| name == Solid::NAME)
        .expect("typed shader must be registered");
    assert_eq!(entry.1, Solid::WGSL);
    assert!(
        app.shaders().iter().any(|(name, _)| name == "apply_mask"),
        "built-in mask shader is registered by default"
    );
}

#[test]
fn generated_wgsl_and_stdlib_are_valid() {
    for wgsl in [
        Solid::WGSL,
        Textured::WGSL,
        StdLuminance::WGSL,
        StdSrgb::WGSL,
        StdLinear::WGSL,
        StdHash::WGSL,
        StdNoise::WGSL,
        StdFbm::WGSL,
        StdRotate::WGSL,
        StdRemap::WGSL,
        StdContrast::WGSL,
        StdSaturate::WGSL,
        StdChecker::WGSL,
        StdVignette::WGSL,
    ] {
        assert!(wgsl.contains("fn vs_main"));
        assert!(wgsl.contains("fn fs_main"));
        assert!(wgsl.contains("fn fs_user"));
    }
}
