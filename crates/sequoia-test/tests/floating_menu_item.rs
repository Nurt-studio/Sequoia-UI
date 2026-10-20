use std::cell::Cell;
use std::rc::Rc;

use sequoia_core::Color;
use sequoia_elements::{Container, FloatingMenu};
use sequoia_input::PointerButton;
use sequoia_test::Harness;
use sequoia_view::prelude::*;

fn built(fired: &Rc<Cell<usize>>) -> FloatingMenu {
    let f = fired.clone();
    let menu = FloatingMenu::wrap(
        Container::new()
            .width(120.0)
            .height(40.0)
            .background(Color::hex(0x202124))
            .child(Text::new("cel").size(14.0)),
    )
    .item("Duplikuj")
    .icon("copy")
    .shortcut("Ctrl+D")
    .on_click(move || f.set(f.get() + 1));
    let f = fired.clone();

    let menu = menu
        .item("Add mask")
        .disabled(true)
        .icon("mask")
        .on_click(move || f.set(f.get() + 100));
    let f = fired.clone();

    menu.item("Delete").danger(true).on_click(move || {
        f.set(f.get() + 1000);
    })
}

#[test]
fn item_builder_fires_only_enabled_row() {
    let fired = Rc::new(Cell::new(0usize));
    let menu = built(&fired);
    let h = Harness::new(Column::new().fill_size().padding(40.0).child(menu));
    h.layout(400.0, 300.0);
    h.paint();
    h.tick();

    h.pointer_down_at(100.0, 60.0, PointerButton::Right);
    h.pointer_up_at(100.0, 60.0, PointerButton::Right);
    h.layout(400.0, 300.0);
    h.paint();
    h.tick();
    assert_eq!(h.overlay_ids().len(), 1, "PPM otwiera menu");
    let r = h.overlay_rect(h.overlay_ids()[0]).unwrap();

    h.click(r.left() + 60.0, r.top() + 56.0);
    h.layout(400.0, 300.0);
    h.paint();
    h.tick();
    assert_eq!(fired.get(), 0, "disabled line does not run");

    h.click(r.left() + 60.0, r.top() + 22.0);
    h.layout(400.0, 300.0);
    h.paint();
    h.tick();
    assert_eq!(fired.get(), 1, "aktywny wiersz odpalony");
}
