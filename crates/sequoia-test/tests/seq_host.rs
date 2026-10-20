#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::cell::Cell;
use std::rc::Rc;

use sequoia_reactive::Signal;
use sequoia_seq::{HostBindings, SeqLoader};
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

#[test]
fn dynamic_text_subscribes_to_host() {
    let src = "app A {\n  title: \"A\"\n  size: (200, 100)\n  prop label: String\n  window M {\n    Text(dynamic: label)\n  }\n}\n";
    let label = Signal::new("A".to_string());
    let b = HostBindings::new().prop_str("label", label.clone());
    let loaded = SeqLoader::load_str_with("Main.seq", src, &b).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(200.0, 100.0);
    assert_eq!(glyphs(&h), 1);
    label.set("ABCD".to_string());
    h.layout(200.0, 100.0);
    h.tick();
    assert_eq!(glyphs(&h), 4, "Text::with reloaded after the set");
}

#[test]
fn on_click_fires_host_action() {
    let src = "app A {\n  title: \"A\"\n  size: (200, 100)\n  on add()\n  window M {\n    Button(\"Go\", on_click: { add() })\n  }\n}\n";
    let fired = Rc::new(Cell::new(0u32));
    let f = fired.clone();
    let b = HostBindings::new().on("add", move || f.set(f.get() + 1));
    let loaded = SeqLoader::load_str_with("Main.seq", src, &b).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(200.0, 100.0);
    h.click(10.0, 10.0);
    h.tick();
    assert_eq!(fired.get(), 1);
}

#[test]
fn on_int_gets_literal_arg() {
    let src = "app A {\n  title: \"A\"\n  size: (200, 100)\n  on pick(id: Int)\n  window M {\n    Button(\"Go\", on_click: { pick(7) })\n  }\n}\n";
    let got = Rc::new(Cell::new(0i64));
    let g = got.clone();
    let b = HostBindings::new().on_int("pick", move |id| g.set(id));
    let loaded = SeqLoader::load_str_with("Main.seq", src, &b).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(200.0, 100.0);
    h.click(10.0, 10.0);
    h.tick();
    assert_eq!(got.get(), 7);
}

#[test]
fn prop_type_mismatch_reports_decl_line() {
    let src = "app A {\n  title: \"A\"\n  size: (200, 100)\n  prop n: Int\n  window M {\n    Text(\"x\")\n  }\n}\n";
    let b = HostBindings::new().prop_str("n", Signal::new("x".to_string()));
    let err = match SeqLoader::load_str_with("Main.seq", src, &b) {
        Err(e) => e,
        Ok(_) => panic!("oczekiwano bledu"),
    };
    assert!(
        err.message.contains("Int") && err.message.contains("String"),
        "{err}"
    );
    assert_eq!(err.line, 4);
}

#[test]
fn on_signature_mismatch_reports_decl_line() {
    let src = "app A {\n  title: \"A\"\n  size: (200, 100)\n  on pick(id: Int)\n  window M {\n    Text(\"x\")\n  }\n}\n";
    let b = HostBindings::new().on("pick", || {});
    let err = match SeqLoader::load_str_with("Main.seq", src, &b) {
        Err(e) => e,
        Ok(_) => panic!("oczekiwano bledu"),
    };
    assert!(err.message.contains("sygnatura"), "{err}");
    assert_eq!(err.line, 4);
}

#[test]
fn unknown_prop_reports_use_line() {
    let src = "app A {\n  title: \"A\"\n  size: (200, 100)\n  window M {\n    Text(dynamic: nope)\n  }\n}\n";
    let b = HostBindings::new();
    let err = match SeqLoader::load_str_with("Main.seq", src, &b) {
        Err(e) => e,
        Ok(_) => panic!("oczekiwano bledu"),
    };
    assert!(err.message.contains("nope"), "{err}");
    assert_eq!(err.line, 5);
}

#[test]
fn plain_position_reads_prop_snapshot() {
    let src = "app A {\n  title: \"A\"\n  size: (200, 100)\n  prop s: Float\n  window M {\n    Text(\"x\", size: s)\n  }\n}\n";
    let b = HostBindings::new().prop_float("s", Signal::new(20.0));
    let loaded = SeqLoader::load_str_with("Main.seq", src, &b).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(200.0, 100.0);
    assert!(!h.paint().is_empty());
}

#[test]
fn unknown_callback_reports_call_line() {
    let src = "app A {\n  title: \"A\"\n  size: (200, 100)\n  window M {\n    Button(\"Go\", on_click: { nope() })\n  }\n}\n";
    let b = HostBindings::new();
    let err = match SeqLoader::load_str_with("Main.seq", src, &b) {
        Err(e) => e,
        Ok(_) => panic!("oczekiwano bledu"),
    };
    assert!(err.message.contains("nope"), "{err}");
    assert_eq!(err.line, 5);
}
