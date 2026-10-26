#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_core::prelude::*;
use sequoia_reactive::Signal;
use sequoia_text::TextSystem;
use sequoia_view::prelude::*;

fn paint_root<V: View + 'static>(view: V) -> (Runtime, RenderList, TextSystem) {
    let rt = Runtime::new();
    rt.mount(view);
    let mut text = TextSystem::new();
    rt.layout(Rect::from_xywh(0.0, 0.0, 400.0, 400.0), &mut text);
    let mut list = RenderList::new();
    rt.paint(&mut list, &mut text);
    (rt, list, text)
}

#[test]
fn opacity_modifier_modulates_alpha() {
    let (_, list, _) = paint_root(
        Panel::new()
            .background(Color::RED)
            .width(200.0)
            .height(100.0)
            .opacity(0.5),
    );
    let q = list.quads().next().expect("panel maluje quad");
    assert_eq!(q.color, Color::RED, "item alpha stays untouched");
    assert!(
        (list.opacity_at(0) - 0.5).abs() < 1e-6,
        "group alpha po .opacity(0.5): {}",
        list.opacity_at(0)
    );
}

#[test]
fn scale_modifier_scales_paint_rect() {
    let (_, list, _) = paint_root(
        Panel::new()
            .background(Color::RED)
            .width(100.0)
            .height(50.0)
            .scale(2.0),
    );
    let q = list.quads().next().expect("panel maluje quad");
    assert!(
        (q.rect.size.width - 200.0).abs() < 1.0,
        "width after .scale(2): {:?}",
        q.rect.size
    );
    assert!(
        (q.rect.size.height - 100.0).abs() < 1.0,
        "height after .scale(2): {:?}",
        q.rect.size
    );
}

#[test]
fn reactive_background_follows_signal() {
    let bg = Signal::new(Color::RED);
    let rt = Runtime::new();
    rt.mount(Panel::new().width(50.0).height(50.0).bg(bg.clone()));
    let mut text = TextSystem::new();
    rt.layout(Rect::from_xywh(0.0, 0.0, 400.0, 400.0), &mut text);
    let mut list = RenderList::new();
    rt.paint(&mut list, &mut text);
    assert_eq!(list.quads().next().unwrap().color, Color::RED);

    bg.set(Color::BLUE);
    rt.tick();
    rt.layout(Rect::from_xywh(0.0, 0.0, 400.0, 400.0), &mut text);
    let mut list = RenderList::new();
    rt.paint(&mut list, &mut text);
    assert_eq!(list.quads().next().unwrap().color, Color::BLUE);
}

#[test]
fn when_mounts_and_unmounts_child() {
    let open = Signal::new(true);
    let rt = Runtime::new();
    rt.mount(when(
        open.clone(),
        Panel::new().background(Color::RED).width(50.0).height(50.0),
    ));
    let mut text = TextSystem::new();
    rt.layout(Rect::from_xywh(0.0, 0.0, 400.0, 400.0), &mut text);
    let mut list = RenderList::new();
    rt.paint(&mut list, &mut text);

    rt.advance_for_test(std::time::Duration::from_millis(16));
    rt.layout(Rect::from_xywh(0.0, 0.0, 400.0, 400.0), &mut text);
    let mut list = RenderList::new();
    rt.paint(&mut list, &mut text);
    assert_eq!(list.quads().count(), 1, "otwarte montuje dziecko");

    open.set(false);
    rt.tick();

    for _ in 0..60 {
        rt.advance_for_test(std::time::Duration::from_millis(10));
        rt.tick();
        rt.layout(Rect::from_xywh(0.0, 0.0, 400.0, 400.0), &mut text);
        let mut scratch = RenderList::new();
        rt.paint(&mut scratch, &mut text);
    }
    rt.tick();
    rt.layout(Rect::from_xywh(0.0, 0.0, 400.0, 400.0), &mut text);
    let mut list = RenderList::new();
    rt.paint(&mut list, &mut text);
    assert_eq!(list.quads().count(), 0, "closed, unfastens after leaving");
}

#[test]
fn enter_fades_in_from_zero() {
    let v = Panel::new()
        .background(Color::RED)
        .width(50.0)
        .height(50.0)
        .enter(fade());

    let op = v.paint_opacity().unwrap_or(1.0);
    assert!(op < 0.2, "start input: {op}");

    std::thread::sleep(std::time::Duration::from_millis(300));
    let op = v.paint_opacity().unwrap_or(0.0);
    assert!(op > 0.99, "end of input: {op}");
}

#[test]
fn canvas_shorts_build() {
    let c = Canvas::new()
        .grid(20.0)
        .axes()
        .snap_every(4.0)
        .on_draw(|_| {});
    assert_eq!(c.snap_step(), Some(4.0));
    assert!((c.snap_value(7.0) - 8.0).abs() < 1e-6);
    let (_, list, _) = paint_root(c);

    let _ = list;
}
