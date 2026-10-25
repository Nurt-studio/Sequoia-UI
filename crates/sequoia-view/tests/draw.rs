#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_core::prelude::*;
use sequoia_text::TextSystem;
use sequoia_view::prelude::*;
use sequoia_view::{FillRule, Path, StrokeStyle};

fn rgb(color: Color) -> (f32, f32, f32) {
    (color.r, color.g, color.b)
}

struct DrawDemo {
    rule: FillRule,
}

impl View for DrawDemo {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn is_leaf(&self) -> bool {
        true
    }

    fn paint_self(&self, ctx: &mut PaintCtx) {
        let _ = ctx.rect;
        let mut star = Path::new();
        star.polygon(&[
            Point::new(20.0, 0.0),
            Point::new(26.0, 14.0),
            Point::new(40.0, 14.0),
            Point::new(28.0, 24.0),
            Point::new(33.0, 40.0),
            Point::new(20.0, 30.0),
            Point::new(7.0, 40.0),
            Point::new(12.0, 24.0),
            Point::new(0.0, 14.0),
            Point::new(14.0, 14.0),
        ]);
        ctx.fill_rule(&star, self.rule, Color::WHITE);

        ctx.line(
            Point::new(0.0, 50.0),
            Point::new(40.0, 50.0),
            2.0,
            Color::RED,
        );

        let mut circle = Path::new();
        circle.circle((60.0, 20.0), 10.0);
        ctx.stroke(&circle, &StrokeStyle::new(2.0), Color::GREEN);
    }
}

#[test]
fn paint_ctx_draws_paths_and_line() {
    let rt = Runtime::new();
    rt.mount(DrawDemo {
        rule: FillRule::NonZero,
    });
    let mut text = TextSystem::new();
    rt.layout(Rect::from_xywh(0.0, 0.0, 200.0, 200.0), &mut text);
    let mut list = RenderList::new();
    rt.paint(&mut list, &mut text);

    let quads: Vec<_> = list.quads().collect();
    assert!(quads.len() >= 3, "quads: {}", quads.len());

    let white = quads
        .iter()
        .filter(|q| rgb(q.color) == rgb(Color::WHITE))
        .count();
    assert!(white > 1, "star should give several spans, got {white}");
    for q in quads.iter().filter(|q| rgb(q.color) == rgb(Color::WHITE)) {
        assert!(
            q.rect.right() <= 41.0 && q.rect.bottom() <= 41.0,
            "{:?}",
            q.rect
        );
    }

    let red: Vec<_> = quads
        .iter()
        .filter(|q| rgb(q.color) == rgb(Color::RED))
        .collect();
    assert_eq!(red.len(), 1, "{red:?}");
    assert!((red[0].rect.size.height - 2.0).abs() < 0.51);
}

#[test]
fn draw_offset_moves_geometry_without_reuse_loss() {
    struct OffsetView;
    impl View for OffsetView {
        fn body(&self) -> impl View + 'static {
            Unit
        }

        fn is_leaf(&self) -> bool {
            true
        }

        fn paint_self(&self, ctx: &mut PaintCtx) {
            let mut circle = Path::new();
            circle.circle((0.0, 0.0), 5.0);
            ctx.translate(30.0, 40.0);
            ctx.fill(&circle, Color::BLUE);
        }
    }

    let rt = Runtime::new();
    rt.mount(OffsetView);
    let mut text = TextSystem::new();
    rt.layout(Rect::from_xywh(0.0, 0.0, 100.0, 100.0), &mut text);
    let mut list = RenderList::new();
    rt.paint(&mut list, &mut text);

    let quad = list.quads().next().expect("circle quad");
    assert_eq!(quad.rect.size, Size::new(10.0, 10.0));
    assert!((quad.rect.left() - 25.0).abs() < 1e-3);
    assert!((quad.rect.top() - 35.0).abs() < 1e-3);
    assert_eq!(quad.radius, 5.0);

    let mut list2 = RenderList::new();
    rt.paint(&mut list2, &mut text);
    let stats = rt.shapes().stats();
    assert_eq!(stats.misses, 1, "{stats:?}");
    assert!(stats.hits >= 1, "{stats:?}");
}

#[test]
fn fill_rule_changes_geometry_from_cache() {
    let rt = Runtime::new();
    rt.mount(DrawDemo {
        rule: FillRule::EvenOdd,
    });
    let mut text = TextSystem::new();
    rt.layout(Rect::from_xywh(0.0, 0.0, 200.0, 200.0), &mut text);
    let mut list = RenderList::new();
    rt.paint(&mut list, &mut text);

    let stats = rt.shapes().stats();
    assert_eq!(stats.misses, 3, "{stats:?}");
    assert!(!rt.shapes().is_empty());
}

#[test]
fn horizontal_gradient_makes_abutting_bands() {
    struct Card;
    impl View for Card {
        fn body(&self) -> impl View + 'static {
            Unit
        }
        fn is_leaf(&self) -> bool {
            true
        }
        fn paint_self(&self, ctx: &mut PaintCtx) {
            let gradient = Gradient::linear(
                (0.0, 0.0),
                (100.0, 0.0),
                vec![
                    GradientStop::new(0.0, Color::RED),
                    GradientStop::new(1.0, Color::BLUE),
                ],
            );
            ctx.rounded_rect_gradient(Rect::from_xywh(0.0, 0.0, 100.0, 40.0), 8.0, &gradient);
        }
    }

    let rt = Runtime::new();
    rt.mount(Card);
    let mut text = TextSystem::new();
    rt.layout(Rect::from_xywh(0.0, 0.0, 100.0, 40.0), &mut text);
    let mut list = RenderList::new();
    rt.paint(&mut list, &mut text);

    let all: Vec<_> = list.quads().collect();
    assert!(all.len() > 20, "quads: {}", all.len());

    let mut rows: std::collections::HashMap<i32, Vec<_>> = std::collections::HashMap::new();
    for q in &all {
        rows.entry((q.rect.top() * 64.0).round() as i32)
            .or_default()
            .push(q);
    }
    let widest = rows.values_mut().max_by_key(|row| row.len()).expect("row");
    widest.sort_by(|a, b| a.rect.left().partial_cmp(&b.rect.left()).unwrap());
    assert!(widest.len() > 20, "bands in a row: {}", widest.len());

    let left = widest.first().expect("pasmo");
    let right = widest.last().expect("pasmo");
    assert!(left.color.r > 0.9 && left.color.b < 0.2, "{:?}", left.color);
    assert!(
        right.color.b > 0.9 && right.color.r < 0.2,
        "{:?}",
        right.color
    );

    for pair in widest.windows(2) {
        let (a, b) = (&pair[0], &pair[1]);
        let seam = a.rect.right() - b.rect.left();
        assert!(
            seam.abs() <= 0.5 + 1e-3,
            "seam: {:?} vs {:?} (offset {seam}, max overlap 0.5)",
            a.rect,
            b.rect
        );
    }

    for pair in widest.windows(2) {
        assert!(pair[1].color.r <= pair[0].color.r + 1e-3);
        assert!(pair[1].color.b >= pair[0].color.b - 1e-3);
    }
}

#[test]
fn vertical_gradient_keeps_alpha_edges() {
    struct Column;
    impl View for Column {
        fn body(&self) -> impl View + 'static {
            Unit
        }
        fn is_leaf(&self) -> bool {
            true
        }
        fn paint_self(&self, ctx: &mut PaintCtx) {
            let gradient = Gradient::linear(
                (0.0, 0.0),
                (0.0, 40.0),
                vec![
                    GradientStop::new(0.0, Color::WHITE),
                    GradientStop::new(
                        1.0,
                        Color {
                            a: 0.0,
                            ..Color::WHITE
                        },
                    ),
                ],
            );
            let mut path = Path::new();
            path.rect(Rect::from_xywh(0.0, 0.0, 30.0, 40.0));
            ctx.fill_gradient(&path, &gradient);
        }
    }

    let rt = Runtime::new();
    rt.mount(Column);
    let mut text = TextSystem::new();
    rt.layout(Rect::from_xywh(0.0, 0.0, 30.0, 40.0), &mut text);
    let mut list = RenderList::new();
    rt.paint(&mut list, &mut text);

    let quads: Vec<_> = list.quads().collect();
    assert!(quads.len() > 10, "pasm: {}", quads.len());

    let top = quads
        .iter()
        .min_by(|a, b| a.rect.top().total_cmp(&b.rect.top()))
        .unwrap();
    let bottom = quads
        .iter()
        .max_by(|a, b| a.rect.top().total_cmp(&b.rect.top()))
        .unwrap();
    assert!(top.color.a > 0.9, "{:?}", top.color);
    assert!(bottom.color.a < 0.1, "{:?}", bottom.color);
    assert!(
        quads
            .iter()
            .all(|q| (q.rect.size.width - 30.0).abs() < 1e-3)
    );
}

#[test]
fn outer_shadow_is_one_primitive_with_offset_and_blur() {
    struct Card;
    impl View for Card {
        fn body(&self) -> impl View + 'static {
            Unit
        }
        fn is_leaf(&self) -> bool {
            true
        }
        fn paint_self(&self, ctx: &mut PaintCtx) {
            ctx.shadow(
                Rect::from_xywh(0.0, 0.0, 100.0, 60.0),
                8.0,
                12.0,
                Point::new(0.0, 4.0),
                Color {
                    a: 0.4,
                    ..Color::BLACK
                },
            );
        }
    }

    let rt = Runtime::new();
    rt.mount(Card);
    let mut text = TextSystem::new();
    rt.layout(Rect::from_xywh(0.0, 0.0, 160.0, 120.0), &mut text);
    let mut list = RenderList::new();
    rt.paint(&mut list, &mut text);

    assert!(
        list.quads().next().is_none(),
        "shadow is one primitive, not a layer stack"
    );
    let shadows: Vec<_> = list.shadows().collect();
    assert_eq!(shadows.len(), 1, "single shadow primitive");
    let s = shadows[0];
    assert_eq!(
        s.rect,
        Rect::from_xywh(0.0, 0.0, 100.0, 60.0),
        "caster shape"
    );
    assert_eq!(s.radius, 8.0);
    assert_eq!(s.blur, 12.0);
    assert_eq!(s.spread, 0.0);
    assert_eq!(s.offset, Vec2::new(0.0, 4.0));
    assert!(!s.inset, "outer shadow");
    assert_eq!(s.color.a, 0.4);

    let b = s.bounds();
    assert!((b.left() - (-6.0)).abs() < 1e-3, "{b:?}");
    assert!((b.top() - (-2.0)).abs() < 1e-3, "{b:?}");
    assert!((b.size.width - 112.0).abs() < 1e-3, "{b:?}");
}

#[test]
fn inner_shadow_is_inset_and_stays_inside_the_shape() {
    struct Card;
    impl View for Card {
        fn body(&self) -> impl View + 'static {
            Unit
        }
        fn is_leaf(&self) -> bool {
            true
        }
        fn paint_self(&self, ctx: &mut PaintCtx) {
            ctx.inner_shadow(
                Rect::from_xywh(10.0, 10.0, 80.0, 40.0),
                4.0,
                10.0,
                Point::new(0.0, 2.0),
                Color {
                    a: 0.5,
                    ..Color::BLACK
                },
            );
        }
    }

    let rt = Runtime::new();
    rt.mount(Card);
    let mut text = TextSystem::new();
    rt.layout(Rect::from_xywh(0.0, 0.0, 160.0, 120.0), &mut text);
    let mut list = RenderList::new();
    rt.paint(&mut list, &mut text);

    let s = *list.shadows().next().expect("inner shadow");
    assert!(s.inset, "inset");
    assert_eq!(s.rect, Rect::from_xywh(10.0, 10.0, 80.0, 40.0));
    assert_eq!(s.blur, 10.0);
    assert_eq!(s.offset, Vec2::new(0.0, 2.0));
    assert_eq!(s.bounds(), s.rect, "inner shadow stays inside the shape");
}

#[test]
fn invisible_shadows_paint_nothing() {
    struct Card;
    impl View for Card {
        fn body(&self) -> impl View + 'static {
            Unit
        }
        fn is_leaf(&self) -> bool {
            true
        }
        fn paint_self(&self, ctx: &mut PaintCtx) {
            let r = Rect::from_xywh(0.0, 0.0, 40.0, 20.0);

            ctx.shadow(r, 4.0, 0.0, Point::ZERO, Color::BLACK);
            ctx.inner_shadow(r, 4.0, 8.0, Point::ZERO, Color::TRANSPARENT);
            ctx.shadow(Rect::ZERO, 4.0, 8.0, Point::ZERO, Color::BLACK);
        }
    }

    let rt = Runtime::new();
    rt.mount(Card);
    let mut text = TextSystem::new();
    rt.layout(Rect::from_xywh(0.0, 0.0, 80.0, 60.0), &mut text);
    let mut list = RenderList::new();
    rt.paint(&mut list, &mut text);
    assert!(list.is_empty(), "paints nothing: {:?}", list.items());
}

#[test]
fn convex_shapes_are_single_quads() {
    struct Shapes;
    impl View for Shapes {
        fn body(&self) -> impl View + 'static {
            Unit
        }

        fn is_leaf(&self) -> bool {
            true
        }

        fn paint_self(&self, ctx: &mut PaintCtx) {
            ctx.disk(Point::new(10.0, 10.0), 8.0, Color::RED);
            ctx.rounded_rect(Rect::from_xywh(0.0, 30.0, 40.0, 20.0), 4.0, Color::GREEN);
            ctx.fill_rect(Rect::from_xywh(0.0, 60.0, 20.0, 10.0), Color::BLUE);
            let _ = ctx.draw_svg("M 0 80 L 10 80 L 10 90 Z", None, Color::WHITE);
            let _ = ctx.draw_svg(
                "M 0 80 L 10 80 L 10 90",
                Some(&StrokeStyle::new(1.0)),
                Color::RED,
            );
        }
    }

    let rt = Runtime::new();
    rt.mount(Shapes);
    let mut text = TextSystem::new();
    rt.layout(Rect::from_xywh(0.0, 0.0, 100.0, 100.0), &mut text);
    let mut list = RenderList::new();
    rt.paint(&mut list, &mut text);

    let quads: Vec<_> = list.quads().collect();

    assert!(
        quads
            .iter()
            .filter(|q| rgb(q.color) == rgb(Color::RED))
            .count()
            >= 2
    );
    let disk = quads
        .iter()
        .find(|q| q.radius > 0.0 && rgb(q.color) == rgb(Color::RED))
        .expect("circle");
    assert_eq!(disk.radius, 8.0);
    assert_eq!(disk.rect.size, Size::new(16.0, 16.0));

    let rounded = quads
        .iter()
        .find(|q| q.radius > 0.0 && rgb(q.color) == rgb(Color::GREEN))
        .expect("rounded rect");
    assert_eq!(rounded.radius, 4.0);

    let svg_quads: Vec<_> = quads
        .iter()
        .filter(|q| rgb(q.color) == rgb(Color::WHITE))
        .collect();
    assert!(!svg_quads.is_empty(), "SVG triangle did not paint");
    for q in &svg_quads {
        assert!(
            q.rect.left() >= 0.0
                && q.rect.right() <= 11.0
                && q.rect.top() >= 79.0
                && q.rect.bottom() <= 91.0,
            "{:?}",
            q.rect
        );
    }
}

struct Toolkit;

impl View for Toolkit {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn is_leaf(&self) -> bool {
        true
    }

    fn paint_self(&self, ctx: &mut PaintCtx) {
        let mut ring = Path::new();
        ring.circle((40.0, 40.0), 20.0);
        let sweep = Gradient::linear(
            (20.0, 40.0),
            (60.0, 40.0),
            vec![
                GradientStop::new(0.0, Color::RED),
                GradientStop::new(1.0, Color::BLUE),
            ],
        );
        ctx.stroke_gradient(&ring, &StrokeStyle::new(4.0), &sweep);

        ctx.rounded_rect_corners(
            Rect::from_xywh(0.0, 70.0, 60.0, 30.0),
            [20.0, 0.0, 20.0, 0.0],
            Color::GREEN,
        );

        ctx.sector(Point::new(120.0, 40.0), 20.0, 0.0, 90.0, Color::WHITE);
        ctx.donut_sector(
            Point::new(120.0, 100.0),
            12.0,
            20.0,
            0.0,
            180.0,
            Color::WHITE,
        );

        ctx.dotted_line(
            Point::new(0.0, 130.0),
            Point::new(80.0, 130.0),
            4.0,
            10.0,
            Color::BLACK,
        );
    }
}

fn paint(view: impl View + 'static, size: (f32, f32)) -> Vec<sequoia_core::Quad> {
    let rt = Runtime::new();
    rt.mount(view);
    let mut text = TextSystem::new();
    rt.layout(Rect::from_xywh(0.0, 0.0, size.0, size.1), &mut text);
    let mut list = RenderList::new();
    rt.paint(&mut list, &mut text);
    list.quads().copied().collect()
}

#[test]
fn stroke_gradient_paints_both_ends() {
    let quads = paint(Toolkit, (200.0, 200.0));
    let reds = quads.iter().filter(|q| rgb(q.color) == rgb(Color::RED));
    let blues = quads.iter().filter(|q| rgb(q.color) == rgb(Color::BLUE));
    assert!(reds.count() > 0, "missing left end of outline gradient");
    assert!(blues.count() > 0, "missing right end of outline gradient");
}

#[test]
fn per_corner_radii_are_tessellated_not_single_quad() {
    let quads = paint(Toolkit, (200.0, 200.0));
    let green: Vec<_> = quads
        .iter()
        .filter(|q| rgb(q.color) == rgb(Color::GREEN))
        .collect();
    assert!(!green.is_empty(), "missing rounded-corner rect");

    assert!(
        green.iter().all(|q| q.radius == 0.0),
        "uneven radius took the fast path"
    );
    assert!(green.len() > 3, "corner quads: {}", green.len());
    for q in &green {
        assert!(
            q.rect.top() >= 70.0 && q.rect.bottom() <= 101.0,
            "{:?}",
            q.rect
        );
    }
}

#[test]
fn sector_covers_its_quadrant_only() {
    let quads = paint(Toolkit, (200.0, 200.0));
    let wedge: Vec<_> = quads
        .iter()
        .filter(|q| {
            rgb(q.color) == rgb(Color::WHITE) && q.rect.top() < 70.0 && q.rect.left() > 100.0
        })
        .collect();
    assert!(!wedge.is_empty(), "missing slice");
    for q in &wedge {
        assert!(
            q.rect.left() >= 119.0 && q.rect.right() <= 141.0,
            "{:?}",
            q.rect
        );
        assert!(
            q.rect.top() >= 39.0 && q.rect.bottom() <= 61.0,
            "{:?}",
            q.rect
        );
    }
}

#[test]
fn donut_sector_leaves_the_hole_empty() {
    let quads = paint(Toolkit, (200.0, 200.0));
    let ring: Vec<_> = quads
        .iter()
        .filter(|q| rgb(q.color) == rgb(Color::WHITE) && q.rect.top() > 95.0)
        .collect();
    assert!(!ring.is_empty(), "missing ring");

    let hole = Point::new(120.0, 106.0);
    for q in &ring {
        let covers = hole.x >= q.rect.left()
            && hole.x <= q.rect.right()
            && hole.y >= q.rect.top()
            && hole.y <= q.rect.bottom();
        assert!(!covers, "ring hole painted: {:?}", q.rect);
    }
    for q in &ring {
        assert!(
            q.rect.right() <= 141.0 && q.rect.bottom() <= 121.0,
            "{:?}",
            q.rect
        );
    }
}

#[test]
fn dotted_line_makes_one_disk_per_step() {
    let quads = paint(Toolkit, (200.0, 200.0));
    let dots: Vec<_> = quads
        .iter()
        .filter(|q| rgb(q.color) == rgb(Color::BLACK))
        .collect();

    assert_eq!(dots.len(), 9, "kropek: {}", dots.len());
    for d in &dots {
        assert_eq!(d.radius, 2.0);
        assert_eq!(d.rect.size, Size::new(4.0, 4.0));
        assert!((d.rect.top() - 128.0).abs() < 1e-3, "{:?}", d.rect);
    }
}

#[test]
fn crisp_line_lands_on_a_pixel_boundary() {
    struct Hairline;
    impl View for Hairline {
        fn body(&self) -> impl View + 'static {
            Unit
        }
        fn is_leaf(&self) -> bool {
            true
        }
        fn paint_self(&self, ctx: &mut PaintCtx) {
            ctx.line_crisp(
                Point::new(0.0, 50.0),
                Point::new(80.0, 50.0),
                1.0,
                Color::RED,
            );
        }
    }

    let quads = paint(Hairline, (100.0, 100.0));
    let red: Vec<_> = quads
        .iter()
        .filter(|q| rgb(q.color) == rgb(Color::RED))
        .collect();
    assert_eq!(red.len(), 1, "{red:?}");
    assert!((red[0].rect.top() - 50.0).abs() < 1e-3, "{:?}", red[0].rect);
    assert!(
        (red[0].rect.bottom() - 51.0).abs() < 1e-3,
        "{:?}",
        red[0].rect
    );
    assert!(
        (red[0].color.a - 1.0).abs() < 1e-3,
        "krycie {:?}",
        red[0].color
    );
}

#[test]
fn plain_line_snaps_integer_hairlines() {
    struct Hairline;
    impl View for Hairline {
        fn body(&self) -> impl View + 'static {
            Unit
        }
        fn is_leaf(&self) -> bool {
            true
        }
        fn paint_self(&self, ctx: &mut PaintCtx) {
            ctx.line(
                Point::new(0.0, 50.0),
                Point::new(80.0, 50.0),
                1.0,
                Color::RED,
            );
        }
    }

    let quads = paint(Hairline, (100.0, 100.0));
    let red: Vec<_> = quads
        .iter()
        .filter(|q| rgb(q.color) == rgb(Color::RED))
        .collect();
    assert_eq!(red.len(), 1, "{red:?}");
    assert!((red[0].rect.top() - 50.0).abs() < 1e-3, "{:?}", red[0].rect);
    assert!(
        (red[0].rect.bottom() - 51.0).abs() < 1e-3,
        "{:?}",
        red[0].rect
    );
    assert!((red[0].color.a - 1.0).abs() < 1e-3, "{:?}", red[0].color);
}

#[test]
fn crisp_line_preserves_half_pixel_hairline() {
    struct Hairline;
    impl View for Hairline {
        fn body(&self) -> impl View + 'static {
            Unit
        }
        fn is_leaf(&self) -> bool {
            true
        }
        fn paint_self(&self, ctx: &mut PaintCtx) {
            ctx.line_crisp(
                Point::new(0.0, 50.5),
                Point::new(80.0, 50.5),
                1.0,
                Color::RED,
            );
        }
    }

    let quads = paint(Hairline, (100.0, 100.0));
    let red: Vec<_> = quads
        .iter()
        .filter(|q| rgb(q.color) == rgb(Color::RED))
        .collect();
    assert_eq!(red.len(), 1, "{red:?}");
    assert!((red[0].rect.top() - 50.0).abs() < 1e-3, "{:?}", red[0].rect);
    assert!(
        (red[0].rect.bottom() - 51.0).abs() < 1e-3,
        "{:?}",
        red[0].rect
    );
    assert!((red[0].color.a - 1.0).abs() < 1e-3, "{:?}", red[0].color);
}

#[test]
fn crisp_line_leaves_diagonals_alone() {
    struct Diagonal;
    impl View for Diagonal {
        fn body(&self) -> impl View + 'static {
            Unit
        }
        fn is_leaf(&self) -> bool {
            true
        }
        fn paint_self(&self, ctx: &mut PaintCtx) {
            ctx.line_crisp(
                Point::new(0.0, 0.0),
                Point::new(40.0, 40.0),
                1.0,
                Color::RED,
            );
        }
    }

    let quads = paint(Diagonal, (60.0, 60.0));
    let red: Vec<_> = quads
        .iter()
        .filter(|q| rgb(q.color) == rgb(Color::RED))
        .collect();
    assert!(!red.is_empty());

    assert!(
        red.iter().any(|q| q.color.a > 0.05 && q.color.a < 0.95),
        "missing partial coverage on diagonal"
    );
    let left = red
        .iter()
        .map(|q| q.rect.left())
        .fold(f32::INFINITY, f32::min);
    let right = red
        .iter()
        .map(|q| q.rect.right())
        .fold(f32::NEG_INFINITY, f32::max);
    let top = red
        .iter()
        .map(|q| q.rect.top())
        .fold(f32::INFINITY, f32::min);
    let bottom = red
        .iter()
        .map(|q| q.rect.bottom())
        .fold(f32::NEG_INFINITY, f32::max);

    assert!(
        (right - left) > 38.0 && (bottom - top) > 38.0,
        "bbox {left}/{top} — {right}/{bottom}"
    );
}
