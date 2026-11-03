//! Stage 3 painter API: offscreen groups, target inputs, blits, and masks.

use sequoia_core::{BlendMode, Color, Fill, PassNode, Point, Rect, RenderItem, Size, TexInput};
use sequoia_shader::{ApplyMask, SequoiShader, shader};
use sequoia_test::Harness;
use sequoia_view::prelude::*;
use sequoia_view::{Canvas, PaintCtx, Painter, View};

shader! {
    pub fn invert(uv: FragUv, content: Sampler2D) -> Color {
        let c = textureSample(content, content_sampler, uv);
        return vec4<f32>(1.0 - c.rgb, c.a);
    }
}

shader! {
    pub fn white() -> Color {
        return vec4<f32>(1.0);
    }
}

fn red_quad(g: &mut Painter) {
    g.fill(
        Rect::from_xywh(10.0, 10.0, 40.0, 40.0),
        Fill::solid(Color::RED),
    );
}

fn shader_index(list: &sequoia_core::RenderList) -> usize {
    list.items()
        .iter()
        .position(|item| matches!(item, RenderItem::Shader(_)))
        .expect("shader draw is recorded")
}

#[test]
fn with_effect_renders_content_into_a_target_then_shades_it() {
    let h = Harness::new(Canvas::new().with_effect(Invert, red_quad));
    let id = h.runtime().register_shader_id(Invert::NAME);
    h.layout(200.0, 200.0);
    let list = h.paint();
    assert!(list.has_targets());

    let plan = list.target_plan();
    assert_eq!(plan.len(), 2, "{plan:?}");
    let PassNode::Render { target, items, .. } = &plan[0] else {
        panic!("expected the content pass first: {plan:?}");
    };
    assert!(matches!(list.items()[items.start], RenderItem::Quad(_)));

    let draw = list.shaders().next().expect("effect shader draw");
    assert_eq!(draw.shader, id);
    assert_eq!(Invert::TEXTURES, &["content"]);
    assert_eq!(draw.textures.as_ref(), &[(0, TexInput::Target(*target))]);

    let PassNode::Present { items: present } = &plan[1] else {
        panic!("expected the present pass second: {plan:?}");
    };
    assert!(present.contains(&shader_index(&list)));
    assert!(
        !present.contains(&items.start),
        "content must not be drawn directly into the surface"
    );
}

#[test]
fn with_mask_composites_layer_through_mask_alpha() {
    let h = Harness::new(Canvas::new().with_mask(
        |g| {
            g.circle(Point::new(100.0, 100.0), 40.0, Fill::solid(Color::WHITE));
        },
        |g| {
            g.fill(
                Rect::from_xywh(0.0, 0.0, 200.0, 200.0),
                Fill::solid(Color::BLUE),
            );
        },
    ));
    let mask_id = h.runtime().register_shader_id(ApplyMask::NAME);
    h.layout(200.0, 200.0);
    let list = h.paint();

    let targets: Vec<_> = list
        .target_plan()
        .into_iter()
        .filter_map(|node| match node {
            PassNode::Render { target, items, .. } => Some((target, items)),
            _ => None,
        })
        .collect();
    assert_eq!(targets.len(), 2, "mask and layer each need a target");

    let draw = list.shaders().next().expect("mask composite draw");
    assert_eq!(draw.shader, mask_id);
    assert_eq!(ApplyMask::TEXTURES, &["mask", "layer"]);
    assert_eq!(
        draw.textures.as_ref(),
        &[
            (0, TexInput::Target(targets[0].0)),
            (1, TexInput::Target(targets[1].0)),
        ]
    );
}

struct GroupView;

impl View for GroupView {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn is_leaf(&self) -> bool {
        true
    }

    fn paint_self(&self, ctx: &mut PaintCtx) {
        let mut g = Painter::new(ctx);
        let (target, _) = g.group(Size::new(64.0, 32.0), BlendMode::Normal, |inner| {
            inner.fill(
                Rect::from_xywh(0.0, 0.0, 64.0, 32.0),
                Fill::solid(Color::RED),
            );
        });
        g.blit(
            target,
            Rect::from_xywh(0.0, 0.0, 64.0, 32.0),
            [0.0, 0.0, 1.0, 1.0],
        );
    }
}

#[test]
fn group_and_blit_record_target_pass_before_the_blit() {
    let h = Harness::new(GroupView);
    h.layout(200.0, 200.0);
    let list = h.paint();
    assert!(matches!(list.items()[0], RenderItem::TargetBegin { .. }));
    assert!(matches!(list.items()[1], RenderItem::Quad(_)));
    assert!(matches!(list.items()[2], RenderItem::TargetEnd { .. }));
    assert!(matches!(list.items()[3], RenderItem::Blit { .. }));

    let plan = list.target_plan();
    assert_eq!(plan.len(), 2, "{plan:?}");
    let PassNode::Render {
        target,
        size,
        items,
        ..
    } = &plan[0]
    else {
        panic!("expected a Render pass: {plan:?}");
    };
    assert_eq!(*size, Size::new(64.0, 32.0));
    assert_eq!(items.start, 1);
    assert_eq!(items.end, 2);

    let PassNode::Present { items: present } = &plan[1] else {
        panic!("expected a Present pass: {plan:?}");
    };
    assert!(matches!(
        list.items()[present.start],
        RenderItem::Blit { target: id, .. } if id == *target
    ));
}

struct BlendView;

impl View for BlendView {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn is_leaf(&self) -> bool {
        true
    }

    fn paint_self(&self, ctx: &mut PaintCtx) {
        let mut g = Painter::new(ctx);
        g.with_blend(BlendMode::Screen, |g| {
            g.shader_at(
                White,
                Rect::from_xywh(0.0, 0.0, 10.0, 10.0),
                [0.25, 0.5, 0.75, 1.0],
                &[],
            );
        });
        assert_eq!(g.current_blend(), BlendMode::Normal);
    }
}

#[test]
fn with_blend_and_shader_at_set_blend_and_uv() {
    let h = Harness::new(BlendView);
    h.runtime().register_shader_id(White::NAME);
    h.layout(100.0, 100.0);
    let list = h.paint();
    let draw = list.shaders().next().expect("shader draw");
    assert_eq!(draw.blend, BlendMode::Screen);
    assert_eq!(draw.uv, [0.25, 0.5, 0.75, 1.0]);
}

#[test]
fn unregistered_target_shader_draws_nothing() {
    let h = Harness::new(Canvas::new().with_effect(Invert, red_quad));
    h.layout(100.0, 100.0);
    let list = h.paint();
    assert!(list.shaders().next().is_none());
    assert!(list.has_targets(), "content pass is still recorded");
}
