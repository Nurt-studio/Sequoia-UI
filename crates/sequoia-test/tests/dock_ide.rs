#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::cell::Cell;
use std::rc::Rc;

use sequoia_core::Rect;
use sequoia_dock::{DockDrop, DockModel, DockOp, PanelId};
use sequoia_reactive::Signal;
use sequoia_test::Harness;
use sequoia_view::prelude::*;

fn p(id: u64) -> PanelId {
    PanelId::new(id)
}

fn render(_id: PanelId) -> AnyView {
    AnyView::new(Panel::new())
}

fn approx(a: f32, b: f32) -> bool {
    (a - b).abs() < 1.0
}

fn rect_of(h: &Harness, id: ElementId) -> Rect {
    h.rect(id).expect("rect")
}

#[test]
fn split_layout_follows_model_frac() {
    let model = Signal::new(DockModel::single(p(1)));
    model.update(|m| {
        m.apply(DockOp::Insert {
            panel: p(2),
            target: p(1),
            drop: DockDrop::Right,
            frac: 0.5,
            moving: false,
        });
    });
    let h = Harness::new(DockView::new(model.clone(), render).fill_size());
    h.layout(400.0, 300.0);
    let root = h.root().unwrap();

    let split = h.children(root);
    assert_eq!(split.len(), 1, "root + one split");
    let slots = h.children(split[0]);
    assert_eq!(slots.len(), 2);
    let left = rect_of(&h, slots[0]);
    let right = rect_of(&h, slots[1]);

    assert!(approx(left.size.width, 196.5), "lewy: {left:?}");
    assert!(approx(right.size.width, 196.5), "prawy: {right:?}");
    assert!(approx(left.left(), 0.0), "{left:?}");
    assert!(approx(right.left(), 203.5), "{right:?}");
    assert!(approx(left.size.height, 300.0), "{left:?}");

    model.update(|m| {
        m.set_frac_at(&[], 0.25);
    });
    h.layout(400.0, 300.0);
    let slots = h.children(h.children(h.root().unwrap())[0]);
    let left = rect_of(&h, slots[0]);
    assert!(approx(left.size.width, 393.0 * 0.25), "frac 0.25: {left:?}");
}

#[test]
fn tabs_strip_lists_all_and_body_is_below() {
    let model = Signal::new(DockModel::single(p(1)));
    model.update(|m| {
        m.apply(DockOp::Insert {
            panel: p(2),
            target: p(1),
            drop: DockDrop::Tab,
            frac: 0.5,
            moving: false,
        });
        m.apply(DockOp::Insert {
            panel: p(3),
            target: p(1),
            drop: DockDrop::Tab,
            frac: 0.5,
            moving: false,
        });
    });
    let h = Harness::new(DockView::new(model.clone(), render).fill_size());
    h.layout(400.0, 300.0);

    let view_kids = h.children(h.root().unwrap());
    assert_eq!(view_kids.len(), 1, "DockView -> DockTabs");
    let parts = h.children(view_kids[0]);
    assert_eq!(parts.len(), 2, "bar + content");
    let strip_kids = h.children(parts[0]);
    assert_eq!(strip_kids.len(), 3, "trzy taby");
    let strip = rect_of(&h, parts[0]);
    let body = rect_of(&h, parts[1]);
    assert!(approx(strip.size.height, 28.0), "{strip:?}");
    assert!(approx(body.top(), 28.0), "{body:?}");
    assert!(approx(body.size.height, 272.0), "{body:?}");

    model.update(|m| {
        m.apply(DockOp::Activate { panel: p(1) });
    });
    h.tick();
    h.layout(400.0, 300.0);
    let view_kids = h.children(h.root().unwrap());
    let parts = h.children(view_kids[0]);
    assert_eq!(parts.len(), 2);
}

#[test]
fn float_layer_pins_cards_at_model_xy() {
    let model = Signal::new(DockModel::single(p(1)));
    model.update(|m| {
        m.apply(DockOp::Insert {
            panel: p(2),
            target: p(1),
            drop: DockDrop::Right,
            frac: 0.5,
            moving: false,
        });
        m.apply(DockOp::Float {
            panel: p(2),
            x: 50.0,
            y: 60.0,
            w: 200.0,
            h: 150.0,
        });
    });
    let h = Harness::new(
        Stack::new()
            .child(DockView::new(model.clone(), render).fill_size())
            .child(DockFloatLayer::new(model.clone(), render).fill_size()),
    );
    h.layout(800.0, 600.0);
    let layers = h.children(h.root().unwrap());
    assert_eq!(layers.len(), 2);
    let cards = h.children(layers[1]);
    assert_eq!(cards.len(), 1, "jedna karta overlay");
    let card = rect_of(&h, cards[0]);
    assert!(approx(card.left(), 50.0), "{card:?}");
    assert!(approx(card.top(), 60.0), "{card:?}");
    assert!(approx(card.size.width, 200.0), "{card:?}");
    assert!(approx(card.size.height, 150.0), "{card:?}");

    model.update(|m| {
        m.apply(DockOp::Float {
            panel: p(2),
            x: 100.0,
            y: 120.0,
            w: 200.0,
            h: 150.0,
        });
    });
    h.tick();
    h.layout(800.0, 600.0);
    let cards = h.children(h.children(h.root().unwrap())[1]);
    let card = rect_of(&h, cards[0]);
    assert!(approx(card.left(), 100.0), "{card:?}");
    assert!(approx(card.top(), 120.0), "{card:?}");
}

#[test]
fn dock_area_reorder_moves_first_to_front() {
    let area = DockArea::new()
        .width(400.0)
        .height(300.0)
        .last_child_fill(false)
        .left(Panel::new().width(100.0))
        .left(Panel::new().width(80.0))
        .left(Panel::new().width(60.0))
        .move_child(2, 0);
    assert_eq!(
        area.docks(),
        &[
            sequoia_view::Dock::Left,
            sequoia_view::Dock::Left,
            sequoia_view::Dock::Left
        ]
    );
    let h = Harness::new(area);
    h.layout(400.0, 300.0);
    let kids = h.children(h.root().unwrap());
    assert_eq!(kids.len(), 3);

    assert!(
        approx(rect_of(&h, kids[0]).size.width, 60.0),
        "{:?}",
        rect_of(&h, kids[0])
    );
    assert!(
        approx(rect_of(&h, kids[1]).size.width, 100.0),
        "{:?}",
        rect_of(&h, kids[1])
    );
    assert!(
        approx(rect_of(&h, kids[2]).size.width, 80.0),
        "{:?}",
        rect_of(&h, kids[2])
    );

    let area = DockArea::new()
        .last_child_fill(false)
        .left(Panel::new().width(10.0))
        .dock_at(0, Panel::new().width(20.0), sequoia_view::Dock::Top)
        .set_dock(1, sequoia_view::Dock::Bottom);
    assert_eq!(area.len(), 2);
    assert!(!area.is_empty());
    let area = area.remove(0);
    assert_eq!(area.len(), 1);
    let _ = area.remove(99);
}

#[test]
fn window_host_calls_back_instead_of_winit() {
    let model = Signal::new(DockModel::single(p(1)));
    model.update(|m| {
        m.apply(DockOp::Float {
            panel: p(1),
            x: 0.0,
            y: 0.0,
            w: 400.0,
            h: 300.0,
        });
        m.attach_window(p(1), Some(9));
    });
    let opened: Rc<Cell<u64>> = Rc::new(Cell::new(0));
    let closed: Rc<Cell<u64>> = Rc::new(Cell::new(0));
    let opened2 = opened.clone();
    let closed2 = closed.clone();
    let host = DockWindowHost::new(model.clone(), render)
        .on_open(move |d: DetachedWindow| {
            opened2.set(d.panel.get());
        })
        .on_close(move |_win: u64| {
            closed2.set(1);
        });
    let h = Harness::new(host);
    h.layout(800.0, 600.0);
    h.paint();
    assert_eq!(opened.get(), 1, "host reported window without winit");
    assert!(h.root().is_some());

    model.update(|m| {
        m.dock_first_tab(p(1));
    });
    h.paint();
    assert_eq!(closed.get(), 1, "return closes the window");
}

#[test]
fn persist_roundtrip_through_view_model() {
    let model = Signal::new(DockModel::single(p(1)));
    model.update(|m| {
        m.apply(DockOp::Insert {
            panel: p(2),
            target: p(1),
            drop: DockDrop::Right,
            frac: 0.3,
            moving: false,
        });
    });
    let text = sequoia_view::dock_encode(&model.get_untracked());
    let back = sequoia_view::dock_decode(&text).expect("roundtrip");
    assert_eq!(back, model.get_untracked());
}

fn float_shell() -> (Signal<DockModel>, Harness) {
    let model = Signal::new(DockModel::single(p(1)));
    model.update(|m| {
        m.apply(DockOp::Insert {
            panel: p(2),
            target: p(1),
            drop: DockDrop::Right,
            frac: 0.5,
            moving: false,
        });
        m.apply(DockOp::Float {
            panel: p(2),
            x: 50.0,
            y: 60.0,
            w: 200.0,
            h: 150.0,
        });
    });
    let h = Harness::new(
        Stack::new()
            .child(DockView::new(model.clone(), render).fill_size())
            .child(DockFloatLayer::new(model.clone(), render).fill_size()),
    );
    h.layout(800.0, 600.0);
    h.tick();
    h.layout(800.0, 600.0);
    (model, h)
}

fn float_card(h: &Harness) -> ElementId {
    let layers = h.children(h.root().unwrap());
    assert_eq!(layers.len(), 2, "dock + float");
    let cards = h.children(layers[1]);
    assert_eq!(cards.len(), 1, "jedna karta");
    cards[0]
}

#[test]
fn float_card_drags_by_title() {
    let (model, h) = float_shell();

    h.pointer_down(60.0, 70.0);
    h.pointer_move(80.0, 90.0);
    h.pointer_move(100.0, 100.0);
    h.pointer_up(100.0, 100.0);
    let f = model
        .get_untracked()
        .floating_of(p(2))
        .expect("still floating");
    assert!(approx(f.x, 90.0), "x follows cursor: {f:?}");
    assert!(approx(f.y, 90.0), "y follows cursor: {f:?}");
    h.tick();
    h.layout(800.0, 600.0);
    let card = rect_of(&h, float_card(&h));
    assert!(approx(card.left(), 90.0), "{card:?}");
    assert!(approx(card.top(), 90.0), "{card:?}");
}

#[test]
fn float_card_survives_rebuild_mid_drag() {
    let (model, h) = float_shell();
    h.pointer_down(60.0, 70.0);

    h.tick();
    h.layout(800.0, 600.0);
    h.pointer_move(80.0, 90.0);
    h.tick();
    h.layout(800.0, 600.0);
    h.pointer_move(110.0, 120.0);
    h.tick();
    h.layout(800.0, 600.0);
    h.pointer_up(110.0, 120.0);
    let f = model
        .get_untracked()
        .floating_of(p(2))
        .expect("still floating");
    assert!(approx(f.x, 100.0), "full delta after rebuilds: {f:?}");
    assert!(approx(f.y, 110.0), "full delta after rebuilds: {f:?}");
}

#[test]
fn split_divider_survives_rebuild_mid_drag() {
    let model = Signal::new(DockModel::single(p(1)));
    model.update(|m| {
        m.apply(DockOp::Insert {
            panel: p(2),
            target: p(1),
            drop: DockDrop::Right,
            frac: 0.5,
            moving: false,
        });
    });
    let h = Harness::new(DockView::new(model.clone(), render).fill_size());
    h.layout(400.0, 300.0);
    h.tick();
    h.layout(400.0, 300.0);

    h.pointer_down(200.0, 150.0);
    h.tick();
    h.layout(400.0, 300.0);
    h.pointer_move(240.0, 150.0);
    h.tick();
    h.layout(400.0, 300.0);
    h.pointer_move(280.0, 150.0);
    h.pointer_up(280.0, 150.0);
    let frac = model.get_untracked().frac_at(&[]).expect("split is alive");

    assert!((frac - 0.70).abs() < 0.05, "frac follows: {frac}");
}

#[test]
fn float_card_ignores_content_drag() {
    let (model, h) = float_shell();

    h.pointer_down(100.0, 120.0);
    h.pointer_move(300.0, 300.0);
    h.pointer_up(300.0, 300.0);
    let f = model
        .get_untracked()
        .floating_of(p(2))
        .expect("still floating");
    assert!(approx(f.x, 50.0), "karta stoi: {f:?}");
    assert!(approx(f.y, 60.0), "karta stoi: {f:?}");
}

#[test]
fn float_buttons_close_and_dock_back() {
    let (model, h) = float_shell();

    h.click(237.0, 73.0);
    assert!(
        !model.get_untracked().contains(p(2)),
        "× usuwa panel z modelu"
    );

    model.update(|m| {
        m.apply(DockOp::Float {
            panel: p(1),
            x: 50.0,
            y: 60.0,
            w: 200.0,
            h: 150.0,
        });
    });
    h.tick();
    h.layout(800.0, 600.0);
    h.click(211.0, 73.0);
    let m = model.get_untracked();
    assert!(!m.is_floating(p(1)), "⧉ dokleja z powrotem");
    assert!(m.root.as_ref().is_some_and(|r| r.contains(p(1))));
}

#[test]
fn tab_down_starts_drag_and_click_activates() {
    let model = Signal::new(DockModel::single(p(1)));
    model.update(|m| {
        m.apply(DockOp::Insert {
            panel: p(2),
            target: p(1),
            drop: DockDrop::Tab,
            frac: 0.5,
            moving: false,
        });
    });
    let h = Harness::new(DockView::new(model.clone(), render).fill_size());
    h.layout(400.0, 300.0);
    h.tick();
    h.layout(400.0, 300.0);

    let tabs = h.children(h.children(h.root().unwrap())[0]);
    let strip = tabs[0];
    let buttons = h.children(strip);
    assert_eq!(buttons.len(), 2, "dwa taby");
    let second = rect_of(&h, buttons[1]);

    h.pointer_down(
        second.left() + second.size.width * 0.5,
        second.top() + second.size.height * 0.5,
    );
    assert!(h.has_pending_drag(), "tab is the source of drugs");

    h.pointer_up(
        second.left() + second.size.width * 0.5,
        second.top() + second.size.height * 0.5,
    );
    assert_eq!(model.get_untracked().active_tab_of(p(2)), Some(1));
}
