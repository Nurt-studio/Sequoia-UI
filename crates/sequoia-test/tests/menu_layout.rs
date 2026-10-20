#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_elements::{Container, FloatingMenu};
use sequoia_test::Harness;
use sequoia_view::prelude::*;

fn approx(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}

#[test]
fn floating_menu_forwards_fill_child() {
    let h = Harness::new(
        Stack::new()
            .fill_size()
            .child(FloatingMenu::wrap(Container::new().fill_size()).text_row("Fazka", || {})),
    );
    h.layout(400.0, 300.0);
    let root = h.root().unwrap();
    let kids = h.children(root);
    assert_eq!(kids.len(), 1);
    let r = h.rect(kids[0]).unwrap();
    assert!(
        approx(r.size.width, 400.0) && approx(r.size.height, 300.0),
        "wrapper Fill fills Stack: {r:?}"
    );
}

#[test]
fn floating_menu_keeps_fit_child_fit() {
    let h = Harness::new(Stack::new().fill_size().child(
        FloatingMenu::wrap(Container::new().width(120.0).height(40.0)).text_row("Fazka", || {}),
    ));
    h.layout(400.0, 300.0);
    let root = h.root().unwrap();
    let kids = h.children(root);
    assert_eq!(kids.len(), 1);
    let r = h.rect(kids[0]).unwrap();
    assert!(
        approx(r.size.width, 120.0) && approx(r.size.height, 40.0),
        "baby Fit doesn't stretch: {r:?}"
    );
}
