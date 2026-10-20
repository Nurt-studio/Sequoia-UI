#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_core::Color;
use sequoia_elements::Container;
use sequoia_test::Harness;
use sequoia_view::prelude::*;

fn approx(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}

#[test]
fn child_flex_splits_free_space_by_weight() {
    let h = Harness::new(
        Row::new()
            .width(400.0)
            .height(50.0)
            .child_flex(Panel::new().height(50.0), 1.0)
            .child_flex(Panel::new().height(50.0), 3.0),
    );
    h.layout(400.0, 100.0);
    let kids = h.children(h.root().unwrap());
    assert_eq!(kids.len(), 2);
    let a = h.rect(kids[0]).unwrap();
    let b = h.rect(kids[1]).unwrap();
    assert!(approx(a.size.width, 100.0), "weight 1/4: {a:?}");
    assert!(approx(b.size.width, 300.0), "weight 3/4: {b:?}");
}

#[test]
fn expand_takes_leftover_after_max_cap() {
    let h = Harness::new(
        Row::new().width(400.0).height(50.0).children((
            Container::new()
                .background(Color::RED)
                .fill_height()
                .fill_width()
                .max_width(200.0),
            Container::new()
                .background(Color::GREEN)
                .fill_height()
                .fill_width()
                .expand(),
        )),
    );
    h.layout(400.0, 100.0);
    let kids = h.children(h.root().unwrap());
    assert_eq!(kids.len(), 2);
    let a = h.rect(kids[0]).unwrap();
    let b = h.rect(kids[1]).unwrap();
    assert!(approx(a.size.width, 200.0), "clipped by max: {a:?}");
    assert!(approx(b.size.width, 200.0), "rest for expand: {b:?}");
    assert!(approx(b.left(), 200.0), "attached to the first: {b:?}");
}

#[test]
fn child_flex_zero_behaves_like_child() {
    let h = Harness::new(
        Row::new()
            .width(300.0)
            .height(40.0)
            .child(Panel::new().width(60.0).height(40.0))
            .child_flex(Panel::new().width(60.0).height(40.0), 0.0),
    );
    h.layout(300.0, 100.0);
    let kids = h.children(h.root().unwrap());
    let b = h.rect(kids[1]).unwrap();
    assert!(
        approx(b.size.width, 60.0),
        "weight 0 does not stretch: {b:?}"
    );
}

#[test]
fn column_child_flex_grows_vertically() {
    let h = Harness::new(
        Column::new()
            .width(100.0)
            .height(300.0)
            .child(Panel::new().width(100.0).height(50.0))
            .child_flex(Panel::new().width(100.0), 2.0),
    );
    h.layout(400.0, 400.0);
    let kids = h.children(h.root().unwrap());
    assert_eq!(kids.len(), 2);
    let b = h.rect(kids[1]).unwrap();
    assert!(approx(b.size.height, 250.0), "remaining height: {b:?}");
    assert!(approx(b.top(), 50.0), "below the first: {b:?}");
}

#[test]
fn flex_builder_matches_fill_weight() {
    let h = Harness::new(
        Row::new()
            .width(400.0)
            .height(50.0)
            .child(Panel::new().width(100.0).height(50.0).flex(1.0))
            .child(Panel::new().width(100.0).height(50.0).flex(3.0)),
    );
    h.layout(400.0, 100.0);
    let kids = h.children(h.root().unwrap());
    let a = h.rect(kids[0]).unwrap();
    let b = h.rect(kids[1]).unwrap();

    assert!(approx(a.size.width, 150.0), "{a:?}");
    assert!(approx(b.size.width, 250.0), "{b:?}");
}

#[test]
fn parent_weight_combines_with_child_expand() {
    let h = Harness::new(
        Row::new()
            .width(300.0)
            .height(40.0)
            .child(Panel::new().width(50.0).height(40.0).expand())
            .child_flex(Panel::new().width(50.0).height(40.0), 2.0),
    );
    h.layout(300.0, 100.0);
    let kids = h.children(h.root().unwrap());
    let a = h.rect(kids[0]).unwrap();
    let b = h.rect(kids[1]).unwrap();

    assert!(approx(a.size.width, 50.0 + 200.0 / 3.0), "{a:?}");
    assert!(approx(b.size.width, 50.0 + 400.0 / 3.0), "{b:?}");
}
