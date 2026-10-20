#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_elements::Container;
use sequoia_test::Harness;
use sequoia_view::prelude::*;

fn approx(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}

#[test]
fn stack_absolute_child_uses_inset() {
    let h = Harness::new(
        Stack::new()
            .fill_size()
            .child(Container::new().fill_size())
            .child(
                Container::new()
                    .width(120.0)
                    .height(80.0)
                    .absolute_at(20.0, 30.0),
            ),
    );
    h.layout(400.0, 300.0);
    let root = h.root().unwrap();
    let kids = h.children(root);
    assert_eq!(kids.len(), 2);
    let flow = h.rect(kids[0]).unwrap();
    assert!(
        approx(flow.size.width, 400.0) && approx(flow.size.height, 300.0),
        "flow child becomes filler: {flow:?}"
    );
    let abs = h.rect(kids[1]).unwrap();
    assert!(
        approx(abs.left(), 20.0) && approx(abs.top(), 30.0),
        "inset with `absolute_at` watches from the top left corner: {abs:?}"
    );
    assert!(
        approx(abs.size.width, 120.0) && approx(abs.size.height, 80.0),
        "size with `width`/`height`, not stretched: {abs:?}"
    );
}

#[test]
fn stack_pins_and_absolute_coexist() {
    let h = Harness::new(
        Stack::new()
            .fill_size()
            .child(Container::new().fill_size())
            .child_at(
                Container::new().width(50.0).height(20.0),
                StackPos::top_right().offset(-6.0, -6.0),
            )
            .child(
                Container::new()
                    .width(120.0)
                    .height(80.0)
                    .absolute_at(20.0, 30.0),
            ),
    );
    h.layout(400.0, 300.0);
    let root = h.root().unwrap();
    let kids = h.children(root);
    assert_eq!(kids.len(), 3);
    let pinned = h.rect(kids[1]).unwrap();
    assert!(
        approx(pinned.right(), 394.0) && approx(pinned.top(), -6.0),
        "pin `top_right` still works next to child absolute: {pinned:?}"
    );
    let abs = h.rect(kids[2]).unwrap();
    assert!(
        approx(abs.left(), 20.0) && approx(abs.top(), 30.0),
        "absolute_at does not corrupt adjacent pins: {abs:?}"
    );
}
