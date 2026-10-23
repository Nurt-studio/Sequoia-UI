#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::cell::Cell;
use std::rc::Rc;

use sequoia_async::Resource;
use sequoia_reactive::{I18n, Memo, Signal, memo};
use sequoia_seq::{HostBindings, SeqLoader, check, parse};
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
fn if_static_true_renders() {
    let src = "app A {\n  title: \"A\"\n  size: (200, 100)\n  window M {\n    If(when: true) {\n      Text(\"widoczne\")\n    }\n  }\n}\n";
    let loaded = SeqLoader::load_str("Main.seq", src).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(200.0, 100.0);
    assert_eq!(glyphs(&h), 8);
}

#[test]
fn if_static_false_renders_nothing() {
    let src = "app A {\n  title: \"A\"\n  size: (200, 100)\n  window M {\n    If(when: false) {\n      Text(\"widoczne\")\n    }\n  }\n}\n";
    let loaded = SeqLoader::load_str("Main.seq", src).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(200.0, 100.0);
    assert_eq!(glyphs(&h), 0);
}

#[test]
fn if_toggles_with_host_bool() {
    let src = "app A {\n  title: \"A\"\n  size: (200, 100)\n  prop show: Bool\n  window M {\n    If(when: show) {\n      Text(\"widoczne\")\n    }\n  }\n}\n";
    let show = Signal::new(true);
    let b = HostBindings::new().prop_bool("show", show.clone());
    let loaded = SeqLoader::load_str_with("Main.seq", src, &b).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(200.0, 100.0);
    h.paint();
    h.tick();
    assert_eq!(glyphs(&h), 8);
    show.set(false);

    for _ in 0..30 {
        std::thread::sleep(std::time::Duration::from_millis(10));
        h.tick();
        h.layout(200.0, 100.0);
        h.paint();
    }
    h.tick();
    h.layout(200.0, 100.0);
    assert_eq!(glyphs(&h), 0);
}

#[test]
fn if_requires_when() {
    let src = "app A {\n  title: \"A\"\n  size: (200, 100)\n  window M {\n    If {\n      Text(\"x\")\n    }\n  }\n}\n";
    let app = parse("Main.seq", src).unwrap();
    check("Main.seq", &app).unwrap();
    let err = match SeqLoader::load_str("Main.seq", src) {
        Err(e) => e,
        Ok(_) => panic!("oczekiwano bledu"),
    };
    assert!(err.message.contains("when"), "{err}");
}

#[test]
fn memo_str_feeds_dynamic_text() {
    let src = "app A {\n  title: \"A\"\n  size: (200, 100)\n  prop label: String\n  window M {\n    Text(dynamic: label)\n  }\n}\n";
    let base = Signal::new("ab".to_string());
    let b2 = base.clone();
    let upper: Memo<String> = memo(move || b2.get().to_uppercase());
    let b = HostBindings::new().memo_str("label", upper);
    let loaded = SeqLoader::load_str_with("Main.seq", src, &b).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(200.0, 100.0);
    assert_eq!(glyphs(&h), 2);
    base.set("abcd".to_string());
    h.layout(200.0, 100.0);
    h.tick();
    assert_eq!(glyphs(&h), 4);
}

#[test]
fn memo_bool_drives_if() {
    let src = "app A {\n  title: \"A\"\n  size: (200, 100)\n  prop show: Bool\n  window M {\n    If(when: show) {\n      Text(\"tak\")\n    }\n  }\n}\n";
    let flag = Signal::new(true);
    let f2 = flag.clone();
    let derived: Memo<bool> = memo(move || f2.get());
    let b = HostBindings::new().memo_bool("show", derived);
    let loaded = SeqLoader::load_str_with("Main.seq", src, &b).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(200.0, 100.0);
    h.paint();
    h.tick();
    assert_eq!(glyphs(&h), 3);
}

#[test]
fn env_value_in_text_and_number() {
    let src = "app A {\n  title: \"A\"\n  size: (200, 100)\n  window M {\n    Text(text: env.tytul, size: env.rozmiar)\n  }\n}\n";
    let b = HostBindings::new()
        .env_str("tytul", Signal::new("hej".to_string()))
        .env_float("rozmiar", Signal::new(14.0));
    let loaded = SeqLoader::load_str_with("Main.seq", src, &b).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(200.0, 100.0);
    assert_eq!(glyphs(&h), 3);
}

#[test]
fn env_dynamic_subscribes() {
    let src = "app A {\n  title: \"A\"\n  size: (200, 100)\n  window M {\n    Text(dynamic: env.etykieta)\n  }\n}\n";

    let label = Signal::new("A".to_string());
    let b = HostBindings::new().env_str("etykieta", label.clone());
    let loaded = SeqLoader::load_str_with("Main.seq", src, &b).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(200.0, 100.0);
    assert_eq!(glyphs(&h), 1);
    label.set("ABC".to_string());
    h.layout(200.0, 100.0);
    h.tick();
    assert_eq!(glyphs(&h), 3);
}

#[test]
fn t_key_follows_locale() {
    let src = "app A {\n  title: \"A\"\n  size: (200, 100)\n  window M {\n    Text(t: \"save\")\n  }\n}\n";
    let i18n = I18n::new("en");
    i18n.add("en", "save", "Save");
    i18n.add("pl", "save", "Zapisz");
    let b = HostBindings::new().i18n(i18n.clone());
    let loaded = SeqLoader::load_str_with("Main.seq", src, &b).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(200.0, 100.0);
    assert_eq!(glyphs(&h), 4);
    i18n.set_locale("pl");
    h.layout(200.0, 100.0);
    h.tick();
    assert_eq!(glyphs(&h), 6);
}

#[test]
fn t_without_i18n_binding_is_rejected() {
    let src = "app A {\n  title: \"A\"\n  size: (200, 100)\n  window M {\n    Text(t: \"save\")\n  }\n}\n";
    let b = HostBindings::new();
    let err = match SeqLoader::load_str_with("Main.seq", src, &b) {
        Err(e) => e,
        Ok(_) => panic!("oczekiwano bledu"),
    };
    assert!(err.message.contains("i18n"), "{err}");
}

#[test]
fn async_loading_then_data() {
    let src = "app A {\n  title: \"A\"\n  size: (300, 200)\n  window M {\n    Async(source: users) {\n      Loading {\n        Text(\"laduje\")\n      }\n      Data {\n        Text(dynamic: data)\n      }\n      Error {\n        Text(dynamic: error)\n      }\n    }\n  }\n}\n";
    let res: Resource<String> = Resource::new();
    let b = HostBindings::new().resource("users", res.clone());
    let loaded = SeqLoader::load_str_with("Main.seq", src, &b).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(300.0, 200.0);
    h.tick();
    assert_eq!(glyphs(&h), 6, "Idle pokazuje Loading");
    let generation = res.begin_loading();
    h.layout(300.0, 200.0);
    h.tick();
    assert_eq!(glyphs(&h), 6, "Loading w locie");
    res.complete(generation, "gotowe!".to_string());
    h.tick();
    h.layout(300.0, 200.0);
    assert_eq!(glyphs(&h), 7, "Data po complete");
}

#[test]
fn async_error_branch() {
    let src = "app A {\n  title: \"A\"\n  size: (300, 200)\n  window M {\n    Async(source: users) {\n      Loading {\n        Text(\"laduje\")\n      }\n      Data {\n        Text(dynamic: data)\n      }\n      Error {\n        Text(dynamic: error)\n      }\n    }\n  }\n}\n";
    let res: Resource<String> = Resource::new();
    let b = HostBindings::new().resource("users", res.clone());
    let loaded = SeqLoader::load_str_with("Main.seq", src, &b).unwrap();
    let h = Harness::new(loaded.view);
    let generation = res.begin_loading();
    res.fail(generation, "bum".to_string());
    h.layout(300.0, 200.0);
    h.tick();
    assert_eq!(glyphs(&h), 3, "Error po fail");
}

#[test]
fn async_unknown_source_reports_line() {
    let src = "app A {\n  title: \"A\"\n  size: (300, 200)\n  window M {\n    Async(source: nope) {\n      Loading {\n        Text(\"x\")\n      }\n    }\n  }\n}\n";
    let b = HostBindings::new();
    let err = match SeqLoader::load_str_with("Main.seq", src, &b) {
        Err(e) => e,
        Ok(_) => panic!("oczekiwano bledu"),
    };
    assert!(err.message.contains("nope"), "{err}");
    assert_eq!(err.line, 5);
}

#[test]
fn effect_fires_on_track_change_not_on_mount() {
    let src = "app A {\n  title: \"A\"\n  size: (200, 100)\n  prop n: Int\n  on changed()\n  window M {\n    Column {\n      Text(\"x\")\n      Effect(track: n, on_run: { changed() })\n    }\n  }\n}\n";
    let n = Signal::new(1i64);
    let fired = Rc::new(Cell::new(0u32));
    let f = fired.clone();
    let b = HostBindings::new()
        .prop_int("n", n.clone())
        .on("changed", move || f.set(f.get() + 1));
    let loaded = SeqLoader::load_str_with("Main.seq", src, &b).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(200.0, 100.0);
    h.tick();
    assert_eq!(fired.get(), 0, "montowanie nie odpala");
    n.set(2);
    h.tick();
    assert_eq!(fired.get(), 1, "zmiana track odpala");
}
