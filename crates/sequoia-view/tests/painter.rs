#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_core::prelude::*;
use sequoia_text::TextSystem;
use sequoia_view::prelude::*;

fn paint(view: impl View + 'static, w: f32, h: f32) -> RenderList {
    let rt = Runtime::new();
    rt.mount(view);
    let mut text = TextSystem::new();
    rt.layout(Rect::from_xywh(0.0, 0.0, w, h), &mut text);
    let mut list = RenderList::new();
    rt.paint(&mut list, &mut text);
    list
}

struct Nested {
    fill: Fill,
}

impl View for Nested {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn is_leaf(&self) -> bool {
        true
    }

    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle::default()
            .width(Length::Px(100.0))
            .height(Length::Px(60.0))
    }

    fn paint_self(&self, ctx: &mut PaintCtx) {
        let mut g = Painter::new(ctx);
        g.fill(Rect::from_xywh(5.0, 5.0, 20.0, 20.0), self.fill.clone());
    }
}

#[test]
fn local_rect_plus_origin_once() {
    struct Pad;
    impl View for Pad {
        fn body(&self) -> impl View + 'static {
            Column::new().padding(10.0).child(Nested {
                fill: Fill::solid(Color::RED),
            })
        }
    }
    let list = paint(Pad, 200.0, 200.0);
    let quads: Vec<_> = list.quads().collect();
    assert_eq!(quads.len(), 1, "1 rectangle = 1 quad: {quads:?}");
    assert_eq!(
        quads[0].rect,
        Rect::from_xywh(15.0, 15.0, 20.0, 20.0),
        "lokalne (5,5) + origin (10,10) — raz, nie zero/dwa razy"
    );
}

#[test]
fn gradient_is_single_gpu_item() {
    struct Grad;
    impl View for Grad {
        fn body(&self) -> impl View + 'static {
            Unit
        }
        fn is_leaf(&self) -> bool {
            true
        }
        fn layout_style(&self) -> LayoutStyle {
            LayoutStyle::default()
                .width(Length::Px(100.0))
                .height(Length::Px(40.0))
        }
        fn paint_self(&self, ctx: &mut PaintCtx) {
            let mut g = Painter::new(ctx);
            g.fill(
                Rect::from_xywh(0.0, 0.0, 100.0, 40.0),
                Fill::horizontal(GradientStops::two(Color::RED, Color::BLUE)),
            );
        }
    }
    let list = paint(Grad, 100.0, 40.0);
    assert_eq!(list.gradient_count(), 1, "gradient = 1 prymityw GPU");
    assert_eq!(list.quads().count(), 0, "zero stripes 1px");
    let g = list.gradients().next().expect("gradient");
    assert_eq!(g.rect, Rect::from_xywh(0.0, 0.0, 100.0, 40.0));
}

#[test]
fn radial_gradient_fallback_draws_concentric_circles() {
    struct RG;
    impl View for RG {
        fn body(&self) -> impl View + 'static {
            Unit
        }
        fn is_leaf(&self) -> bool {
            true
        }
        fn layout_style(&self) -> LayoutStyle {
            LayoutStyle::default()
                .width(Length::Px(100.0))
                .height(Length::Px(100.0))
        }
        fn paint_self(&self, ctx: &mut PaintCtx) {
            let mut g = Painter::new(ctx);
            let stops = GradientStops::new(vec![
                CoreGradientStop::new(0.0, Color::RED),
                CoreGradientStop::new(0.25, Color::GREEN),
                CoreGradientStop::new(0.5, Color::BLUE),
                CoreGradientStop::new(0.75, Color::WHITE),
                CoreGradientStop::new(1.0, Color::BLACK),
            ]);
            g.circle(Point::new(50.0, 50.0), 50.0, Fill::radial(stops));
        }
    }

    let list = paint(RG, 100.0, 100.0);
    assert_eq!(list.gradient_count(), 0, "fallback uses bands");
    let quads: Vec<_> = list.quads().collect();
    assert_eq!(quads.len(), 64);
    for q in &quads {
        let half = q.rect.size.width * 0.5;
        assert!((q.rect.size.width - q.rect.size.height).abs() < 0.01);
        assert!((q.radius - half).abs() < 0.01, "band must be a disc");
        assert!((q.rect.left() + half - 50.0).abs() < 0.01);
        assert!((q.rect.top() + half - 50.0).abs() < 0.01);
    }
    let outer = quads.first().expect("outer band");
    let inner = quads.last().expect("inner band");
    assert!((outer.rect.size.width - 100.0).abs() < 0.01);
    assert!((inner.rect.size.width - 100.0 / 64.0).abs() < 0.01);
}

#[test]
fn rounded_gradient_keeps_radius_and_corners() {
    struct RGrad;
    impl View for RGrad {
        fn body(&self) -> impl View + 'static {
            Unit
        }
        fn is_leaf(&self) -> bool {
            true
        }
        fn layout_style(&self) -> LayoutStyle {
            LayoutStyle::default()
                .width(Length::Px(100.0))
                .height(Length::Px(40.0))
        }
        fn paint_self(&self, ctx: &mut PaintCtx) {
            let mut g = Painter::new(ctx);
            g.rounded(
                Rect::from_xywh(0.0, 0.0, 100.0, 40.0),
                8.0,
                Fill::horizontal(GradientStops::two(Color::RED, Color::BLUE)),
            );
        }
    }
    let list = paint(RGrad, 100.0, 40.0);

    assert_eq!(list.gradient_count(), 1);
    let g = list.gradients().next().expect("gradient");
    assert_eq!(g.radius, 8.0, "radius on instance, not flattened");
}

#[test]
fn empty_and_transparent_paint_nothing() {
    struct Empty;
    impl View for Empty {
        fn body(&self) -> impl View + 'static {
            Unit
        }
        fn is_leaf(&self) -> bool {
            true
        }
        fn paint_self(&self, ctx: &mut PaintCtx) {
            let mut g = Painter::new(ctx);

            g.fill(Rect::ZERO, Fill::solid(Color::RED));
            g.fill(
                Rect::from_xywh(0.0, 0.0, 10.0, 10.0),
                Fill::solid(Color::TRANSPARENT),
            );
            g.circle(Point::new(5.0, 5.0), 0.0, Fill::solid(Color::RED));
            g.circle(Point::new(5.0, 5.0), -3.0, Fill::solid(Color::RED));
        }
    }
    let list = paint(Empty, 100.0, 100.0);
    assert!(list.is_empty(), "nic nie maluje: {:?}", list.items());
}

#[test]
fn clip_scope_pops_after_closure() {
    struct Clipped;
    impl View for Clipped {
        fn body(&self) -> impl View + 'static {
            Unit
        }
        fn is_leaf(&self) -> bool {
            true
        }
        fn layout_style(&self) -> LayoutStyle {
            LayoutStyle::default()
                .width(Length::Px(100.0))
                .height(Length::Px(100.0))
        }
        fn paint_self(&self, ctx: &mut PaintCtx) {
            let depth = ctx.list_clip_depth();
            {
                let mut g = Painter::new(ctx);
                g.clipped(Rect::from_xywh(10.0, 10.0, 20.0, 20.0), |_| {});
            }
            assert_eq!(
                ctx.list_clip_depth(),
                depth,
                "scope took down clip (old push_clip without pop leaked)"
            );
        }
    }
    let _ = paint(Clipped, 100.0, 100.0);
}

#[test]
fn knob_is_two_circles() {
    struct Knob;
    impl View for Knob {
        fn body(&self) -> impl View + 'static {
            Unit
        }
        fn is_leaf(&self) -> bool {
            true
        }
        fn layout_style(&self) -> LayoutStyle {
            LayoutStyle::default()
                .width(Length::Px(100.0))
                .height(Length::Px(100.0))
        }
        fn paint_self(&self, ctx: &mut PaintCtx) {
            let mut g = Painter::new(ctx);
            g.knob(Point::new(50.0, 50.0), 7.0, Color::RED);
        }
    }
    let list = paint(Knob, 100.0, 100.0);
    assert_eq!(list.quads().count(), 2, "ring + filling");
}

#[test]
fn curve_paints_wire_and_degenerate_is_noop() {
    struct Wire;
    impl View for Wire {
        fn body(&self) -> impl View + 'static {
            Unit
        }
        fn is_leaf(&self) -> bool {
            true
        }
        fn layout_style(&self) -> LayoutStyle {
            LayoutStyle::default()
                .width(Length::Px(100.0))
                .height(Length::Px(40.0))
        }
        fn paint_self(&self, ctx: &mut PaintCtx) {
            let mut g = Painter::new(ctx);
            let r = g.curve(
                Point::new(5.0, 20.0),
                Point::new(35.0, 20.0),
                Point::new(65.0, 20.0),
                Point::new(95.0, 20.0),
                BrushStroke::round(2.5),
                Color::RED,
            );
            assert!(!r.is_empty(), "the curve paints something");

            let p = Point::new(10.0, 10.0);
            g.curve(p, p, p, p, BrushStroke::round(2.5), Color::RED);
        }
    }
    let list = paint(Wire, 100.0, 40.0);
    assert!(
        list.quads().count() >= 1,
        "obrys krzywej to spany: {}",
        list.quads().count()
    );
}

#[test]
fn polyline_needs_two_points() {
    struct One;
    impl View for One {
        fn body(&self) -> impl View + 'static {
            Unit
        }
        fn is_leaf(&self) -> bool {
            true
        }
        fn layout_style(&self) -> LayoutStyle {
            LayoutStyle::default()
                .width(Length::Px(100.0))
                .height(Length::Px(40.0))
        }
        fn paint_self(&self, ctx: &mut PaintCtx) {
            let mut g = Painter::new(ctx);
            let r = g.polyline(
                &[Point::new(10.0, 10.0)],
                BrushStroke::round(2.0),
                Color::RED,
            );
            assert_eq!(r, Rect::ZERO, "1 point is not enough");
        }
    }
    let list = paint(One, 100.0, 40.0);
    assert_eq!(list.quads().count(), 0, "nic nie namalowane");
}

#[test]
fn marquee_covers_drag_rect() {
    struct Marq;
    impl View for Marq {
        fn body(&self) -> impl View + 'static {
            Unit
        }
        fn is_leaf(&self) -> bool {
            true
        }
        fn layout_style(&self) -> LayoutStyle {
            LayoutStyle::default()
                .width(Length::Px(100.0))
                .height(Length::Px(60.0))
        }
        fn paint_self(&self, ctx: &mut PaintCtx) {
            let mut g = Painter::new(ctx);

            let r = g.marquee(
                Point::new(60.0, 40.0),
                Point::new(10.0, 10.0),
                Fill::solid(Color::rgba(1.0, 1.0, 1.0, 0.2)),
                Color::RED,
            );
            assert_eq!(r, Rect::from_xywh(10.0, 10.0, 50.0, 30.0));
            let z = g.marquee(
                Point::new(5.0, 5.0),
                Point::new(5.0, 5.0),
                Fill::solid(Color::RED),
                Color::RED,
            );
            assert_eq!(z, Rect::ZERO, "zdegenerowane to no-op");
        }
    }
    let list = paint(Marq, 100.0, 60.0);
    assert!(
        list.quads().count() >= 2,
        "fill + stroke: {}",
        list.quads().count()
    );
}

#[test]
fn clipped_rounded_culls_outside() {
    struct Clipped;
    impl View for Clipped {
        fn body(&self) -> impl View + 'static {
            Unit
        }
        fn is_leaf(&self) -> bool {
            true
        }
        fn layout_style(&self) -> LayoutStyle {
            LayoutStyle::default()
                .width(Length::Px(100.0))
                .height(Length::Px(100.0))
        }
        fn paint_self(&self, ctx: &mut PaintCtx) {
            let depth = ctx.list_clip_depth();
            {
                let mut g = Painter::new(ctx);
                g.clipped_rounded(Rect::from_xywh(10.0, 10.0, 20.0, 20.0), 4.0, |g| {
                    g.fill(
                        Rect::from_xywh(100.0, 100.0, 10.0, 10.0),
                        Fill::solid(Color::RED),
                    );
                });
            }
            assert_eq!(ctx.list_clip_depth(), depth, "scope took off clip");
        }
    }
    let list = paint(Clipped, 100.0, 100.0);
    assert_eq!(list.quads().count(), 0, "poza clipem nic nie ma");
}

#[test]
fn text_measure_helpers_are_sane() {
    use sequoia_text::FontWeight;
    struct Probe;
    impl View for Probe {
        fn body(&self) -> impl View + 'static {
            Unit
        }
        fn is_leaf(&self) -> bool {
            true
        }
        fn layout_style(&self) -> LayoutStyle {
            LayoutStyle::default()
                .width(Length::Px(100.0))
                .height(Length::Px(40.0))
        }
        fn paint_self(&self, ctx: &mut PaintCtx) {
            let metrics = ctx.text_metrics("hi", 14.0);
            assert_eq!(metrics, ctx.text_metrics("hi", 14.0));
            assert!(metrics.width.is_finite() && metrics.width >= 0.0);
            assert!(metrics.height.is_finite() && metrics.height >= 0.0);

            let mut g = Painter::new(ctx);
            assert_eq!(g.text_width("", 14.0), 0.0);
            assert_eq!(g.text_width("hi", 0.0), 0.0);
            let w = g.text_width("hi", 14.0);
            assert!(w.is_finite() && w >= 0.0);
            let ww = g.text_width_weighted("hi", 14.0, FontWeight::BOLD);
            assert!(ww.is_finite() && ww >= 0.0);

            g.text_center_x("hi", 50.0, 5.0, 14.0, Color::RED);
            g.text_center(
                Rect::from_xywh(0.0, 0.0, 100.0, 40.0),
                "hi",
                14.0,
                Color::RED,
            );

            g.text_styled(
                "hi",
                Point::new(5.0, 20.0),
                14.0,
                FontWeight::BOLD,
                Color::RED,
            );
        }
    }
    let _ = paint(Probe, 100.0, 40.0);
}

#[test]
fn with_transform_maps_world_once() {
    struct T;
    impl View for T {
        fn body(&self) -> impl View + 'static {
            Unit
        }
        fn is_leaf(&self) -> bool {
            true
        }
        fn layout_style(&self) -> LayoutStyle {
            LayoutStyle::default()
                .width(Length::Px(100.0))
                .height(Length::Px(60.0))
        }
        fn paint_self(&self, ctx: &mut PaintCtx) {
            let mut g = Painter::new(ctx);
            g.with_transform(Transform::translate(10.0, 5.0), |g| {
                g.fill(
                    Rect::from_xywh(5.0, 5.0, 20.0, 20.0),
                    Fill::solid(Color::RED),
                );
            });
        }
    }
    let list = paint(T, 200.0, 200.0);
    let quads: Vec<_> = list.quads().collect();
    assert_eq!(quads.len(), 1);
    assert_eq!(quads[0].rect, Rect::from_xywh(15.0, 10.0, 20.0, 20.0));
}

#[test]
fn with_transform_nests_by_composition() {
    struct T;
    impl View for T {
        fn body(&self) -> impl View + 'static {
            Unit
        }
        fn is_leaf(&self) -> bool {
            true
        }
        fn layout_style(&self) -> LayoutStyle {
            LayoutStyle::default()
                .width(Length::Px(100.0))
                .height(Length::Px(60.0))
        }
        fn paint_self(&self, ctx: &mut PaintCtx) {
            let mut g = Painter::new(ctx);
            g.with_transform(Transform::translate(10.0, 0.0), |g| {
                g.with_transform(Transform::translate(0.0, 7.0), |g| {
                    g.fill(
                        Rect::from_xywh(5.0, 5.0, 20.0, 20.0),
                        Fill::solid(Color::RED),
                    );
                });
            });
        }
    }
    let list = paint(T, 200.0, 200.0);
    let quads: Vec<_> = list.quads().collect();
    assert_eq!(quads.len(), 1);
    assert_eq!(quads[0].rect, Rect::from_xywh(15.0, 12.0, 20.0, 20.0));
}

#[test]
fn unproject_inverts_view_transform() {
    struct T;
    impl View for T {
        fn body(&self) -> impl View + 'static {
            Unit
        }
        fn is_leaf(&self) -> bool {
            true
        }
        fn layout_style(&self) -> LayoutStyle {
            LayoutStyle::default()
                .width(Length::Px(100.0))
                .height(Length::Px(60.0))
        }
        fn paint_self(&self, ctx: &mut PaintCtx) {
            let mut g = Painter::new(ctx);
            g.with_transform(Transform::translate(10.0, 5.0), |g| {
                assert_eq!(
                    g.unproject(Point::new(25.0, 15.0)),
                    Some(Point::new(15.0, 10.0))
                );
            });

            g.with_transform(Transform::scale(0.0, 0.0), |g| {
                assert_eq!(g.unproject(Point::new(1.0, 1.0)), None);
            });
        }
    }
    let _ = paint(T, 200.0, 200.0);
}

#[test]
fn hit_shapes_share_paint_and_hit_geometry() {
    let c = HitCircle::new(Point::new(10.0, 10.0), 5.0);
    assert!(c.contains(Point::new(10.0, 10.0), 0.0));
    assert!(c.contains(Point::new(15.0, 10.0), 0.0), "edge is hit");
    assert!(!c.contains(Point::new(16.0, 10.0), 0.0));
    assert!(c.contains(Point::new(16.0, 10.0), 1.0), "tolerancja chwytu");

    let r = HitRect::new(Rect::from_xywh(0.0, 0.0, 20.0, 10.0), 2.0);
    assert!(r.contains(Point::new(5.0, 5.0), 0.0));
    assert!(!r.contains(Point::new(25.0, 5.0), 0.0));
    assert!(
        r.contains(Point::new(25.0, 5.0), 6.0),
        "tolerance on all sides"
    );
}
