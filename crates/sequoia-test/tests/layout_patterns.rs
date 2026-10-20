#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_core::Color;
use sequoia_test::Harness;
use sequoia_view::prelude::*;

fn square(color: Color) -> impl View + 'static {
    Column::new()
        .square(32.0)
        .center()
        .background(color)
        .child(Text::new("x").size(11.0).color(Color::WHITE))
}

#[test]
fn row_places_children_with_gap() {
    let h = Harness::new(Row::new().gap(8.0).children((
        square(Color::RED),
        square(Color::GREEN),
        square(Color::BLUE),
    )));
    h.layout(400.0, 100.0);
    let kids = h.children(h.root().unwrap());
    assert_eq!(kids.len(), 3);
    let xs: Vec<f32> = kids.iter().map(|&k| h.rect(k).unwrap().left()).collect();
    assert_eq!(xs, vec![0.0, 40.0, 80.0], "32px blocks + 8px gaps");
    for k in kids {
        assert_eq!(h.rect(k).unwrap().size.width, 32.0);
    }
}

#[test]
fn justify_modes_pin_to_expected_edges() {
    for (justify, first_x, second_x) in [
        (Justify::SpaceBetween, 0.0, 368.0),
        (Justify::Center, 168.0, 200.0),
        (Justify::End, 336.0, 368.0),
    ] {
        let h = Harness::new(
            Row::new()
                .width(400.0)
                .justify_content(justify)
                .children((square(Color::RED), square(Color::BLUE))),
        );
        h.layout(400.0, 100.0);
        let kids = h.children(h.root().unwrap());
        let a = h.rect(kids[0]).unwrap();
        let b = h.rect(kids[1]).unwrap();
        assert!(
            (a.left() - first_x).abs() < 0.01,
            "{justify:?}: first at {first_x}, got {}",
            a.left()
        );
        assert!(
            (b.left() - second_x).abs() < 0.01,
            "{justify:?}: second at {second_x}, got {}",
            b.left()
        );
    }
}

#[test]
fn flex_child_takes_remaining_space() {
    let h = Harness::new(
        Row::new()
            .width(400.0)
            .gap(8.0)
            .child(Column::new().size(64.0, 32.0).background(Color::RED))
            .child_flex(Column::new().height(32.0).background(Color::GREEN), 1.0)
            .child(Column::new().size(64.0, 32.0).background(Color::BLUE)),
    );
    h.layout(400.0, 100.0);
    let kids = h.children(h.root().unwrap());
    let mid = h.rect(kids[1]).unwrap();
    assert!((mid.size.width - 256.0).abs() < 0.01, "400 - 64 - 64 - 2*8");
    assert!((mid.left() - 72.0).abs() < 0.01);
}

#[test]
fn grid_basic_splits_into_equal_tracks() {
    let h = Harness::new(
        Grid::new([Track::Fill(1.0), Track::Fill(1.0), Track::Fill(1.0)])
            .width(300.0)
            .gap(4.0)
            .children((
                square(Color::RED),
                square(Color::GREEN),
                square(Color::BLUE),
                square(Color::RED),
                square(Color::GREEN),
                square(Color::BLUE),
            )),
    );
    h.layout(300.0, 200.0);
    let kids = h.children(h.root().unwrap());
    assert_eq!(kids.len(), 6);

    let cell = (300.0 - 8.0) / 3.0;
    for (i, x) in [0.0, cell + 4.0, 2.0 * (cell + 4.0)].iter().enumerate() {
        let r = h.rect(kids[i]).unwrap();
        assert!(
            (r.left() - x).abs() < 0.01,
            "column {i} at x={x}, got {}",
            r.left()
        );
        assert_eq!(r.size.width, 32.0, "child keeps its own 32px size");
    }
    let fourth = h.rect(kids[3]).unwrap();
    assert!(
        (fourth.left() - 0.0).abs() < 0.01,
        "second row starts at x=0"
    );
    assert!((fourth.top() - 36.0).abs() < 0.01, "32px row + 4px gap");
}

#[test]
fn grid_span_covers_multiple_tracks() {
    let h = Harness::new(
        Grid::new([
            Track::Fill(1.0),
            Track::Fill(1.0),
            Track::Fill(1.0),
            Track::Fill(1.0),
        ])
        .width(400.0)
        .rows([Track::Px(24.0), Track::Px(48.0), Track::Px(48.0)])
        .gap(4.0)
        .child_span(Panel::new().background(Color::RED), 0, 0, 4, 1)
        .child_span(Panel::new().background(Color::GREEN), 0, 1, 1, 2)
        .child_span(Panel::new().background(Color::BLUE), 1, 1, 3, 2),
    );
    h.layout(400.0, 300.0);
    let kids = h.children(h.root().unwrap());
    assert_eq!(kids.len(), 3);
    let col = (400.0 - 12.0) / 4.0;
    let header = h.rect(kids[0]).unwrap();
    assert!(
        (header.size.width - 400.0).abs() < 0.01,
        "header spans all 4 columns"
    );
    assert!((header.size.height - 24.0).abs() < 0.01);
    let side = h.rect(kids[1]).unwrap();
    assert!((side.left() - 0.0).abs() < 0.01);
    assert!((side.size.width - col).abs() < 0.01);
    assert!((side.top() - 28.0).abs() < 0.01, "24px row + 4px gap");
    let content = h.rect(kids[2]).unwrap();
    assert!((content.left() - (col + 4.0)).abs() < 0.01);
    assert!((content.size.width - (3.0 * col + 8.0)).abs() < 0.01);
}

#[test]
fn center_puts_child_in_the_middle() {
    let h = Harness::new(
        Column::new()
            .size(400.0, 80.0)
            .center()
            .background(Color::hex(0x2A2A2E))
            .child(square(Color::BLUE)),
    );
    h.layout(400.0, 80.0);
    let kids = h.children(h.root().unwrap());
    let r = h.rect(kids[0]).unwrap();
    assert!((r.left() - 184.0).abs() < 0.01, "(400-32)/2");
    assert!((r.top() - 24.0).abs() < 0.01, "(80-32)/2");
}

#[test]
fn stack_offsets_overlap_like_gpui_absolute() {
    let h = Harness::new(
        Stack::new()
            .size(200.0, 200.0)
            .background(Color::hex(0x2A2A2E))
            .child_at(
                Panel::new().square(40.0).background(Color::RED),
                StackPos::top_left().offset(8.0, 8.0),
            )
            .child_at(
                Panel::new().square(40.0).background(Color::GREEN),
                StackPos::top_left().offset(16.0, 16.0),
            )
            .child_at(
                Panel::new().square(40.0).background(Color::BLUE),
                StackPos::top_left().offset(24.0, 24.0),
            ),
    );
    h.layout(200.0, 200.0);
    let kids = h.children(h.root().unwrap());
    assert_eq!(kids.len(), 3);
    for (k, o) in kids.iter().zip([8.0, 16.0, 24.0]) {
        let r = h.rect(*k).unwrap();
        assert!((r.left() - o).abs() < 0.01, "offset x={o}");
        assert!((r.top() - o).abs() < 0.01, "offset y={o}");
        assert_eq!(r.size.width, 40.0);
    }

    let list = h.paint();
    let quads: Vec<_> = list.quads().collect();
    assert_eq!(quads.len(), 4);
    assert_eq!(quads[1].color, Color::RED);
    assert_eq!(quads[3].color, Color::BLUE);
}

#[test]
fn app_shell_header_sidebar_content_rects() {
    let h = Harness::new(
        Column::new()
            .size(400.0, 128.0)
            .child(
                Column::new()
                    .fill_width()
                    .height(24.0)
                    .background(Color::RED),
            )
            .child_flex(
                Row::new()
                    .child(Column::new().width(64.0).background(Color::GREEN))
                    .child_flex(Column::new().background(Color::BLUE), 1.0),
                1.0,
            ),
    );
    h.layout(400.0, 128.0);
    let root = h.root().unwrap();
    let top = h.children(root);
    let header = h.rect(top[0]).unwrap();
    assert_eq!(header.size.height, 24.0);
    let body = h.children(top[1]);
    let side = h.rect(body[0]).unwrap();
    let content = h.rect(body[1]).unwrap();
    assert_eq!(side.size.width, 64.0);
    assert!(
        (content.size.width - 336.0).abs() < 0.01,
        "400 - 64 sidebar"
    );
    assert!((content.left() - 64.0).abs() < 0.01);
}
