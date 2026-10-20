#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::HashMap;

use sequoia_reactive::Signal;
use sequoia_test::Harness;
use sequoia_view::prelude::*;

const W: f32 = 400.0;
const H: f32 = 480.0;

struct Demo {
    items: Signal<Vec<u64>>,
}

impl View for Demo {
    fn body(&self) -> impl View + 'static {
        VirtualList::new({
            let s = self.items.clone();
            move || s.get()
        })
        .key(|id: &u64| *id)
        .item(|id: &u64| Text::new(format!("Item {id}")))
        .item_height(24.0)
        .overscan(4)
    }
}

fn harness(n: u64) -> Harness {
    let items: Vec<u64> = (1..=n).collect();
    Harness::new(Demo {
        items: Signal::new(items),
    })
}

fn settle(h: &Harness) {
    h.layout(W, H);
    let _ = h.paint();
    h.tick();
    h.layout(W, H);
}

fn vl_of(h: &Harness) -> sequoia_view::ElementId {
    h.children(h.root().unwrap())[0]
}

fn key_map(h: &Harness, vl: sequoia_view::ElementId) -> HashMap<u64, sequoia_view::ElementId> {
    h.children(vl)
        .iter()
        .map(|&id| (h.key_of(id).unwrap(), id))
        .collect()
}

#[test]
fn fallback_window_before_first_paint() {
    let h = harness(1000);
    h.layout(W, H);
    let vl = vl_of(&h);
    assert_eq!(
        h.children(vl).len(),
        64,
        "FALLBACK_WINDOW before first paint"
    );
}

#[test]
fn mounts_only_visible_window() {
    let h = harness(1000);
    settle(&h);

    let vl = vl_of(&h);
    let count = h.children(vl).len();

    assert_eq!(count, 25, "start window = 25 (no top overscan)");

    assert!(
        h.element_count() < 120,
        "1000 rows -> ~52 items, got {}",
        h.element_count()
    );
}

#[test]
fn scroll_reuses_overlapping_keys() {
    let h = harness(1000);
    settle(&h);

    let vl = vl_of(&h);
    let before = key_map(&h, vl);
    assert_eq!(before.len(), 25);
    let mounts_before = h.stats().mounts;
    let unmounts_before = h.stats().unmounts;

    h.wheel(200.0, 240.0, -240.0);
    h.tick();
    settle(&h);

    let after = key_map(&h, vl);
    let s = h.stats();

    assert_eq!(after.len(), 29);

    for k in 7..=25u64 {
        assert_eq!(after.get(&k), before.get(&k), "key {k} must keep ElementId");
    }

    assert_eq!(s.mounts - mounts_before, 20, "klucze 26..=35 × 2 elementy");

    assert_eq!(
        s.unmounts - unmounts_before,
        12,
        "klucze 1..=6 × 2 elementy"
    );
}

#[test]
fn offset_clamps_at_end_last_item_present() {
    let h = harness(1000);
    settle(&h);

    h.wheel(200.0, 240.0, -100_000.0);
    h.tick();
    settle(&h);

    let vl = vl_of(&h);
    let keys: Vec<u64> = h
        .children(vl)
        .iter()
        .map(|&id| h.key_of(id).unwrap())
        .collect();
    assert!(keys.contains(&1000), "last row mounted");
    assert!(!keys.contains(&1), "list head unmounted");
}

#[test]
fn rows_positioned_by_index_minus_offset() {
    let h = harness(1000);
    settle(&h);

    let vl = vl_of(&h);
    let vl_rect = h.rect(vl).unwrap();

    let mut tops: Vec<(u64, f32)> = h
        .children(vl)
        .iter()
        .map(|&id| (h.key_of(id).unwrap(), h.rect(id).unwrap().top()))
        .collect();
    tops.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    assert_eq!(tops[0].0, 1);
    assert!((tops[0].1 - vl_rect.top()).abs() < 0.01);
    for w in tops.windows(2) {
        assert!((w[1].1 - w[0].1 - 24.0).abs() < 0.01, "rows every 24px");
    }

    h.wheel(200.0, 240.0, -240.0);
    h.tick();
    settle(&h);
    let vl_rect = h.rect(vl).unwrap();
    let mut tops: Vec<(u64, f32)> = h
        .children(vl)
        .iter()
        .map(|&id| (h.key_of(id).unwrap(), h.rect(id).unwrap().top()))
        .collect();
    tops.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    assert_eq!(tops[0].0, 7);
    assert!((tops[0].1 - (vl_rect.top() + 6.0 * 24.0 - 240.0)).abs() < 0.01);
    let top_11 = tops.iter().find(|(k, _)| *k == 11).unwrap().1;
    assert!(
        (top_11 - vl_rect.top()).abs() < 0.01,
        "row 11 at viewport top"
    );
}

#[test]
fn viewport_heals_during_layout_without_paint() {
    let h = harness(1000);

    h.layout(W, H);
    h.tick();
    h.layout(W, H);
    let vl = vl_of(&h);
    assert_eq!(
        h.children(vl).len(),
        25,
        "window after tick+layout, no paint"
    );
}

#[test]
fn scroll_back_and_forth_does_not_leak() {
    let h = harness(500);
    settle(&h);
    let base_elements = h.element_count();
    let base_slots = h.children(vl_of(&h)).len();

    for i in 0..10 {
        h.wheel(200.0, 240.0, if i % 2 == 0 { -960.0 } else { 960.0 });
        h.tick();
        settle(&h);
    }

    assert_eq!(
        h.element_count(),
        base_elements,
        "no leaks after scroll-scroll"
    );
    assert_eq!(h.children(vl_of(&h)).len(), base_slots);
}

fn vl_view(h: &Harness) -> std::rc::Rc<VirtualList<u64>> {
    h.view::<VirtualList<u64>>(vl_of(h)).expect("VirtualList")
}

#[test]
fn on_scroll_fires_on_change_only() {
    use std::cell::Cell;
    let seen: std::rc::Rc<Cell<f32>> = std::rc::Rc::new(Cell::new(-1.0));
    let count: std::rc::Rc<Cell<u32>> = std::rc::Rc::new(Cell::new(0));
    let seen_cb = seen.clone();
    let count_cb = count.clone();
    let h = Harness::new(
        VirtualList::new({
            let items: Vec<u64> = (1..=1000).collect();
            move || items.clone()
        })
        .key(|id: &u64| *id)
        .item(|id: &u64| Text::new(format!("Item {id}")))
        .item_height(24.0)
        .overscan(4)
        .on_scroll(move |off| {
            seen_cb.set(off);
            count_cb.set(count_cb.get() + 1);
        }),
    );
    settle(&h);

    let vl = h
        .view::<VirtualList<u64>>(h.root().unwrap())
        .expect("VirtualList");
    vl.scroll_to(240.0);
    assert_eq!(count.get(), 1);
    assert_eq!(seen.get(), 240.0);
    vl.scroll_to(240.0);
    assert_eq!(count.get(), 1, "same offset does not call again");
    vl.scroll_by(24.0);
    assert_eq!(count.get(), 2);
}

#[test]
fn programmatic_scroll_api_clamps() {
    let h = harness(1000);
    settle(&h);
    let vl = vl_view(&h);
    let max = vl.max_offset();
    assert_eq!(max, 1000.0 * 24.0 - H, "max = content - viewport");

    vl.scroll_to(1_000_000.0);
    assert_eq!(VirtualList::offset(&vl), max, "clamp do max");
    vl.scroll_to(-50.0);
    assert_eq!(VirtualList::offset(&vl), 0.0, "clamp do zera");
    vl.scroll_to_bottom();
    assert_eq!(VirtualList::offset(&vl), max);
    vl.page_up();
    assert_eq!(VirtualList::offset(&vl), max - H);
    vl.page_down();
    assert_eq!(VirtualList::offset(&vl), max);
    vl.scroll_to_top();
    assert_eq!(VirtualList::offset(&vl), 0.0);
    vl.scroll_by(100.0);
    assert_eq!(VirtualList::offset(&vl), 100.0);

    vl.scroll_to_bottom();
    h.tick();
    h.layout(W, H);
    let first_key = h
        .children(vl_of(&h))
        .into_iter()
        .filter_map(|id| h.key_of(id))
        .min()
        .expect("window keys");
    assert!(first_key > 900, "bottom of list: first_key={first_key}");
}

#[test]
fn keyboard_arrows_pages_home_end() {
    let h = harness(1000);
    settle(&h);
    h.pointer_down(200.0, 240.0);
    let vl = vl_view(&h);

    h.key(Key::ARROW_DOWN);
    assert_eq!(VirtualList::offset(&vl), 40.0, "default step");
    h.key(Key::ARROW_UP);
    assert_eq!(VirtualList::offset(&vl), 0.0);
    h.key(Key::PAGE_DOWN);
    assert_eq!(VirtualList::offset(&vl), H);
    h.key(Key::PAGE_UP);
    assert_eq!(VirtualList::offset(&vl), 0.0);
    h.key(Key::END);
    assert_eq!(VirtualList::offset(&vl), vl.max_offset());
    h.key(Key::HOME);
    assert_eq!(VirtualList::offset(&vl), 0.0);
}

#[test]
fn scrollbar_paints_only_when_scrollable() {
    let small = harness(3);
    small.layout(W, H);
    assert_eq!(
        small.paint().quads().count(),
        0,
        "3 wiersze < viewport: brak paska"
    );

    let big = harness(1000);
    big.layout(W, H);
    assert_eq!(
        big.paint().quads().count(),
        2,
        "track + thumb, teksty to glyphy nie quady"
    );
}

#[test]
fn thumb_drag_moves_offset() {
    let h = harness(1000);
    settle(&h);
    let vl = vl_view(&h);
    assert_eq!(VirtualList::offset(&vl), 0.0);

    h.pointer_down(394.0, 10.0);
    assert_eq!(VirtualList::offset(&vl), 0.0, "chwyt nie skacze");
    h.pointer_move(394.0, 200.0);
    let off = VirtualList::offset(&vl);
    assert!(off > 1000.0, "drag down: offset={off}");
    h.pointer_up(394.0, 200.0);
    assert_eq!(VirtualList::offset(&vl), off, "puszczenie nie cofa");
}

#[test]
fn track_click_jumps_and_drags() {
    let h = harness(1000);
    settle(&h);
    let vl = vl_view(&h);

    h.pointer_down(394.0, 460.0);
    let off = VirtualList::offset(&vl);
    assert!(off > 10_000.0, "jump down track: offset={off}");
    h.pointer_up(394.0, 460.0);
}
