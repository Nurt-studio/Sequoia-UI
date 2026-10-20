#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_core::{LengthExt, Rect};
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
fn sides_dock_in_order_and_fill_takes_rest() {
    let h = Harness::new(
        DockArea::new()
            .width(400.0)
            .height(300.0)
            .left(Panel::new().width(100.0))
            .right(Panel::new().width(80.0))
            .top(Panel::new().height(50.0))
            .bottom(Panel::new().height(30.0))
            .fill(Panel::new()),
    );
    h.layout(400.0, 300.0);
    let kids = h.children(h.root().unwrap());
    assert_eq!(kids.len(), 5);
    assert_rect(h.rect(kids[2]).unwrap(), 0.0, 0.0, 400.0, 50.0);
    assert_rect(h.rect(kids[3]).unwrap(), 0.0, 270.0, 400.0, 30.0);
    assert_rect(h.rect(kids[0]).unwrap(), 0.0, 50.0, 100.0, 220.0);
    assert_rect(h.rect(kids[1]).unwrap(), 320.0, 50.0, 80.0, 220.0);
    assert_rect(h.rect(kids[4]).unwrap(), 100.0, 50.0, 220.0, 220.0);
}

#[test]
fn legacy_ordered_sides_dock_sequentially() {
    let h = Harness::new(
        DockArea::new()
            .width(400.0)
            .height(300.0)
            .ordered(true)
            .left(Panel::new().width(100.0))
            .right(Panel::new().width(80.0))
            .top(Panel::new().height(50.0))
            .bottom(Panel::new().height(30.0))
            .fill(Panel::new()),
    );
    h.layout(400.0, 300.0);
    let kids = h.children(h.root().unwrap());
    assert_eq!(kids.len(), 5);
    assert_rect(h.rect(kids[0]).unwrap(), 0.0, 0.0, 100.0, 300.0);
    assert_rect(h.rect(kids[1]).unwrap(), 320.0, 0.0, 80.0, 300.0);
    assert_rect(h.rect(kids[2]).unwrap(), 100.0, 0.0, 220.0, 50.0);
    assert_rect(h.rect(kids[3]).unwrap(), 100.0, 270.0, 220.0, 30.0);
    assert_rect(h.rect(kids[4]).unwrap(), 100.0, 50.0, 220.0, 220.0);
}

#[test]
fn dock_order_matters_top_before_left() {
    let h = Harness::new(
        DockArea::new()
            .width(400.0)
            .height(300.0)
            .top(Panel::new().height(50.0))
            .left(Panel::new().width(100.0))
            .fill(Panel::new()),
    );
    h.layout(400.0, 300.0);
    let kids = h.children(h.root().unwrap());
    assert_eq!(kids.len(), 3);
    assert_rect(h.rect(kids[0]).unwrap(), 0.0, 0.0, 400.0, 50.0);
    assert_rect(h.rect(kids[1]).unwrap(), 0.0, 50.0, 100.0, 250.0);
    assert_rect(h.rect(kids[2]).unwrap(), 100.0, 50.0, 300.0, 250.0);
}

#[test]
fn border_order_is_declaration_independent() {
    for area in [
        DockArea::new()
            .width(400.0)
            .height(300.0)
            .top(Panel::new().height(50.0))
            .left(Panel::new().width(100.0))
            .fill(Panel::new()),
        DockArea::new()
            .width(400.0)
            .height(300.0)
            .left(Panel::new().width(100.0))
            .top(Panel::new().height(50.0))
            .fill(Panel::new()),
        DockArea::new()
            .width(400.0)
            .height(300.0)
            .fill(Panel::new())
            .left(Panel::new().width(100.0))
            .top(Panel::new().height(50.0)),
    ] {
        let h = Harness::new(area);
        h.layout(400.0, 300.0);
        let kids = h.children(h.root().unwrap());
        assert_eq!(kids.len(), 3);
        let mut rects: Vec<Rect> = kids.iter().map(|k| h.rect(*k).unwrap()).collect();
        rects.sort_by(|a, b| {
            a.top()
                .partial_cmp(&b.top())
                .unwrap()
                .then(a.left().partial_cmp(&b.left()).unwrap())
        });
        assert_rect(rects[0], 0.0, 0.0, 400.0, 50.0);
        assert_rect(rects[1], 0.0, 50.0, 100.0, 250.0);
        assert_rect(rects[2], 100.0, 50.0, 300.0, 250.0);
    }
}

#[test]
fn fill_is_hoisted_before_bottom() {
    let h = Harness::new(
        DockArea::new()
            .width(400.0)
            .height(300.0)
            .top(Panel::new().height(50.0))
            .fill(Panel::new())
            .bottom(Panel::new().height(30.0)),
    );
    h.layout(400.0, 300.0);
    let kids = h.children(h.root().unwrap());
    assert_eq!(kids.len(), 3);
    assert_rect(h.rect(kids[0]).unwrap(), 0.0, 0.0, 400.0, 50.0);
    assert_rect(h.rect(kids[2]).unwrap(), 0.0, 270.0, 400.0, 30.0);
    assert_rect(h.rect(kids[1]).unwrap(), 0.0, 50.0, 400.0, 220.0);
}

#[test]
fn last_side_takes_rest_by_default() {
    let h = Harness::new(
        DockArea::new()
            .width(400.0)
            .height(300.0)
            .left(Panel::new().width(100.0))
            .left(Panel::new().width(80.0)),
    );
    h.layout(400.0, 300.0);
    let kids = h.children(h.root().unwrap());
    assert_eq!(kids.len(), 2);
    assert_rect(h.rect(kids[0]).unwrap(), 0.0, 0.0, 100.0, 300.0);
    assert_rect(h.rect(kids[1]).unwrap(), 100.0, 0.0, 80.0, 300.0);
}

#[test]
fn legacy_ordered_last_child_fill_takes_rest() {
    let h = Harness::new(
        DockArea::new()
            .width(400.0)
            .height(300.0)
            .ordered(true)
            .last_child_fill(true)
            .left(Panel::new().width(100.0))
            .left(Panel::new().width(80.0)),
    );
    h.layout(400.0, 300.0);
    let kids = h.children(h.root().unwrap());
    assert_eq!(kids.len(), 2);
    assert_rect(h.rect(kids[0]).unwrap(), 0.0, 0.0, 100.0, 300.0);
    assert_rect(h.rect(kids[1]).unwrap(), 100.0, 0.0, 300.0, 300.0);
}

#[test]
fn last_child_fill_false_leaves_rest_empty() {
    let h = Harness::new(
        DockArea::new()
            .width(400.0)
            .height(300.0)
            .last_child_fill(false)
            .left(Panel::new().width(100.0))
            .left(Panel::new().width(80.0)),
    );
    h.layout(400.0, 300.0);
    let kids = h.children(h.root().unwrap());
    assert_eq!(kids.len(), 2);
    assert_rect(h.rect(kids[0]).unwrap(), 0.0, 0.0, 100.0, 300.0);
    assert_rect(h.rect(kids[1]).unwrap(), 100.0, 0.0, 80.0, 300.0);
}

#[test]
fn gap_separates_side_docks() {
    let h = Harness::new(
        DockArea::new()
            .width(400.0)
            .height(300.0)
            .gap(10.0)
            .left(Panel::new().width(100.0))
            .fill(Panel::new()),
    );
    h.layout(400.0, 300.0);
    let kids = h.children(h.root().unwrap());
    assert_eq!(kids.len(), 2);
    assert_rect(h.rect(kids[0]).unwrap(), 0.0, 0.0, 100.0, 300.0);
    assert_rect(h.rect(kids[1]).unwrap(), 110.0, 0.0, 290.0, 300.0);
}

#[test]
fn padding_shrinks_dock_area() {
    let h = Harness::new(
        DockArea::new()
            .width(400.0)
            .height(300.0)
            .padding(10.0)
            .left(Panel::new().width(100.0))
            .fill(Panel::new()),
    );
    h.layout(400.0, 300.0);
    let kids = h.children(h.root().unwrap());
    assert_eq!(kids.len(), 2);
    assert_rect(h.rect(kids[0]).unwrap(), 10.0, 10.0, 100.0, 280.0);
    assert_rect(h.rect(kids[1]).unwrap(), 110.0, 10.0, 280.0, 280.0);
}

#[test]
fn percent_thickness_scales_with_remaining() {
    let h = Harness::new(
        DockArea::new()
            .width(400.0)
            .height(300.0)
            .left(Panel::new().width(25.0_f32.percent()))
            .fill(Panel::new()),
    );
    h.layout(400.0, 300.0);
    let kids = h.children(h.root().unwrap());
    assert_eq!(kids.len(), 2);
    assert_rect(h.rect(kids[0]).unwrap(), 0.0, 0.0, 100.0, 300.0);
    assert_rect(h.rect(kids[1]).unwrap(), 100.0, 0.0, 300.0, 300.0);
}

#[test]
fn auto_wraps_to_content() {
    let h = Harness::new(
        DockArea::new()
            .left(Panel::new().width(100.0).height(200.0))
            .fill(Panel::new().width(50.0).height(60.0)),
    );
    h.layout(400.0, 400.0);
    let root = h.root().unwrap();
    assert_rect(h.rect(root).unwrap(), 0.0, 0.0, 150.0, 200.0);
    let kids = h.children(root);
    assert_eq!(kids.len(), 2);
    assert_rect(h.rect(kids[0]).unwrap(), 0.0, 0.0, 100.0, 200.0);
    assert_rect(h.rect(kids[1]).unwrap(), 100.0, 0.0, 50.0, 200.0);
}

#[test]
fn dock_area_measures_inside_column() {
    struct Demo;
    impl View for Demo {
        fn body(&self) -> impl View + 'static {
            Column::new()
                .child(
                    DockArea::new()
                        .left(Panel::new().width(100.0).height(200.0))
                        .fill(Panel::new().width(50.0).height(60.0)),
                )
                .child(Panel::new().width(400.0.fill()).height(30.0))
        }
    }
    let h = Harness::new(Demo);
    h.layout(400.0, 400.0);
    let col = h.children(h.root().unwrap())[0];
    let rows = h.children(col);
    assert_eq!(rows.len(), 2);
    assert_rect(h.rect(rows[0]).unwrap(), 0.0, 0.0, 150.0, 200.0);
    assert_rect(h.rect(rows[1]).unwrap(), 0.0, 200.0, 400.0, 30.0);
    let docked = h.children(rows[0]);
    assert_rect(h.rect(docked[0]).unwrap(), 0.0, 0.0, 100.0, 200.0);
    assert_rect(h.rect(docked[1]).unwrap(), 100.0, 0.0, 50.0, 200.0);
}

#[test]
fn overflow_clamps_to_remaining() {
    let h = Harness::new(
        DockArea::new()
            .width(400.0)
            .height(300.0)
            .last_child_fill(false)
            .left(Panel::new().width(300.0))
            .right(Panel::new().width(300.0)),
    );
    h.layout(400.0, 300.0);
    let kids = h.children(h.root().unwrap());
    assert_eq!(kids.len(), 2);
    assert_rect(h.rect(kids[0]).unwrap(), 0.0, 0.0, 300.0, 300.0);
    assert_rect(h.rect(kids[1]).unwrap(), 300.0, 0.0, 100.0, 300.0);
}

#[test]
fn second_fill_gets_empty_rect() {
    let h = Harness::new(
        DockArea::new()
            .width(400.0)
            .height(300.0)
            .last_child_fill(false)
            .fill(Panel::new())
            .fill(Panel::new()),
    );
    h.layout(400.0, 300.0);
    let kids = h.children(h.root().unwrap());
    assert_eq!(kids.len(), 2);
    assert_rect(h.rect(kids[0]).unwrap(), 0.0, 0.0, 400.0, 300.0);
    assert_rect(h.rect(kids[1]).unwrap(), 0.0, 0.0, 0.0, 0.0);
}

#[test]
fn side_stretches_across_remaining() {
    let h = Harness::new(
        DockArea::new()
            .width(400.0)
            .height(300.0)
            .left(Panel::new().width(100.0).height(40.0))
            .fill(Panel::new()),
    );
    h.layout(400.0, 300.0);
    let kids = h.children(h.root().unwrap());
    assert_rect(h.rect(kids[0]).unwrap(), 0.0, 0.0, 100.0, 300.0);
    assert_rect(h.rect(kids[1]).unwrap(), 100.0, 0.0, 300.0, 300.0);
}

#[test]
fn conditional_dock_helpers() {
    let h = Harness::new(
        DockArea::new()
            .width(400.0)
            .height(300.0)
            .left_if(false, || Panel::new().width(999.0))
            .left_if(true, || Panel::new().width(100.0))
            .right_opt(None::<Panel>)
            .right_opt(Some(Panel::new().width(80.0)))
            .fill(Panel::new()),
    );
    h.layout(400.0, 300.0);
    let kids = h.children(h.root().unwrap());
    assert_eq!(kids.len(), 3);
    assert_rect(h.rect(kids[0]).unwrap(), 0.0, 0.0, 100.0, 300.0);
    assert_rect(h.rect(kids[1]).unwrap(), 320.0, 0.0, 80.0, 300.0);
    assert_rect(h.rect(kids[2]).unwrap(), 100.0, 0.0, 220.0, 300.0);
}

#[test]
fn mut_api_moves_and_clears() {
    let mut area = DockArea::new()
        .width(400.0)
        .height(300.0)
        .left(Panel::new().width(100.0))
        .left(Panel::new().width(80.0))
        .left(Panel::new().width(60.0));
    area.move_child_mut(2, 0);
    assert_eq!(area.len(), 3);
    assert_eq!(area.get_dock(0), Some(sequoia_view::Dock::Left));
    area.set_dock_mut(0, sequoia_view::Dock::Top);
    assert_eq!(area.get_dock(0), Some(sequoia_view::Dock::Top));
    area.remove_mut(0);
    assert_eq!(area.len(), 2);
    area.dock_at_mut(0, Panel::new().width(20.0), sequoia_view::Dock::Top);
    assert_eq!(area.len(), 3);
    area.clear_mut();
    assert!(area.is_empty());
    assert_eq!(area.len(), 0);
    assert_eq!(area.get_dock(0), None);
}

#[test]
fn dock_panel_header_and_body_layout() {
    use sequoia_view::PanelId;
    let id = PanelId::new(7);
    let h = Harness::new(
        DockArea::new()
            .width(400.0)
            .height(300.0)
            .left(DockPanel::new(id, "Warstwy", Panel::new()).header_height(28.0))
            .fill(Panel::new()),
    );
    h.layout(400.0, 300.0);
    h.tick();
    let kids = h.children(h.root().unwrap());
    assert_eq!(kids.len(), 2);

    let panel_kids = h.children(kids[0]);
    assert_eq!(panel_kids.len(), 2);
    let head = h.rect(panel_kids[0]).unwrap();
    let body = h.rect(panel_kids[1]).unwrap();
    assert!(approx(head.size.height, 28.0), "{head:?}");
    assert!(approx(body.top(), head.bottom()), "{body:?} vs {head:?}");
}

#[test]
fn dock_panel_close_calls_back() {
    use sequoia_view::PanelId;
    use std::cell::Cell;
    use std::rc::Rc;
    let closed = Rc::new(Cell::new(false));
    let closed2 = closed.clone();
    let id = PanelId::new(9);
    let panel = DockPanel::new(id, "X", Panel::new())
        .header_height(28.0)
        .on_close(move || closed2.set(true));
    let h = Harness::new(
        DockArea::new()
            .width(400.0)
            .height(300.0)
            .left(panel)
            .fill(Panel::new()),
    );
    h.layout(400.0, 300.0);
    h.tick();
    let kids = h.children(h.root().unwrap());
    let panel_kids = h.children(kids[0]);
    assert_eq!(panel_kids.len(), 3, "header + × + content");
    let close_rect = h.rect(panel_kids[1]).unwrap();
    h.click(
        close_rect.left() + close_rect.size.width * 0.5,
        close_rect.top() + close_rect.size.height * 0.5,
    );
    assert!(closed.get(), "× calls on_close");
}
