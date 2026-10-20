#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::rc::Rc;

use sequoia_core::{Color, RenderItem};
use sequoia_reactive::Signal;
use sequoia_test::Harness;
use sequoia_text::{RichAlign, TextSpan};
use sequoia_view::prelude::*;

fn code_spans() -> Vec<TextSpan> {
    vec![
        TextSpan::new("fn ").color(Color::BLUE).bold(),
        TextSpan::new("main").color(Color::WHITE),
        TextSpan::new("() {}").color(Color::GREEN),
    ]
}

#[test]
fn paints_one_text_draw_per_color() {
    let h = Harness::new(RichText::new(code_spans()).size(16.0));
    h.layout(300.0, 100.0);
    let mut colors: Vec<Color> = Vec::new();
    for item in h.paint().items() {
        if let RenderItem::Text(t) = item {
            colors.push(t.color);
        }
    }
    assert_eq!(colors, vec![Color::BLUE, Color::WHITE, Color::GREEN]);
}

#[test]
fn span_background_paints_a_quad_under_text() {
    let h = Harness::new(
        RichText::new(vec![TextSpan::new("ERR failing").background(Color::RED)]).size(16.0),
    );
    h.layout(300.0, 100.0);
    let list = h.paint();
    let quads = list.quads().count();
    assert_eq!(quads, 1, "exactly the background quad");
    let has_text = list
        .items()
        .iter()
        .any(|i| matches!(i, RenderItem::Text(_)));
    assert!(has_text);
}

#[test]
fn wrap_width_grows_intrinsic_height() {
    let spans = vec![TextSpan::new("abcdefghij")];
    let flat = Harness::new(RichText::new(spans.clone()).size(16.0));
    flat.layout(300.0, 200.0);
    let flat_h = flat.rect(flat.root().unwrap()).unwrap().size.height;

    let wrapped = Harness::new(
        RichText::new(spans)
            .size(16.0)
            .wrap_width(40.0)
            .wrap_mode(WrapMode::Character),
    );
    wrapped.layout(300.0, 200.0);
    let wrapped_h = wrapped.rect(wrapped.root().unwrap()).unwrap().size.height;

    assert!(flat_h > 0.0 && flat_h < 30.0, "single line: {flat_h}");
    assert!(
        wrapped_h > flat_h * 2.0,
        "wrapped: {wrapped_h} vs flat: {flat_h}"
    );
}

#[test]
fn click_at_edges_hits_first_and_last_span() {
    use std::cell::Cell;

    let spans = vec![
        TextSpan::new("left ").color(Color::WHITE),
        TextSpan::new("right").color(Color::BLUE),
    ];
    let hits: Rc<Cell<Vec<usize>>> = Rc::new(Cell::new(Vec::new()));
    let h = Harness::new({
        let hits = hits.clone();
        RichText::new(spans).size(16.0).on_span_click(move |s| {
            let mut v = hits.take();
            v.push(s);
            hits.set(v);
        })
    });
    h.layout(300.0, 100.0);
    let rect = h.rect(h.root().unwrap()).unwrap();
    let _ = h.paint();

    h.pointer_down(rect.left() + 1.0, rect.top() + 8.0);
    h.pointer_up(rect.left() + 1.0, rect.top() + 8.0);
    h.pointer_down(rect.right() - 2.0, rect.top() + 8.0);
    h.pointer_up(rect.right() - 2.0, rect.top() + 8.0);
    assert_eq!(hits.take(), vec![0, 1]);
}

#[test]
fn clickable_paragraph_shows_pointer_cursor() {
    let h = Harness::new(RichText::new(code_spans()).size(16.0).on_span_click(|_| {}));
    h.layout(300.0, 100.0);
    assert_eq!(h.runtime().cursor_icon(), CursorIcon::Default);
    h.pointer_move(10.0, 10.0);
    assert_eq!(h.runtime().cursor_icon(), CursorIcon::Pointer);
}

#[test]
fn centered_paragraph_keeps_glyphs_inside() {
    let h = Harness::new(
        RichText::new(code_spans())
            .size(16.0)
            .wrap_width(280.0)
            .width(280.0)
            .align(RichAlign::Center),
    );
    h.layout(300.0, 100.0);
    let list = h.paint();
    let rect = h.rect(h.root().unwrap()).unwrap();
    let glyphs: usize = list
        .items()
        .iter()
        .map(|i| match i {
            RenderItem::Text(t) => t.glyphs.len(),
            _ => 0,
        })
        .sum();
    assert!(glyphs > 0);
    assert!(rect.size.width > 0.0);
}

#[test]
fn reactive_spans_rebuild_on_signal() {
    let counter: Signal<u32> = Signal::new(0);
    let c = counter.clone();
    let h = Harness::new(RichText::with(move || {
        vec![TextSpan::new(format!("n={}", c.get())).color(Color::GREEN)]
    }));
    h.layout(300.0, 100.0);
    let before = h.paint();
    h.set(&counter, 1);
    h.layout(300.0, 100.0);
    let after = h.paint();
    assert_ne!(format!("{before:?}"), format!("{after:?}"));
}
