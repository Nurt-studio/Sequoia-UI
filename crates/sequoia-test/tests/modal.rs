#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_elements::Modal;
use sequoia_reactive::Signal;
use sequoia_style::Backdrop;
use sequoia_test::Harness;
use sequoia_view::prelude::*;

fn harness(open: &Signal<bool>) -> Harness {
    let h = Harness::new(
        Column::new().child(Text::new("Pod spodem")).child(
            Modal::new(open.clone())
                .width(200.0)
                .height(100.0)
                .child(Text::new("Dialog")),
        ),
    );
    h.layout(400.0, 400.0);
    h.paint();
    h.tick();
    h
}

fn open_two_passes(h: &Harness, open: &Signal<bool>) {
    h.set(open, true);
    h.layout(400.0, 400.0);
    h.paint();
    h.tick();
    h.layout(400.0, 400.0);
    h.paint();
}

#[test]
fn closed_paints_without_overlay() {
    let open = Signal::new(false);
    let h = harness(&open);
    assert!(h.overlay_ids().is_empty(), "closed: no overlay");
    assert!(!open.get_untracked());
}

#[test]
fn open_centers_panel_in_viewport() {
    let open = Signal::new(false);
    let h = harness(&open);
    open_two_passes(&h, &open);
    assert_eq!(h.overlay_ids().len(), 1, "open: one overlay");
    let r = h.overlay_rect(h.overlay_ids()[0]).unwrap();
    assert!(
        (r.size.width - 200.0).abs() < 1.0,
        "explicit width kept: {r:?}"
    );
    assert!(
        (r.size.height - 100.0).abs() < 1.0,
        "explicit height kept: {r:?}"
    );
    assert!((r.left() - 100.0).abs() < 1.0, "centered x: {r:?}");
    assert!((r.top() - 150.0).abs() < 1.0, "centered y: {r:?}");
}

#[test]
fn outside_click_dismisses_and_closes_signal() {
    let open = Signal::new(false);
    let h = harness(&open);
    open_two_passes(&h, &open);
    assert_eq!(h.overlay_ids().len(), 1);

    h.click(10.0, 10.0);
    h.layout(400.0, 400.0);
    h.paint();
    h.tick();

    assert!(h.overlay_ids().is_empty(), "outside click hides overlay");
    assert!(!open.get_untracked(), "signal follows runtime dismiss");
}

#[test]
fn escape_does_not_close_outside_click_modal() {
    use sequoia_input::Key;

    let open = Signal::new(false);
    let h = harness(&open);
    open_two_passes(&h, &open);
    assert_eq!(h.overlay_ids().len(), 1);

    h.key(Key::ESCAPE);
    h.layout(400.0, 400.0);
    h.paint();
    h.tick();

    assert_eq!(h.overlay_ids().len(), 1, "Escape keeps OutsideClick modal");
    assert!(open.get_untracked());
}

#[test]
fn escape_closes_escape_modal() {
    use sequoia_input::Key;

    let open = Signal::new(false);
    let h = Harness::new(
        Column::new().child(
            Modal::new(open.clone())
                .width(200.0)
                .height(100.0)
                .dismiss(Dismiss::Escape)
                .child(Text::new("Dialog")),
        ),
    );
    h.layout(400.0, 400.0);
    h.paint();
    h.tick();
    open_two_passes(&h, &open);
    assert_eq!(h.overlay_ids().len(), 1);

    h.key(Key::ESCAPE);
    h.layout(400.0, 400.0);
    h.paint();
    h.tick();

    assert!(h.overlay_ids().is_empty(), "Escape hides overlay");
    assert!(!open.get_untracked(), "signal follows runtime dismiss");
}

#[test]
fn scrim_paints_only_when_open() {
    let open = Signal::new(false);
    let modal = Modal::new(open.clone())
        .width(200.0)
        .height(100.0)
        .scrim_backdrop(Backdrop::blur(4.0))
        .child(Text::new("Dialog"));
    let h = Harness::new(
        Column::new()
            .child(Text::new("Pod spodem"))
            .child(modal.scrim())
            .child(modal),
    );
    h.layout(400.0, 400.0);
    let closed = h.paint();
    h.tick();
    assert!(
        !closed.has_backdrops(),
        "closed scrim: zero-size, no marker"
    );
    let closed_quads = closed.quads().count();

    open_two_passes(&h, &open);
    let list = h.paint();
    assert!(list.has_backdrops(), "open scrim: fullscreen blur marker");
    assert!(
        list.quads().count() > closed_quads,
        "open: scrim + panel paint quads"
    );
}

#[test]
fn scrim_fade_survives_rebuild_while_open() {
    use sequoia_core::RenderList;
    use sequoia_elements::ModalScrim;

    fn scrim_alpha(list: &RenderList) -> f32 {
        list.quads()
            .filter(|q| q.rect.size.width >= 399.0 && q.rect.size.height >= 399.0)
            .map(|q| q.color.a)
            .fold(0.0_f32, f32::max)
    }

    let open = Signal::new(true);
    let label = Signal::new("a".to_string());
    let probe = label.clone();
    let h = Harness::new(
        Stack::new()
            .child_at(Text::with(move || probe.get()), StackPos::fill())
            .child_at(ModalScrim::new(open.clone()), StackPos::fill()),
    );
    h.layout(400.0, 400.0);
    let first = h.paint();
    h.tick();
    assert!(
        scrim_alpha(&first) < 0.05,
        "fade startuje od zera przy otwarciu"
    );

    std::thread::sleep(std::time::Duration::from_millis(350));
    h.set(&label, "b".to_string());
    h.layout(400.0, 400.0);
    let second = h.paint();
    h.tick();
    let alpha = scrim_alpha(&second);
    assert!(
        alpha > 0.4,
        "fade nie restartuje po przebudowie, alpha={alpha}"
    );
}
