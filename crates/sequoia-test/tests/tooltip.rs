use sequoia_elements::{Button, Tooltip};
use sequoia_test::Harness;

fn hovered_harness() -> Harness {
    let h = Harness::new(Tooltip::new(
        Button::labeled("Save").on_click(|| {}),
        "Saves the project",
    ));
    h.layout(300.0, 100.0);

    h.pointer_move(10.0, 10.0);
    h.tick();
    h.paint();
    h
}

#[test]
fn hover_shows_tooltip_overlay() {
    let h = hovered_harness();
    assert_eq!(h.overlay_ids().len(), 1, "bubble visible on hover");
}

#[test]
fn leave_hides_tooltip_without_leak() {
    let h = hovered_harness();
    assert_eq!(h.overlay_ids().len(), 1);

    h.pointer_move(290.0, 90.0);
    h.tick();
    h.paint();
    assert!(h.overlay_ids().is_empty(), "no overlay leak");

    h.paint();
    assert!(h.overlay_ids().is_empty());
}

#[test]
fn rebuild_does_not_leak_tooltip() {
    use sequoia_reactive::Signal;
    let n = Signal::new(0);
    let h = Harness::new(Tooltip::new(
        Button::labeled("Save").on_click({
            let n = n.clone();
            move || n.set(n.get_untracked() + 1)
        }),
        "tip",
    ));
    h.layout(300.0, 100.0);
    h.pointer_move(10.0, 10.0);
    h.tick();
    h.paint();
    assert_eq!(h.overlay_ids().len(), 1);

    h.set(&n, 1);
    h.paint();
    assert_eq!(h.overlay_ids().len(), 1, "no duplicates after rebuild");
}
