use sequoia_core::{BlendMode, Rect, RenderItem};
use sequoia_test::Harness;
use sequoia_view::prelude::*;
use sequoia_view::{PaintCtx, Painter, View};

struct ShaderTestView;
impl View for ShaderTestView {
    fn body(&self) -> impl View + 'static {
        Unit
    }
    fn is_leaf(&self) -> bool {
        true
    }
    fn paint_self(&self, ctx: &mut PaintCtx) {
        let mut g = Painter::new(ctx);
        g.shader_raw(
            "passthrough",
            Rect::from_xywh(0.0, 0.0, 100.0, 100.0),
            &[0u8; 16],
        );
    }
}

#[test]
fn canvas_shader_raw_emits_shader_item() {
    let h = Harness::new(ShaderTestView);
    h.layout(400.0, 300.0);
    let list = h.paint();
    let mut found = false;
    for item in list.items() {
        if let RenderItem::Shader(shader) = item {
            assert_eq!(shader.rect, Rect::from_xywh(0.0, 0.0, 100.0, 100.0));
            assert_eq!(shader.blend, BlendMode::Normal);
            found = true;
        }
    }
    assert!(
        found,
        "RenderList must contain ShaderDraw item from shader_raw"
    );
}
