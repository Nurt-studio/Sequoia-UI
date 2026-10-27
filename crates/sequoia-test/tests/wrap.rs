#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_test::Harness;
use sequoia_view::prelude::*;

fn cell(w: f32, h: f32) -> Panel {
    Panel::new().width(w).height(h)
}

fn approx(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}

#[test]
fn overflowing_child_moves_to_the_next_line() {
    let h =
        Harness::new(Wrap::new().children([cell(40.0, 20.0), cell(40.0, 20.0), cell(40.0, 20.0)]));
    h.layout(100.0, 200.0);

    let kids = h.children(h.root().unwrap());
    assert_eq!(kids.len(), 3);
    let r0 = h.rect(kids[0]).unwrap();
    let r1 = h.rect(kids[1]).unwrap();
    let r2 = h.rect(kids[2]).unwrap();

    assert!(approx(r0.left(), 0.0) && approx(r0.top(), 0.0));
    assert!(approx(r1.left(), 40.0) && approx(r1.top(), 0.0));
    assert!(approx(r2.left(), 0.0), "start nowej linii: {r2:?}");
    assert!(approx(r2.top(), 20.0), "line under the first: {r2:?}");
}

#[test]
fn wrap_never_squeezes_children() {
    let h = Harness::new(Wrap::new().children([
        cell(40.0, 20.0),
        cell(40.0, 20.0),
        cell(40.0, 20.0),
        cell(40.0, 20.0),
        cell(40.0, 20.0),
    ]));
    h.layout(100.0, 200.0);

    let root = h.root().unwrap();
    for child in h.children(root) {
        let r = h.rect(child).unwrap();
        assert!(approx(r.size.width, 40.0), "squeezed: {r:?}");
        assert!(approx(r.size.height, 20.0), "squeezed: {r:?}");
    }

    assert!(approx(h.rect(root).unwrap().size.height, 60.0));
}

#[test]
fn gap_counts_toward_the_line_width() {
    let h = Harness::new(Wrap::new().gap(10.0).children([
        cell(40.0, 20.0),
        cell(40.0, 20.0),
        cell(40.0, 20.0),
    ]));
    h.layout(100.0, 200.0);

    let kids = h.children(h.root().unwrap());
    assert!(approx(h.rect(kids[1]).unwrap().left(), 50.0));

    assert!(approx(h.rect(kids[2]).unwrap().top(), 30.0));
}

#[test]
fn vertical_wrap_stacks_lines_as_columns() {
    let h = Harness::new(Wrap::vertical().width(200.0).children([
        cell(40.0, 30.0),
        cell(40.0, 30.0),
        cell(40.0, 30.0),
    ]));
    h.layout(200.0, 70.0);

    let kids = h.children(h.root().unwrap());
    let r1 = h.rect(kids[1]).unwrap();
    let r2 = h.rect(kids[2]).unwrap();
    assert!(approx(r1.top(), 30.0));
    assert!(approx(r2.top(), 0.0), "new column from the top: {r2:?}");
    assert!(approx(r2.left(), 40.0), "column next to it: {r2:?}");
}

#[test]
fn align_items_centers_in_the_line() {
    let h = Harness::new(
        Wrap::new()
            .align_items(Align::Center)
            .children([cell(40.0, 40.0), cell(40.0, 20.0)]),
    );
    h.layout(100.0, 200.0);

    let kids = h.children(h.root().unwrap());

    assert!(approx(h.rect(kids[1]).unwrap().top(), 10.0));
}

#[test]
fn wider_than_the_line_gets_its_own_line() {
    let h = Harness::new(Wrap::new().children([cell(150.0, 20.0), cell(30.0, 20.0)]));
    h.layout(100.0, 200.0);

    let kids = h.children(h.root().unwrap());
    let r0 = h.rect(kids[0]).unwrap();
    let r1 = h.rect(kids[1]).unwrap();
    assert!(approx(r0.size.width, 150.0), "no squeezing: {r0:?}");
    assert!(approx(r1.top(), 20.0), "second on a new line: {r1:?}");
}

#[test]
fn explicit_columns_force_line_breaks() {
    let h = Harness::new(Wrap::new().columns(2).children([
        cell(10.0, 20.0),
        cell(10.0, 20.0),
        cell(10.0, 20.0),
    ]));
    h.layout(1000.0, 200.0);

    let kids = h.children(h.root().unwrap());
    assert!(approx(h.rect(kids[0]).unwrap().left(), 0.0));
    assert!(approx(h.rect(kids[1]).unwrap().left(), 10.0));
    assert!(approx(h.rect(kids[2]).unwrap().left(), 0.0));
    assert!(approx(h.rect(kids[2]).unwrap().top(), 20.0));
    assert!(approx(h.rect(h.root().unwrap()).unwrap().size.height, 40.0));
}

#[test]
fn explicit_rows_limit_the_number_of_lines() {
    let h = Harness::new(Wrap::new().rows(2).children([
        cell(10.0, 20.0),
        cell(10.0, 20.0),
        cell(10.0, 20.0),
        cell(10.0, 20.0),
        cell(10.0, 20.0),
        cell(10.0, 20.0),
    ]));
    h.layout(1000.0, 200.0);

    let kids = h.children(h.root().unwrap());
    assert!(approx(h.rect(kids[2]).unwrap().left(), 20.0));
    assert!(approx(h.rect(kids[3]).unwrap().left(), 0.0));
    assert!(approx(h.rect(kids[3]).unwrap().top(), 20.0));
    assert!(approx(h.rect(h.root().unwrap()).unwrap().size.height, 40.0));
}

#[test]
fn explicit_rows_on_vertical_wrap_force_column_breaks() {
    let h = Harness::new(Wrap::vertical().width(200.0).rows(2).children([
        cell(40.0, 30.0),
        cell(40.0, 30.0),
        cell(40.0, 30.0),
    ]));
    h.layout(200.0, 1000.0);

    let kids = h.children(h.root().unwrap());
    assert!(approx(h.rect(kids[1]).unwrap().top(), 30.0));
    assert!(approx(h.rect(kids[2]).unwrap().left(), 40.0));
    assert!(approx(h.rect(kids[2]).unwrap().top(), 0.0));
}

#[test]
fn fixed_card_three_tiles_wrap_to_two_lines() {
    use sequoia_elements::Container;
    let h = Harness::new(Container::new().width(320.0).padding(8.0).child(
        Wrap::new().fill_width().gap(8.0).children([
            cell(140.0, 48.0),
            cell(140.0, 48.0),
            cell(140.0, 48.0),
        ]),
    ));
    h.layout(800.0, 600.0);

    let card = h.root().unwrap();
    let wrap = h.children(card)[0];
    let kids = h.children(wrap);
    assert_eq!(kids.len(), 3);
    let r0 = h.rect(kids[0]).unwrap();
    let r1 = h.rect(kids[1]).unwrap();
    let r2 = h.rect(kids[2]).unwrap();
    assert!(approx(r0.left(), 8.0) && approx(r0.top(), 8.0));
    assert!(approx(r1.left(), 156.0) && approx(r1.top(), 8.0));
    assert!(approx(r2.left(), 8.0), "trzeci od lewej: {r2:?}");
    assert!(approx(r2.top(), 64.0), "trzeci w drugiej linii: {r2:?}");
    for (i, k) in kids.iter().enumerate() {
        let r = h.rect(*k).unwrap();
        assert!(approx(r.size.width, 140.0), "tile {i} keeps width: {r:?}");
    }
}
