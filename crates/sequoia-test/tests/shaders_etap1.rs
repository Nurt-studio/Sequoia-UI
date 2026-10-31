use sequoia_core::{BlendMode, Rect, RenderItem, RenderList, ShaderDraw, ShaderId, Size, TexInput};
use sequoia_render::{RenderError, compose_user_shader, passthrough_wgsl, validate_user_shader};
use sequoia_test::Harness;
use sequoia_view::prelude::*;
use sequoia_view::{PaintCtx, Painter, View};

struct ShaderTestView {
    blend: BlendMode,
    uniforms: Vec<u8>,
}

impl View for ShaderTestView {
    fn body(&self) -> impl View + 'static {
        Unit
    }
    fn is_leaf(&self) -> bool {
        true
    }
    fn paint_self(&self, ctx: &mut PaintCtx) {
        let mut g = Painter::new(ctx);
        g.shader_raw_with_blend(
            "passthrough",
            Rect::from_xywh(0.0, 0.0, 100.0, 100.0),
            &self.uniforms,
            self.blend,
        );
    }
}

fn red_uniforms() -> Vec<u8> {
    [1.0f32, 0.0, 0.0, 1.0]
        .into_iter()
        .flat_map(f32::to_le_bytes)
        .collect()
}

#[test]
fn canvas_shader_raw_emits_shader_item_with_uniforms() {
    let h = Harness::new(ShaderTestView {
        blend: BlendMode::Normal,
        uniforms: red_uniforms(),
    });
    h.layout(400.0, 300.0);
    let list = h.paint();
    let mut found = false;
    for item in list.items() {
        if let RenderItem::Shader(shader) = item {
            assert_eq!(shader.rect, Rect::from_xywh(0.0, 0.0, 100.0, 100.0));
            assert_eq!(shader.blend, BlendMode::Normal);
            assert_eq!(shader.shader, ShaderId::new(0));
            assert_eq!(&shader.uniforms[..16], red_uniforms().as_slice());
            found = true;
        }
    }
    assert!(
        found,
        "RenderList must contain ShaderDraw item from shader_raw"
    );
}

#[test]
fn canvas_shader_raw_keeps_explicit_blend_mode() {
    let h = Harness::new(ShaderTestView {
        blend: BlendMode::Multiply,
        uniforms: red_uniforms(),
    });
    h.layout(400.0, 300.0);
    let list = h.paint();
    let shader = list
        .shaders()
        .next()
        .expect("RenderList must contain a shader draw");
    assert_eq!(shader.blend, BlendMode::Multiply);
}

#[test]
fn invalid_wgsl_reports_source_line() {
    let source = "@fragment\nfn fs_main() -> @location(0) vec4<f32> {\n    let x = ;\n    return vec4<f32>(1.0);\n}";
    let err = validate_user_shader(source).expect_err("invalid WGSL must fail");
    let RenderError::ShaderCompile(message) = err else {
        panic!("expected ShaderCompile, got {err:?}");
    };
    assert!(
        message.contains("3:"),
        "diagnostic should mention line 3: {message}"
    );
}

#[test]
fn missing_entry_points_are_rejected() {
    let err = validate_user_shader(
        "@fragment fn fs_main() -> @location(0) vec4<f32> { return vec4<f32>(1.0); }",
    )
    .expect_err("missing vs_main must fail");
    let RenderError::ShaderCompile(message) = err else {
        panic!("expected ShaderCompile, got {err:?}");
    };
    assert!(message.contains("vs_main"), "{message}");
}

#[test]
fn composed_shaders_and_passthrough_validate() {
    let source = compose_user_shader(
        "fn fs_user(frag: ShaderVarying) -> vec4<f32> { return vec4<f32>(1.0, 0.0, 0.0, 1.0); }",
    );
    assert!(source.contains("fn vs_main"));
    assert!(source.contains("fn fs_main"));
    validate_user_shader(&source).expect("composed shader validates");
    validate_user_shader(&passthrough_wgsl()).expect("passthrough validates");
}

#[test]
fn render_list_sanitizes_shader_draws() {
    let mut list = RenderList::new();
    list.push_shader(ShaderDraw {
        rect: Rect::from_xywh(0.0, 0.0, 10.0, 10.0),
        uv: [0.0, 0.0, 1.0, 1.0],
        shader: ShaderId::new(9),
        uniforms: vec![0xABu8; sequoia_core::MAX_SHADER_UNIFORM_BYTES + 4].into(),
        textures: vec![
            (2, TexInput::Image(1)),
            (1, TexInput::Image(2)),
            (2, TexInput::Image(3)),
        ]
        .into(),
        blend: BlendMode::Screen,
        opacity: 2.0,
    });

    let draw = list.shaders().next().expect("shader draw");
    assert_eq!(draw.uniforms.len(), sequoia_core::MAX_SHADER_UNIFORM_BYTES);
    assert_eq!(draw.opacity, 1.0);
    assert_eq!(
        draw.textures.as_ref(),
        &[(1, TexInput::Image(2)), (2, TexInput::Image(3))]
    );
}

#[test]
fn render_list_records_target_markers_and_blits() {
    let mut list = RenderList::new();
    let target = list.begin_target(Size::new(32.0, 16.0), BlendMode::Normal);
    list.end_target(target);
    list.push_blit(
        Rect::from_xywh(0.0, 0.0, 32.0, 16.0),
        [0.0, 0.0, 1.0, 1.0],
        target,
        BlendMode::Add,
    );

    assert!(matches!(
        list.items()[0],
        RenderItem::TargetBegin { id, .. } if id == target
    ));
    assert!(matches!(
        list.items()[1],
        RenderItem::TargetEnd { id } if id == target
    ));
    assert!(matches!(
        list.items()[2],
        RenderItem::Blit { target: id, .. } if id == target
    ));
}
