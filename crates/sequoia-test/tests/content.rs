use std::cell::Cell;
use std::rc::Rc;

use sequoia_core::RenderItem;
use sequoia_elements::Button;
use sequoia_reactive::Signal;
use sequoia_test::Harness;
use sequoia_view::prelude::*;

const LONG: &str = "aaa bbb ccc ddd eee fff ggg hhh iii jjj kkk lll mmm nnn";

#[test]
fn wrapped_text_is_narrower_and_taller() {
    let single = Harness::new(Text::new(LONG).size(16.0));
    single.layout(800.0, 600.0);
    let single_rect = single.rect(single.root().unwrap()).unwrap();

    let wrapped = Harness::new(Text::new(LONG).size(16.0).wrap_width(120.0));
    wrapped.layout(800.0, 600.0);
    let wrapped_rect = wrapped.rect(wrapped.root().unwrap()).unwrap();

    assert!(
        single_rect.size.width > 120.0,
        "control: single line is wide"
    );
    assert!(
        wrapped_rect.size.width <= 120.0 + 0.5,
        "wrapped within the limit: {}",
        wrapped_rect.size.width
    );
    assert!(
        wrapped_rect.size.height > single_rect.size.height,
        "block is taller: {} > {}",
        wrapped_rect.size.height,
        single_rect.size.height
    );
}

#[test]
fn wrapped_text_paints_glyphs() {
    if sequoia_text::TextSystem::new()
        .shape("a", 16.0)
        .glyph_count()
        == 0
    {
        eprintln!("skip: no fonts in this environment");
        return;
    }
    let h = Harness::new(Text::new(LONG).size(16.0).wrap_width(120.0));
    h.layout(800.0, 600.0);
    let list = h.paint();
    let glyphs: usize = list
        .items()
        .iter()
        .map(|i| match i {
            RenderItem::Text(t) => t.glyphs.len(),
            RenderItem::Quad(_)
            | RenderItem::Image(_)
            | RenderItem::Shadow(_)
            | RenderItem::Waveform(_) => 0,

            _ => 0,
        })
        .sum();
    assert!(glyphs > 20, "many lines of glyphs: {glyphs}");
}

#[test]
fn truncated_text_fits_and_paints() {
    let content = "Ann has a cat, a dog and a hamster";
    let h = Harness::new(Text::new(content).size(16.0).truncate(80.0));
    h.layout(800.0, 600.0);
    let rect = h.rect(h.root().unwrap()).unwrap();
    assert!(
        rect.size.width <= 80.0 + 0.5,
        "truncated to the limit: {}",
        rect.size.width
    );
    let list = h.paint();
    let glyphs: usize = list
        .items()
        .iter()
        .map(|i| match i {
            RenderItem::Text(t) => t.glyphs.len(),
            RenderItem::Quad(_)
            | RenderItem::Image(_)
            | RenderItem::Shadow(_)
            | RenderItem::Waveform(_) => 0,

            _ => 0,
        })
        .sum();
    assert!(glyphs > 0, "the ellipsis has glyphs too");

    let full = Harness::new(Text::new(content).size(16.0));
    full.layout(800.0, 600.0);
    let full_rect = full.rect(full.root().unwrap()).unwrap();
    assert!(full_rect.size.width > 80.0, "control: full text is wider");
}

#[test]
fn image_rgba_paints_image_item() {
    let data = vec![255u8; 4 * 3 * 4];
    let h = Harness::new(Image::rgba(42, 4, 3, data).width(200.0).height(150.0));
    h.layout(400.0, 400.0);

    let rect = h.rect(h.root().unwrap()).unwrap();
    assert_eq!((rect.size.width, rect.size.height), (200.0, 150.0));

    let list = h.paint();
    let images: Vec<_> = list.images().collect();
    assert_eq!(images.len(), 1);

    assert_eq!(
        images[0].rect,
        sequoia_core::Rect::from_xywh(0.0, 0.0, 200.0, 150.0)
    );
    assert_eq!(images[0].uv, [0.0, 0.0, 1.0, 1.0]);
}

#[test]
fn image_contains_letterbox_for_other_aspect() {
    let data = vec![255u8; 8 * 2 * 4];
    let h = Harness::new(Image::rgba(7, 8, 2, data).width(200.0).height(200.0));
    h.layout(400.0, 400.0);
    let list = h.paint();
    let images: Vec<_> = list.images().collect();
    assert_eq!(images.len(), 1);

    assert_eq!(
        images[0].rect,
        sequoia_core::Rect::from_xywh(0.0, 75.0, 200.0, 50.0)
    );
}

fn find_by_type(h: &Harness, id: ElementId, needle: &str) -> Option<ElementId> {
    if h.type_name_of(id).is_some_and(|n| n.contains(needle)) {
        return Some(id);
    }
    h.children(id)
        .into_iter()
        .find_map(|c| find_by_type(h, c, needle))
}

#[test]
fn dynamic_text_subscribes_to_signal() {
    let count = Signal::new(0i32);
    let shown = count.clone();
    let h = Harness::new(Text::with(move || format!("n = {}", shown.get())).size(20.0));
    h.layout(400.0, 300.0);
    let before = h.stats().rebuilds;
    count.set(1);
    h.tick();
    assert_eq!(
        h.stats().rebuilds,
        before + 1,
        "signal change must rebuild the text element"
    );
}

#[test]
fn button_click_on_dynamic_text_schedules_frame() {
    let count = Signal::new(0i32);
    let up = count.clone();
    let shown = count.clone();
    let h = Harness::new(
        Column::new()
            .center()
            .gap(8.0)
            .fill_size()
            .child(Text::with(move || format!("n = {}", shown.get())).size(20.0))
            .child(Button::labeled("+1").on_click(move || up.update(|v| *v += 1))),
    );
    h.layout(400.0, 300.0);

    let redraws = Rc::new(Cell::new(0usize));
    let counter = Rc::clone(&redraws);
    h.runtime()
        .set_redraw_hook(move || counter.set(counter.get() + 1));

    let button = find_by_type(&h, h.root().unwrap(), "Button").expect("button in the tree");
    let rect = h.rect(button).expect("button after layout");
    h.click(
        rect.left() + rect.size.width * 0.5,
        rect.top() + rect.size.height * 0.5,
    );

    assert_eq!(count.get_untracked(), 1, "click changes the value");
    assert!(
        redraws.get() > 0,
        "signal change must schedule a frame (otherwise the click hangs)"
    );
    let before = h.stats().rebuilds;
    h.tick();
    assert!(
        h.stats().rebuilds > before,
        "text element must rebuild after the click"
    );
}
