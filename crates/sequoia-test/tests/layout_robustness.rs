#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_core::{Length, LengthExt, Rect};
use sequoia_test::Harness;
use sequoia_view::prelude::*;

fn approx(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}

fn assert_rect(rect: Rect, x: f32, y: f32, w: f32, h: f32) {
    assert!(
        approx(rect.left(), x)
            && approx(rect.top(), y)
            && approx(rect.size.width, w)
            && approx(rect.size.height, h),
        "expected ({x}, {y}, {w}, {h}), got {rect:?}"
    );
}

fn assert_finite_rect(rect: Rect, what: &str) {
    assert!(
        rect.left().is_finite()
            && rect.top().is_finite()
            && rect.size.width.is_finite()
            && rect.size.height.is_finite(),
        "{what} has a non-finite rect: {rect:?}"
    );
    assert!(
        rect.size.width >= 0.0 && rect.size.height >= 0.0,
        "{what} has a negative size: {rect:?}"
    );
}

#[test]
fn percent_height_in_unbounded_scroll_content_stays_finite() {
    let h = Harness::new(ScrollView::new(
        Column::new()
            .width(1.0.fill())
            .child(Panel::new().width(1.0.fill()).height(50.0.percent()))
            .child(Panel::new().width(1.0.fill()).height(20.0)),
    ));
    h.layout(300.0, 200.0);

    let root = h.root().unwrap();
    let col = h.children(root)[0];
    let kids = h.children(col);
    assert_eq!(kids.len(), 2);

    assert_finite_rect(h.rect(col).unwrap(), "scroll content column");
    for (i, k) in kids.iter().enumerate() {
        assert_finite_rect(h.rect(*k).unwrap(), &format!("child {i}"));
    }
    assert!(
        h.rect(kids[1]).unwrap().size.height >= 20.0 - 0.01,
        "fixed sibling keeps its height: {:?}",
        h.rect(kids[1]).unwrap()
    );
}

#[test]
fn percent_width_in_horizontal_scroll_content_stays_finite() {
    let h = Harness::new(
        ScrollView::new(
            Row::new()
                .height(1.0.fill())
                .child(Panel::new().width(30.0.percent()).height(20.0))
                .child(Panel::new().width(40.0).height(20.0)),
        )
        .horizontal(),
    );
    h.layout(300.0, 200.0);

    let root = h.root().unwrap();
    let row = h.children(root)[0];
    for k in h.children(row) {
        assert_finite_rect(h.rect(k).unwrap(), "scroll row child");
    }
}

#[test]
fn percent_sizes_resolve_against_the_parent_content_box() {
    let h = Harness::new(
        Row::new()
            .width(200.0)
            .height(100.0)
            .padding(20.0)
            .child(Panel::new().width(50.0.percent()).height(50.0.percent())),
    );
    h.layout(400.0, 400.0);
    let kid = h.children(h.root().unwrap())[0];
    assert_rect(h.rect(kid).unwrap(), 20.0, 20.0, 80.0, 30.0);
}

#[test]
fn padding_insets_flow_children() {
    let h = Harness::new(
        Column::new()
            .size(200.0, 100.0)
            .padding(10.0)
            .child(Panel::new().width(50.0).height(20.0))
            .child(Panel::new().width(60.0).height(20.0)),
    );
    h.layout(300.0, 300.0);
    let root = h.root().unwrap();
    assert_rect(h.rect(root).unwrap(), 0.0, 0.0, 200.0, 100.0);
    let kids = h.children(root);
    assert_rect(h.rect(kids[0]).unwrap(), 10.0, 10.0, 50.0, 20.0);
    assert_rect(h.rect(kids[1]).unwrap(), 10.0, 30.0, 60.0, 20.0);
}

#[test]
fn fill_child_stays_inside_the_padded_content_box() {
    let h = Harness::new(
        Column::new()
            .size(200.0, 100.0)
            .padding(10.0)
            .child(Panel::new().fill_size()),
    );
    h.layout(300.0, 300.0);
    let kid = h.children(h.root().unwrap())[0];
    assert_rect(h.rect(kid).unwrap(), 10.0, 10.0, 180.0, 80.0);
}

#[test]
fn margin_offsets_children_and_reserves_main_space() {
    let h = Harness::new(
        Row::new()
            .size(200.0, 50.0)
            .child(Panel::new().width(50.0).height(20.0).margin(10.0))
            .child(Panel::new().width(50.0).height(20.0).margin(10.0)),
    );
    h.layout(300.0, 300.0);
    let kids = h.children(h.root().unwrap());
    assert_rect(h.rect(kids[0]).unwrap(), 10.0, 10.0, 50.0, 20.0);
    assert_rect(h.rect(kids[1]).unwrap(), 80.0, 10.0, 50.0, 20.0);
}

#[test]
fn min_width_stops_shrink_and_the_sibling_absorbs_the_deficit() {
    let h = Harness::new(
        Row::new()
            .width(100.0)
            .height(40.0)
            .child(
                Panel::new()
                    .width(80.0.percent())
                    .height(40.0)
                    .min_width(70.0),
            )
            .child(Panel::new().width(80.0.percent()).height(40.0)),
    );
    h.layout(300.0, 300.0);
    let kids = h.children(h.root().unwrap());
    let a = h.rect(kids[0]).unwrap();
    let b = h.rect(kids[1]).unwrap();
    assert!(approx(a.size.width, 70.0), "min floor: {a:?}");
    assert!(
        approx(b.size.width, 30.0),
        "sibling gives up the rest: {b:?}"
    );
    assert!(approx(b.left(), 70.0), "no gap between them: {b:?}");
}

#[test]
fn percent_sized_direct_scroll_content_stays_finite() {
    let h = Harness::new(ScrollView::new(
        Panel::new().width(1.0.fill()).height(50.0.percent()),
    ));
    h.layout(300.0, 200.0);
    let root = h.root().unwrap();
    for child in h.children(root) {
        assert_finite_rect(h.rect(child).unwrap(), "scroll content panel");
    }
}

#[test]
fn explicit_px_width_is_not_squashed_by_a_narrow_row() {
    let h = Harness::new(
        Row::new()
            .width(100.0)
            .height(40.0)
            .child(Panel::new().width(120.0).height(40.0))
            .child(Panel::new().width(40.0).height(40.0)),
    );
    h.layout(300.0, 300.0);
    let kids = h.children(h.root().unwrap());
    assert!(
        approx(h.rect(kids[0]).unwrap().size.width, 120.0),
        "explicit px keeps its width: {:?}",
        h.rect(kids[0]).unwrap()
    );
}

#[test]
fn max_width_caps_a_filling_child_and_hands_rest_to_the_sibling() {
    let h = Harness::new(
        Row::new()
            .width(200.0)
            .height(40.0)
            .child(
                Column::new()
                    .fill_width()
                    .max_width(40.0)
                    .height(1.0.fill()),
            )
            .child(Column::new().fill_width().height(1.0.fill())),
    );
    h.layout(300.0, 300.0);
    let kids = h.children(h.root().unwrap());
    assert!(approx(h.rect(kids[0]).unwrap().size.width, 40.0));
    assert!(approx(h.rect(kids[1]).unwrap().size.width, 160.0));
}

#[test]
fn absolute_child_is_removed_from_the_flow() {
    let h = Harness::new(
        Column::new()
            .size(200.0, 100.0)
            .child(Panel::new().width(50.0).height(20.0))
            .child(
                Panel::new()
                    .width(50.0)
                    .height(20.0)
                    .absolute_at(100.0, 50.0),
            )
            .child(Panel::new().width(50.0).height(20.0)),
    );
    h.layout(300.0, 300.0);
    let kids = h.children(h.root().unwrap());
    assert_eq!(kids.len(), 3);
    assert_rect(h.rect(kids[0]).unwrap(), 0.0, 0.0, 50.0, 20.0);
    assert_rect(h.rect(kids[2]).unwrap(), 0.0, 20.0, 50.0, 20.0);
    assert_rect(h.rect(kids[1]).unwrap(), 100.0, 50.0, 50.0, 20.0);
}

#[test]
fn absolute_child_with_two_insets_stretches_between_them() {
    let h = Harness::new(Column::new().size(200.0, 100.0).child(
        Panel::new().absolute_inset(Inset::only(Some(10.0), Some(5.0), Some(10.0), Some(5.0))),
    ));
    h.layout(300.0, 300.0);
    let kid = h.children(h.root().unwrap())[0];
    assert_rect(h.rect(kid).unwrap(), 10.0, 5.0, 180.0, 90.0);
}

#[test]
fn absolute_child_is_placed_inside_the_parent_padding() {
    let h = Harness::new(
        Column::new()
            .size(200.0, 100.0)
            .padding(10.0)
            .child(Panel::new().width(20.0).height(20.0).absolute_at(0.0, 0.0)),
    );
    h.layout(300.0, 300.0);
    let kid = h.children(h.root().unwrap())[0];
    assert_rect(h.rect(kid).unwrap(), 10.0, 10.0, 20.0, 20.0);
}

#[test]
fn grid_row_fill_tracks_split_the_height() {
    let h = Harness::new(
        Grid::new([Track::Px(100.0)])
            .rows([Track::Fill(1.0), Track::Fill(1.0)])
            .height(200.0)
            .child(Panel::new().width(100.0))
            .child(Panel::new().width(100.0)),
    );
    h.layout(300.0, 300.0);
    let kids = h.children(h.root().unwrap());
    assert_rect(h.rect(kids[0]).unwrap(), 0.0, 0.0, 100.0, 100.0);
    assert_rect(h.rect(kids[1]).unwrap(), 0.0, 100.0, 100.0, 100.0);
}

#[test]
fn grid_spanning_child_blocks_the_cells_it_covers() {
    let h = Harness::new(
        Grid::new([Track::Px(50.0), Track::Px(50.0)])
            .rows([Track::Px(20.0), Track::Px(20.0)])
            .child_span(Panel::new(), 0, 0, 2, 1)
            .child(Panel::new()),
    );
    h.layout(300.0, 300.0);
    let kids = h.children(h.root().unwrap());
    assert_rect(h.rect(kids[0]).unwrap(), 0.0, 0.0, 100.0, 20.0);
    assert_rect(h.rect(kids[1]).unwrap(), 0.0, 20.0, 50.0, 20.0);
}

#[test]
fn dock_gap_separates_docked_sides_and_fill() {
    let h = Harness::new(
        DockArea::new()
            .width(400.0)
            .height(300.0)
            .gap(10.0)
            .top(Panel::new().height(50.0))
            .left(Panel::new().width(100.0))
            .fill(Panel::new()),
    );
    h.layout(400.0, 300.0);
    let kids = h.children(h.root().unwrap());
    assert_eq!(kids.len(), 3);
    let top = h.rect(kids[0]).unwrap();
    let left = h.rect(kids[1]).unwrap();
    let fill = h.rect(kids[2]).unwrap();
    assert_rect(top, 0.0, 0.0, 400.0, 50.0);
    assert_rect(left, 0.0, 60.0, 100.0, 240.0);
    assert_rect(fill, 110.0, 60.0, 290.0, 240.0);
}

#[test]
fn dock_percent_thickness_scales_with_the_area() {
    let h = Harness::new(
        DockArea::new()
            .width(400.0)
            .height(300.0)
            .left(Panel::new().width(25.0.percent()))
            .fill(Panel::new()),
    );
    h.layout(400.0, 300.0);
    let kids = h.children(h.root().unwrap());
    assert!(
        approx(h.rect(kids[0]).unwrap().size.width, 100.0),
        "25% of 400: {:?}",
        h.rect(kids[0]).unwrap()
    );
    assert_rect(h.rect(kids[1]).unwrap(), 100.0, 0.0, 300.0, 300.0);
}

#[test]
fn wrap_row_gap_is_used_between_lines_only() {
    let h = Harness::new(Wrap::new().width(100.0).gap(4.0).row_gap(6.0).children([
        Panel::new().width(40.0).height(20.0),
        Panel::new().width(40.0).height(20.0),
        Panel::new().width(40.0).height(20.0),
    ]));
    h.layout(300.0, 300.0);
    let kids = h.children(h.root().unwrap());
    assert_rect(h.rect(kids[1]).unwrap(), 44.0, 0.0, 40.0, 20.0);
    assert_rect(h.rect(kids[2]).unwrap(), 0.0, 26.0, 40.0, 20.0);
}

#[test]
fn nested_rows_and_columns_keep_every_rect_inside_its_parent() {
    let root = Column::new()
        .width(1.0.fill())
        .child(
            Row::new()
                .height(30.0)
                .child(Panel::new().width(40.0))
                .child(
                    Column::new().fill_width().child(
                        Row::new()
                            .height(20.0)
                            .child(Panel::new().width(30.0))
                            .child(Panel::new().fill_width()),
                    ),
                ),
        )
        .child(Panel::new().fill_size());
    let h = Harness::new(root);
    h.layout(300.0, 200.0);

    fn walk(h: &Harness, id: sequoia_view::ElementId, parent: Rect, depth: usize) {
        if depth > 12 {
            return;
        }
        let rect = h.rect(id).unwrap();
        assert_finite_rect(rect, "nested node");
        assert!(
            rect.left() >= parent.left() - 0.01
                && rect.top() >= parent.top() - 0.01
                && rect.right() <= parent.right() + 0.01
                && rect.bottom() <= parent.bottom() + 0.01,
            "child {rect:?} escapes parent {parent:?}"
        );
        for child in h.children(id) {
            walk(h, child, rect, depth + 1);
        }
    }

    let root_id = h.root().unwrap();
    walk(&h, root_id, h.rect(root_id).unwrap(), 0);
}

#[test]
fn repeated_layout_is_stable() {
    let h = Harness::new(
        Column::new()
            .fill_size()
            .padding(8.0)
            .gap(6.0)
            .child(Panel::new().fill_width().height(40.0))
            .child(
                Row::new()
                    .fill_width()
                    .flex(1.0)
                    .child(Panel::new().width(60.0).height(1.0.fill()))
                    .child(Text::new("hello").flex(1.0)),
            )
            .child(Wrap::new().fill_width().gap(4.0).children([
                Panel::new().width(50.0).height(20.0),
                Panel::new().width(50.0).height(20.0),
                Panel::new().width(50.0).height(20.0),
            ])),
    );
    h.layout(320.0, 240.0);
    let first = h.dump();
    h.layout(320.0, 240.0);
    let second = h.dump();
    assert_eq!(first, second, "layout must be idempotent");
}

#[test]
fn zero_sized_parent_keeps_children_finite() {
    let h = Harness::new(
        Column::new()
            .size(0.0, 0.0)
            .child(Panel::new().width(20.0).height(20.0))
            .child(Panel::new().fill_width().height(1.0.fill())),
    );
    h.layout(300.0, 300.0);
    for k in h.children(h.root().unwrap()) {
        assert_finite_rect(h.rect(k).unwrap(), "child of a zero-size parent");
    }
}

#[test]
fn layout_with_zero_window_does_not_panic_or_emit_non_finite_rects() {
    let h = Harness::new(
        Column::new()
            .fill_size()
            .child(Panel::new().fill_width().height(0.5.fill())),
    );
    h.layout(0.0, 0.0);
    let root = h.root().unwrap();
    for id in std::iter::once(root).chain(h.children(root)) {
        assert_finite_rect(h.rect(id).unwrap(), "zero-window node");
    }
}

#[test]
fn sibling_rects_do_not_overlap_in_a_row() {
    let widths = [10.0, 30.0, 20.0, 60.0, 5.0, 45.0];
    let mut row = Row::new().width(200.0).height(40.0);
    for w in widths {
        row = row.child(Panel::new().width(w).height(20.0));
    }
    let h = Harness::new(row);
    h.layout(400.0, 400.0);
    let kids = h.children(h.root().unwrap());
    for pair in kids.windows(2) {
        let a = h.rect(pair[0]).unwrap();
        let b = h.rect(pair[1]).unwrap();
        assert!(
            b.left() >= a.right() - 0.01,
            "siblings overlap: {a:?} and {b:?}"
        );
    }
}

#[test]
fn growing_children_never_exceed_the_container_main_axis() {
    for weight in [1.0, 2.5, 10.0] {
        let h = Harness::new(
            Row::new()
                .width(400.0)
                .height(40.0)
                .child(Panel::new().width(50.0).height(40.0))
                .child(Panel::new().flex(weight).height(40.0))
                .child(Panel::new().width(50.0).height(40.0)),
        );
        h.layout(400.0, 400.0);
        let kids = h.children(h.root().unwrap());
        let used: f32 = kids.iter().map(|k| h.rect(*k).unwrap().size.width).sum();
        assert!(
            (used - 400.0).abs() < 0.05,
            "weight {weight}: row used {used} of 400"
        );
    }
}

#[test]
fn length_percent_and_px_are_not_confused() {
    let h = Harness::new(
        Column::new()
            .size(100.0, 100.0)
            .child(
                Panel::new()
                    .width(Length::Px(20.0))
                    .height(Length::Px(20.0)),
            )
            .child(Panel::new().width(50.0.percent()).height(25.0.percent())),
    );
    h.layout(300.0, 300.0);
    let kids = h.children(h.root().unwrap());
    assert_rect(h.rect(kids[0]).unwrap(), 0.0, 0.0, 20.0, 20.0);
    assert_rect(h.rect(kids[1]).unwrap(), 0.0, 20.0, 50.0, 25.0);
}
