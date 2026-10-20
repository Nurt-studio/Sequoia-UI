#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_core::Color;
use sequoia_elements::{Container, ContextMenu};
use sequoia_input::PointerButton;
use sequoia_test::Harness;
use sequoia_view::prelude::*;

#[test]
fn context_menu_stays_at_open_point() {
    let h = Harness::new(
        Column::new().fill_size().padding(40.0).child(
            ContextMenu::new(
                Container::new()
                    .width(120.0)
                    .height(40.0)
                    .background(Color::hex(0x202124))
                    .child(Text::new("cel").size(14.0)),
            )
            .item("Fazka", || {}),
        ),
    );
    h.layout(400.0, 300.0);
    h.paint();
    h.tick();
    h.pointer_down_at(100.0, 60.0, PointerButton::Right);
    h.pointer_up_at(100.0, 60.0, PointerButton::Right);
    h.layout(400.0, 300.0);
    h.paint();
    h.tick();
    assert_eq!(h.overlay_ids().len(), 1, "PPM otwiera menu");
    let before = h.overlay_rect(h.overlay_ids()[0]).unwrap();
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
