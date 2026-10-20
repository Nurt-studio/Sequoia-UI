#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::cell::Cell;
use std::rc::Rc;

use sequoia_reactive::Signal;
use sequoia_seq::{HostBindings, SeqLoader};
use sequoia_test::Harness;

const BOARD: &str = "app B {\n  title: \"B\"\n  size: (300, 200)\n  prop sel_r: Float\n  on click_at(x: Float, y: Float)\n  on drag_to(x: Float, y: Float)\n  window M {\n    Canvas(background: theme.bg, cursor: crosshair, on_click_at: { click_at(x, y) }, on_drag: { drag_to(x, y) }) {\n      Grid(step: 24, color: theme.border)\n      Rect(x: 40, y: 40, w: 120, h: 80, fill: theme.surface, radius: 6, name: \"a\")\n      Rect(x: 200, y: 100, w: 120, h: 80, fill: theme.accent, radius: 6, name: \"b\")\n      Circle(c: (40, 40), r: dynamic(sel_r), fill: #FFFFFF, name: \"sel\")\n    }\n  }\n}\n";

type BoardFixture = (
    Harness,
    Signal<f64>,
    Rc<Cell<Vec<(f64, f64)>>>,
    Rc<Cell<Vec<(f64, f64)>>>,
);

fn board() -> BoardFixture {
    let r = Signal::new(6.0);
    let clicks = Rc::new(Cell::new(Vec::new()));
    let drags = Rc::new(Cell::new(Vec::new()));
    let c = clicks.clone();
    let d = drags.clone();
    let b = HostBindings::new()
        .prop_float("sel_r", r.clone())
        .on_point("click_at", move |x, y| {
            let mut v = c.take();
            v.push((x, y));
            c.set(v);
        })
        .on_point("drag_to", move |x, y| {
            let mut v = d.take();
            v.push((x, y));
            d.set(v);
        });
    let loaded = SeqLoader::load_str_with("Board.seq", BOARD, &b).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(300.0, 200.0);
    h.paint();
    h.tick();
    (h, r, clicks, drags)
}

#[test]
fn board_paints_shapes() {
    let (h, _r, _c, _d) = board();
    assert!(!h.paint().is_empty(), "canvas maluje");
}

#[test]
fn click_on_topmost_named_shape_fires() {
    let (h, _r, clicks, drags) = board();

    h.click(250.0, 140.0);
    h.tick();
    assert_eq!(clicks.take().len(), 1, "click on the shape fires");
    assert!(drags.take().is_empty(), "klik to nie drag");
}

#[test]
fn click_on_empty_space_fires_without_pick() {
    let (h, _r, clicks, _d) = board();

    h.click(10.0, 190.0);
    h.tick();
    assert_eq!(clicks.take().len(), 1, "click on empty also fires");
}

#[test]
fn drag_fires_drag_not_click() {
    let (h, _r, clicks, drags) = board();
    h.pointer_down(250.0, 140.0);
    h.pointer_move(270.0, 160.0);
    h.pointer_up(270.0, 160.0);
    h.tick();
    assert!(
        !drags.take().is_empty(),
        "moving above the threshold is drag"
    );
    assert!(clicks.take().is_empty(), "drag nie klika");
}

#[test]
fn dynamic_radius_changes_hit_area() {
    let src = "app B {\n  title: \"B\"\n  size: (300, 200)\n  prop sel_r: Float\n  on pick(x: Float, y: Float, name: String)\n  window M {\n    Canvas(on_click_at: { pick(x, y, name) }) {\n      Circle(c: (40, 40), r: dynamic(sel_r), fill: #FFFFFF, name: \"sel\")\n    }\n  }\n}\n";
    let r = Signal::new(6.0);
    let picked = Rc::new(Cell::new(String::from("?")));
    let p = picked.clone();
    let b = HostBindings::new()
        .prop_float("sel_r", r.clone())
        .on_point_named("pick", move |_x, _y, name| p.set(name));
    let loaded = SeqLoader::load_str_with("P.seq", src, &b).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(300.0, 200.0);
    h.paint();
    h.tick();

    h.click(40.0, 60.0);
    h.tick();
    assert_eq!(picked.take(), "", "small wheel doesn't catch");
    r.set(30.0);
    h.layout(300.0, 200.0);
    h.paint();
    h.tick();
    h.click(40.0, 60.0);
    h.tick();
    assert_eq!(picked.take(), "sel", "big wheel catches (dynamic reread)");
}

#[test]
fn named_pick_reaches_three_arg_callback() {
    let src = "app B {\n  title: \"B\"\n  size: (300, 200)\n  on pick(x: Float, y: Float, name: String)\n  window M {\n    Canvas(on_click_at: { pick(x, y, name) }) {\n      Rect(x: 40, y: 40, w: 120, h: 80, fill: #FFFFFF, name: \"a\")\n    }\n  }\n}\n";
    let picked = Rc::new(Cell::new(String::new()));
    let p = picked.clone();
    let b = HostBindings::new().on_point_named("pick", move |_x, _y, name| p.set(name));
    let loaded = SeqLoader::load_str_with("P.seq", src, &b).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(300.0, 200.0);
    h.paint();
    h.tick();
    h.click(100.0, 80.0);
    h.tick();
    assert_eq!(picked.take(), "a", "the hit carries the name");
    h.click(10.0, 10.0);
    h.tick();
    assert_eq!(picked.take(), "", "the box carries an empty name");
}

#[test]
fn shape_outside_canvas_is_rejected() {
    let src = "app A {\n  title: \"A\"\n  size: (10, 10)\n  window M {\n    Rect(x: 0, y: 0, w: 10, h: 10, fill: #FFFFFF)\n  }\n}\n";
    let err = match SeqLoader::load_str("Main.seq", src) {
        Err(e) => e,
        Ok(_) => panic!("oczekiwano bledu"),
    };
    assert!(err.message.contains("Canvas"), "{err}");
}

#[test]
fn touch_tap_fires_click_through_mouse_compat() {
    let (h, _r, clicks, drags) = board();
    h.touch_down(1, 250.0, 140.0, None);
    h.touch_up(1, 250.0, 140.0);
    h.tick();
    assert_eq!(clicks.take().len(), 1, "tap palcem klika raz");
    assert!(drags.take().is_empty(), "tap to nie drag");
}

#[test]
fn touch_drag_fires_drag_not_click() {
    let (h, _r, clicks, drags) = board();
    h.touch_down(1, 250.0, 140.0, None);
    h.touch_move(1, 270.0, 160.0, None);
    h.touch_up(1, 270.0, 160.0);
    h.tick();
    assert!(!drags.take().is_empty(), "swiping is drag");
    assert!(clicks.take().is_empty(), "drag palcem nie klika");
}

#[test]
fn pinch_fires_on_pinch() {
    let src = "app B {\n  title: \"B\"\n  size: (300, 200)\n  on zoom_at(x: Float, y: Float, s: Float)\n  window M {\n    Canvas(on_pinch: { zoom_at(x, y, s) }) {\n      Rect(x: 40, y: 40, w: 120, h: 80, fill: #FFFFFF)\n    }\n  }\n}\n";
    let got = Rc::new(Cell::new(Vec::new()));
    let g = got.clone();
    let b = HostBindings::new().on_pinch("zoom_at", move |x, y, s| {
        let mut v = g.take();
        v.push((x, y, s));
        g.set(v);
    });
    let loaded = SeqLoader::load_str_with("P.seq", src, &b).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(300.0, 200.0);
    h.paint();
    h.tick();
    h.pinch(100.0, 80.0, 0.5);
    h.tick();
    let v = got.take();
    assert_eq!(v.len(), 1, "pinch odpala on_pinch raz");
    assert!((v[0].2 - 0.5).abs() < 1e-9, "delta skali przechodzi");
}

#[test]
fn on_pinch_rejected_without_binding() {
    let src = "app B {\n  title: \"B\"\n  size: (300, 200)\n  on zoom_at(x: Float, y: Float, s: Float)\n  window M {\n    Canvas(on_pinch: { zoom_at(x, y, s) }) {\n      Rect(x: 40, y: 40, w: 120, h: 80, fill: #FFFFFF)\n    }\n  }\n}\n";
    let err = match SeqLoader::load_str_with("P.seq", src, &HostBindings::new()) {
        Err(e) => e,
        Ok(_) => panic!("oczekiwano bledu"),
    };
    assert!(err.message.contains("without binding"), "{err}");
}

fn transformed_center(list: &sequoia_core::RenderList) -> sequoia_core::Point {
    use sequoia_core::{Point, RenderItem};
    for (i, item) in list.items().iter().enumerate() {
        if let RenderItem::Quad(q) = item {
            let c = Point::new(q.rect.left() + 10.0, q.rect.top() + 10.0);
            return list.item_transform(i).apply(c);
        }
    }
    panic!("oczekiwano jednego quada")
}

fn pivot_case(rotate: &str, bindings: HostBindings) -> sequoia_core::Point {
    let src = format!(
        "app B {{\n  title: \"B\"\n  size: (300, 300)\n  prop a: Float\n  window M {{\n    Canvas(width: fill(1), height: fill(1)) {{\n      Group(pivot: (150, 150), rotate: {rotate}) {{\n        Rect(x: 140, y: 20, w: 20, h: 20, fill: #FFFFFF)\n      }}\n    }}\n  }}\n}}\n"
    );
    let loaded = SeqLoader::load_str_with("P.seq", &src, &bindings).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(300.0, 300.0);
    transformed_center(&h.paint())
}

#[test]
fn group_pivot_rotates_around_center() {
    let b = HostBindings::new().prop_float("a", Signal::new(0.0));
    let c = pivot_case("90", b);

    assert!((c.x - 270.0).abs() < 0.01, "x po obrocie: {c:?}");
    assert!((c.y - 150.0).abs() < 0.01, "y po obrocie: {c:?}");
}

#[test]
fn group_pivot_accepts_dynamic_angle() {
    let b = HostBindings::new().prop_float("a", Signal::new(90.0));
    let c = pivot_case("dynamic(a)", b);
    assert!((c.x - 270.0).abs() < 0.01, "x po obrocie: {c:?}");
    assert!((c.y - 150.0).abs() < 0.01, "y po obrocie: {c:?}");
}

#[test]
fn group_without_pivot_keeps_legacy_origin_rotation() {
    let src = "app B {\n  title: \"B\"\n  size: (300, 300)\n  window M {\n    Canvas(width: fill(1), height: fill(1)) {\n      Group(rotate: 30) {\n        Rect(x: 140, y: 20, w: 20, h: 20, fill: #FFFFFF)\n      }\n    }\n  }\n}\n";
    let loaded = SeqLoader::load_str("P.seq", src).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(300.0, 300.0);
    let c = transformed_center(&h.paint());
    assert!((c.x - 114.90).abs() < 0.05, "x bez pivota: {c:?}");
    assert!((c.y - 100.98).abs() < 0.05, "y bez pivota: {c:?}");
}

#[test]
fn group_pivot_anchors_canvas_origin() {
    let src = "app B {\n  title: \"B\"\n  size: (400, 300)\n  window M {\n    Row(width: fill(1), height: fill(1)) {\n      Container(width: px(50), height: fill(1)) {\n      }\n      Canvas(width: fill(1), height: fill(1)) {\n        Group(pivot: (150, 150), rotate: 90) {\n          Rect(x: 140, y: 20, w: 20, h: 20, fill: #FFFFFF)\n        }\n      }\n    }\n  }\n}\n";
    let loaded = SeqLoader::load_str("P.seq", src).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(400.0, 300.0);
    let c = transformed_center(&h.paint());
    assert!((c.x - 320.0).abs() < 0.01, "x with anchor og: {c:?}");
    assert!((c.y - 150.0).abs() < 0.01, "y with anchor og: {c:?}");
}

#[test]
fn group_pivot_outside_group_is_rejected() {
    let src = "app B {\n  title: \"B\"\n  size: (300, 300)\n  window M {\n    Canvas(width: fill(1), height: fill(1)) {\n      Rect(x: 0, y: 0, w: 10, h: 10, fill: #FFFFFF, pivot: (5, 5))\n    }\n  }\n}\n";
    let err = match SeqLoader::load_str("P.seq", src) {
        Err(e) => e,
        Ok(_) => panic!("oczekiwano bledu"),
    };
    assert!(err.message.contains("pivot"), "{err}");
}
