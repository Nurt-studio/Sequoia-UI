use sequoia_core::{Color, Rect};
use sequoia_elements::{Button, Container};
use sequoia_test::Harness;
use sequoia_view::prelude::*;

#[test]
fn box_style_border_edges_are_plain_widths() {
    let deco = sequoia_style::BoxStyle::new()
        .border(2.0, Color::RED)
        .border_right(20.0, Color::BLUE);
    let e = deco.border_edges();
    assert_eq!(e.left, 2.0);
    assert_eq!(e.top, 2.0);
    assert_eq!(e.right, 20.0);
    assert_eq!(e.bottom, 2.0);

    assert_eq!(
        sequoia_style::BoxStyle::new().border_edges(),
        sequoia_core::Edges::ZERO
    );
}

#[test]
fn column_border_adds_to_box_and_insets_children() {
    let h = Harness::new(
        Row::new().fill_size().child(
            Column::new()
                .background(Color::rgb8(0x19, 0x19, 0x19))
                .border_right(20.0, Color::rgb8(0x1F, 0x1F, 0x1F))
                .fill_height()
                .child(Button::icon("name").size(54.0)),
        ),
    );
    h.layout(400.0, 300.0);
    h.paint();
    h.tick();

    let row = h.root().unwrap();
    let tb = h.children(row)[0];
    let tb_r = h.rect(tb).unwrap();

    assert!((tb_r.size.width - 74.0).abs() < 0.01, "box: {tb_r:?}");

    let kid_r = h.rect(h.children(tb)[0]).unwrap();
    assert!((kid_r.size.width - 54.0).abs() < 0.01, "dziecko: {kid_r:?}");
    assert_eq!(kid_r, Rect::from_xywh(0.0, 0.0, 54.0, 54.0));

    let list = h.paint();
    let bar = list
        .quads()
        .find(|q| q.rect.size.height > 100.0 && (q.rect.size.width - 20.0).abs() < 0.01)
        .expect("pas borderu");
    assert_eq!(bar.rect, Rect::from_xywh(54.0, 0.0, 20.0, 300.0));
}

#[test]
fn container_border_insets_child() {
    let h = Harness::new(
        Container::new()
            .border_right(20.0, Color::WHITE)
            .child(Button::icon("").size(54.0)),
    );
    h.layout(400.0, 300.0);
    h.paint();
    h.tick();
    let root = h.root().unwrap();
    let box_r = h.rect(root).unwrap();
    let kid = h.children(root)[0];
    let kid_r = h.rect(kid).unwrap();
    assert!((box_r.size.width - 74.0).abs() < 0.01, "box: {box_r:?}");
    assert!((kid_r.size.width - 54.0).abs() < 0.01, "dziecko: {kid_r:?}");
}

#[test]
fn border_with_padding_adds_up() {
    let h = Harness::new(
        Container::new()
            .padding(4.0)
            .border_right(20.0, Color::WHITE)
            .child(Button::icon("").size(54.0)),
    );
    h.layout(400.0, 300.0);
    h.paint();
    h.tick();
    let root = h.root().unwrap();
    let box_r = h.rect(root).unwrap();
    let kid_r = h.rect(h.children(root)[0]).unwrap();

    assert!(
        (box_r.size.width - 82.0).abs() < 0.01,
        "54 + 8 + 20: {box_r:?}"
    );
    assert!(
        (kid_r.left() - 4.0).abs() < 0.01,
        "dziecko po paddingu: {kid_r:?}"
    );
    assert!((kid_r.size.width - 54.0).abs() < 0.01, "content: {kid_r:?}");
}
