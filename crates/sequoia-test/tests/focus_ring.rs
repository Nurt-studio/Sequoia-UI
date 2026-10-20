#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_core::{Color, Rect};
use sequoia_elements::Button;
use sequoia_test::Harness;
use sequoia_view::prelude::*;

fn rings(reserve: bool) -> (Harness, ElementId, ElementId, ElementId) {
    let wrap = |b: Button| {
        if reserve {
            FocusRing::wrap(b).reserve_space(true)
        } else {
            FocusRing::wrap(b)
        }
    };
    let a = wrap(
        Button::icon("brush")
            .size(8.0)
            .background(Color::TRANSPARENT),
    );
    let b = wrap(
        Button::icon("brush")
            .size(8.0)
            .background(Color::TRANSPARENT),
    );
    let h = Harness::new(Row::new().fill_width().height(40.0).children((a, b)));
    h.layout(400.0, 40.0);
    h.paint();
    h.tick();
    let row = h.root().unwrap();
    let kids = h.children(row);
    (h, row, kids[0], kids[1])
}

#[test]
fn default_rings_stay_flush() {
    let (h, row, ra, rb) = rings(false);
    let a = h.rect(ra).unwrap();
    let b = h.rect(rb).unwrap();
    assert_eq!(a.size.width, 8.0, "box == dziecko: {a:?}");
    assert!(
        (a.right() - b.left()).abs() < 1e-3,
        "sasiednie ringi przylegaja: {a:?} vs {b:?}"
    );
    let btn = h.children(ra)[0];
    assert_eq!(h.rect(btn).unwrap(), a, "dziecko wypelnia wrapper");
    let _ = row;
}

#[test]
fn optin_reserve_keeps_rings_inside_their_boxes() {
    let (h, row, ra, rb) = rings(true);
    let a = h.rect(ra).unwrap();
    let b = h.rect(rb).unwrap();
    assert!((a.size.width - 14.0).abs() < 0.01, "box ringa: {a:?}");
    assert!(
        a.right() <= b.left() + 1e-3,
        "boxy ringow nie nachodza: {a:?} vs {b:?}"
    );
    let btn = h.children(ra)[0];
    assert_eq!(
        h.rect(btn).unwrap(),
        Rect::from_xywh(a.left() + 3.0, a.top() + 3.0, 8.0, 8.0)
    );
    let _ = row;
}

#[test]
fn optin_margin_is_click_through() {
    let (h, row, ra, _rb) = rings(true);
    let a = h.rect(ra).unwrap();
    let hit = h.hit_test(a.right() - 1.0, a.top() + a.size.height * 0.5);
    assert_eq!(hit, Some(row), "margines klik-przezroczysty: {hit:?}");

    let pressable = h.children(h.children(ra)[0])[0];
    let hit_btn = h.hit_test(a.left() + 5.0, a.top() + 7.0);
    assert_eq!(hit_btn, Some(pressable), "ringa interactive: {hit_btn:?}");
}

#[test]
fn focus_paints_ring_when_focused() {
    let (h, _row, ra, _rb) = rings(false);
    let btn = h.children(ra)[0];
    let before = h.paint().quads().count();

    h.runtime().set_focus(Some(btn));
    h.paint();
    let after = h.paint().quads().count();
    assert!(
        after > before,
        "focus maluje pierscien: {before} -> {after}"
    );
}

#[test]
fn optin_ring_stays_within_reserved_box() {
    let (h, _row, ra, _rb) = rings(true);
    let btn = h.children(ra)[0];
    let a = h.rect(ra).unwrap();
    h.runtime().set_focus(Some(btn));
    h.paint();
    let path_left = a.left() + 3.0 - 2.0;
    let found = h
        .paint()
        .quads()
        .any(|q| (q.rect.left() - path_left).abs() < 0.6);
    assert!(found, "quad ringa w rezerwacji wrappera (x={path_left})");
}

#[test]
fn disabled_ring_is_pure_passthrough() {
    let inner = Button::icon("brush")
        .size(8.0)
        .background(Color::TRANSPARENT);
    let h = Harness::new(
        Row::new()
            .fill_width()
            .height(40.0)
            .child(FocusRing::wrap(inner).enabled(false)),
    );
    h.layout(400.0, 40.0);
    h.paint();
    h.tick();
    let row = h.root().unwrap();
    let r = h.rect(h.children(row)[0]).unwrap();
    assert_eq!(r.size.width, 8.0, "ring disabled = no reservations: {r:?}");
}
