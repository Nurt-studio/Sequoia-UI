use std::rc::Rc;

use sequoia_core::{Point, Rect};
use sequoia_draw::{
    Dash, FillRule, LineCap, LineJoin, Path, RasterOptions, ShapeCache, Span, StrokeStyle,
    TOLERANCE, fill, stroke, stroke_path,
};

fn sum_area(spans: &[Span]) -> f32 {
    spans
        .iter()
        .map(|s| s.rect.size.width.max(0.0) * s.rect.size.height.max(0.0) * s.alpha)
        .sum()
}

fn opts() -> RasterOptions {
    RasterOptions::default()
}

fn polygon_circle(center: Point, radius: f32) -> Path {
    let mut p = Path::new();
    p.ellipse(center, radius, radius);
    p
}

#[test]
fn builder_tracks_pen_and_bounds() {
    let mut path = Path::new();
    path.move_to((10.0, 20.0))
        .line_to((110.0, 20.0))
        .line_to((110.0, 70.0))
        .close();
    let bounds = path.bounds();
    assert!((bounds.left() - 10.0).abs() < 1e-4);
    assert!((bounds.top() - 20.0).abs() < 1e-4);
    assert!((bounds.size.width - 100.0).abs() < 1e-4);
    assert!((bounds.size.height - 50.0).abs() < 1e-4);
}

#[test]
fn svg_path_parses_and_round_trips() {
    let path = Path::from_svg("M 0 0 l 10 0 v 10 h -10 Z").expect("valid path");
    let bounds = path.bounds();
    assert!((bounds.size.width - 10.0).abs() < 1e-3, "{bounds:?}");
    assert!((bounds.size.height - 10.0).abs() < 1e-3, "{bounds:?}");

    let text = path.to_svg();
    let again = Path::from_svg(&text).expect("round-trip");
    let b2 = again.bounds();
    assert!((b2.size.width - bounds.size.width).abs() < 1e-3);
    assert!((b2.size.height - bounds.size.height).abs() < 1e-3);

    let arc = Path::from_svg("M 50 0 A 50 50 0 1 1 -50 0 A 50 50 0 1 1 50 0").expect("arc");
    let b = arc.bounds();
    assert!(b.size.width > 98.0 && b.size.height > 98.0, "{b:?}");
    assert!(
        (b.left() + 50.0).abs() < 1.0 && (b.top() + 50.0).abs() < 1.0,
        "{b:?}"
    );

    assert!(Path::from_svg("M 0 0 L").is_err());
}

#[test]
fn flattening_keeps_circle_within_tolerance() {
    let radius = 100.0;
    let path = polygon_circle(Point::ZERO, radius);
    let contour = &path.contours(TOLERANCE)[0];
    assert!(contour.closed);

    assert!(contour.points.len() > 32, "{}", contour.points.len());
    assert!(contour.points.len() < 300, "{}", contour.points.len());
    for p in &contour.points {
        let r = (p.x * p.x + p.y * p.y).sqrt();
        assert!((r - radius).abs() < 0.5, "radius {r} at point {p:?}");
    }
}

#[test]
fn fill_fast_paths_are_single_quad() {
    let mut rect = Path::new();
    rect.rect(Rect::from_xywh(0.0, 0.0, 40.0, 20.0));
    let mut spans = Vec::new();
    let bounds = fill(&rect, FillRule::NonZero, opts(), &mut spans);
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].radius, 0.0);
    assert_eq!(bounds.size.width, 40.0);

    let mut round = Path::new();
    round.rounded_rect(Rect::from_xywh(0.0, 0.0, 40.0, 20.0), 6.0);
    spans.clear();
    fill(&round, FillRule::NonZero, opts(), &mut spans);
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].radius, 6.0);

    let mut disk = Path::new();
    disk.circle((10.0, 10.0), 8.0);
    spans.clear();
    fill(&disk, FillRule::NonZero, opts(), &mut spans);
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].radius, 8.0);
    assert_eq!(spans[0].rect.size.width, 16.0);
}

#[test]
fn fill_of_two_rects_keeps_both() {
    let mut path = Path::new();
    path.rect(Rect::from_xywh(0.0, 0.0, 10.0, 10.0));
    path.rect(Rect::from_xywh(30.0, 0.0, 10.0, 10.0));
    let mut spans = Vec::new();
    let bounds = fill(&path, FillRule::NonZero, opts(), &mut spans);
    assert!((sum_area(&spans) - 200.0).abs() < 1.0);
    assert!((bounds.right() - 40.0).abs() < 0.5);
}

#[test]
fn fill_circle_area_and_no_gaps() {
    let radius = 60.0;
    let path = polygon_circle(Point::ZERO, radius);
    let mut spans = Vec::new();
    let bounds = fill(&path, FillRule::NonZero, opts(), &mut spans);

    let expected = std::f32::consts::PI * radius * radius;
    let area = sum_area(&spans);
    assert!(
        (area - expected).abs() / expected < 0.03,
        "pole {area} vs {expected}"
    );

    assert!(spans.len() < 600, "quads: {}", spans.len());

    for s in &spans {
        assert!(s.rect.size.width > 0.0 && s.rect.size.height > 0.0);
        assert!(s.rect.size.height >= 1.0 - 1e-3, "{:?}", s.rect);
        assert!(s.alpha > 0.0 && s.alpha <= 1.0, "alfa {:?}", s.alpha);
    }

    let mut crisp = Vec::new();
    fill(
        &polygon_circle(Point::ZERO, radius),
        FillRule::NonZero,
        opts().aa_samples(1),
        &mut crisp,
    );
    assert!(
        crisp.len() < spans.len(),
        "sharp variant should be cheaper: {} vs {}",
        crisp.len(),
        spans.len()
    );
    assert!((bounds.left() + radius).abs() < 1.5, "{bounds:?}");
    assert!((bounds.right() - radius).abs() < 1.5, "{bounds:?}");
    assert!((bounds.top() + radius).abs() < 1.5, "{bounds:?}");
    assert!((bounds.bottom() - radius).abs() < 1.5, "{bounds:?}");
}

#[test]
fn fill_rule_nonzero_vs_evenodd() {
    let mut path = Path::new();
    path.rect(Rect::from_xywh(0.0, 0.0, 100.0, 100.0));
    path.rect(Rect::from_xywh(25.0, 25.0, 50.0, 50.0));

    let mut spans = Vec::new();
    fill(&path, FillRule::NonZero, opts(), &mut spans);
    assert!(
        (sum_area(&spans) - 10_000.0).abs() < 1.0,
        "nonzero without a hole"
    );

    spans.clear();
    fill(&path, FillRule::EvenOdd, opts(), &mut spans);
    assert!(
        (sum_area(&spans) - 7_500.0).abs() < 1.0,
        "evenodd with a hole"
    );
}

#[test]
fn fill_of_empty_path_is_zero() {
    let mut spans = Vec::new();
    let bounds = fill(&Path::new(), FillRule::NonZero, opts(), &mut spans);
    assert!(bounds.is_empty());
    assert!(spans.is_empty());
}

#[test]
fn stroke_line_has_exact_extent() {
    let mut line = Path::new();
    line.move_to((0.0, 50.0)).line_to((100.0, 50.0));
    let contours = line.contours(TOLERANCE);

    let mut spans = Vec::new();
    let bounds = stroke(&contours, &StrokeStyle::new(4.0), opts(), &mut spans);
    assert!(
        (sum_area(&spans) - 400.0).abs() < 0.5,
        "{:?}",
        sum_area(&spans)
    );
    assert!((bounds.top() - 48.0).abs() < 0.51, "{bounds:?}");
    assert!((bounds.bottom() - 52.0).abs() < 0.51, "{bounds:?}");
    assert!((bounds.left() - 0.0).abs() < 0.51, "{bounds:?}");
    assert!((bounds.right() - 100.0).abs() < 0.51, "{bounds:?}");
}

#[test]
fn caps_grow_the_line_endings() {
    let mut line = Path::new();
    line.move_to((0.0, 50.0)).line_to((100.0, 50.0));
    let contours = line.contours(TOLERANCE);

    let mut butt = Vec::new();
    stroke(&contours, &StrokeStyle::new(4.0), opts(), &mut butt);

    let mut square = Vec::new();
    let style = StrokeStyle::new(4.0).cap(LineCap::Square);
    stroke(&contours, &style, opts(), &mut square);
    assert!(
        (sum_area(&square) - 416.0).abs() < 0.5,
        "{:?}",
        sum_area(&square)
    );

    let mut round = Vec::new();
    let style = StrokeStyle::round(4.0);
    stroke(&contours, &style, opts(), &mut round);
    let round_area = sum_area(&round);

    assert!(
        (round_area - (400.0 + 4.0 * std::f32::consts::PI)).abs() < 3.0,
        "{round_area}"
    );
}

#[test]
fn dash_splits_the_line() {
    let mut line = Path::new();
    line.move_to((0.0, 50.0)).line_to((100.0, 50.0));
    let contours = line.contours(TOLERANCE);

    let style = StrokeStyle::new(4.0).dash(Dash::new(vec![10.0, 10.0]));
    let mut spans = Vec::new();
    stroke(&contours, &style, opts(), &mut spans);

    assert!(
        (sum_area(&spans) - 200.0).abs() < 2.0,
        "{:?}",
        sum_area(&spans)
    );

    let mut offset_spans = Vec::new();
    let style = StrokeStyle::new(4.0).dash(Dash::new(vec![10.0, 10.0]).offset(5.0));
    stroke(&contours, &style, opts(), &mut offset_spans);
    let left = offset_spans
        .iter()
        .map(|s| s.rect.left())
        .fold(f32::INFINITY, f32::min);
    assert!(
        (left - 0.0).abs() < 0.51,
        "shifted dash starts at 0: {left}"
    );
}

#[test]
fn miter_limit_falls_back_to_bevel() {
    let mut v = Path::new();
    v.move_to((0.0, 0.0))
        .line_to((50.0, 50.0))
        .line_to((100.0, 0.0));
    let contours = v.contours(TOLERANCE);

    let mut sharp = Vec::new();
    let style = StrokeStyle::new(4.0).join(LineJoin::Miter).miter_limit(4.0);
    let sharp_bounds = stroke(&contours, &style, opts(), &mut sharp);

    let mut blunt = Vec::new();
    let style = StrokeStyle::new(4.0).join(LineJoin::Miter).miter_limit(1.0);
    let blunt_bounds = stroke(&contours, &style, opts(), &mut blunt);

    assert!(sharp_bounds.bottom() > 52.0, "{sharp_bounds:?}");

    assert!(
        blunt_bounds.bottom() < sharp_bounds.bottom(),
        "{blunt_bounds:?} vs {sharp_bounds:?}"
    );
    assert!(blunt_bounds.bottom() < 52.5, "{blunt_bounds:?}");

    let mut round = Vec::new();
    let style = StrokeStyle::new(4.0).join(LineJoin::Round);
    stroke(&contours, &style, opts(), &mut round);
    assert!(sum_area(&round) > sum_area(&blunt));
}

#[test]
fn stroke_of_diagonal_makes_a_clean_band() {
    let mut line = Path::new();
    line.move_to((0.0, 0.0)).line_to((100.0, 100.0));
    let mut spans = Vec::new();
    let style = StrokeStyle::new(2.0);
    let bounds = stroke_path(&line, &style, opts(), &mut spans);

    let area = sum_area(&spans);
    assert!((area - 282.8).abs() < 15.0, "{area}");
    assert!((bounds.size.width - 101.4).abs() < 2.0, "{bounds:?}");

    assert!(spans.len() < 600, "quads: {}", spans.len());
    assert!(
        spans.iter().any(|s| s.alpha > 0.05 && s.alpha < 0.95),
        "diagonal edge must have half coverage (AA)"
    );
}

#[test]
fn aa_samples_one_is_crisp_and_cheap() {
    let mut line = Path::new();
    line.move_to((0.0, 0.0)).line_to((100.0, 100.0));
    let style = StrokeStyle::new(2.0);

    let mut smooth = Vec::new();
    stroke_path(&line, &style, opts(), &mut smooth);
    let mut crisp = Vec::new();
    stroke_path(&line, &style, opts().aa_samples(1), &mut crisp);

    assert!(
        crisp.len() < smooth.len(),
        "sharp variant should be cheaper: {} vs {}",
        crisp.len(),
        smooth.len()
    );

    for s in &smooth {
        assert!(
            (s.rect.origin.x * 64.0).round() / 64.0 == s.rect.origin.x,
            "x off grid: {:?}",
            s.rect
        );
        assert!(
            (s.rect.origin.y * 64.0).round() / 64.0 == s.rect.origin.y,
            "y off grid: {:?}",
            s.rect
        );
    }
}

fn shallow_wedge() -> Path {
    let mut p = Path::new();
    p.polygon(&[
        Point::new(0.0, 0.0),
        Point::new(400.0, 10.0),
        Point::new(0.0, 20.0),
    ]);
    p
}

fn pixel_coverage(spans: &[Span]) -> std::collections::HashMap<(i32, i32), f32> {
    let mut map: std::collections::HashMap<(i32, i32), f32> = std::collections::HashMap::new();
    for s in spans {
        let x0 = s.rect.left().round() as i32;
        let x1 = s.rect.right().round() as i32;
        let y0 = s.rect.top().round() as i32;
        let y1 = s.rect.bottom().round() as i32;
        for y in y0..y1 {
            for x in x0..x1 {
                *map.entry((x, y)).or_insert(0.0) += s.alpha;
            }
        }
    }
    map
}

#[test]
fn shallow_edge_is_graded_not_staircased() {
    let path = shallow_wedge();
    let mut spans = Vec::new();
    fill(&path, FillRule::NonZero, opts(), &mut spans);
    let cov = pixel_coverage(&spans);

    assert!((cov.get(&(200, 6)).copied().unwrap_or(0.0) - 1.0).abs() < 0.05);
    assert_eq!(cov.get(&(200, 40)).copied().unwrap_or(0.0), 0.0);

    let partial = cov.values().filter(|a| **a > 0.05 && **a < 0.95).count();

    assert!(partial > 400, "too few partial-coverage pixels: {partial}");

    for y in 0..=19 {
        let row_partial = (0..400)
            .filter(|x| {
                let a = cov.get(&(*x, y)).copied().unwrap_or(0.0);
                a > 0.05 && a < 0.95
            })
            .count();
        assert!(
            row_partial >= 5,
            "row {y}: only {row_partial} partial pixels (step?)"
        );
    }
}

#[test]
fn shallow_continuous_line_is_not_staircased() {
    let mut line = Path::new();
    line.move_to((0.0, 20.0)).line_to((600.0, 30.0));
    let mut spans = Vec::new();
    stroke_path(&line, &StrokeStyle::new(2.0), opts(), &mut spans);
    let cov = pixel_coverage(&spans);

    let mut graded_rows = 0;
    for y in 17..34 {
        let total: f32 = (0..600)
            .map(|x| cov.get(&(x, y)).copied().unwrap_or(0.0))
            .sum();

        if total <= 1.0 {
            continue;
        }
        let partial = (0..600)
            .filter(|x| {
                let a = cov.get(&(*x, y)).copied().unwrap_or(0.0);
                a > 0.05 && a < 0.95
            })
            .count();
        assert!(
            partial >= 3,
            "row {y}: only {partial} partial pixels - edge staircases"
        );
        graded_rows += 1;
    }
    assert!(graded_rows >= 8, "rowy z brzegiem: {graded_rows}");

    let area = sum_area(&spans);
    let expected = (600.0f32 * 600.0 + 10.0 * 10.0).sqrt() * 2.0;
    assert!((area - expected).abs() < 8.0, "pole {area} vs {expected}");
}

#[test]
fn radius_array_order_matches_css() {
    let rect = Rect::from_xywh(0.0, 0.0, 40.0, 40.0);
    let mut path = Path::new();
    path.rounded_rect(rect, [10.0, 0.0, 10.0, 0.0]);
    assert_eq!(path.as_rounded_rect(), None);
    assert_eq!(path.bounds().size.width, 40.0);

    let uniform = sequoia_draw::Radius::from([6.0; 4]);
    assert_eq!(uniform.uniform(), Some(6.0));
    let mut path = Path::new();
    path.rounded_rect(rect, uniform);
    assert_eq!(path.as_rounded_rect(), Some((rect, 6.0)));

    let named = sequoia_draw::Radius::new(1.0, 2.0, 3.0, 4.0);
    assert_eq!(
        (named.tl, named.tr, named.br, named.bl),
        (1.0, 2.0, 3.0, 4.0)
    );
}

#[test]
fn axis_aligned_fill_is_crisp_and_cheap() {
    let mut path = Path::new();
    path.polygon(&[
        Point::new(10.0, 10.0),
        Point::new(210.0, 10.0),
        Point::new(210.0, 110.0),
        Point::new(10.0, 110.0),
    ]);
    let mut spans = Vec::new();
    fill(&path, FillRule::NonZero, opts(), &mut spans);
    assert!(
        spans.iter().all(|s| (s.alpha - 1.0).abs() < 1e-6),
        "axis-aligned shape has no edge pixels"
    );
    assert!(spans.len() <= 4, "quads: {}", spans.len());
    assert!((sum_area(&spans) - 20_000.0).abs() < 1.0);
}

#[test]
fn half_covered_edge_cell_gets_half_alpha() {
    let mut path = Path::new();
    path.polygon(&[
        Point::new(10.5, 0.0),
        Point::new(30.5, 0.0),
        Point::new(30.5, 10.0),
        Point::new(10.5, 10.0),
    ]);
    let mut spans = Vec::new();
    fill(&path, FillRule::NonZero, opts(), &mut spans);

    let left = spans
        .iter()
        .filter(|s| (s.rect.left() - 10.0).abs() < 1e-6)
        .map(|s| (s.alpha, s.rect.size.width, s.rect.size.height))
        .collect::<Vec<_>>();
    assert!(!left.is_empty(), "missing edge cell");
    let weighted: f32 = left.iter().map(|(a, w, h)| a * w * h).sum();
    let full: f32 = left.iter().map(|(_, w, h)| w * h).sum();
    assert!(
        (weighted / full - 0.5).abs() < 0.06,
        "edge cell half covered: {}",
        weighted / full
    );
}

#[test]
fn transformed_circle_keeps_fast_path() {
    let mut circle = Path::new();
    circle.circle((0.0, 0.0), 20.0);
    let t =
        sequoia_draw::Transform::rotate(0.8).then(sequoia_draw::Transform::translate(10.0, 5.0));
    let moved = circle.transformed(t);
    let (center, radius) = moved.as_circle().expect("circle after rotation");
    assert!((radius - 20.0).abs() < 1e-3);
    assert!((center.x - 10.0).abs() < 1e-3);

    let mut spans = Vec::new();
    fill(&moved, FillRule::NonZero, opts(), &mut spans);
    assert_eq!(spans.len(), 1);
}

#[test]
fn contains_follows_the_fill_rule() {
    let mut circle = Path::new();
    circle.circle((50.0, 50.0), 20.0);
    assert!(circle.contains(Point::new(50.0, 50.0), FillRule::NonZero));
    assert!(!circle.contains(Point::new(50.0, 95.0), FillRule::NonZero));

    let mut rings = Path::new();
    rings.rect(Rect::from_xywh(0.0, 0.0, 100.0, 100.0));
    rings.rect(Rect::from_xywh(25.0, 25.0, 50.0, 50.0));
    assert!(rings.contains(Point::new(50.0, 50.0), FillRule::NonZero));
    assert!(!rings.contains(Point::new(50.0, 50.0), FillRule::EvenOdd));
    assert!(rings.contains(Point::new(5.0, 5.0), FillRule::EvenOdd));
}

#[test]
fn cache_reuses_geometry_and_counts_hits() {
    let cache = ShapeCache::new(4);
    let mut path = Path::new();
    path.circle((0.0, 0.0), 10.0);

    let first = cache.fill(&path, FillRule::NonZero, opts());
    let second = cache.fill(&path, FillRule::NonZero, opts());
    assert!(Rc::ptr_eq(&first, &second));
    assert_eq!(cache.stats().hits, 1);
    assert_eq!(cache.stats().misses, 1);
    assert!((cache.stats().hit_rate() - 0.5).abs() < 1e-6);
    assert_eq!(first.len(), 1);

    let stroke_shape = cache.stroke(&path, &StrokeStyle::new(2.0), opts());
    assert!(!Rc::ptr_eq(&first, &stroke_shape));
    assert_eq!(cache.stats().misses, 2);
}

#[test]
fn cache_ignores_color_and_evicts_old_entries() {
    let cache = ShapeCache::new(1);
    let mut a = Path::new();
    a.rect(Rect::from_xywh(0.0, 0.0, 4.0, 4.0));
    let mut b = Path::new();
    b.rect(Rect::from_xywh(10.0, 0.0, 4.0, 4.0));

    cache.fill(&a, FillRule::NonZero, opts());
    cache.fill(&b, FillRule::NonZero, opts());
    assert!(cache.len() <= 1);
    assert_eq!(cache.stats().evictions, 1);

    let cache = ShapeCache::new(8);
    cache.fill(&a, FillRule::NonZero, opts());
    cache.fill(&a, FillRule::NonZero, opts().row_height(0.5));
    assert_eq!(cache.stats().misses, 2);
}

#[test]
fn options_for_scale_tightens_rows() {
    let base = RasterOptions::default();
    let hidpi = base.for_scale(2.0);
    assert!((hidpi.row_height - 0.5).abs() < 1e-6);
    assert!(hidpi.tolerance < base.tolerance);
    assert!(hidpi.merge_tol < base.merge_tol);

    let weird = base.for_scale(0.0);
    assert!((weird.row_height - 1.0).abs() < 1e-6);
}

#[test]
fn arc_on_empty_path_starts_with_move_not_line_from_origin() {
    use sequoia_draw::PathSeg;
    let mut p = Path::new();
    p.arc((231.0f32, 745.0f32), 8.0, 0.0, 359.9);
    for s in p.segs() {
        if let PathSeg::Line(pt) = s {
            assert!(
                pt.x.abs() > 1e-3 || pt.y.abs() > 1e-3,
                "kreska z originu: {:?}",
                p.segs()
            );
        }
    }
    assert!(
        matches!(p.segs().first(), Some(PathSeg::Move(_))),
        "arc starts with Move: {:?}",
        p.segs()
    );
}
