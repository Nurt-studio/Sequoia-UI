#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::cell::Cell;
use std::rc::Rc;

use sequoia_core::{ImageSource, RenderItem};
use sequoia_reactive::Signal;
use sequoia_seq::{HostBindings, SeqLoader};
use sequoia_test::Harness;

fn image_sources(h: &Harness) -> Vec<ImageSource> {
    h.paint()
        .items()
        .iter()
        .filter_map(|item| match item {
            RenderItem::Image(d) => Some(d.source.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn static_src_paints_one_image() {
    let src = "app A {\n  title: \"A\"\n  size: (300, 300)\n  window M {\n    Canvas(width: fill(1), height: px(300)) {\n      Image2d(src: \"cover.jpg\", x: 0, y: 0, w: 300, h: 300)\n    }\n  }\n}\n";
    let loaded = SeqLoader::load_str("Main.seq", src).unwrap();
    let h = Harness::new(loaded.view);
    let stats = h.paint_stats(300.0, 300.0);
    assert_eq!(stats.images, 1, "jeden Image2d to jeden obrazek");
    assert_eq!(
        image_sources(&h),
        vec![ImageSource::File("cover.jpg".to_string())],
        "path goes to RenderList"
    );
}

#[test]
fn rect_and_uv_forms_paint() {
    let src = "app A {\n  title: \"A\"\n  size: (300, 300)\n  window M {\n    Canvas(width: fill(1), height: px(300)) {\n      Image2d(src: \"cover.jpg\", rect: (10, 20, 100, 80), uv: (0, 0, 1, 1))\n    }\n  }\n}\n";
    let loaded = SeqLoader::load_str("Main.seq", src).unwrap();
    let h = Harness::new(loaded.view);
    let stats = h.paint_stats(300.0, 300.0);
    assert_eq!(stats.images, 1, "form rect:/uv: also paints");
}

#[test]
fn dynamic_src_follows_host_signal() {
    let src = "app A {\n  title: \"A\"\n  size: (300, 300)\n  prop cover: String\n  window M {\n    Canvas(width: fill(1), height: px(300)) {\n      Image2d(src: dynamic(cover), x: 0, y: 0, w: 300, h: 300)\n    }\n  }\n}\n";
    let cover = Signal::new("a.png".to_string());
    let b = HostBindings::new().prop_str("cover", cover.clone());
    let loaded = SeqLoader::load_str_with("Main.seq", src, &b).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(300.0, 300.0);
    assert_eq!(
        image_sources(&h),
        vec![ImageSource::File("a.png".to_string())],
        "initial src from signal"
    );
    cover.set("b.png".to_string());
    h.tick();
    assert_eq!(
        image_sources(&h),
        vec![ImageSource::File("b.png".to_string())],
        "signal change reloads src (track subscribes)"
    );
}

#[test]
fn hidden_image_paints_nothing() {
    let src = "app A {\n  title: \"A\"\n  size: (300, 300)\n  window M {\n    Canvas(width: fill(1), height: px(300)) {\n      Image2d(src: \"cover.jpg\", x: 0, y: 0, w: 300, h: 300, visible: false)\n    }\n  }\n}\n";
    let loaded = SeqLoader::load_str("Main.seq", src).unwrap();
    let h = Harness::new(loaded.view);
    let stats = h.paint_stats(300.0, 300.0);
    assert_eq!(stats.images, 0, "visible: false nic nie maluje");
}

#[test]
fn named_image_is_pickable() {
    let src = "app A {\n  title: \"A\"\n  size: (300, 300)\n  on pick(x: Float, y: Float, name: String)\n  window M {\n    Canvas(on_click_at: { pick(x, y, name) }) {\n      Image2d(src: \"cover.jpg\", x: 40, y: 40, w: 120, h: 80, name: \"art\")\n    }\n  }\n}\n";
    let picked = Rc::new(Cell::new(String::new()));
    let p = picked.clone();
    let b = HostBindings::new().on_point_named("pick", move |_x, _y, name| p.set(name));
    let loaded = SeqLoader::load_str_with("P.seq", src, &b).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(300.0, 300.0);
    h.paint();
    h.tick();
    h.click(100.0, 80.0);
    h.tick();
    assert_eq!(
        picked.take(),
        "art",
        "click on the image rectangle gives the name"
    );
    h.click(10.0, 10.0);
    h.tick();
    assert_eq!(picked.take(), "", "a click outside the image is a miss");
}

#[test]
fn missing_src_is_rejected() {
    let src = "app A {\n  title: \"A\"\n  size: (10, 10)\n  window M {\n    Canvas {\n      Image2d(x: 0, y: 0, w: 10, h: 10)\n    }\n  }\n}\n";
    let err = match SeqLoader::load_str("Main.seq", src) {
        Err(e) => e,
        Ok(_) => panic!("oczekiwano bledu"),
    };
    assert!(err.message.contains("src"), "{err}");
}

#[test]
fn image_outside_canvas_is_rejected() {
    let src = "app A {\n  title: \"A\"\n  size: (10, 10)\n  window M {\n    Image2d(src: \"cover.jpg\", x: 0, y: 0, w: 10, h: 10)\n  }\n}\n";
    let err = match SeqLoader::load_str("Main.seq", src) {
        Err(e) => e,
        Ok(_) => panic!("oczekiwano bledu"),
    };
    assert!(err.message.contains("Canvas"), "{err}");
}

#[test]
fn bad_uv_is_rejected() {
    let src = "app A {\n  title: \"A\"\n  size: (10, 10)\n  window M {\n    Canvas {\n      Image2d(src: \"cover.jpg\", x: 0, y: 0, w: 10, h: 10, uv: (0, 0, 1))\n    }\n  }\n}\n";
    let err = match SeqLoader::load_str("Main.seq", src) {
        Err(e) => e,
        Ok(_) => panic!("oczekiwano bledu"),
    };
    assert!(err.message.contains("uv"), "{err}");
}
