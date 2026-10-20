#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_core::Rect;
use sequoia_elements::{Button, FILL_COLOR, Slider, THUMB_CENTER, THUMB_OUTLINE, TRACK_COLOR};
use sequoia_reactive::Signal;
use sequoia_test::Harness;
use sequoia_view::prelude::*;

const TRACK_X0: f32 = 7.0;
const TRACK_W: f32 = 386.0;

fn approx(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}

#[test]
fn paints_track_fill_and_thumb() {
    let value = Signal::new(0.5);
    let h = Harness::new(Slider::new(value));
    h.layout(400.0, 400.0);
    let list = h.paint();

    let quads: Vec<_> = list.quads().collect();
    assert_eq!(quads.len(), 4, "track + fill + outline + center");

    assert_eq!(quads[0].color, TRACK_COLOR);
    assert_eq!(quads[0].rect, Rect::from_xywh(0.0, 8.0, 400.0, 4.0));

    assert_eq!(quads[1].color, FILL_COLOR);
    assert_eq!(quads[1].rect, Rect::from_xywh(0.0, 8.0, 200.0, 4.0));

    assert_eq!(quads[2].color, THUMB_OUTLINE);
    assert_eq!(quads[2].rect, Rect::from_xywh(193.0, 3.0, 14.0, 14.0));
    assert_eq!(quads[2].radius, 7.0);
    assert_eq!(quads[3].color, THUMB_CENTER);
    assert_eq!(quads[3].rect, Rect::from_xywh(195.0, 5.0, 10.0, 10.0));
    assert_eq!(quads[3].radius, 5.0);
}

#[test]
fn drag_sets_value_and_clamps() {
    let value = Signal::new(0.0);
    let h = Harness::new(Slider::new(value.clone()));
    h.layout(400.0, 400.0);

    h.pointer_move(300.0, 10.0);
    assert!(approx(value.get_untracked(), 0.0));

    h.pointer_down(300.0, 10.0);
    assert!(approx(value.get_untracked(), (300.0 - TRACK_X0) / TRACK_W));

    h.pointer_move(100.0, 10.0);
    assert!(approx(value.get_untracked(), (100.0 - TRACK_X0) / TRACK_W));
    h.pointer_up(100.0, 10.0);

    h.pointer_down(399.0, 10.0);
    assert!(approx(value.get_untracked(), 1.0));
    h.pointer_up(399.0, 10.0);
    h.pointer_down(0.0, 10.0);
    assert!(approx(value.get_untracked(), 0.0));
    h.pointer_up(0.0, 10.0);
}

#[test]
fn arrows_move_by_step_when_focused() {
    let value = Signal::new(0.5);
    let h = Harness::new(Slider::new(value.clone()));
    h.layout(400.0, 400.0);

    h.key(Key::ARROW_RIGHT);
    assert!(approx(value.get_untracked(), 0.5));

    h.runtime().set_focus(h.root());
    h.key(Key::ARROW_RIGHT);
    assert!(approx(value.get_untracked(), 0.55));
    h.key(Key::ARROW_LEFT);
    h.key(Key::ARROW_LEFT);
    assert!(approx(value.get_untracked(), 0.45));
}

#[test]
fn cursor_grab_grabbing_and_default() {
    let value = Signal::new(0.5);
    let h = Harness::new(Slider::new(value));
    h.layout(400.0, 400.0);
    let rt = h.runtime();

    assert_eq!(rt.cursor_icon(), CursorIcon::Default);
    h.pointer_move(200.0, 10.0);
    assert_eq!(rt.cursor_icon(), CursorIcon::Grab);

    h.pointer_down(200.0, 10.0);
    assert_eq!(rt.cursor_icon(), CursorIcon::Grabbing);
    h.pointer_move(350.0, 300.0);
    assert_eq!(rt.cursor_icon(), CursorIcon::Grabbing);
    h.pointer_up(350.0, 300.0);

    assert_eq!(rt.cursor_icon(), CursorIcon::Default);
}

#[test]
fn button_shows_pointer() {
    let h = Harness::new(Button::labeled("Ok").on_click(|| {}));
    h.layout(400.0, 400.0);
    let r = h.rect(h.root().unwrap()).unwrap();
    let rt = h.runtime();

    h.pointer_move(r.left() + 2.0, r.top() + 2.0);
    assert_eq!(rt.cursor_icon(), CursorIcon::Pointer);
    h.pointer_move(r.right() + 50.0, r.bottom() + 50.0);
    assert_eq!(rt.cursor_icon(), CursorIcon::Default);
}

#[test]
fn external_set_rebuilds_standalone_slider() {
    let value = Signal::new(0.5);
    let h = Harness::new(Slider::new(value.clone()));
    h.layout(400.0, 400.0);
    h.tick();
    let before = h.stats().rebuilds;
    h.set(&value, 0.9);
    assert!(
        h.stats().rebuilds > before,
        "set must schedule slider rebuild"
    );
}

#[test]
fn hover_grows_outline_with_tween() {
    use std::time::{Duration, Instant};
    let t0 = Instant::now();
    sequoia_elements::slider::__set_test_now(Some(t0));
    struct Guard;
    impl Drop for Guard {
        fn drop(&mut self) {
            sequoia_elements::slider::__set_test_now(None);
        }
    }
    let _guard = Guard;

    let value = Signal::new(0.5);
    let h = Harness::new(Slider::new(value));
    h.layout(400.0, 400.0);

    let inner0 = h.paint().quads().nth(3).unwrap().rect;
    assert_eq!(inner0, Rect::from_xywh(195.0, 5.0, 10.0, 10.0));

    h.pointer_move(200.0, 10.0);
    let inner1 = h.paint().quads().nth(3).unwrap().rect;
    assert!(
        inner1.size.width < 10.0 && inner1.size.width > 9.0,
        "outline grew a bit: {}",
        inner1.size.width
    );

    sequoia_elements::slider::__set_test_now(Some(t0 + Duration::from_millis(200)));
    let inner2 = h.paint().quads().nth(3).unwrap().rect;
    assert_eq!(inner2, Rect::from_xywh(196.5, 6.5, 7.0, 7.0));

    h.pointer_move(350.0, 300.0);
    let inner3 = h.paint().quads().nth(3).unwrap().rect;
    assert!(
        inner3.size.width > 7.0 && inner3.size.width < 8.0,
        "outline shrinks: {}",
        inner3.size.width
    );
}

#[test]
fn min_max_remap_value() {
    let value = Signal::new(15.0);
    let h = Harness::new(Slider::new(value.clone()).min(10.0).max(20.0));
    h.layout(400.0, 400.0);

    h.pointer_down(200.0, 10.0);
    assert!(approx(value.get_untracked(), 15.0));
    h.pointer_up(200.0, 10.0);

    h.pointer_down(TRACK_X0, 10.0);
    assert!(approx(value.get_untracked(), 10.0));
    h.pointer_up(TRACK_X0, 10.0);

    h.pointer_down(TRACK_X0 + TRACK_W, 10.0);
    assert!(approx(value.get_untracked(), 20.0));
    h.pointer_up(TRACK_X0 + TRACK_W, 10.0);
}

#[test]
fn nan_bounds_do_not_poison_the_signal() {
    let value = Signal::new(0.0);
    let h = Harness::new(Slider::new(value.clone()).min(f32::NAN).max(1.0));
    h.layout(400.0, 400.0);
    h.pointer_down(399.0, 10.0);
    h.pointer_up(399.0, 10.0);
    assert!(
        value.get_untracked().is_finite(),
        "min(NaN) not absorbing: {}",
        value.get_untracked()
    );

    let value = Signal::new(0.0);
    let h = Harness::new(Slider::new(value.clone()).min(0.0).max(f32::INFINITY));
    h.layout(400.0, 400.0);
    h.pointer_down(TRACK_X0, 10.0);
    h.pointer_up(TRACK_X0, 10.0);
    assert!(
        value.get_untracked().is_finite(),
        "max(∞) does not absorb: {}",
        value.get_untracked()
    );
}
