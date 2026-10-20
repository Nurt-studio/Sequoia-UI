use sequoia_elements::Container;
use sequoia_test::Harness;
use sequoia_view::{Anchor, OverlayEntry, OverlayOverflow, Placement, prelude::*};

fn overflowing_tree(overflow: Overflow) -> Harness {
    let h = Harness::new(
        Stack::new()
            .width(200.0)
            .height(200.0)
            .overflow(overflow)
            .child_at(
                Panel::new().width(100.0).height(100.0),
                StackPos {
                    top: Some(0.0),
                    left: Some(0.0),
                    bottom: None,
                    right: None,
                    dx: 250.0,
                    dy: 0.0,
                    z: 0,
                },
            ),
    );
    h.layout(800.0, 600.0);
    let _ = h.paint();
    h
}

fn max_quad_right(h: &Harness) -> f32 {
    let list = h.paint();
    list.quads().map(|q| q.rect.right()).fold(0.0, f32::max)
}

#[test]
fn hidden_clips_child_to_parent_box() {
    let h = overflowing_tree(Overflow::Hidden);
    assert!(max_quad_right(&h) <= 200.0 + 1.0, "clipped to 200px");
}

#[test]
fn visible_keeps_overflowing_paint() {
    let h = overflowing_tree(Overflow::Visible);
    assert!(
        max_quad_right(&h) >= 349.0,
        "sticks out to ~350px (default so far)"
    );
}

#[test]
fn container_supports_overflow_builder() {
    let h = Harness::new(
        Container::new()
            .width(200.0)
            .height(200.0)
            .overflow(Overflow::Hidden)
            .child(Stack::new().width(200.0).height(200.0).child_at(
                Panel::new().width(100.0).height(100.0),
                StackPos {
                    top: Some(0.0),
                    left: Some(0.0),
                    bottom: None,
                    right: None,
                    dx: 250.0,
                    dy: 0.0,
                    z: 0,
                },
            )),
    );
    h.layout(800.0, 600.0);
    let list = h.paint();
    let max_right = list.quads().map(|q| q.rect.right()).fold(0.0, f32::max);
    assert!(max_right <= 200.0 + 1.0, "container clips too");
}

fn popup_rect(policy: OverlayOverflow) -> sequoia_core::Rect {
    let h = Harness::new(Column::new().fill_size());
    h.layout(800.0, 600.0);
    let _ = h.paint();
    let id = h.overlay_show(
        OverlayEntry::new(Panel::new().width(200.0).height(200.0))
            .anchor(Anchor::Rect(sequoia_core::Rect::from_xywh(
                100.0, 500.0, 120.0, 32.0,
            )))
            .placement(Placement::Bottom)
            .overflow(policy),
    );
    h.layout(800.0, 600.0);
    let _ = h.paint();
    h.overlay_rect(id).expect("popup has rect")
}

#[test]
fn overlay_default_flips_above_when_no_room_below() {
    let r = popup_rect(OverlayOverflow::FlipAndClamp);
    assert!(r.bottom() <= 500.0 + 1.0, "flipped above the anchor: {r:?}");
}

#[test]
fn overlay_clamp_stays_inside_viewport() {
    let r = popup_rect(OverlayOverflow::Clamp);
    assert!(
        r.top() >= 0.0 && r.bottom() <= 600.0 + 1.0,
        "centered: {r:?}"
    );
    assert!(
        r.left() >= 0.0 && r.right() <= 800.0 + 1.0,
        "horizontally: {r:?}"
    );
}

#[test]
fn overlay_visible_may_bleed() {
    let r = popup_rect(OverlayOverflow::Visible);
    assert!(
        (r.top() - 532.0).abs() < 1.0,
        "exactly under the anchor: {r:?}"
    );
    assert!(r.bottom() > 600.0, "sticks out past viewport: {r:?}");
}
