#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::cell::Cell;
use std::rc::Rc;

use sequoia_reactive::Signal;
use sequoia_seq::{HostBindings, SeqLoader};
use sequoia_test::Harness;

#[test]
fn canvas_click_name_reaches_fn_1arg() {
    let src = "app W {\n  title: \"W\"\n  size: (300, 200)\n  prop note: String\n\n  fn pick(name: String) { note = name; }\n\n  window M {\n    Canvas(on_click_at: { pick(name) }) {\n      Rect(x: 40, y: 40, w: 120, h: 80, fill: #FFFFFF, name: \"a\")\n    }\n  }\n}\n";
    let note = Signal::new(String::new());
    let b = HostBindings::new().prop_str("note", note.clone());
    let loaded = SeqLoader::load_str_with("W.seq", src, &b).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(300.0, 200.0);
    h.paint();
    h.tick();
    h.click(100.0, 80.0);
    h.tick();
    assert_eq!(
        note.get_untracked(),
        "a",
        "the hit carries the name of the shape"
    );
    h.click(10.0, 10.0);
    h.tick();
    assert_eq!(note.get_untracked(), "", "the box carries an empty name");
}

#[test]
fn canvas_click_xy_reaches_fn_2arg() {
    let src = "app W {\n  title: \"W\"\n  size: (300, 200)\n  prop px: Float\n  prop py: Float\n\n  fn hit(x: Float, y: Float) { px = x; py = y; }\n\n  window M {\n    Canvas(on_click_at: { hit(x, y) }) {\n      Rect(x: 40, y: 40, w: 120, h: 80, fill: #FFFFFF, name: \"a\")\n    }\n  }\n}\n";
    let px = Signal::new(-1.0);
    let py = Signal::new(-1.0);
    let b = HostBindings::new()
        .prop_float("px", px.clone())
        .prop_float("py", py.clone());
    let loaded = SeqLoader::load_str_with("W.seq", src, &b).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(300.0, 200.0);
    h.paint();
    h.tick();
    h.click(100.0, 80.0);
    h.tick();
    assert_eq!(px.get_untracked(), 100.0);
    assert_eq!(py.get_untracked(), 80.0);
}

#[test]
fn canvas_click_3arg_reaches_fn() {
    let src = "app W {\n  title: \"W\"\n  size: (300, 200)\n  prop px: Float\n  prop note: String\n\n  fn full(x: Float, y: Float, name: String) { px = x; note = name; }\n\n  window M {\n    Canvas(on_click_at: { full(x, y, name) }) {\n      Rect(x: 40, y: 40, w: 120, h: 80, fill: #FFFFFF, name: \"a\")\n    }\n  }\n}\n";
    let px = Signal::new(-1.0);
    let note = Signal::new(String::new());
    let b = HostBindings::new()
        .prop_float("px", px.clone())
        .prop_str("note", note.clone());
    let loaded = SeqLoader::load_str_with("W.seq", src, &b).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(300.0, 200.0);
    h.paint();
    h.tick();
    h.click(100.0, 80.0);
    h.tick();
    assert_eq!(px.get_untracked(), 100.0);
    assert_eq!(note.get_untracked(), "a");
}

#[test]
fn canvas_wheel_reaches_fn_3arg() {
    let src = "app W {\n  title: \"W\"\n  size: (300, 200)\n  prop delta: Float\n\n  fn scroll(x: Float, y: Float, d: Float) { delta = d; }\n\n  window M {\n    Canvas(on_wheel: { scroll(x, y, d) }) {\n      Rect(x: 40, y: 40, w: 120, h: 80, fill: #FFFFFF)\n    }\n  }\n}\n";
    let delta = Signal::new(0.0);
    let b = HostBindings::new().prop_float("delta", delta.clone());
    let loaded = SeqLoader::load_str_with("W.seq", src, &b).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(300.0, 200.0);
    h.paint();
    h.tick();
    h.wheel(150.0, 100.0, -2.5);
    h.tick();
    assert_eq!(delta.get_untracked(), -2.5, "circle delta goes to fn");
}

#[test]
fn fn_shadows_host_callback_of_same_name() {
    let src = "app W {\n  title: \"W\"\n  size: (300, 200)\n  prop note: String\n  on poke(x: Float, y: Float)\n\n  fn poke(x: Float, y: Float) { note = \"fn\"; }\n\n  window M {\n    Canvas(on_click_at: { poke(x, y) }) {\n      Rect(x: 40, y: 40, w: 120, h: 80, fill: #FFFFFF)\n    }\n  }\n}\n";
    let note = Signal::new(String::new());
    let host_calls = Rc::new(Cell::new(0u32));
    let hc = host_calls.clone();
    let b = HostBindings::new()
        .prop_str("note", note.clone())
        .on_point("poke", move |_x, _y| hc.set(hc.get() + 1));
    let loaded = SeqLoader::load_str_with("W.seq", src, &b).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(300.0, 200.0);
    h.paint();
    h.tick();
    h.click(100.0, 80.0);
    h.tick();
    assert_eq!(note.get_untracked(), "fn", "fn takes priority");
    assert_eq!(host_calls.get(), 0, "callback host is NOT being called");
}

#[test]
fn checkbox_live_toggle_reaches_fn() {
    let src = "app W {\n  title: \"W\"\n  size: (400, 300)\n  prop flag: Bool\n  prop loud: Bool\n\n  fn arm(v: Bool) { loud = v; }\n\n  window M {\n    Checkbox(checked: flag, label: \"Zgoda\", on_toggle: { arm(value) })\n  }\n}\n";
    let flag = Signal::new(false);
    let loud = Signal::new(false);
    let b = HostBindings::new()
        .prop_bool("flag", flag.clone())
        .prop_bool("loud", loud.clone());
    let loaded = SeqLoader::load_str_with("W.seq", src, &b).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(400.0, 300.0);
    h.paint();
    h.tick();
    let root = h.root().unwrap();
    let el = h.children(root)[0];
    let r = h.rect(el).unwrap();
    h.click(r.left() + 10.0, r.top() + 10.0);
    h.tick();
    assert!(flag.get_untracked(), "klik toggluje prop");
    assert!(loud.get_untracked(), "fn got value=true");
    h.click(r.left() + 10.0, r.top() + 10.0);
    h.tick();
    assert!(!loud.get_untracked(), "drugi klik niesie value=false");
}

#[test]
fn effect_on_run_fn_fires_on_track_change_not_mount() {
    let src = "app W {\n  title: \"W\"\n  size: (300, 200)\n  prop n: Int\n  prop runs: Int\n\n  fn ran() { runs = runs + 1; }\n\n  window M {\n    Column {\n      Text(\"x\")\n      Effect(track: n, on_run: { ran() })\n    }\n  }\n}\n";
    let n = Signal::new(1i64);
    let runs = Signal::new(0i64);
    let b = HostBindings::new()
        .prop_int("n", n.clone())
        .prop_int("runs", runs.clone());
    let loaded = SeqLoader::load_str_with("W.seq", src, &b).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(300.0, 200.0);
    h.tick();
    assert_eq!(runs.get_untracked(), 0, "montowanie nie odpala");
    n.set(2);
    h.tick();
    assert_eq!(runs.get_untracked(), 1, "zmiana track odpala fn");
}
