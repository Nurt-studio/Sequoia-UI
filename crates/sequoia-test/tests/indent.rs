#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_test::Harness;
use sequoia_view::prelude::*;
use sequoia_view::{Indents, LayoutStyle};

fn cells(h: &Harness, root: ElementId) -> Vec<sequoia_core::Rect> {
    h.children(root)
        .iter()
        .map(|c| h.rect(*c).expect("rect"))
        .collect()
}

#[test]
fn row_indent_left_overrides_gap() {
    let h = Harness::new(
        Row::new()
            .gap(10.0)
            .child(Panel::new().size(50.0, 20.0))
            .child(Panel::new().size(50.0, 20.0).indent_left(30.0))
            .child(Panel::new().size(50.0, 20.0)),
    );
    h.layout(400.0, 100.0);
    let r = cells(&h, h.root().unwrap());
    assert_eq!(r.len(), 3);
    assert!((r[0].origin.x - 0.0).abs() < 0.01, "{r:?}");

    assert!((r[1].origin.x - 80.0).abs() < 0.5, "{r:?}");

    assert!((r[2].origin.x - 140.0).abs() < 0.5, "{r:?}");
}

#[test]
fn row_indent_right_shifts_next() {
    let h = Harness::new(
        Row::new()
            .gap(10.0)
            .child(Panel::new().size(50.0, 20.0).indent_right(25.0))
            .child(Panel::new().size(50.0, 20.0)),
    );
    h.layout(400.0, 100.0);
    let r = cells(&h, h.root().unwrap());
    assert!((r[1].origin.x - (50.0 + 25.0 + 10.0)).abs() < 0.5, "{r:?}");
}

#[test]
fn row_indent_zero_collapses_gap() {
    let h = Harness::new(
        Row::new()
            .gap(10.0)
            .child(Panel::new().size(50.0, 20.0))
            .child(Panel::new().size(50.0, 20.0).indent_left(0.0)),
    );
    h.layout(400.0, 100.0);
    let r = cells(&h, h.root().unwrap());
    assert!((r[1].origin.x - 50.0).abs() < 0.5, "{r:?}");
}

#[test]
fn row_cross_indent_top_shifts_y() {
    let h = Harness::new(
        Row::new()
            .gap(0.0)
            .child(Panel::new().size(50.0, 20.0).indent_top(7.0)),
    );
    h.layout(400.0, 100.0);
    let r = cells(&h, h.root().unwrap());
    assert!((r[0].origin.y - 7.0).abs() < 0.5, "{r:?}");
}

#[test]
fn column_indent_top_overrides_gap() {
    let h = Harness::new(
        Column::new()
            .gap(10.0)
            .child(Panel::new().size(50.0, 20.0))
            .child(Panel::new().size(50.0, 20.0).indent_top(30.0))
            .child(Panel::new().size(50.0, 20.0)),
    );
    h.layout(400.0, 400.0);
    let r = cells(&h, h.root().unwrap());
    assert!((r[1].origin.y - 50.0).abs() < 0.5, "{r:?}");
    assert!((r[2].origin.y - 80.0).abs() < 0.5, "{r:?}");
}

#[test]
fn column_indent_bottom_shifts_next() {
    let h = Harness::new(
        Column::new()
            .gap(10.0)
            .child(Panel::new().size(50.0, 20.0).indent_bottom(25.0))
            .child(Panel::new().size(50.0, 20.0)),
    );
    h.layout(400.0, 400.0);
    let r = cells(&h, h.root().unwrap());
    assert!((r[1].origin.y - (20.0 + 25.0 + 10.0)).abs() < 0.5, "{r:?}");
}

#[test]
fn column_cross_indent_left_shifts_x() {
    let h = Harness::new(
        Column::new()
            .gap(0.0)
            .child(Panel::new().size(50.0, 20.0).indent_left(9.0)),
    );
    h.layout(400.0, 400.0);
    let r = cells(&h, h.root().unwrap());
    assert!((r[0].origin.x - 9.0).abs() < 0.5, "{r:?}");
}

#[test]
fn indent_none_falls_back_to_gap() {
    let h = Harness::new(
        Row::new()
            .gap(12.0)
            .child(Panel::new().size(40.0, 20.0))
            .child(Panel::new().size(40.0, 20.0)),
    );
    h.layout(400.0, 100.0);
    let r = cells(&h, h.root().unwrap());
    assert!((r[1].origin.x - 52.0).abs() < 0.5, "{r:?}");
}

#[test]
fn indent_builders_cover_api() {
    let s = LayoutStyle::default()
        .indent_top(1.0)
        .indent_bottom(2.0)
        .indent_left(3.0)
        .indent_right(4.0);
    assert_eq!(s.indent_top, Some(1.0));
    assert_eq!(s.indent_bottom, Some(2.0));
    assert_eq!(s.indent_left, Some(3.0));
    assert_eq!(s.indent_right, Some(4.0));
    let s = s.indent(Indents::xy(5.0, 6.0));
    assert_eq!(s.indent_left, Some(5.0));
    assert_eq!(s.indent_right, Some(5.0));
    assert_eq!(s.indent_top, Some(6.0));
    assert_eq!(s.indent_bottom, Some(6.0));
    let s = s.indent_all(7.0);
    assert_eq!(s.indents(), Indents::all(7.0));
    let s = s.indent_axes(8.0, 9.0);
    assert_eq!(s.indent_left, Some(8.0));
    assert_eq!(s.indent_right, Some(8.0));
    assert_eq!(s.indent_top, Some(9.0));
    assert_eq!(s.indent_bottom, Some(9.0));
    let s = s.clear_indents();
    assert!(s.indents().is_none());

    let s = LayoutStyle::default()
        .indent_left(-5.0)
        .indent_top(f32::NAN)
        .indent_right(f32::INFINITY);
    assert_eq!(s.indent_left, Some(0.0));
    assert_eq!(s.indent_top, Some(0.0));
    assert_eq!(s.indent_right, Some(0.0));
    assert_eq!(s.indent_bottom, None);
}
