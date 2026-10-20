#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_core::Length;
use sequoia_layout::{
    Align, Axis, ContainerStyle, FlexItem, Justify, distribute_flex, justify_layout, resolve_cross,
    resolve_flex_basis, resolve_main, wrap_lines,
};

fn approx(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}

fn item(base: f32, grow: f32, shrink: f32, min: f32, max: f32) -> FlexItem {
    FlexItem::new(base, grow, shrink, min, max)
}

fn total(sizes: &[f32]) -> f32 {
    sizes.iter().sum()
}

#[test]
fn empty_input_is_a_noop() {
    let mut sizes: [f32; 0] = [];
    distribute_flex(&[], &mut sizes, 10.0, 100.0);
    assert!(sizes.is_empty());

    let lines = wrap_lines(&[], 100.0, 8.0);
    assert!(lines.is_empty());
}

#[test]
fn growth_accounts_for_gaps() {
    let items = [item(100.0, 1.0, 1.0, 0.0, f32::INFINITY); 2];
    let mut sizes = [100.0, 100.0];
    distribute_flex(&items, &mut sizes, 20.0, 300.0);
    assert!(approx(total(&sizes) + 20.0, 300.0), "{sizes:?}");
    assert!(
        approx(sizes[0], 140.0) && approx(sizes[1], 140.0),
        "{sizes:?}"
    );
}

#[test]
fn shrink_accounts_for_gaps() {
    let items = [item(100.0, 0.0, 1.0, 0.0, f32::INFINITY); 2];
    let mut sizes = [100.0, 100.0];
    distribute_flex(&items, &mut sizes, 20.0, 150.0);
    assert!(approx(total(&sizes) + 20.0, 150.0), "{sizes:?}");
    assert!(
        approx(sizes[0], 65.0) && approx(sizes[1], 65.0),
        "{sizes:?}"
    );
}

#[test]
fn shrink_stops_at_min_and_overflows_the_rest() {
    let items = [
        item(100.0, 0.0, 1.0, 80.0, f32::INFINITY),
        item(100.0, 0.0, 1.0, 0.0, f32::INFINITY),
    ];
    let mut sizes = [100.0, 100.0];
    distribute_flex(&items, &mut sizes, 0.0, 100.0);
    assert!(approx(sizes[0], 80.0), "min floor wins: {sizes:?}");
    assert!(
        approx(sizes[1], 20.0),
        "deficit lands on the sibling: {sizes:?}"
    );
    assert!(approx(total(&sizes), 100.0));
}

#[test]
fn grow_that_hits_max_hands_free_space_to_siblings() {
    let items = [
        item(0.0, 1.0, 1.0, 0.0, 50.0),
        item(0.0, 1.0, 1.0, 0.0, f32::INFINITY),
    ];
    let mut sizes = [0.0, 0.0];
    distribute_flex(&items, &mut sizes, 0.0, 300.0);
    assert!(approx(sizes[0], 50.0), "{sizes:?}");
    assert!(approx(sizes[1], 250.0), "{sizes:?}");
    assert!(approx(total(&sizes), 300.0));
}

#[test]
fn non_finite_available_only_clamps_items() {
    let items = [
        item(500.0, 1.0, 1.0, 0.0, 80.0),
        item(10.0, 1.0, 1.0, 0.0, f32::INFINITY),
    ];
    for avail in [f32::INFINITY, f32::NEG_INFINITY, f32::NAN] {
        let mut sizes = [500.0, 10.0];
        distribute_flex(&items, &mut sizes, 0.0, avail);
        assert!(approx(sizes[0], 80.0), "avail={avail}: {sizes:?}");
        assert!(approx(sizes[1], 10.0), "avail={avail}: {sizes:?}");
    }
}

#[test]
fn zero_available_shrinks_to_min_without_going_negative() {
    let items = [
        item(100.0, 0.0, 1.0, 0.0, f32::INFINITY),
        item(100.0, 0.0, 1.0, 10.0, f32::INFINITY),
    ];
    let mut sizes = [100.0, 100.0];
    distribute_flex(&items, &mut sizes, 0.0, 0.0);
    assert!(approx(sizes[0], 0.0), "{sizes:?}");
    assert!(approx(sizes[1], 10.0), "min is not negotiable: {sizes:?}");
}

#[test]
fn negative_available_never_produces_negative_sizes() {
    let items = [item(30.0, 1.0, 1.0, 0.0, f32::INFINITY); 3];
    let mut sizes = [30.0, 30.0, 30.0];
    distribute_flex(&items, &mut sizes, 0.0, -100.0);
    for s in sizes {
        assert!(s >= 0.0, "negative size: {sizes:?}");
    }
}

#[test]
fn min_above_max_collapses_to_max_instead_of_panicking() {
    let items = [item(100.0, 1.0, 1.0, 500.0, 40.0)];
    let mut sizes = [100.0];
    distribute_flex(&items, &mut sizes, 0.0, 200.0);
    assert!(approx(sizes[0], 40.0), "{sizes:?}");
}

#[test]
fn nan_inputs_are_sanitized() {
    let items = [
        item(f32::NAN, f32::NAN, f32::NAN, f32::NAN, f32::NAN),
        item(10.0, -4.0, -1.0, 0.0, f32::INFINITY),
    ];
    let mut sizes = [f32::NAN, 10.0];
    distribute_flex(&items, &mut sizes, f32::NAN, 100.0);
    assert!(sizes.iter().all(|s| s.is_finite()), "{sizes:?}");
    assert!(sizes.iter().all(|s| *s >= 0.0), "{sizes:?}");
}

#[test]
fn single_item_grows_and_shrinks_to_the_line() {
    for avail in [10.0, 250.0] {
        let items = [item(100.0, 1.0, 1.0, 0.0, f32::INFINITY)];
        let mut sizes = [100.0];
        distribute_flex(&items, &mut sizes, 0.0, avail);
        assert!(approx(sizes[0], avail), "avail={avail}: {sizes:?}");
    }
}

#[test]
fn justify_modes_place_series_at_expected_offsets() {
    let extra = 90.0;
    assert_eq!(Justify::Start.distribute(3, 0.0, extra), (0.0, 0.0));
    assert_eq!(Justify::Center.distribute(3, 0.0, extra), (45.0, 0.0));
    assert_eq!(Justify::End.distribute(3, 0.0, extra), (90.0, 0.0));
    assert_eq!(Justify::SpaceBetween.distribute(3, 0.0, extra), (0.0, 45.0));
    assert_eq!(Justify::SpaceAround.distribute(3, 0.0, extra), (15.0, 30.0));
    assert_eq!(Justify::SpaceEvenly.distribute(3, 0.0, extra), (22.5, 22.5));
}

#[test]
fn justify_with_no_children_does_not_divide_by_zero() {
    for justify in [
        Justify::Start,
        Justify::Center,
        Justify::End,
        Justify::SpaceBetween,
        Justify::SpaceAround,
        Justify::SpaceEvenly,
    ] {
        let (start, gap) = justify.distribute(0, 8.0, 100.0);
        assert!(start.is_finite() && gap.is_finite(), "{justify:?}");
        if matches!(
            justify,
            Justify::SpaceBetween | Justify::SpaceAround | Justify::SpaceEvenly
        ) {
            assert_eq!((start, gap), (0.0, 0.0), "{justify:?}");
        }
    }
}

#[test]
fn justify_never_emits_negative_or_non_finite_offsets() {
    for count in 0..=6 {
        for extra in [0.0, 1.0, 90.0, f32::NAN, f32::INFINITY, -50.0] {
            for justify in [
                Justify::Start,
                Justify::Center,
                Justify::End,
                Justify::SpaceBetween,
                Justify::SpaceAround,
                Justify::SpaceEvenly,
            ] {
                let (start, gap) = justify.distribute(count, 0.0, extra);
                assert!(
                    start.is_finite() && gap.is_finite(),
                    "{justify:?} count={count} extra={extra}: ({start}, {gap})"
                );
                assert!(start >= 0.0 && gap >= 0.0, "{justify:?}");
            }
        }
    }
}

#[test]
fn justify_layout_never_returns_negative_space() {
    for avail in [0.0, 50.0, 400.0, f32::INFINITY, f32::NAN] {
        let (start, gap) = justify_layout(Justify::SpaceBetween, 3, 10.0, 500.0, avail);
        assert!(start.is_finite() && gap.is_finite(), "avail={avail}");
        assert!(start >= 0.0 && gap >= 0.0);
    }
}

#[test]
fn align_offsets_stay_inside_the_slack() {
    for avail in [0.0f32, 10.0, 100.0] {
        for child in [0.0f32, 40.0, 400.0] {
            let slack: f32 = (avail - child).max(0.0);
            for align in [Align::Start, Align::Center, Align::End, Align::Stretch] {
                let off = align.offset(avail, child);
                assert!(off.is_finite(), "{align:?}");
                assert!(
                    off >= 0.0 && off <= slack + 0.001,
                    "{align:?} {off} > {slack}"
                );
            }
        }
    }
    assert_eq!(Align::Center.offset(f32::INFINITY, 10.0), 0.0);
}

#[test]
fn axis_split_and_merge_roundtrip() {
    for axis in [Axis::Horizontal, Axis::Vertical] {
        let (main, cross) = axis.split_main_cross(300.0, 100.0);
        let (w, h) = axis.merge_main_cross(main, cross);
        assert_eq!((w, h), (300.0, 100.0), "{axis:?}");
    }
}

#[test]
fn container_gap_sanitizes_and_only_counts_between_children() {
    let c = ContainerStyle::new(Axis::Horizontal, f32::NAN);
    assert_eq!(c.gap, 0.0);
    assert_eq!(c.cross_gap, 0.0);

    let c = ContainerStyle::new(Axis::Horizontal, 8.0);
    assert_eq!(c.main_gaps_total(0), 0.0);
    assert_eq!(c.main_gaps_total(1), 0.0);
    assert_eq!(c.main_gaps_total(4), 24.0);

    let c = c.main_gap(-5.0).cross_gap(f32::INFINITY);
    assert_eq!(c.gap, 0.0);
    assert_eq!(c.cross_gap, 0.0);
}

#[test]
fn wrap_lines_cover_every_child_exactly_once() {
    for count in 0..=12 {
        let sizes: Vec<f32> = (0..count).map(|i| 10.0 + (i as f32 % 5.0) * 15.0).collect();
        for avail in [0.0, 50.0, 120.0, f32::INFINITY] {
            let lines = wrap_lines(&sizes, avail, 4.0);
            if count == 0 {
                assert!(lines.is_empty());
                continue;
            }
            assert_eq!(lines.first().unwrap().0, 0);
            assert_eq!(lines.last().unwrap().1, count);
            for pair in lines.windows(2) {
                assert_eq!(pair[0].1, pair[1].0, "lines must be contiguous: {lines:?}");
            }
            for &(s, e) in &lines {
                assert!(s < e, "empty line {lines:?}");
            }
        }
    }
}

#[test]
fn percent_lengths_fall_back_to_intrinsic_on_unbounded_axes() {
    assert_eq!(
        resolve_main(Length::Percent(50.0), 30.0, f32::INFINITY),
        (30.0, 0.0),
        "percent of an unknown axis cannot be resolved"
    );
    assert_eq!(
        resolve_main(Length::Percent(50.0), 30.0, f32::NAN),
        (30.0, 0.0)
    );
    assert_eq!(
        resolve_main(Length::Percent(50.0), 30.0, 200.0),
        (100.0, 0.0)
    );
    assert_eq!(
        resolve_cross(Length::Percent(50.0), 30.0, f32::INFINITY),
        30.0
    );
    assert_eq!(resolve_cross(Length::Percent(f32::NAN), 30.0, 200.0), 30.0);
    assert_eq!(resolve_cross(Length::Percent(50.0), 30.0, 200.0), 100.0);
    assert_eq!(resolve_cross(Length::Fill(1.0), 30.0, f32::INFINITY), 30.0);
    assert_eq!(resolve_cross(Length::Fill(1.0), 30.0, 200.0), 200.0);
}

#[test]
fn flex_basis_resolution_edge_cases() {
    assert_eq!(
        resolve_flex_basis(None, (40.0, 1.0), 10.0, 200.0),
        (40.0, 1.0),
        "no basis keeps the fallback"
    );
    assert_eq!(
        resolve_flex_basis(Some(Length::Px(-10.0)), (40.0, 0.0), 10.0, 200.0),
        (0.0, 0.0)
    );
    assert_eq!(
        resolve_flex_basis(Some(Length::Percent(25.0)), (40.0, 0.0), 10.0, 200.0),
        (50.0, 0.0)
    );
    assert_eq!(
        resolve_flex_basis(
            Some(Length::Percent(25.0)),
            (40.0, 2.0),
            10.0,
            f32::INFINITY
        ),
        (40.0, 2.0),
        "unbounded percent falls back"
    );
}

#[test]
fn wrap_lines_only_overflow_for_a_single_wider_child() {
    let sizes = [30.0, 30.0, 30.0, 30.0, 30.0];
    for avail in [0.0, 25.0, 61.0, 90.0, 150.0] {
        for gap in [0.0, 5.0, 30.0] {
            let lines = wrap_lines(&sizes, avail, gap);
            for &(s, e) in &lines {
                if e - s <= 1 {
                    continue;
                }
                let used: f32 = sizes[s..e].iter().sum::<f32>() + gap * (e - s - 1) as f32;
                assert!(
                    used <= avail + 0.01,
                    "line {s}..{e} uses {used} > {avail} (gap {gap}): {lines:?}"
                );
            }
        }
    }
}
