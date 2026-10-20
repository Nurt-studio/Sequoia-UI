#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::HashSet;

use sequoia_reactive::Signal;
use sequoia_seq::{HostBindings, SeqLoader};
use sequoia_test::Harness;

const SRC: &str = "app S {\n  title: \"S\"\n  size: (400, 400)\n  prop out: Int\n  prop vol: Float\n  prop note: String\n  prop loud: Bool\n  prop rebuild: Bool\n\n  state counter: Int = 0\n  state volume: Float = 0.5\n  state title: String = \"init\"\n  state armed: Bool = false\n\n  fn inc() { counter = counter + 1; out = counter; }\n  fn echo() { vol = volume; note = title; loud = armed; }\n  fn remember(s: String) { title = s; note = title; }\n  fn arm() { armed = true; loud = armed; }\n\n  window M {\n    Column(gap: 0) {\n      Button(\"inc\", height: px(36), on_click: { inc() })\n      Button(\"echo\", height: px(36), on_click: { echo() })\n      Button(\"remember\", height: px(36), on_click: { remember(\"first\") })\n      Button(\"arm\", height: px(36), on_click: { arm() })\n      If(when: rebuild) {\n        Text(\"rebuilt\")\n      }\n    }\n  }\n}\n";

struct Fixture {
    h: Harness,
    out: Signal<i64>,
    vol: Signal<f64>,
    note: Signal<String>,
    loud: Signal<bool>,
    rebuild: Signal<bool>,
}

fn fixture() -> Fixture {
    let out = Signal::new(0);
    let vol = Signal::new(0.0);
    let note = Signal::new(String::new());
    let loud = Signal::new(false);
    let rebuild = Signal::new(false);
    let b = HostBindings::new()
        .prop_int("out", out.clone())
        .prop_float("vol", vol.clone())
        .prop_str("note", note.clone())
        .prop_bool("loud", loud.clone())
        .prop_bool("rebuild", rebuild.clone());
    let loaded = SeqLoader::load_str_with("State.seq", SRC, &b).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(400.0, 400.0);
    h.paint();
    h.tick();
    Fixture {
        h,
        out,
        vol,
        note,
        loud,
        rebuild,
    }
}

fn click_button(h: &Harness, i: usize) {
    let root = h.root().unwrap();
    let col = h.children(root)[0];
    let btn = h.children(col)[i];
    let r = h.rect(btn).unwrap();
    h.click(r.left() + r.size.width * 0.5, r.top() + r.size.height * 0.5);
    h.tick();
}

fn visible_texts(h: &Harness) -> HashSet<String> {
    fn rec(h: &Harness, id: sequoia_view::ElementId, out: &mut HashSet<String>) {
        if let Some(t) = h.text(id) {
            out.insert(t);
        }
        for c in h.children(id) {
            rec(h, c, out);
        }
    }
    let mut out = HashSet::new();
    if let Some(root) = h.root() {
        rec(h, root, &mut out);
    }
    out
}

#[test]
fn state_accumulates_across_calls() {
    let f = fixture();
    click_button(&f.h, 0);
    assert_eq!(f.out.get_untracked(), 1, "first click");
    click_button(&f.h, 0);
    assert_eq!(f.out.get_untracked(), 2, "state remembers previous call");
}

#[test]
fn state_init_literals_are_readable() {
    let f = fixture();
    click_button(&f.h, 1);
    assert_eq!(f.vol.get_untracked(), 0.5, "state Float = 0.5");
    assert_eq!(f.note.get_untracked(), "init", "state String = \"init\"");
    assert!(!f.loud.get_untracked(), "state Bool = false");
}

#[test]
fn string_and_bool_state_writes() {
    let f = fixture();
    click_button(&f.h, 2);
    assert_eq!(f.note.get_untracked(), "first");
    click_button(&f.h, 3);
    assert!(f.loud.get_untracked(), "state Bool ← true");
    click_button(&f.h, 1);
    assert_eq!(f.note.get_untracked(), "first", "echo czyta nowy stan");
    assert!(f.loud.get_untracked(), "echo czyta Bool");
}

#[test]
fn state_survives_view_rebuild() {
    let f = fixture();
    click_button(&f.h, 0);
    assert_eq!(f.out.get_untracked(), 1);
    f.rebuild.set(true);
    f.h.tick();
    f.h.layout(400.0, 400.0);
    assert!(
        visible_texts(&f.h).contains("rebuilt"),
        "If He Rebuilt the Tree"
    );
    click_button(&f.h, 0);
    assert_eq!(
        f.out.get_untracked(),
        2,
        "the state has undergone reconstruction"
    );
}

#[test]
fn state_init_must_match_declared_type() {
    let src = "app A {\n  title: \"A\"\n  size: (10, 10)\n  state n: Int = 1.5\n  window M {\n    Text(\"x\")\n  }\n}\n";
    let err = match SeqLoader::load_str("Main.seq", src) {
        Err(e) => e,
        Ok(_) => panic!("oczekiwano bledu"),
    };
    assert!(err.message.contains("initial value"), "{err}");
    assert_eq!(err.line, 4, "linia state: {err}");
}

#[test]
fn state_init_must_be_literal() {
    let src = "app A {\n  title: \"A\"\n  size: (10, 10)\n  state n: Int = other\n  window M {\n    Text(\"x\")\n  }\n}\n";
    let err = match SeqLoader::load_str("Main.seq", src) {
        Err(e) => e,
        Ok(_) => panic!("oczekiwano bledu"),
    };
    assert!(err.message.contains("literal"), "{err}");
}
