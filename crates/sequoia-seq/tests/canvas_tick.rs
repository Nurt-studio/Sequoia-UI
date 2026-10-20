#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_core::{Point, RenderItem};
use sequoia_reactive::Signal;
use sequoia_seq::{HostBindings, SeqLoader};
use sequoia_test::Harness;

const PIVOT: (f32, f32) = (150.0, 150.0);
const RADIUS: f32 = 120.0;

fn marker(h: &Harness) -> Point {
    let list = h.paint();
    for (i, item) in list.items().iter().enumerate() {
        if let RenderItem::Quad(q) = item {
            let c = Point::new(q.rect.left() + 10.0, q.rect.top() + 10.0);
            return list.item_transform(i).apply(c);
        }
    }
    panic!("oczekiwano markera");
}

fn dist(a: Point, b: (f32, f32)) -> f32 {
    ((a.x - b.0).powi(2) + (a.y - b.1).powi(2)).sqrt()
}

fn tick_app(running: &str) -> String {
    format!(
        "app B {{\n  title: \"B\"\n  size: (300, 300)\n  prop play: Bool\n  window M {{\n    Canvas(width: fill(1), height: fill(1), tick: a, speed: 360, running: {running}) {{\n      Group(pivot: (150, 150), rotate: dynamic(a)) {{\n        Rect(x: 140, y: 20, w: 20, h: 20, fill: #FFFFFF)\n      }}\n    }}\n  }}\n}}\n"
    )
}

fn harness(running_src: &str, playing: bool) -> (Harness, Signal<bool>) {
    let play = Signal::new(playing);
    let b = HostBindings::new().prop_bool("play", play.clone());
    let loaded = SeqLoader::load_str_with("P.seq", &tick_app(running_src), &b).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(300.0, 300.0);
    (h, play)
}

#[test]
fn tick_spins_while_running() {
    let (h, _play) = harness("dynamic(play)", true);
    let c0 = marker(&h);
    assert!(
        (dist(c0, PIVOT) - RADIUS).abs() < 3.0,
        "start nad pivotem: {c0:?}"
    );
    std::thread::sleep(std::time::Duration::from_millis(200));
    let c1 = marker(&h);

    assert!(
        dist(c0, (c1.x, c1.y)) > 5.0,
        "marker jedzie: {c0:?} -> {c1:?}"
    );
    assert!(
        (dist(c1, PIVOT) - RADIUS).abs() < 3.0,
        "in a circle: {c1:?}"
    );
}

#[test]
fn tick_paused_stands_still() {
    let (h, _play) = harness("false", false);
    let c0 = marker(&h);
    std::thread::sleep(std::time::Duration::from_millis(150));
    let c1 = marker(&h);
    assert_eq!((c0.x, c0.y), (c1.x, c1.y), "pauza stoi w miejscu");
}

#[test]
fn tick_toggle_resumes_without_jump() {
    let (h, play) = harness("dynamic(play)", true);
    let _ = marker(&h);
    std::thread::sleep(std::time::Duration::from_millis(150));
    let c1 = marker(&h);
    play.set(false);
    h.tick();
    let c2 = marker(&h);
    std::thread::sleep(std::time::Duration::from_millis(100));
    let c3 = marker(&h);
    assert_eq!((c2.x, c2.y), (c3.x, c3.y), "po pauzie stoi");

    play.set(true);
    h.tick();
    let c4 = marker(&h);
    assert!(
        dist(c3, (c4.x, c4.y)) < 1.0,
        "wznowienie bez skoku: {c3:?} -> {c4:?}"
    );
    std::thread::sleep(std::time::Duration::from_millis(150));
    let c5 = marker(&h);
    assert!(
        dist(c4, (c5.x, c5.y)) > 5.0,
        "a potem znowu jedzie: {c4:?} -> {c5:?}"
    );
    let _ = c1;
}

#[test]
fn tick_requires_host() {
    let src = "app B {\n  title: \"B\"\n  size: (10, 10)\n  window M {\n    Canvas(tick: a) {\n      Rect(x: 0, y: 0, w: 10, h: 10, fill: #FFFFFF)\n    }\n  }\n}\n";
    let err = match SeqLoader::load_str("P.seq", src) {
        Err(e) => e,
        Ok(_) => panic!("oczekiwano bledu"),
    };
    assert!(err.message.contains("tick"), "{err}");
}

#[test]
fn running_without_tick_is_rejected() {
    let src = "app B {\n  title: \"B\"\n  size: (10, 10)\n  prop play: Bool\n  window M {\n    Canvas(running: dynamic(play)) {\n      Rect(x: 0, y: 0, w: 10, h: 10, fill: #FFFFFF)\n    }\n  }\n}\n";
    let b = HostBindings::new().prop_bool("play", Signal::new(true));
    let err = match SeqLoader::load_str_with("P.seq", src, &b) {
        Err(e) => e,
        Ok(_) => panic!("oczekiwano bledu"),
    };
    assert!(err.message.contains("tick"), "{err}");
}

#[test]
fn unknown_tick_name_is_rejected() {
    let src = "app B {\n  title: \"B\"\n  size: (10, 10)\n  window M {\n    Canvas(tick: a) {\n      Rect(x: dynamic(zzz), y: 0, w: 10, h: 10, fill: #FFFFFF)\n    }\n  }\n}\n";
    let b = HostBindings::new();
    let err = match SeqLoader::load_str_with("P.seq", src, &b) {
        Err(e) => e,
        Ok(_) => panic!("oczekiwano bledu"),
    };
    assert!(err.message.contains("zzz"), "{err}");
}
