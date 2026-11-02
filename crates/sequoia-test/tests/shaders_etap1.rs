use sequoia_core::{
    BlendMode, ImageSource, Rect, RenderItem, RenderList, ShaderDraw, ShaderId, Size, TexInput,
};
use sequoia_render::{RenderError, compose_user_shader, passthrough_wgsl, validate_user_shader};
use sequoia_shader::{Color, SequoiShader, shader};
use sequoia_test::Harness;
use sequoia_view::prelude::*;
use sequoia_view::{PaintCtx, Painter, View};
use std::sync::Arc;

fn tex(id: u64) -> TexInput {
    TexInput::Image(ImageSource::Rgba {
        id,
        width: 1,
        height: 1,
        data: Arc::new(vec![255; 4]),
    })
}

shader! {
    pub fn red_solid(color: Color) -> Color {
        return color;
    }
}

struct ShaderTestView {
    blend: BlendMode,
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
        let shader = RedSolid {
            color: Color::rgba(1.0, 0.0, 0.0, 1.0),
        };
        g.with_blend(self.blend, |g| {
            g.shader(shader, Rect::from_xywh(0.0, 0.0, 100.0, 100.0), &[]);
        });
    }
}

#[test]
fn typed_shader_emits_item_with_uniforms() {
    let h = Harness::new(ShaderTestView {
        blend: BlendMode::Normal,
    });
    let id = h.runtime().register_shader_id(RedSolid::NAME);
    h.layout(400.0, 300.0);
    let list = h.paint();
    let shader = list
        .shaders()
        .next()
        .expect("RenderList must contain a shader draw");
    assert_eq!(shader.rect, Rect::from_xywh(0.0, 0.0, 100.0, 100.0));
    assert_eq!(shader.blend, BlendMode::Normal);
    assert_eq!(shader.shader, id);
    assert_eq!(&shader.uniforms[0..4], &1.0f32.to_le_bytes());
    assert_eq!(&shader.uniforms[12..16], &1.0f32.to_le_bytes());
}

#[test]
fn with_blend_keeps_explicit_blend_mode() {
    let h = Harness::new(ShaderTestView {
        blend: BlendMode::Multiply,
    });
    h.runtime().register_shader_id(RedSolid::NAME);
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
        textures: vec![(2, tex(1)), (1, tex(2)), (2, tex(3))].into(),
        blend: BlendMode::Screen,
        opacity: 2.0,
    });

    let draw = list.shaders().next().expect("shader draw");
    assert_eq!(draw.uniforms.len(), sequoia_core::MAX_SHADER_UNIFORM_BYTES);
    assert_eq!(draw.opacity, 1.0);
    assert_eq!(draw.textures.as_ref(), &[(1, tex(2)), (2, tex(3))]);
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
