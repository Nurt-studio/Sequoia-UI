#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::cell::Cell;
use std::rc::Rc;

use sequoia_core::{Color, Length, Rect};
use sequoia_elements::Container;
use sequoia_reactive::Signal;
use sequoia_test::Harness;
use sequoia_view::prelude::*;

const W: f32 = 400.0;
const H: f32 = 300.0;

struct Bar {
    h: f32,
    painted: Rc<Cell<u32>>,
}

impl View for Bar {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn is_leaf(&self) -> bool {
        true
    }

    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fill(1.0),
            height: Length::Px(self.h),
            ..LayoutStyle::default()
        }
    }

    fn paint_self(&self, ctx: &mut PaintCtx) {
        self.painted.set(self.painted.get() + 1);
        ctx.quad(ctx.rect, Color::RED, 0.0);
    }
}

fn tall_list(n: usize) -> (impl View, Rc<Cell<u32>>) {
    let painted = Rc::new(Cell::new(0));
    let mut col = Column::new();
    for _ in 0..n {
        col = col.child(Bar {
            h: 20.0,
            painted: Rc::clone(&painted),
        });
    }
    (ScrollView::new(col), painted)
}

fn paint_disabled(h: &Harness) -> sequoia_core::RenderList {
    let mut list = sequoia_core::RenderList::new();
    let mut text = sequoia_text::TextSystem::new();
    h.runtime()
        .paint_region(&mut list, &mut text, sequoia_view::CullCtx::DISABLED);
    list
}

fn scroll_of(h: &Harness) -> (Rc<ScrollView>, sequoia_view::ElementId) {
    let root = h.root().expect("root");
    if let Some(sv) = h.view::<ScrollView>(root) {
        return (sv, root);
    }
    let el = h.children(root)[0];
    let sv = h.view::<ScrollView>(el).expect("ScrollView");
    (sv, el)
}

#[test]
fn culling_reduces_paint_work_when_scrolled() {
    let (root, painted) = tall_list(100);
    let h = Harness::new(root);
    h.layout(W, H);
    h.paint();
    h.tick();
    h.layout(W, H);

    painted.set(0);
    let full_top = paint_disabled(&h);
    let full_top_paints = painted.get();

    painted.set(0);
    let culled_top = h.paint_culled();
    let culled_top_paints = painted.get();

    // RenderList clips fully off-screen quads at push time, so the visible
    // output is identical. The saving is in skipped `paint_self` calls.
    assert!(
        culled_top_paints < full_top_paints,
        "top: culled paints={} full paints={}",
        culled_top_paints,
        full_top_paints
    );
    assert_eq!(
        culled_top.len(),
        full_top.len(),
        "culling must not change the visible item list"
    );

    // Scroll to the bottom — culling saves the most there.
    let (sv, el) = scroll_of(&h);
    sv.scroll_to_bottom(&h.runtime(), el);
    h.layout(W, H);

    painted.set(0);
    let full_bottom = paint_disabled(&h);
    let full_bottom_paints = painted.get();

    painted.set(0);
    let culled_bottom = h.paint_culled();
    let culled_bottom_paints = painted.get();

    assert!(
        culled_bottom_paints < full_bottom_paints,
        "bottom: culled paints={} full paints={}",
        culled_bottom_paints,
        full_bottom_paints
    );
    // Rows 0..~85 are outside the clip — culled mode skips painting them.
    let saved = full_bottom_paints.saturating_sub(culled_bottom_paints);
    assert!(
        saved >= 80,
        "expected >=80 paint calls saved at bottom, got {saved} \
         (culled={culled_bottom_paints} full={full_bottom_paints})"
    );
    assert_eq!(
        culled_bottom.len(),
        full_bottom.len(),
        "culling must not change the visible item list"
    );
}

#[test]
fn culled_and_full_agree_on_visible_quads() {
    // Visible quads (those intersecting the viewport and ScrollView clip)
    // must be identical in both modes. Culling must not lose anything visible.
    let (root, _painted) = tall_list(50);
    let h = Harness::new(root);
    h.layout(W, H);
    h.paint();
    h.tick();
    h.layout(W, H);

    let (sv, el) = scroll_of(&h);
    sv.scroll_to_y(&h.runtime(), el, 240.0);
    h.layout(W, H);

    let full = paint_disabled(&h);
    let culled = h.paint_culled();

    let clip = Rect::from_xywh(0.0, 0.0, W, H);
    let visible_full: Vec<Rect> = full
        .quads()
        .filter(|q| q.rect.intersects(clip))
        .map(|q| q.rect)
        .collect();
    let visible_culled: Vec<Rect> = culled
        .quads()
        .filter(|q| q.rect.intersects(clip))
        .map(|q| q.rect)
        .collect();

    // Drawing order of visible quads must be preserved.
    assert_eq!(
        visible_full, visible_culled,
        "culling must not change the order or set of visible quads"
    );
}

#[test]
fn culled_keeps_static_scrollbar_and_visible_text() {
    // Scrollbar and text inside the viewport must survive culling.
    let h = Harness::new(ScrollView::new(
        Column::new()
            .gap(4.0)
            .child(Text::new("visible line 1"))
            .child(Text::new("visible line 2"))
            .child(Text::new("visible line 3")),
    ));
    h.layout(W, H);
    h.paint();
    h.tick();
    h.layout(W, H);

    let culled = h.paint_culled();
    let texts: Vec<_> = culled
        .items()
        .iter()
        .filter_map(|i| match i {
            sequoia_core::RenderItem::Text(t) => Some(t),
            _ => None,
        })
        .collect();
    assert!(
        texts.len() >= 3,
        "three visible texts must be in the list, got {}",
        texts.len()
    );
}

#[test]
fn disabled_cull_matches_plain_paint() {
    // DISABLED does more CPU work, but the visible item list is identical:
    // RenderList clips fully off-screen items at push time in both modes.
    let (root, _painted) = tall_list(20);
    let h = Harness::new(root);
    h.layout(W, H);
    let plain = h.paint();
    let disabled = paint_disabled(&h);
    assert_eq!(
        plain.len(),
        disabled.len(),
        "DISABLED must not change the visible item count"
    );
}

#[test]
fn cull_survives_absolute_child_outside_parent() {
    // `Stack` with an absolute child outside the stack rect — culling must
    // keep it, because Stack does not have `clip_self` and the child overflows.
    let h = Harness::new(ScrollView::new(
        Stack::new()
            .width(200.0)
            .height(200.0)
            .child(Text::new("base"))
            .child_at(Text::new("out"), StackPos::top_left().offset(-10.0, -10.0))
            .child_at(Text::new("out2"), StackPos::top_left().offset(250.0, 250.0)),
    ));
    h.layout(W, H);
    h.paint();
    h.tick();
    h.layout(W, H);

    let culled = h.paint_culled();
    // The overflow children intersect the viewport even though they are
    // outside the 200x200 Stack; Stack does not clip them.
    let n_texts = culled
        .items()
        .iter()
        .filter(|i| matches!(i, sequoia_core::RenderItem::Text(_)))
        .count();
    assert_eq!(
        n_texts, 3,
        "overflow children in Stack must not be culled (Stack does not clip)"
    );
}

#[test]
fn live_signal_in_clipped_subtree_still_culled_off_screen() {
    // A signal changes text outside the clip — culling should not force
    // building quads for invisible text.
    let n = Signal::new(0u32);
    let n2 = n.clone();
    let h = Harness::new(ScrollView::new(
        Column::new()
            .child(Text::new("top"))
            .child(Container::new().height(1000.0))
            .child(Text::with(move || format!("n={}", n2.get()))),
    ));
    h.layout(W, H);
    h.paint();
    h.tick();
    h.layout(W, H);

    let before = h.paint_culled_stats(W, H);
    n.set(1);
    h.tick();
    h.layout(W, H);
    let after = h.paint_culled_stats(W, H);

    // The live text is below the viewport. Updating its signal rebuilds and
    // lays out the text, but culling keeps it out of the render list.
    assert_eq!(
        before.quads, after.quads,
        "off-screen live text must not add quads in cull mode"
    );
}
