#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_core::prelude::*;
use sequoia_text::TextSystem;
use sequoia_view::prelude::*;

#[test]
fn canvas_paints_pixel_buffer_as_image() {
    let mut pixels = Pixels::filled(4, 4, Color::RED);
    pixels.set(0, 0, Color::BLUE);
    let id = pixels.id();

    let rt = Runtime::new();
    rt.mount(Canvas::new().on_draw(move |g: &mut Painter| {
        g.pixels(g.rect(), &pixels);
    }));
    let mut text = TextSystem::new();
    rt.layout(Rect::from_xywh(0.0, 0.0, 100.0, 100.0), &mut text);
    let mut list = RenderList::new();
    rt.paint(&mut list, &mut text);

    let images: Vec<_> = list.images().collect();
    assert_eq!(images.len(), 1, "one image item");
    assert_eq!(images[0].source.cache_key(), id);
    assert_eq!(images[0].source.extent(), Some((4, 4)));
    assert_eq!(images[0].uv, [0.0, 0.0, 1.0, 1.0]);
    assert_eq!(images[0].rect, Rect::from_xywh(0.0, 0.0, 100.0, 100.0));
    assert_eq!(images[0].sampling, ImageSampling::Linear);
}

#[test]
fn canvas_can_request_nearest_pixel_sampling() {
    let pixels = Pixels::filled(2, 2, Color::RED);

    let rt = Runtime::new();
    rt.mount(Canvas::new().on_draw(move |g: &mut Painter| {
        g.pixels_nearest(g.rect(), &pixels);
    }));
    let mut text = TextSystem::new();
    rt.layout(Rect::from_xywh(0.0, 0.0, 100.0, 100.0), &mut text);
    let mut list = RenderList::new();
    rt.paint(&mut list, &mut text);

    let images: Vec<_> = list.images().collect();
    assert_eq!(images.len(), 1);
    assert_eq!(images[0].sampling, ImageSampling::Nearest);
}

#[test]
fn canvas_paints_zoomed_pixel_region_with_matching_uv() {
    let pixels = Pixels::filled(100, 50, Color::GREEN);
    let revision = pixels.revision();

    let rt = Runtime::new();
    rt.mount(Canvas::new().on_draw(move |g: &mut Painter| {
        let dst = Rect::from_xywh(0.0, 0.0, 200.0, 100.0);
        g.pixels_region(dst, &pixels, Rect::from_xywh(25.0, 10.0, 50.0, 25.0));
    }));
    let mut text = TextSystem::new();
    rt.layout(Rect::from_xywh(0.0, 0.0, 300.0, 200.0), &mut text);
    let mut list = RenderList::new();
    rt.paint(&mut list, &mut text);

    let images: Vec<_> = list.images().collect();
    assert_eq!(images.len(), 1);
    assert_eq!(images[0].source.dynamic_version(), Some(revision));
    assert_eq!(
        images[0].uv,
        [0.25, 0.2, 0.75, 0.7],
        "source rect mapped proportionally"
    );
}

#[test]
fn empty_pixel_buffer_paints_nothing() {
    let pixels = Pixels::new(0, 0);

    let rt = Runtime::new();
    rt.mount(Canvas::new().on_draw(move |g: &mut Painter| {
        g.pixels(g.rect(), &pixels);
    }));
    let mut text = TextSystem::new();
    rt.layout(Rect::from_xywh(0.0, 0.0, 50.0, 50.0), &mut text);
    let mut list = RenderList::new();
    rt.paint(&mut list, &mut text);

    assert_eq!(list.images().count(), 0);
}
