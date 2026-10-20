#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_reactive::Signal;
use sequoia_test::Harness;
use sequoia_view::prelude::*;

fn harness(frac: &Signal<f32>) -> Harness {
    let h = Harness::new(
        SplitPanel::new(frac.clone())
            .first(Panel::new())
            .second(Panel::new()),
    );
    h.layout(400.0, 300.0);
    h.paint();
    h.tick();
    h
}

fn panes(h: &Harness) -> (sequoia_core::Rect, sequoia_core::Rect) {
    let kids = h.children(h.root().unwrap());
    assert_eq!(kids.len(), 2);
    (h.rect(kids[0]).unwrap(), h.rect(kids[1]).unwrap())
}

#[test]
fn half_split_divides_evenly_minus_handle() {
    let frac = Signal::new(0.5);
    let h = harness(&frac);
    let (a, b) = panes(&h);

    assert!((a.size.width - 196.5).abs() < 0.01, "first: {a:?}");
    assert!((b.size.width - 196.5).abs() < 0.01, "second: {b:?}");
    assert!(
        (b.left() - (a.left() + a.size.width + 7.0)).abs() < 0.01,
        "{a:?} vs {b:?}"
    );
    assert!((a.size.height - 300.0).abs() < 0.01, "full height: {a:?}");
}

#[test]
fn signal_change_relayouts_panes() {
    let frac = Signal::new(0.5);
    let h = harness(&frac);
    h.set(&frac, 0.25);
    h.layout(400.0, 300.0);
    h.paint();
    h.tick();
    let (a, b) = panes(&h);

    assert!((a.size.width - 98.25).abs() < 0.01, "first: {a:?}");
    assert!(
        (b.size.width - (393.0 - 98.25)).abs() < 0.01,
        "second: {b:?}"
    );
}

#[test]
fn drag_handle_writes_fraction_back() {
    let frac = Signal::new(0.5);
    let h = harness(&frac);

    h.drag((200.0, 150.0), (300.0, 150.0));
    let v = frac.get_untracked();
    assert!(
        (v - (0.5 + 100.0 / 393.0)).abs() < 0.05,
        "fraction follows drag: {v}"
    );
    h.layout(400.0, 300.0);
    h.paint();
    h.tick();
    let (a, _) = panes(&h);
    assert!(
        (a.size.width - 393.0 * v).abs() < 0.5,
        "layout follows dragged fraction: {a:?} vs {v}"
    );
}

#[test]
fn vertical_axis_splits_top_bottom() {
    let frac = Signal::new(0.5);
    let h = Harness::new(
        SplitPanel::new(frac.clone())
            .axis(Axis::Vertical)
            .first(Panel::new())
            .second(Panel::new()),
    );
    h.layout(400.0, 300.0);
    h.paint();
    h.tick();
    let kids = h.children(h.root().unwrap());
    let a = h.rect(kids[0]).unwrap();
    let b = h.rect(kids[1]).unwrap();

    assert!((a.size.height - 146.5).abs() < 0.01, "first: {a:?}");
    assert!(
        (b.top() - (a.top() + a.size.height + 7.0)).abs() < 0.01,
        "{a:?} vs {b:?}"
    );
}
