#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::cell::Cell;
use std::rc::Rc;

use sequoia_core::Rect;
use sequoia_elements::{Container, TabStrip};
use sequoia_input::Key;
use sequoia_reactive::Signal;
use sequoia_test::Harness;
use sequoia_view::ElementId;

fn strip() -> (
    Harness,
    Signal<usize>,
    Rc<Cell<Option<usize>>>,
    Rc<Cell<Option<usize>>>,
) {
    let active = Signal::new(0usize);
    let picked = Rc::new(Cell::new(None));
    let closed = Rc::new(Cell::new(None));
    let p = picked.clone();
    let c = closed.clone();
    let s = TabStrip::new(active.clone(), ["LAYERS", "SETTINGS"])
        .closable([true, false])
        .on_select(move |i| p.set(Some(i)))
        .on_close(move |i| c.set(Some(i)));
    let h = Harness::new(Container::new().width(300.0).height(40.0).child(s));
    h.layout(300.0, 40.0);
    h.paint();
    h.tick();
    (h, active, picked, closed)
}

fn tab_rects(h: &Harness) -> Vec<Rect> {
    let root = h.root().unwrap();
    let strip = h.children(root)[0];
    let row = h.children(strip)[0];
    h.children(row).iter().filter_map(|&c| h.rect(c)).collect()
}

#[test]
fn click_selects_tab_and_updates_signal() {
    let (h, active, picked, closed) = strip();
    let tabs = tab_rects(&h);
    assert_eq!(tabs.len(), 2, "dwie karty");

    let t = tabs[1];
    h.click(t.left() + t.size.width * 0.5, t.top() + t.size.height * 0.5);
    h.tick();
    assert_eq!(picked.get(), Some(1));
    assert_eq!(active.get_untracked(), 1, "program signal shifted");
    assert_eq!(closed.get(), None, "× unpicked");
}

fn card_and_x(h: &Harness, tab: ElementId) -> (ElementId, Option<ElementId>) {
    let card = h.children(tab)[0];
    let row = h.children(card)[0];
    let kids = h.children(row);
    let x = if kids.len() > 1 {
        Some(kids[kids.len() - 1])
    } else {
        None
    };
    (card, x)
}

#[test]
fn close_x_fires_close_not_select() {
    let (h, active, picked, closed) = strip();
    let root = h.root().unwrap();
    let strip = h.children(root)[0];
    let row = h.children(strip)[0];
    let tab0 = h.children(row)[0];
    let (_card, x) = card_and_x(&h, tab0);
    let x = x.expect("karta 0 ma ×");
    let x = h.rect(x).unwrap();
    h.click(x.left() + x.size.width * 0.5, x.top() + x.size.height * 0.5);
    h.tick();
    assert_eq!(closed.get(), Some(0), "× closes tab 0");
    assert_eq!(picked.get(), None, "× does not switch tab");
    assert_eq!(active.get_untracked(), 0);
}

#[test]
fn enter_on_focused_tab_selects() {
    let (h, _active, picked, _closed) = strip();
    let root = h.root().unwrap();
    let strip_el = h.children(root)[0];
    let row = h.children(strip_el)[0];
    let tab0 = h.children(row)[0];

    let (tab, _) = card_and_x(&h, tab0);
    h.runtime().set_focus(Some(tab));
    h.key(Key::ENTER);
    h.tick();
    assert_eq!(picked.get(), Some(0));
}
