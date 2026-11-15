#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::cell::Cell;
use std::rc::Rc;

use sequoia_reactive::Signal;
use sequoia_seq_macros::include_seq;
use sequoia_test::Harness;

fn glyphs(h: &Harness) -> usize {
    use sequoia_core::RenderItem;
    h.paint()
        .items()
        .iter()
        .map(|i| match i {
            RenderItem::Text(t) => t.glyphs.len(),
            _ => 0,
        })
        .sum()
}

fn settle(h: &Harness) {
    for _ in 0..30 {
        std::thread::sleep(std::time::Duration::from_millis(10));
        h.tick();
        h.layout(320.0, 200.0);
        h.paint();
    }
}

#[test]
fn typed_builder_loads_and_renders_imported_component() {
    let count = Signal::new(2i64);
    let label = Signal::new("hi".to_string());
    let saves = Rc::new(Cell::new(0u32));
    let s = saves.clone();

    let loaded = include_seq!("tests/fixtures/codegen_app.seq")
        .prop_count(count.clone())
        .prop_label(label.clone())
        .on_save(move || s.set(s.get() + 1))
        .on_changed(|_| {})
        .on_moved(|_, _| {})
        .on_picked(|_, _, _| {})
        .on_zoom(|_, _, _| {})
        .on_zoom_pinch(|_, _, _| {})
        .load()
        .unwrap();

    assert_eq!(loaded.meta.title, "Demo");
    assert_eq!(loaded.meta.size, (320.0, 200.0));
    let h = Harness::new(loaded.view);
    h.layout(320.0, 200.0);
    h.paint();
    h.tick();
    assert_eq!(glyphs(&h), 4, "Badge + Text render 'hi'");
    label.set("hello".to_string());
    settle(&h);
    assert_eq!(
        glyphs(&h),
        10,
        "prop update also refreshes imported components"
    );
}

#[test]
fn generated_builder_accepts_configure() {
    let count = Signal::new(1i64);
    let label = Signal::new("x".to_string());
    let loaded = include_seq!("tests/fixtures/codegen_app.seq")
        .configure(|b| b.prop_int("count", count.clone()))
        .configure(|b| b.prop_str("label", label.clone()))
        .on_save(|| {})
        .on_changed(|_| {})
        .on_moved(|_, _| {})
        .on_picked(|_, _, _| {})
        .on_zoom(|_, _, _| {})
        .load()
        .unwrap();
    let h = Harness::new(loaded.view);
    h.layout(320.0, 200.0);
    h.paint();
    h.tick();
    assert_eq!(glyphs(&h), 2, "Badge + Text render 'x'");
}
