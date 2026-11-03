//! Shader management: registry, configured passes, chains, and diagnostics.

use sequoia_core::{BlendMode, Color, Fill, PassNode, Rect, RenderItem, TexInput};
use sequoia_shader::{SequoiShader, ShaderRegistry, shader};
use sequoia_test::Harness;
use sequoia_view::Canvas;
use sequoia_view::prelude::*;

shader! {
    pub fn invert(uv: FragUv, content: Sampler2D) -> Color {
        let c = textureSample(content, content_sampler, uv);
        return vec4<f32>(1.0 - c.rgb, c.a);
    }
}

shader! {
    pub fn tint(uv: FragUv, content: Sampler2D) -> Color {
        let c = textureSample(content, content_sampler, uv);
        return vec4<f32>(c.r * 0.5, c.g, c.b, c.a);
    }
}

shader! {
    pub fn white() -> Color {
        return vec4<f32>(1.0);
    }
}

shader! {
    pub fn missing_effect(uv: FragUv, content: Sampler2D) -> Color {
        let c = textureSample(content, content_sampler, uv);
        return c;
    }
}

fn red_quad(g: &mut Painter) {
    g.fill(
        Rect::from_xywh(10.0, 10.0, 40.0, 40.0),
        Fill::solid(Color::RED),
    );
}

fn render_targets(list: &sequoia_core::RenderList) -> Vec<sequoia_core::RenderTargetId> {
    list.target_plan()
        .into_iter()
        .filter_map(|node| match node {
            PassNode::Render { target, .. } => Some(target),
            _ => None,
        })
        .collect()
}

#[test]
fn effect_pass_binds_captured_content_without_naming_the_input() {
    let h = Harness::new(Canvas::new().with_effect_pass(ShaderPass::new(Invert), red_quad));
    let invert = h.runtime().register_shader_id(Invert::NAME);
    h.layout(200.0, 200.0);
    let list = h.paint();

    let targets = render_targets(&list);
    assert_eq!(targets.len(), 1, "{:?}", list.target_plan());
    let draw = list.shaders().next().expect("effect draw");
    assert_eq!(draw.shader, invert);
    assert_eq!(draw.textures.as_ref(), &[(0, TexInput::Target(targets[0]))]);
}

#[test]
fn effect_chain_feeds_each_pass_into_the_next() {
    let h = Harness::new(
        Canvas::new().with_effect_chain(red_quad, ShaderChain::new().effect(Invert).effect(Tint)),
    );
    let invert = h.runtime().register_shader_id(Invert::NAME);
    let tint = h.runtime().register_shader_id(Tint::NAME);
    h.layout(200.0, 200.0);
    let list = h.paint();

    let targets = render_targets(&list);
    assert_eq!(targets.len(), 2, "content plus one intermediate pass");
    let draws: Vec<_> = list.shaders().collect();
    assert_eq!(draws.len(), 2);
    assert_eq!(draws[0].shader, invert);
    assert_eq!(draws[1].shader, tint);
    assert_eq!(
        draws[0].textures.as_ref(),
        &[(0, TexInput::Target(targets[0]))]
    );
    assert_eq!(
        draws[1].textures.as_ref(),
        &[(0, TexInput::Target(targets[1]))]
    );
}

#[test]
fn unregistered_chain_pass_is_skipped_and_forwards_its_input() {
    let h = Harness::new(Canvas::new().with_effect_chain(
        red_quad,
        ShaderChain::new().effect(MissingEffect).effect(Invert),
    ));
    let invert = h.runtime().register_shader_id(Invert::NAME);
    h.layout(200.0, 200.0);
    let list = h.paint();

    let targets = render_targets(&list);
    let draws: Vec<_> = list.shaders().collect();
    assert_eq!(draws.len(), 1, "only the registered pass draws");
    assert_eq!(draws[0].shader, invert);
    assert_eq!(
        draws[0].textures.as_ref(),
        &[(0, TexInput::Target(targets[0]))],
        "the skipped pass must not replace the content target"
    );
}

#[test]
fn missing_final_chain_pass_blits_captured_content_back() {
    let h = Harness::new(
        Canvas::new().with_effect_chain(red_quad, ShaderChain::new().effect(MissingEffect)),
    );
    h.layout(200.0, 200.0);
    let list = h.paint();

    assert_eq!(list.shaders().count(), 0);
    assert!(
        list.items()
            .iter()
            .any(|item| matches!(item, RenderItem::Blit { .. })),
        "captured content must be restored when the last pass is missing"
    );
}

#[test]
fn configured_pass_supports_blend_and_uv_window() {
    let h = Harness::new(
        Canvas::new().with_effect_pass(
            ShaderPass::new(Invert)
                .blend(BlendMode::Screen)
                .uv([0.25, 0.25, 0.75, 0.75]),
            red_quad,
        ),
    );
    h.runtime().register_shader_id(Invert::NAME);
    h.layout(200.0, 200.0);
    let list = h.paint();
    let draw = list.shaders().next().expect("effect draw");
    assert_eq!(draw.blend, BlendMode::Screen);
    assert_eq!(draw.uv, [0.25, 0.25, 0.75, 0.75]);
}

#[test]
fn whole_canvas_pass_draws_procedural_shader() {
    let h = Harness::new(Canvas::new().with_shader_pass(ShaderPass::new(White)));
    let white = h.runtime().register_shader_id(White::NAME);
    h.layout(100.0, 50.0);
    let list = h.paint();

    assert!(!list.has_targets());
    let draws: Vec<_> = list.shaders().collect();
    assert_eq!(draws.len(), 1);
    assert_eq!(draws[0].shader, white);
    assert!(draws[0].textures.is_empty());
    assert!(matches!(
        list.items()
            .iter()
            .find(|item| matches!(item, RenderItem::Shader(_))),
        Some(RenderItem::Shader(_))
    ));
}

#[test]
fn unregistered_pass_records_nothing() {
    let h = Harness::new(Canvas::new().with_shader_pass(ShaderPass::new(White)));
    h.layout(100.0, 50.0);
    let list = h.paint();
    assert_eq!(list.shaders().count(), 0);
}

#[test]
fn registry_dedupes_and_lists_names() {
    let registry = ShaderRegistry::new()
        .with::<Invert>()
        .with_raw(Invert::NAME, "replacement")
        .with_raw("effects/tint", "tint source");

    assert_eq!(registry.len(), 2);
    assert_eq!(registry.get(Invert::NAME), Some("replacement"));
    assert_eq!(registry.get("effects/tint"), Some("tint source"));
    assert!(registry.contains("effects/tint"));
    assert_eq!(
        registry.names().collect::<Vec<_>>(),
        vec![Invert::NAME, "effects/tint"]
    );
    assert_eq!(registry.iter().count(), 2);
}

#[test]
fn runtime_reports_registered_shader_names() {
    let h = Harness::new(Canvas::new());
    h.runtime().register_shader_id("effects/zeta");
    h.runtime().register_shader_id("effects/alpha");
    assert_eq!(
        h.runtime().shader_names(),
        vec!["effects/alpha".to_string(), "effects/zeta".to_string()]
    );
    assert!(h.runtime().has_shader("effects/alpha"));
    assert!(!h.runtime().has_shader("effects/missing"));
}
