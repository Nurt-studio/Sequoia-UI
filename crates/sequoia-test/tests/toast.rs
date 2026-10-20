#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::Duration;

use sequoia_elements::{ToastQueue, ToastStack};
use sequoia_test::Harness;
use sequoia_view::prelude::*;

fn harness(queue: &ToastQueue) -> Harness {
    let h = Harness::new(
        Column::new()
            .child(Text::new("Pod spodem"))
            .child(ToastStack::new(queue.clone())),
    );
    h.layout(400.0, 400.0);
    h.paint();
    h.tick();
    h
}

fn settle(h: &Harness) {
    h.layout(400.0, 400.0);
    h.paint();
    h.tick();
    h.layout(400.0, 400.0);
    h.paint();
}

#[test]
fn queue_push_dismiss_clear() {
    let q = ToastQueue::new();
    assert!(q.is_empty());
    let a = q.push(
        sequoia_elements::ToastKind::Info,
        "A",
        "pierwszy",
        Duration::from_secs(60),
    );
    let b = q.success("B", "drugi");
    assert_ne!(a, b, "ids unique");
    assert_eq!(q.len(), 2);
    assert!(q.dismiss(a));
    assert_eq!(q.len(), 1);
    assert!(!q.dismiss(a), "second dismiss reports missing");
    q.clear();
    assert!(q.is_empty());
}

#[test]
fn empty_paints_without_overlay() {
    let q = ToastQueue::new();
    let h = harness(&q);
    assert!(h.overlay_ids().is_empty(), "empty: no overlay");
}

#[test]
fn push_shows_bottom_overlay() {
    let q = ToastQueue::new();
    let h = harness(&q);
    q.success("Zapisano", "Notka zapisana.");
    settle(&h);
    assert_eq!(h.overlay_ids().len(), 1, "one System overlay");
    let r = h.overlay_rect(h.overlay_ids()[0]).unwrap();
    assert!(r.bottom() <= 400.01, "clamped to viewport: {r:?}");
    assert!(r.bottom() >= 370.0, "near bottom edge: {r:?}");
}

#[test]
fn dismiss_hides_overlay() {
    let q = ToastQueue::new();
    let h = harness(&q);
    let id = q.success("Zapisano", "Notka zapisana.");
    settle(&h);
    assert_eq!(h.overlay_ids().len(), 1);

    q.dismiss(id);
    settle(&h);
    assert!(h.overlay_ids().is_empty(), "dismissed: no overlay");
}

#[test]
fn zero_ttl_expires_on_first_paint() {
    let q = ToastQueue::new();
    let h = harness(&q);
    q.push(
        sequoia_elements::ToastKind::Warn,
        "Chwila",
        "zero ttl",
        Duration::ZERO,
    );

    settle(&h);
    assert!(q.is_empty(), "zero ttl znika przy pierwszym paincie");
    settle(&h);
    assert!(h.overlay_ids().is_empty(), "brak overlay po wygaszeniu");
}

#[test]
fn overflow_keeps_single_overlay() {
    let q = ToastQueue::new();
    let h = harness(&q);
    for i in 0..5 {
        q.info(format!("T{i}"), "msg");
    }
    assert_eq!(q.len(), 5);
    settle(&h);
    assert_eq!(
        h.overlay_ids().len(),
        1,
        "one overlay hosts whole stack + counter"
    );
}
