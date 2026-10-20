#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::cell::Cell;
use std::rc::Rc;

use sequoia_elements::DropdownButton;
use sequoia_input::Key;
use sequoia_reactive::Signal;
use sequoia_test::Harness;
use sequoia_view::prelude::*;

fn options() -> Vec<String> {
    vec!["Small".into(), "Medium".into(), "Large".into()]
}

fn harness(sel: &Signal<usize>) -> Harness {
    let h = Harness::new(DropdownButton::new(sel.clone(), options()));
    h.layout(400.0, 400.0);
    h.paint();
    h.tick();
    h
}

fn button_center(h: &Harness) -> (f32, f32) {
    let r = h.rect(h.root().unwrap()).unwrap();
    (r.left() + r.size.width * 0.5, r.top() + r.size.height * 0.5)
}

fn item_center(h: &Harness, i: usize) -> (f32, f32) {
    let id = h.overlay_ids()[0];
    let r = h.overlay_rect(id).unwrap();

    (r.left() + 10.0, r.top() + i as f32 * 28.0 + 14.0)
}

#[test]
fn closed_paints_without_overlay() {
    let sel = Signal::new(0);
    let h = harness(&sel);
    assert!(h.overlay_ids().is_empty(), "closed: no overlay");

    assert!(!h.paint().is_empty());
}

#[test]
fn click_toggles_menu() {
    let sel = Signal::new(0);
    let h = harness(&sel);
    let (x, y) = button_center(&h);

    h.click(x, y);
    h.layout(400.0, 400.0);
    h.paint();
    h.tick();
    assert_eq!(h.overlay_ids().len(), 1, "click opens the list");

    h.click(x, y);
    h.layout(400.0, 400.0);
    h.paint();
    h.tick();
    assert!(h.overlay_ids().is_empty(), "click closes the list");
}

#[test]
fn menu_hugs_content_and_stacks_vertically() {
    let sel = Signal::new(0);
    let h = harness(&sel);
    let (x, y) = button_center(&h);
    h.click(x, y);
    h.layout(400.0, 400.0);
    h.paint();
    h.tick();

    let id = h.overlay_ids()[0];
    let r = h.overlay_rect(id).unwrap();
    assert!(r.size.height >= 3.0 * 28.0 - 1.0, "3 pozycje: {r:?}");
    assert!(r.size.width > 40.0, "width fits content: {r:?}");

    let b = h.rect(h.root().unwrap()).unwrap();
    assert!(
        r.top() >= b.bottom() - 0.01,
        "menu under the button: {r:?} vs {b:?}"
    );
}

#[test]
fn menu_matches_button_width() {
    let sel = Signal::new(0);
    let h = harness(&sel);
    let (x, y) = button_center(&h);
    h.click(x, y);
    h.layout(400.0, 400.0);
    h.paint();
    h.tick();

    let b = h.rect(h.root().unwrap()).unwrap();
    let r = h.overlay_rect(h.overlay_ids()[0]).unwrap();
    assert!(
        (r.size.width - b.size.width).abs() < 0.01,
        "menu jak przycisk: {r:?} vs {b:?}"
    );
    assert!(
        (r.left() - b.left()).abs() < 0.01,
        "left aligned: {r:?} vs {b:?}"
    );
}

#[test]
fn click_item_selects_and_closes() {
    let sel = Signal::new(0);
    let picked = Rc::new(Cell::new(None::<usize>));
    let p = picked.clone();
    let h = Harness::new(
        DropdownButton::new(sel.clone(), options()).on_select(move |i| p.set(Some(i))),
    );
    h.layout(400.0, 400.0);
    h.paint();
    h.tick();

    let (x, y) = button_center(&h);
    h.click(x, y);
    h.layout(400.0, 400.0);
    h.paint();
    h.tick();
    assert_eq!(h.overlay_ids().len(), 1);

    let (ix, iy) = item_center(&h, 2);
    h.pointer_move(ix, iy);
    h.click(ix, iy);
    h.layout(400.0, 400.0);
    h.paint();
    h.tick();

    assert_eq!(sel.get_untracked(), 2, "signal set");
    assert_eq!(picked.get(), Some(2), "callback with index");
    assert!(h.overlay_ids().is_empty(), "selection closes the list");
}

#[test]
fn outside_click_closes_without_changing_selection() {
    let sel = Signal::new(1);
    let h = harness(&sel);
    let (x, y) = button_center(&h);
    h.click(x, y);
    h.layout(400.0, 400.0);
    h.paint();
    h.tick();
    assert_eq!(h.overlay_ids().len(), 1);

    h.pointer_move(390.0, 390.0);
    h.click(390.0, 390.0);
    h.layout(400.0, 400.0);
    h.paint();
    h.tick();

    assert!(h.overlay_ids().is_empty(), "outside click closes");
    assert_eq!(sel.get_untracked(), 1, "unchanged selection");
}

#[test]
fn escape_closes_without_changing_selection() {
    let sel = Signal::new(1);
    let h = harness(&sel);
    let (x, y) = button_center(&h);
    h.click(x, y);
    h.layout(400.0, 400.0);
    h.paint();
    h.tick();
    assert_eq!(h.overlay_ids().len(), 1);

    h.key(Key::ESCAPE);
    h.layout(400.0, 400.0);
    h.paint();
    h.tick();

    assert!(h.overlay_ids().is_empty(), "Escape closes");
    assert_eq!(sel.get_untracked(), 1, "unchanged selection");
}

#[test]
fn keyboard_opens_navigates_and_selects() {
    let sel = Signal::new(0);
    let h = harness(&sel);
    let (x, y) = button_center(&h);

    h.click(x, y);
    h.layout(400.0, 400.0);
    h.paint();
    h.tick();
    h.key(Key::ESCAPE);
    h.layout(400.0, 400.0);
    h.paint();
    h.tick();
    assert!(h.overlay_ids().is_empty());

    h.key(Key::ARROW_DOWN);
    h.layout(400.0, 400.0);
    h.paint();
    h.tick();
    assert_eq!(h.overlay_ids().len(), 1, "arrow opens");

    h.key(Key::ARROW_DOWN);
    h.key(Key::ARROW_DOWN);
    h.key(Key::ENTER);
    h.layout(400.0, 400.0);
    h.paint();
    h.tick();
    assert_eq!(sel.get_untracked(), 2, "two down from 0 -> 2");
    assert!(h.overlay_ids().is_empty(), "Enter closes");
}

#[test]
fn keyboard_up_clamps_at_first() {
    let sel = Signal::new(0);
    let h = harness(&sel);
    let (x, y) = button_center(&h);
    h.click(x, y);
    h.layout(400.0, 400.0);
    h.paint();
    h.tick();

    h.key(Key::ARROW_UP);
    h.key(Key::ENTER);
    h.layout(400.0, 400.0);
    h.paint();
    h.tick();
    assert_eq!(sel.get_untracked(), 0, "stays on the first");
}

#[test]
fn disabled_ignores_click_and_keyboard() {
    let sel = Signal::new(0);
    let h = Harness::new(DropdownButton::new(sel.clone(), options()).disabled(true));
    h.layout(400.0, 400.0);
    h.paint();
    h.tick();

    let (x, y) = button_center(&h);
    h.click(x, y);
    h.key(Key::ARROW_DOWN);
    h.key(Key::ENTER);
    h.layout(400.0, 400.0);
    h.paint();
    h.tick();

    assert!(h.overlay_ids().is_empty(), "disabled does not open");
    assert_eq!(sel.get_untracked(), 0);
}

#[test]
fn empty_options_never_opens() {
    let sel = Signal::new(0);
    let h = Harness::new(DropdownButton::new(sel.clone(), Vec::<String>::new()));
    h.layout(400.0, 400.0);
    h.paint();
    h.tick();

    let (x, y) = button_center(&h);
    h.click(x, y);
    h.key(Key::ARROW_DOWN);
    h.layout(400.0, 400.0);
    h.paint();
    h.tick();
    assert!(h.overlay_ids().is_empty(), "empty list does not open");
}

#[test]
fn placeholder_when_nothing_selected() {
    use sequoia_core::Color;
    let sel = Signal::new(99);
    let h = Harness::new(
        DropdownButton::new(sel.clone(), options())
            .placeholder("Wybierz…")
            .text_color(Color::WHITE),
    );
    h.layout(400.0, 400.0);

    let list = h.paint();
    assert!(!list.is_empty());
    assert!(h.overlay_ids().is_empty());

    let (x, y) = button_center(&h);
    h.click(x, y);
    h.layout(400.0, 400.0);
    h.paint();
    h.tick();
    assert_eq!(h.overlay_ids().len(), 1);
}

#[test]
fn unmount_while_open_hides_overlay() {
    struct Root {
        show: Signal<bool>,
        sel: Signal<usize>,
    }
    impl View for Root {
        fn body(&self) -> impl View + 'static {
            if self.show.get() {
                AnyView::new(DropdownButton::new(self.sel.clone(), options()))
            } else {
                AnyView::new(Unit)
            }
        }
    }
    let show = Signal::new(true);
    let sel = Signal::new(0);
    let h = Harness::new(Root {
        show: show.clone(),
        sel: sel.clone(),
    });
    h.layout(400.0, 400.0);
    h.paint();
    h.tick();

    fn find_button(h: &Harness) -> sequoia_view::ElementId {
        fn rec(h: &Harness, id: sequoia_view::ElementId) -> Option<sequoia_view::ElementId> {
            if h.type_name_of(id)
                .is_some_and(|n| n.contains("DropdownButton"))
            {
                return Some(id);
            }
            for c in h.children(id) {
                if let Some(f) = rec(h, c) {
                    return Some(f);
                }
            }
            None
        }
        rec(h, h.root().unwrap()).expect("button in the tree")
    }
    let b = h.rect(find_button(&h)).unwrap();
    let (x, y) = (b.left() + b.size.width * 0.5, b.top() + b.size.height * 0.5);
    h.click(x, y);
    h.layout(400.0, 400.0);
    h.paint();
    h.tick();
    assert_eq!(h.overlay_ids().len(), 1, "otwarty");

    h.set(&show, false);
    h.layout(400.0, 400.0);
    h.paint();
    h.tick();
    assert!(
        h.overlay_ids().is_empty(),
        "unmounting the anchor hides overlay"
    );
}
