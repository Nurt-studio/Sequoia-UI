use sequoia_input::Key;
use sequoia_test::Harness;
use sequoia_view::prelude::*;

fn menu(dismiss: Dismiss) -> OverlayEntry {
    OverlayEntry::new(Text::new("menu").size(14.0)).dismiss(dismiss)
}

fn show_menu(h: &Harness, dismiss: Dismiss) -> OverlayId {
    let id = h.overlay_show(menu(dismiss));
    h.layout(400.0, 400.0);
    h.paint();
    id
}

#[test]
fn escape_closes_topmost_escape_overlay() {
    let h = Harness::new(Unit);
    h.layout(400.0, 400.0);
    let manual = show_menu(&h, Dismiss::Manual);
    let esc = show_menu(&h, Dismiss::Escape);
    assert_eq!(h.overlay_ids().len(), 2);

    h.key(Key::ESCAPE);
    let ids = h.overlay_ids();
    assert_eq!(ids.len(), 1);
    assert_eq!(ids[0], manual, "manual stays");
    assert!(!ids.contains(&esc));

    h.key(Key::ESCAPE);
    assert_eq!(h.overlay_ids().len(), 1);
}

#[test]
fn outside_click_closes_outside_overlay() {
    let h = Harness::new(Unit);
    h.layout(400.0, 400.0);

    let id = h.overlay_show(
        OverlayEntry::new(Text::new("menu").size(14.0))
            .anchor(Anchor::Rect(sequoia_core::Rect::from_xywh(
                50.0, 50.0, 100.0, 40.0,
            )))
            .placement(Placement::Cursor)
            .dismiss(Dismiss::OutsideClick),
    );
    h.layout(400.0, 400.0);
    h.paint();
    let rect = h.overlay_rect(id).expect("rect after layout");

    h.pointer_down(rect.left() + 5.0, rect.top() + 5.0);
    assert_eq!(h.overlay_ids().len(), 1);

    h.pointer_down(0.0, 0.0);
    assert!(h.overlay_ids().is_empty());
}

#[test]
fn focus_lost_closes_when_clicking_elsewhere() {
    use sequoia_elements::TextField;
    use sequoia_reactive::Signal;

    let h = Harness::new(Column::new().child(TextField::new(Signal::new(String::new()))));
    h.layout(400.0, 400.0);
    let id = show_menu(&h, Dismiss::FocusLost);
    assert_eq!(h.overlay_ids(), vec![id]);

    h.pointer_down(10.0, 10.0);
    assert!(
        h.overlay_ids().is_empty(),
        "focus outside the overlay closes"
    );
}

#[test]
fn manual_overlays_ignore_dismiss() {
    let h = Harness::new(Unit);
    h.layout(400.0, 400.0);
    let id = show_menu(&h, Dismiss::Manual);
    h.key(Key::ESCAPE);
    h.pointer_down(0.0, 0.0);
    assert_eq!(h.overlay_ids(), vec![id], "manual only via overlay_hide");
    assert!(h.overlay_hide(id));
    assert!(h.overlay_ids().is_empty());
}
