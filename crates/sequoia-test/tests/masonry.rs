#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_core::Rect;
use sequoia_test::Harness;
use sequoia_view::prelude::*;

fn cell(height: f32) -> Panel {
    Panel::new().height(height)
}

fn approx(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}

fn assert_rect(rect: Rect, x: f32, y: f32, w: f32, h: f32) {
    assert!(approx(rect.left(), x), "left {} != {x}", rect.left());
    assert!(approx(rect.top(), y), "top {} != {y}", rect.top());
    assert!(
        approx(rect.size.width, w),
        "width {} != {w}",
        rect.size.width
    );
    assert!(
        approx(rect.size.height, h),
        "height {} != {h}",
        rect.size.height
    );
}

#[test]
fn items_are_appended_to_the_shortest_column() {
    let h = Harness::new(Masonry::new(2).gap(10.0).children([
        cell(60.0),
        cell(40.0),
        cell(20.0),
        cell(30.0),
    ]));
    h.layout(100.0, 200.0);

    let masonry = h.root().unwrap();
    let kids = h.children(masonry);
    assert_eq!(kids.len(), 4);
    assert_rect(h.rect(kids[0]).unwrap(), 0.0, 0.0, 45.0, 60.0);
    assert_rect(h.rect(kids[1]).unwrap(), 55.0, 0.0, 45.0, 40.0);
    assert_rect(h.rect(kids[2]).unwrap(), 55.0, 50.0, 45.0, 20.0);
    assert_rect(h.rect(kids[3]).unwrap(), 0.0, 70.0, 45.0, 30.0);
    assert!(approx(h.rect(masonry).unwrap().size.height, 100.0));
}

#[test]
fn padding_and_column_gap_inset_the_columns() {
    let h = Harness::new(
        Masonry::new(2)
            .padding(5.0)
            .gap(10.0)
            .children([cell(20.0), cell(20.0)]),
    );
    h.layout(110.0, 200.0);

    let masonry = h.root().unwrap();
    let kids = h.children(masonry);
    assert_rect(h.rect(kids[0]).unwrap(), 5.0, 5.0, 45.0, 20.0);
    assert_rect(h.rect(kids[1]).unwrap(), 60.0, 5.0, 45.0, 20.0);
    assert!(approx(h.rect(masonry).unwrap().size.height, 30.0));
}

#[test]
fn row_gap_is_only_between_items_in_a_column() {
    let h =
        Harness::new(
            Masonry::new(2)
                .row_gap(10.0)
                .children([cell(50.0), cell(30.0), cell(20.0)]),
        );
    h.layout(100.0, 200.0);

    let kids = h.children(h.root().unwrap());
    assert!(approx(h.rect(kids[1]).unwrap().top(), 0.0));
    assert!(approx(h.rect(kids[2]).unwrap().top(), 40.0));
    assert!(approx(h.rect(h.root().unwrap()).unwrap().size.height, 60.0));
}

#[test]
fn align_items_centers_narrow_cards_in_their_column() {
    let h = Harness::new(Masonry::new(2).align_items(Align::Center).children([
        Panel::new().width(20.0).height(20.0),
        Panel::new().width(40.0).height(20.0),
    ]));
    h.layout(100.0, 200.0);

    let kids = h.children(h.root().unwrap());
    assert!(approx(h.rect(kids[0]).unwrap().left(), 15.0));
    assert!(approx(h.rect(kids[1]).unwrap().left(), 55.0));
}

#[test]
fn fit_width_uses_the_widest_card_for_every_column() {
    let h = Harness::new(Masonry::new(2).fit().gap(10.0).children([
        Panel::new().width(30.0).height(10.0),
        Panel::new().width(50.0).height(10.0),
    ]));
    h.layout(400.0, 200.0);

    let masonry = h.root().unwrap();
    let kids = h.children(masonry);
    assert_rect(h.rect(masonry).unwrap(), 0.0, 0.0, 110.0, 10.0);
    assert_rect(h.rect(kids[0]).unwrap(), 0.0, 0.0, 30.0, 10.0);
    assert_rect(h.rect(kids[1]).unwrap(), 60.0, 0.0, 50.0, 10.0);
}

#[test]
fn absolute_children_do_not_join_the_flow() {
    let h = Harness::new(Masonry::new(1).children([
        cell(20.0),
        Panel::new().width(80.0).height(50.0).absolute_at(3.0, 4.0),
        cell(30.0),
    ]));
    h.layout(80.0, 200.0);

    let masonry = h.root().unwrap();
    let kids = h.children(masonry);
    assert_rect(h.rect(kids[0]).unwrap(), 0.0, 0.0, 80.0, 20.0);
    assert_rect(h.rect(kids[1]).unwrap(), 3.0, 4.0, 80.0, 50.0);
    assert_rect(h.rect(kids[2]).unwrap(), 0.0, 20.0, 80.0, 30.0);
    assert!(approx(h.rect(masonry).unwrap().size.height, 50.0));
}

#[test]
fn height_is_remeasured_after_a_row_allocates_width() {
    let h = Harness::new(
        Row::new()
            .fill_width()
            .gap(10.0)
            .child(Panel::new().width(100.0).height(50.0))
            .child(Masonry::new(2).gap(10.0).children([
                Panel::new().fill_width().aspect(1.0),
                Panel::new().fill_width().aspect(1.0),
            ])),
    );
    h.layout(300.0, 200.0);

    let row = h.root().unwrap();
    let kids = h.children(row);
    let masonry = kids[1];
    assert_rect(h.rect(masonry).unwrap(), 110.0, 0.0, 190.0, 90.0);

    let cards = h.children(masonry);
    assert_rect(h.rect(cards[0]).unwrap(), 110.0, 0.0, 90.0, 90.0);
    assert_rect(h.rect(cards[1]).unwrap(), 210.0, 0.0, 90.0, 90.0);
}
