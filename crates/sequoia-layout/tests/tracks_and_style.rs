#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_core::{Edges, Length, Rect, Size};
use sequoia_layout::{
    Dock, DockStyle, GridFlow, GridPlacement, GridStyle, Indents, Inset, LayoutStyle, Overflow,
    Position, Track, apply_aspect_ratio, auto_cells, dock_layout_order, dock_preview_rect,
    dock_thickness, effective_dock, inset_rect, resolve_dock, resolve_tracks,
    resolve_tracks_clamped, sanitize_frac, sanitize_preview_frac, shrink_by_margin,
    shrink_by_padding, split_axis_for_dock, track_offsets,
};

fn approx(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}

#[test]
fn empty_track_list_is_empty() {
    let sizes = resolve_tracks(&[], &[], 400.0, 10.0);
    assert!(sizes.is_empty());
    assert!(track_offsets(&[], 10.0).is_empty());
}

#[test]
fn fixed_tracks_are_not_shrunk_to_fit() {
    let sizes = resolve_tracks(
        &[Track::Px(300.0), Track::Px(200.0), Track::Fill(1.0)],
        &[0.0; 3],
        100.0,
        0.0,
    );
    assert!(approx(sizes[0], 300.0), "{sizes:?}");
    assert!(approx(sizes[1], 200.0), "{sizes:?}");
    assert!(approx(sizes[2], 0.0), "no space left for fill: {sizes:?}");
}

#[test]
fn fill_capped_by_max_hands_space_to_the_next_fill() {
    let sizes = resolve_tracks_clamped(
        &[Track::Fill(1.0), Track::Fill(1.0)],
        &[0.0, 0.0],
        100.0,
        0.0,
        None,
        Some(&[30.0, f32::INFINITY]),
    );
    assert!(approx(sizes[0], 30.0), "capped at max: {sizes:?}");
    assert!(
        approx(sizes[1], 70.0),
        "leftover goes to the sibling: {sizes:?}"
    );
    assert!(approx(sizes.iter().sum::<f32>(), 100.0));
}

#[test]
fn fill_with_equal_caps_leaves_the_overflow_unused() {
    let sizes = resolve_tracks_clamped(
        &[Track::Fill(1.0), Track::Fill(1.0)],
        &[0.0, 0.0],
        100.0,
        0.0,
        None,
        Some(&[30.0, 30.0]),
    );
    assert!(
        approx(sizes[0], 30.0) && approx(sizes[1], 30.0),
        "{sizes:?}"
    );
}

#[test]
fn minmax_fill_uses_min_as_floor_then_shares_the_rest() {
    let sizes = resolve_tracks(
        &[Track::minmax(80.0, Track::Fill(1.0)), Track::Fill(1.0)],
        &[0.0, 0.0],
        100.0,
        0.0,
    );
    assert!(
        approx(sizes[0], 90.0),
        "floor + half of the rest: {sizes:?}"
    );
    assert!(approx(sizes[1], 10.0), "{sizes:?}");
    assert!(approx(sizes.iter().sum::<f32>(), 100.0));
}

#[test]
fn clamped_mins_and_maxs_apply_to_fixed_tracks() {
    let sizes = resolve_tracks_clamped(
        &[Track::Px(50.0), Track::Px(50.0)],
        &[0.0, 0.0],
        400.0,
        0.0,
        Some(&[120.0, 0.0]),
        Some(&[100.0, 40.0]),
    );
    assert!(approx(sizes[0], 100.0), "cap wins over floor: {sizes:?}");
    assert!(approx(sizes[1], 40.0), "min 0, max 40: {sizes:?}");
}

#[test]
fn fill_tracks_respect_gaps_when_splitting() {
    let sizes = resolve_tracks(
        &[Track::Fill(1.0), Track::Fill(2.0)],
        &[0.0, 0.0],
        110.0,
        10.0,
    );
    assert!(approx(sizes[0], 100.0 / 3.0), "{sizes:?}");
    assert!(approx(sizes[1], 200.0 / 3.0), "{sizes:?}");
    assert!(approx(sizes.iter().sum::<f32>() + 10.0, 110.0));
}

#[test]
fn unbounded_fill_track_uses_content() {
    let sizes = resolve_tracks(
        &[Track::Fill(1.0), Track::Px(20.0)],
        &[64.0, 0.0],
        f32::INFINITY,
        0.0,
    );
    assert!(approx(sizes[0], 64.0), "{sizes:?}");
    assert!(approx(sizes[1], 20.0), "{sizes:?}");
}

#[test]
fn track_offsets_ignore_leading_or_trailing_gaps() {
    let offsets = track_offsets(&[40.0, 20.0], 6.0);
    assert_eq!(offsets, vec![0.0, 46.0]);

    let single = track_offsets(&[40.0], 6.0);
    assert_eq!(single, vec![0.0]);
}

#[test]
fn nested_minmax_collapses_to_its_inner_max() {
    let t = Track::minmax(10.0, Track::minmax(20.0, Track::Fill(2.0)));
    assert!(t.is_fill_like());
    assert_eq!(t.fill_weight(), 2.0);
    assert_eq!(t.min_floor(), 10.0);
}

#[test]
fn auto_cells_row_flow_fills_rows_then_wraps() {
    let cells = auto_cells(5, 2, GridFlow::Row);
    assert_eq!(
        cells,
        vec![(0, 0), (1, 0), (0, 1), (1, 1), (0, 2)],
        "row-major placement"
    );
}

#[test]
fn auto_cells_column_flow_fills_columns_then_wraps() {
    let cells = auto_cells(4, 2, GridFlow::Column);
    assert_eq!(
        cells,
        vec![(0, 0), (0, 1), (1, 0), (1, 1)],
        "column-major placement"
    );
}

#[test]
fn auto_cells_never_leaves_the_declared_columns() {
    for count in 0..=24 {
        for ncols in 1..=4 {
            for flow in [GridFlow::Row, GridFlow::Column] {
                let cells = auto_cells(count, ncols, flow);
                assert_eq!(cells.len(), count);
                let mut seen = std::collections::HashSet::new();
                for &(c, r) in &cells {
                    assert!(c < ncols, "count={count} ncols={ncols} {flow:?}: {cells:?}");
                    assert!(seen.insert((c, r)), "duplicate cell in {cells:?}");
                }
            }
        }
    }
}

#[test]
fn grid_placement_span_is_never_zero() {
    let p = GridPlacement::AUTO.span(0, 0);
    assert_eq!(p.col_span, 1);
    assert_eq!(p.row_span, 1);
    assert!(GridPlacement::AUTO.is_auto());
    assert!(!GridPlacement::AUTO.column(2).is_auto());
    assert!(!GridPlacement::AUTO.span(2, 1).is_auto());
}

#[test]
fn grid_style_sanitizes_gaps() {
    let g = GridStyle::new([Track::Fill(1.0)])
        .gap(f32::NAN)
        .column_gap(-4.0)
        .row_gap(f32::INFINITY);
    assert_eq!(g.column_gap, 0.0);
    assert_eq!(g.row_gap, 0.0);
}

#[test]
fn box_model_shrinks_never_go_negative() {
    for padding in [Edges::all(0.0), Edges::all(10.0), Edges::all(500.0)] {
        for size in [Size::ZERO, Size::new(10.0, 10.0), Size::new(200.0, 80.0)] {
            let p = shrink_by_padding(size, padding);
            let m = shrink_by_margin(size, padding);
            assert!(p.width >= 0.0 && p.height >= 0.0, "{p:?}");
            assert!(m.width >= 0.0 && m.height >= 0.0, "{m:?}");
            let rect = inset_rect(Rect::from_xywh(0.0, 0.0, size.width, size.height), padding);
            assert!(
                rect.size.width >= 0.0 && rect.size.height >= 0.0,
                "{rect:?}"
            );
        }
    }
}

#[test]
fn box_model_offsets_content_by_padding() {
    let rect = Rect::from_xywh(10.0, 20.0, 100.0, 60.0);
    let inner = inset_rect(rect, Edges::only(4.0, 6.0, 8.0, 10.0));
    assert!(approx(inner.left(), 14.0));
    assert!(approx(inner.top(), 26.0));
    assert!(approx(inner.size.width, 88.0));
    assert!(approx(inner.size.height, 44.0));
}

#[test]
fn aspect_ratio_ignores_invalid_ratios() {
    for ratio in [
        None,
        Some(0.0),
        Some(-2.0),
        Some(f32::NAN),
        Some(f32::INFINITY),
    ] {
        assert_eq!(
            apply_aspect_ratio(100.0, 50.0, true, false, ratio),
            (100.0, 50.0)
        );
        assert_eq!(
            apply_aspect_ratio(100.0, 50.0, false, true, ratio),
            (100.0, 50.0)
        );
    }
}

#[test]
fn aspect_ratio_only_derives_the_auto_axis() {
    assert_eq!(
        apply_aspect_ratio(30.0, 40.0, true, false, Some(2.0)),
        (80.0, 40.0)
    );
    assert_eq!(
        apply_aspect_ratio(30.0, 40.0, false, true, Some(2.0)),
        (30.0, 15.0)
    );
    assert_eq!(
        apply_aspect_ratio(0.0, 40.0, true, true, Some(2.0)),
        (80.0, 40.0),
        "width derived from the known height"
    );
    assert_eq!(
        apply_aspect_ratio(30.0, 0.0, true, true, Some(2.0)),
        (30.0, 15.0),
        "height derived from the known width"
    );
    assert_eq!(
        apply_aspect_ratio(0.0, 0.0, true, true, Some(2.0)),
        (0.0, 0.0)
    );
}

#[test]
fn style_defaults_are_neutral() {
    let s = LayoutStyle::default();
    assert_eq!(s.width, Length::Fit);
    assert_eq!(s.height, Length::Fit);
    assert_eq!(s.flex, 0.0);
    assert_eq!(s.flex_shrink, 1.0);
    assert_eq!(s.min_width, 0.0);
    assert_eq!(s.max_width, f32::INFINITY);
    assert_eq!(s.min_height, 0.0);
    assert_eq!(s.max_height, f32::INFINITY);
    assert!(s.aspect_ratio.is_none());
    assert_eq!(s.position, Position::Flow);
    assert_eq!(s.overflow, Overflow::Visible);
    assert!(s.hit_testable);
    assert!(s.indents().is_none());
}

#[test]
fn style_builders_sanitize_weights_and_ratios() {
    let s = LayoutStyle::default()
        .flex(f32::NAN)
        .flex_grow(-1.0)
        .flex_shrink(f32::NAN)
        .aspect(f32::NAN)
        .indent_all(-10.0);
    assert_eq!(s.flex, 0.0);
    assert_eq!(s.flex_shrink, 1.0, "NaN shrink means default shrink");
    assert!(s.aspect_ratio.is_none());
    assert_eq!(s.indents(), Indents::all(0.0));

    let s = LayoutStyle::default().flex_shrink(-1.0);
    assert_eq!(
        s.flex_shrink, 0.0,
        "negative shrink is treated as no shrink"
    );

    let s = LayoutStyle::default().margin(Edges::all(-8.0));
    assert_eq!(s.margin, Edges::ZERO);
}

#[test]
fn style_min_above_max_collapses_to_max() {
    let s = LayoutStyle::default().min_width(200.0).max_width(100.0);
    assert_eq!(s.clamp_width(150.0), 100.0);
    assert_eq!(s.clamp_size(Size::new(150.0, 80.0)).width, 100.0);

    let s = LayoutStyle::default().min_height(200.0).max_height(100.0);
    assert_eq!(s.clamp_height(150.0), 100.0);
}

#[test]
fn style_clamp_primitives_keep_non_finite_values_sane() {
    let s = LayoutStyle::default()
        .min_width(f32::NAN)
        .max_width(f32::NAN)
        .min_height(-10.0)
        .max_height(f32::INFINITY);
    assert_eq!(s.min_width, 0.0);
    assert_eq!(s.max_width, f32::INFINITY);
    assert_eq!(s.min_height, 0.0);
    assert_eq!(s.max_height, f32::INFINITY);
    assert_eq!(s.clamp_width(f32::INFINITY), f32::INFINITY);
}

#[test]
fn style_axis_helpers_follow_the_axis() {
    let s = LayoutStyle::default()
        .margin_x(4.0)
        .margin_y(6.0)
        .indent_left(1.0)
        .indent_right(2.0)
        .indent_top(3.0)
        .indent_bottom(4.0);
    assert_eq!(s.margin_main(sequoia_layout::Axis::Horizontal), 8.0);
    assert_eq!(s.margin_main(sequoia_layout::Axis::Vertical), 12.0);
    assert_eq!(s.margin_cross(sequoia_layout::Axis::Horizontal), 12.0);
    assert_eq!(
        s.indent_main_start(sequoia_layout::Axis::Horizontal),
        Some(1.0)
    );
    assert_eq!(
        s.indent_main_end(sequoia_layout::Axis::Horizontal),
        Some(2.0)
    );
    assert_eq!(
        s.indent_cross_start(sequoia_layout::Axis::Horizontal),
        Some(3.0)
    );
    assert_eq!(
        s.indent_cross_end(sequoia_layout::Axis::Horizontal),
        Some(4.0)
    );
    assert_eq!(s.outer_size(Size::new(50.0, 40.0)), Size::new(58.0, 52.0));
}

#[test]
fn style_indent_helpers_sanitize_and_can_be_cleared() {
    let s = LayoutStyle::default().indent_all(f32::NAN);
    assert_eq!(s.indent_top, Some(0.0));
    assert!(s.clear_indents().indents().is_none());

    let s = LayoutStyle::default().indent(Indents::only(Some(-2.0), None, Some(3.0), None));
    assert_eq!(s.indent_left, Some(0.0));
    assert_eq!(s.indent_right, Some(3.0));
    assert_eq!(s.indent_top, None);
}

#[test]
fn style_absolute_builders_are_consistent() {
    let s = LayoutStyle::default().absolute_at(10.0, 20.0);
    assert!(s.is_absolute());
    assert_eq!(s.absolute_inset_of(), Some(Inset::xy(10.0, 20.0)));
    assert_eq!(LayoutStyle::default().absolute_inset_of(), None);

    let s = LayoutStyle::default().absolute();
    assert_eq!(s.absolute_inset_of(), Some(Inset::AUTO));

    let inset = Inset::only(Some(4.0), None, Some(8.0), None);
    assert_eq!(inset.left, Some(4.0));
    assert_eq!(inset.top, None);
    assert_eq!(inset.right, Some(8.0));
    assert_eq!(inset.bottom, None);
    assert_eq!(Inset::only(None, None, None, None), Inset::AUTO);
}

#[test]
fn overflow_flags_stay_consistent() {
    assert!(!Overflow::Visible.clips());
    assert!(!Overflow::Visible.is_scrollable());
    assert!(Overflow::Hidden.clips());
    assert!(!Overflow::Hidden.is_scrollable());
    for o in [Overflow::Scroll, Overflow::Auto] {
        assert!(o.clips(), "{o:?}");
        assert!(o.is_scrollable(), "{o:?}");
        assert!(!o.is_visible(), "{o:?}");
    }
}

#[test]
fn dock_thickness_edge_cases() {
    assert_eq!(dock_thickness(Length::Px(f32::NAN), 40.0, 400.0), 40.0);
    assert_eq!(dock_thickness(Length::Px(f32::INFINITY), 40.0, 400.0), 40.0);
    assert_eq!(dock_thickness(Length::Percent(-10.0), 40.0, 400.0), 0.0);
    assert_eq!(dock_thickness(Length::Percent(f32::NAN), 40.0, 400.0), 40.0);
    assert_eq!(dock_thickness(Length::Fit, 40.0, 400.0), 40.0);
    assert_eq!(dock_thickness(Length::Fill(2.0), 40.0, 400.0), 40.0);
}

#[test]
fn dock_order_is_a_permutation_and_keeps_groups_stable() {
    use Dock::*;
    let docks = [Fill, Right, Top, Left, Bottom];
    let order = dock_layout_order(&docks, false, false, docks.len());
    let mut sorted = order.clone();
    sorted.sort_unstable();
    assert_eq!(sorted, vec![0, 1, 2, 3, 4]);
    assert_eq!(
        order,
        vec![2, 4, 1, 3, 0],
        "top/bottom, then sides in declaration order, then fill"
    );

    let identity = dock_layout_order(&docks, false, true, docks.len());
    assert_eq!(
        identity,
        vec![0, 1, 2, 3, 4],
        "ordered keeps declaration order"
    );
}

#[test]
fn dock_order_with_zero_children_is_empty() {
    let order = dock_layout_order(&[Dock::Left], true, false, 0);
    assert!(order.is_empty());
}

#[test]
fn resolve_dock_respects_ordered_flag() {
    let docks = [Dock::Left, Dock::Left];
    assert_eq!(resolve_dock(&docks, true, false, 1, 2), Dock::Left);
    assert_eq!(resolve_dock(&docks, true, true, 1, 2), Dock::Fill);
    assert_eq!(resolve_dock(&[], false, true, 0, 1), Dock::Fill);
    assert_eq!(effective_dock(&docks, false, 5, 2), Dock::Fill);
}

#[test]
fn dock_style_builders_sanitize() {
    let d = DockStyle::new([Dock::Left]).gap(f32::NAN);
    assert_eq!(d.gap, 0.0);
    let d = d.gap(-4.0);
    assert_eq!(d.gap, 0.0);
    assert!(
        !d.clone().border(true).ordered,
        "border mode is not sequential order"
    );
    assert!(d.clone().border(false).ordered);
    assert!(d.ordered(true).ordered);
}

#[test]
fn dock_preview_rect_covers_exactly_the_preview_fraction() {
    let host = Rect::from_xywh(100.0, 50.0, 400.0, 300.0);
    let left = dock_preview_rect(host, Dock::Left, 0.25);
    assert!(approx(left.left(), 100.0));
    assert!(approx(left.size.width, 100.0));

    let right = dock_preview_rect(host, Dock::Right, 0.25);
    assert!(approx(right.right(), 500.0));
    assert!(approx(right.size.width, 100.0));

    let top = dock_preview_rect(host, Dock::Top, 0.5);
    assert!(approx(top.top(), 50.0));
    assert!(approx(top.size.height, 150.0));

    let bottom = dock_preview_rect(host, Dock::Bottom, 0.5);
    assert!(approx(bottom.bottom(), 350.0));
    assert!(approx(bottom.size.height, 150.0));

    assert_eq!(dock_preview_rect(host, Dock::Fill, 0.3), host);
}

#[test]
fn dock_preview_rect_clamps_the_fraction() {
    let host = Rect::from_xywh(0.0, 0.0, 100.0, 100.0);
    assert!(approx(
        dock_preview_rect(host, Dock::Left, 5.0).size.width,
        85.0
    ));
    assert!(approx(
        dock_preview_rect(host, Dock::Left, -5.0).size.width,
        15.0
    ));
    assert!(approx(
        dock_preview_rect(host, Dock::Left, f32::NAN).size.width,
        50.0
    ));
    assert_eq!(sanitize_frac(f32::INFINITY), 0.5);
    assert_eq!(sanitize_preview_frac(f32::INFINITY), 0.5);
}

#[test]
fn dock_split_axis_maps_every_side() {
    use sequoia_layout::Axis;
    assert_eq!(
        split_axis_for_dock(Dock::Left),
        Some((Axis::Horizontal, true))
    );
    assert_eq!(
        split_axis_for_dock(Dock::Right),
        Some((Axis::Horizontal, false))
    );
    assert_eq!(split_axis_for_dock(Dock::Top), Some((Axis::Vertical, true)));
    assert_eq!(
        split_axis_for_dock(Dock::Bottom),
        Some((Axis::Vertical, false))
    );
    assert_eq!(split_axis_for_dock(Dock::Fill), None);
}
