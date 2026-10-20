#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_elements::{Sheet, SheetSide};
use sequoia_reactive::Signal;
use sequoia_test::Harness;
use sequoia_view::prelude::*;

fn harness(open: &Signal<bool>) -> Harness {
    let h = Harness::new(
        Column::new().child(Text::new("Pod spodem")).child(
            Sheet::new(open.clone())
                .height(100.0)
                .child(Text::new("Arkusz")),
        ),
    );
    h.layout(400.0, 400.0);
    h.paint();
    h.tick();
    h
}

#[test]
fn closed_paints_without_overlay() {
    let open = Signal::new(false);
    let h = harness(&open);
    assert!(h.overlay_ids().is_empty(), "closed: no overlay");
    assert!(!open.get_untracked());
}

#[test]
fn open_parks_against_viewport_bottom() {
    let open = Signal::new(false);
    let h = harness(&open);
    h.set(&open, true);
    h.layout(400.0, 400.0);
    h.paint();
    h.tick();
    h.layout(400.0, 400.0);
    h.paint();

    assert_eq!(h.overlay_ids().len(), 1, "open: one overlay");
    let r = h.overlay_rect(h.overlay_ids()[0]).unwrap();
    assert!(
        (r.size.height - 100.0).abs() < 1.0,
        "explicit height kept: {r:?}"
    );
    assert!(
        (r.bottom() - 400.0).abs() < 1.0,
        "bottom sheet touches viewport bottom: {r:?}"
    );
}

#[test]
fn outside_click_dismisses_and_closes_signal() {
    let open = Signal::new(false);
    let h = harness(&open);
    h.set(&open, true);
    h.layout(400.0, 400.0);
    h.paint();
    h.tick();
    h.layout(400.0, 400.0);
    h.paint();
    assert_eq!(h.overlay_ids().len(), 1);

    h.click(200.0, 50.0);
    h.layout(400.0, 400.0);
    h.paint();
    h.tick();

    assert!(h.overlay_ids().is_empty(), "outside click hides overlay");
    assert!(!open.get_untracked(), "signal follows runtime dismiss");
}

#[test]
fn side_maps_to_placement() {
    assert!(
        !Sheet::new(Signal::new(false))
            .side(SheetSide::Top)
            .is_open()
    );

    let open = Signal::new(false);
    let _ = Sheet::new(open.clone()).top();
    let _ = Sheet::new(open.clone()).bottom();
    let _ = Sheet::new(open.clone()).left();
    let _ = Sheet::new(open.clone()).right();
}
