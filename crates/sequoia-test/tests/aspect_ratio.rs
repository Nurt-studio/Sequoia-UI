#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_test::Harness;
use sequoia_view::prelude::*;

fn approx(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}

#[test]
fn fixed_width_derives_height_from_ratio() {
    let h = Harness::new(AspectRatio::new(2.0).width(200.0).child(Panel::new()));
    h.layout(800.0, 600.0);
    let r = h.rect(h.root().unwrap()).unwrap();
    assert!(
        approx(r.size.width, 200.0) && approx(r.size.height, 100.0),
        "16:9-like 2.0 ratio from fixed width 200 must give 200x100, got {r:?}"
    );
}

#[test]
fn fixed_height_derives_width_from_ratio() {
    let h = Harness::new(AspectRatio::new(2.0).height(100.0).child(Panel::new()));
    h.layout(800.0, 600.0);
    let r = h.rect(h.root().unwrap()).unwrap();
    assert!(
        approx(r.size.width, 200.0) && approx(r.size.height, 100.0),
        "2.0 ratio from fixed height 100 must give 200x100, got {r:?}"
    );
}

#[test]
fn both_fixed_letterboxes_child_instead_of_stretching() {
    let h = Harness::new(
        AspectRatio::new(16.0 / 9.0)
            .width(800.0)
            .height(600.0)
            .child(Panel::new()),
    );
    h.layout(800.0, 600.0);
    let root = h.root().unwrap();
    let kids = h.children(root);
    assert_eq!(kids.len(), 1);
    let r = h.rect(kids[0]).unwrap();
    assert!(
        approx(r.size.width, 800.0) && approx(r.size.height, 450.0),
        "16:9 in 800x600 must letterbox to 800x450, got {r:?}"
    );
    assert!(
        approx(r.origin.x, 0.0) && approx(r.origin.y, 75.0),
        "letterboxed child must be centered vertically at y=75, got {r:?}"
    );
}
