#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_elements::{Drawer, DrawerSide};
use sequoia_reactive::Signal;
use sequoia_test::Harness;
use sequoia_view::prelude::*;

fn harness(open: &Signal<bool>) -> Harness {
    let h = Harness::new(
        Column::new()
            .child(Text::new("Pod spodem"))
            .child(Drawer::new(open.clone()).child(Text::new("Szuflada"))),
    );
    h.layout(400.0, 400.0);
    h.paint();
    h.tick();
    h
}

fn open_two_passes(h: &Harness, open: &Signal<bool>) {
    h.set(open, true);
    h.layout(400.0, 400.0);
    h.paint();
    h.tick();
    h.layout(400.0, 400.0);
    h.paint();
}

#[test]
fn closed_paints_without_overlay() {
    let open = Signal::new(false);
    let h = harness(&open);
    assert!(h.overlay_ids().is_empty(), "closed: no overlay");
}

#[test]
fn open_parks_against_viewport_right_with_default_width() {
    let open = Signal::new(false);
    let h = harness(&open);
    open_two_passes(&h, &open);
    assert_eq!(h.overlay_ids().len(), 1, "open: one overlay");
    let r = h.overlay_rect(h.overlay_ids()[0]).unwrap();
    assert!((r.size.width - 330.0).abs() < 1.0, "default 330px: {r:?}");
    assert!(
        (r.right() - 400.0).abs() < 1.0,
        "right drawer touches viewport right: {r:?}"
    );
}

#[test]
fn left_side_parks_against_viewport_left() {
    let open = Signal::new(false);
    let h = Harness::new(
        Column::new().child(
            Drawer::new(open.clone())
                .side(DrawerSide::Left)
                .width(200.0)
                .height(100.0)
                .child(Text::new("Szuflada")),
        ),
    );
    h.layout(400.0, 400.0);
    h.paint();
    h.tick();
    open_two_passes(&h, &open);
    let r = h.overlay_rect(h.overlay_ids()[0]).unwrap();
    assert!((r.left() - 0.0).abs() < 1.0, "left drawer at x=0: {r:?}");
    assert!(
        (r.size.width - 200.0).abs() < 1.0,
        "explicit width kept: {r:?}"
    );
}

#[test]
fn outside_click_dismisses_and_closes_signal() {
    let open = Signal::new(false);
    let h = harness(&open);
    open_two_passes(&h, &open);
    assert_eq!(h.overlay_ids().len(), 1);

    h.click(10.0, 10.0);
    h.layout(400.0, 400.0);
    h.paint();
    h.tick();

    assert!(h.overlay_ids().is_empty(), "outside click hides overlay");
    assert!(!open.get_untracked(), "signal follows runtime dismiss");
}

#[test]
fn side_aliases_compile() {
    let open = Signal::new(false);
    let _ = Drawer::new(open.clone()).top();
    let _ = Drawer::new(open.clone()).bottom();
    let _ = Drawer::new(open.clone()).left();
    let _ = Drawer::new(open.clone()).right();
    assert!(!open.get_untracked());
}
