#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_elements::Container;
use sequoia_test::Harness;
use sequoia_view::prelude::*;

#[test]
fn px_children_hold_size_on_main_axis_deficit() {
    let h = Harness::new(
        Row::new().fill_size().child(
            Container::new().width(100.0).height(50.0).child(
                Row::new()
                    .fill_size()
                    .child(Container::new().width(80.0).height(40.0))
                    .child(Container::new().width(80.0).height(40.0)),
            ),
        ),
    );
    h.layout(800.0, 600.0);
    let root = h.root().unwrap();
    let outer = h.children(root)[0];
    let row = h.children(outer)[0];
    let kids = h.children(row);
    assert_eq!(kids.len(), 2);
    assert!(
        (h.rect(kids[0]).unwrap().size.width - 80.0).abs() < 0.5,
        "first keeps 80"
    );
    assert!(
        (h.rect(kids[1]).unwrap().size.width - 80.0).abs() < 0.5,
        "second keeps 80"
    );
}

#[test]
fn px_holds_on_cross_axis() {
    let h = Harness::new(
        Column::new()
            .width(20.0)
            .height(200.0)
            .child(Container::new().width(22.0).height(30.0)),
    );
    h.layout(800.0, 600.0);
    let root = h.root().unwrap();
    let kid = h.children(root)[0];
    assert!(
        (h.rect(kid).unwrap().size.width - 22.0).abs() < 0.5,
        "cross Px not clipped"
    );
}

#[test]
fn explicit_max_still_wins_over_px() {
    let h = Harness::new(
        Row::new()
            .width(50.0)
            .height(50.0)
            .child(Container::new().width(80.0).max_width(40.0).height(40.0)),
    );
    h.layout(800.0, 600.0);
    let root = h.root().unwrap();
    let kid = h.children(root)[0];
    assert!(
        (h.rect(kid).unwrap().size.width - 40.0).abs() < 0.5,
        "max cuts Px"
    );
}

#[test]
fn flex_child_yields_first_then_px_overflows() {
    let h = Harness::new(
        Row::new()
            .width(100.0)
            .height(50.0)
            .child(Container::new().width(80.0).height(40.0))
            .child(Container::new().fill_width().height(40.0)),
    );
    h.layout(800.0, 600.0);
    let root = h.root().unwrap();
    let kids = h.children(root);
    assert!(
        (h.rect(kids[0]).unwrap().size.width - 80.0).abs() < 0.5,
        "rigid keeps"
    );
    assert!(
        (h.rect(kids[1]).unwrap().size.width - 20.0).abs() < 0.5,
        "flexible takes the rest"
    );
}
