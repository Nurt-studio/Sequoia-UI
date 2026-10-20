#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_core::{Color, Rect};
use sequoia_test::Harness;
use sequoia_view::prelude::*;

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
fn grid_fixed_columns_and_auto_flow_rows() {
    let h = Harness::new(
        Grid::new([Track::Px(100.0), Track::Px(200.0)])
            .child(Panel::new().height(50.0).background(Color::RED))
            .child(Panel::new().height(50.0).background(Color::BLUE))
            .child(Panel::new().height(30.0).background(Color::GREEN)),
    );
    h.layout(400.0, 400.0);
    let grid = h.root().unwrap();
    let kids = h.children(grid);
    assert_eq!(kids.len(), 3, "child beyond 2 columns wraps to row 2");
    assert_rect(h.rect(kids[0]).unwrap(), 0.0, 0.0, 100.0, 50.0);
    assert_rect(h.rect(kids[1]).unwrap(), 100.0, 0.0, 200.0, 50.0);
    assert_rect(h.rect(kids[2]).unwrap(), 0.0, 50.0, 100.0, 30.0);
}

#[test]
fn grid_fill_columns_split_remaining_space() {
    let h = Harness::new(
        Grid::new([Track::Px(100.0), Track::Fill(1.0), Track::Fill(3.0)])
            .child(Panel::new().height(40.0))
            .child(Panel::new().height(40.0))
            .child(Panel::new().height(40.0)),
    );
    h.layout(400.0, 400.0);
    let kids = h.children(h.root().unwrap());
    assert_rect(h.rect(kids[0]).unwrap(), 0.0, 0.0, 100.0, 40.0);

    assert_rect(h.rect(kids[1]).unwrap(), 100.0, 0.0, 75.0, 40.0);
    assert_rect(h.rect(kids[2]).unwrap(), 175.0, 0.0, 225.0, 40.0);
}

#[test]
fn grid_percent_columns_scale_with_available() {
    let h = Harness::new(
        Grid::new([Track::Percent(25.0), Track::Percent(75.0)])
            .child(Panel::new().height(40.0))
            .child(Panel::new().height(40.0)),
    );
    h.layout(400.0, 400.0);
    let kids = h.children(h.root().unwrap());
    assert_rect(h.rect(kids[0]).unwrap(), 0.0, 0.0, 100.0, 40.0);
    assert_rect(h.rect(kids[1]).unwrap(), 100.0, 0.0, 300.0, 40.0);
}

#[test]
fn grid_declared_rows_size_cells() {
    let h = Harness::new(
        Grid::new([Track::Px(100.0)])
            .rows([Track::Px(40.0), Track::Px(60.0)])
            .child(Panel::new().background(Color::RED))
            .child(Panel::new().background(Color::BLUE)),
    );
    h.layout(400.0, 400.0);
    let kids = h.children(h.root().unwrap());
    assert_rect(h.rect(kids[0]).unwrap(), 0.0, 0.0, 100.0, 40.0);
    assert_rect(h.rect(kids[1]).unwrap(), 0.0, 40.0, 100.0, 60.0);
}

#[test]
fn grid_gaps_push_cells_apart() {
    let h = Harness::new(
        Grid::new([Track::Px(50.0), Track::Px(50.0)])
            .gap(10.0)
            .child(Panel::new().height(20.0))
            .child(Panel::new().height(20.0))
            .child(Panel::new().height(20.0))
            .child(Panel::new().height(20.0)),
    );
    h.layout(400.0, 400.0);
    let kids = h.children(h.root().unwrap());
    assert_rect(h.rect(kids[0]).unwrap(), 0.0, 0.0, 50.0, 20.0);
    assert_rect(h.rect(kids[1]).unwrap(), 60.0, 0.0, 50.0, 20.0);
    assert_rect(h.rect(kids[2]).unwrap(), 0.0, 30.0, 50.0, 20.0);
    assert_rect(h.rect(kids[3]).unwrap(), 60.0, 30.0, 50.0, 20.0);
}

#[test]
fn grid_auto_columns_size_to_content() {
    let h = Harness::new(
        Grid::new([Track::Fit, Track::Fit])
            .child(Panel::new().width(60.0).height(20.0))
            .child(Panel::new().width(120.0).height(20.0))
            .child(Panel::new().width(90.0).height(20.0)),
    );
    h.layout(400.0, 400.0);
    let kids = h.children(h.root().unwrap());

    assert_rect(h.rect(kids[0]).unwrap(), 0.0, 0.0, 60.0, 20.0);
    assert_rect(h.rect(kids[1]).unwrap(), 90.0, 0.0, 120.0, 20.0);
    assert_rect(h.rect(kids[2]).unwrap(), 0.0, 20.0, 90.0, 20.0);
}

#[test]
fn grid_padding_insets_cells() {
    let h = Harness::new(
        Grid::new([Track::Px(100.0)])
            .padding(10.0)
            .child(Panel::new().height(40.0)),
    );
    h.layout(400.0, 400.0);
    let kids = h.children(h.root().unwrap());
    assert_rect(h.rect(kids[0]).unwrap(), 10.0, 10.0, 100.0, 40.0);
}

#[test]
fn grid_child_px_size_stays_smaller_than_cell() {
    let h = Harness::new(
        Grid::new([Track::Px(200.0), Track::Px(200.0)])
            .child(Panel::new().width(80.0).height(40.0))
            .child(Panel::new().height(40.0)),
    );
    h.layout(400.0, 400.0);
    let kids = h.children(h.root().unwrap());

    assert_rect(h.rect(kids[0]).unwrap(), 0.0, 0.0, 80.0, 40.0);
    assert_rect(h.rect(kids[1]).unwrap(), 200.0, 0.0, 200.0, 40.0);
}
