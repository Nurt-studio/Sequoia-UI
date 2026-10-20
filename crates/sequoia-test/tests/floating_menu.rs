use std::cell::Cell;
use std::rc::Rc;

use sequoia_core::Color;
use sequoia_elements::{Container, FloatingMenu};
use sequoia_input::{Key, PointerButton};
use sequoia_reactive::Signal;
use sequoia_test::Harness;
use sequoia_view::prelude::*;

fn harness() -> (Harness, Rc<Cell<bool>>, FloatingMenuHandle) {
    let fired = Rc::new(Cell::new(false));
    let menu = FloatingMenu::wrap(
        Container::new()
            .width(120.0)
            .height(40.0)
            .background(Color::hex(0x202124))
            .child(Text::new("cel").size(14.0)),
    )
    .row("copy", "Duplikuj", {
        let fired = fired.clone();
        move || fired.set(true)
    })
    .separator()
    .danger_row("trash", "Delete", {
        let fired = fired.clone();
        move || fired.set(true)
    });
    let open = menu.open_signal();
    let h = Harness::new(Column::new().fill_size().padding(40.0).child(menu));
    h.layout(400.0, 300.0);
    h.paint();
    h.tick();
    (h, fired, FloatingMenuHandle { open })
}

struct FloatingMenuHandle {
    open: Signal<bool>,
}

fn right_click(h: &Harness, x: f32, y: f32) {
    h.pointer_down_at(x, y, PointerButton::Right);
    h.pointer_up_at(x, y, PointerButton::Right);
    h.layout(400.0, 300.0);
    h.paint();
    h.tick();
}

#[test]
fn right_click_opens_menu_at_cursor() {
    let (h, _fired, _handle) = harness();
    assert!(h.overlay_ids().is_empty());

    right_click(&h, 100.0, 60.0);
    assert_eq!(h.overlay_ids().len(), 1, "PPM otwiera menu");
}

#[test]
fn empty_menu_never_opens() {
    let h = Harness::new(
        Column::new()
            .fill_size()
            .padding(40.0)
            .child(FloatingMenu::wrap(Text::new("cel"))),
    );
    h.layout(400.0, 300.0);
    h.paint();
    h.tick();
    right_click(&h, 100.0, 60.0);
    assert!(h.overlay_ids().is_empty(), "puste menu bez overlaya");
}

#[test]
fn row_click_fires_action_and_closes() {
    let (h, fired, _handle) = harness();
    right_click(&h, 100.0, 60.0);
    assert_eq!(h.overlay_ids().len(), 1);
    let r = h.overlay_rect(h.overlay_ids()[0]).unwrap();

    h.click(r.left() + 60.0, r.top() + 22.0);
    h.layout(400.0, 300.0);
    h.paint();
    h.tick();
    assert!(fired.get(), "akcja wiersza odpalona");
    assert!(h.overlay_ids().is_empty(), "selection closes menu");
}

#[test]
fn programmatic_open_and_escape_close() {
    let (h, _fired, handle) = harness();

    right_click(&h, 100.0, 60.0);
    assert_eq!(h.overlay_ids().len(), 1, "PPM otwiera menu");
    let _ = handle;

    h.key(Key::ESCAPE);
    h.layout(400.0, 300.0);
    h.paint();
    h.tick();
    assert!(h.overlay_ids().is_empty(), "Escape zamyka menu");
}

#[test]
fn outside_click_dismisses() {
    let (h, _fired, handle) = harness();
    handle.open.set(true);
    h.layout(400.0, 300.0);
    h.paint();
    h.tick();
    h.layout(400.0, 300.0);
    h.paint();
    assert_eq!(h.overlay_ids().len(), 1);

    h.click(390.0, 290.0);
    h.layout(400.0, 300.0);
    h.paint();
    h.tick();
    assert!(h.overlay_ids().is_empty(), "klik z boku zamyka menu");
    assert!(!handle.open.get_untracked());
}

fn click_menu_harness(click_opens: bool, below: bool) -> Harness {
    let mut menu = FloatingMenu::wrap(
        Container::new()
            .width(260.0)
            .height(40.0)
            .background(Color::hex(0x202124))
            .child(Text::new("cel").size(14.0)),
    )
    .text_row("Akcja", || {});
    if click_opens {
        menu = menu.open_on_click();
    }
    if below {
        menu = menu.below_invoker();
    }
    let h = Harness::new(Column::new().fill_size().padding(40.0).child(menu));
    h.layout(400.0, 300.0);
    h.paint();
    h.tick();
    h
}

#[test]
fn left_click_ignored_without_open_on_click() {
    let h = click_menu_harness(false, false);
    h.click(100.0, 60.0);
    h.layout(400.0, 300.0);
    h.paint();
    h.tick();
    assert!(
        h.overlay_ids().is_empty(),
        "bez open_on_click LPM nie otwiera menu"
    );
}

#[test]
fn left_click_opens_with_open_on_click() {
    let h = click_menu_harness(true, false);

    h.click(100.0, 60.0);
    h.layout(400.0, 300.0);
    h.paint();
    h.tick();
    assert_eq!(h.overlay_ids().len(), 1, "LPM otwiera menu");
}

#[test]
fn below_invoker_anchors_under_child() {
    let h = click_menu_harness(true, true);
    h.click(100.0, 60.0);
    h.layout(400.0, 300.0);
    h.paint();
    h.tick();
    assert_eq!(h.overlay_ids().len(), 1);
    let r = h.overlay_rect(h.overlay_ids()[0]).unwrap();

    assert!(
        (r.top() - 84.0).abs() < 1.0,
        "menu pod dzieckiem, jest top={}",
        r.top()
    );
    assert!(
        (r.left() - 40.0).abs() < 1.0,
        "left aligned menu under child, is left={}",
        r.left()
    );
}

#[test]
fn menu_stays_at_open_point_when_cursor_moves() {
    let (h, _fired, _handle) = harness();
    right_click(&h, 100.0, 60.0);
    assert_eq!(h.overlay_ids().len(), 1, "PPM otwiera menu");
    let before = h.overlay_rect(h.overlay_ids()[0]).unwrap();
    assert!(
        (before.left() - 100.0).abs() < 8.0 && (before.top() - 60.0).abs() < 8.0,
        "menu startuje przy kursorze: {before:?}"
    );

    h.pointer_move(320.0, 250.0);
    h.layout(400.0, 300.0);
    h.paint();
    h.tick();
    assert_eq!(h.overlay_ids().len(), 1, "ruch nie zamyka menu");
    let after = h.overlay_rect(h.overlay_ids()[0]).unwrap();
    assert!(
        (after.left() - before.left()).abs() < 1.0 && (after.top() - before.top()).abs() < 1.0,
        "menu nie goni kursora: {before:?} -> {after:?}"
    );
}
