#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::cell::Cell;
use std::collections::HashSet;
use std::rc::Rc;

use sequoia_reactive::Signal;
use sequoia_seq::{FieldSignal, HostBindings, SeqLoader, SeqRow, check, parse};
use sequoia_test::Harness;

#[derive(Clone)]
struct Song {
    id: i64,
    title: Signal<String>,
}

impl PartialEq for Song {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl SeqRow for Song {
    fn id(&self) -> i64 {
        self.id
    }

    fn fields(&self) -> Vec<(String, FieldSignal)> {
        vec![("title".to_string(), FieldSignal::Str(self.title.clone()))]
    }
}

fn songs(n: i64) -> Signal<Vec<Song>> {
    Signal::new(
        (0..n)
            .map(|i| Song {
                id: i,
                title: Signal::new(format!("R{i:02}")),
            })
            .collect(),
    )
}

fn visible_titles(h: &Harness) -> HashSet<String> {
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

const SRC: &str = "app A {\n  title: \"A\"\n  size: (400, 200)\n  on play(id: Int)\n  window M {\n    VirtualList(items: songs, key: id, item_height: 48, overscan: 2) {\n      Row(width: fill(1), height: px(48), on_click: { play(item.id) }) {\n        Text(dynamic: item.title)\n      }\n    }\n  }\n}\n";

#[test]
fn renders_windowed_subset() {
    let rows = songs(20);
    let b = HostBindings::new()
        .rows("songs", rows)
        .on_int("play", |_| {});
    let loaded = SeqLoader::load_str_with("Main.seq", SRC, &b).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(400.0, 200.0);
    let shown = visible_titles(&h);
    assert!(!shown.is_empty(), "cos widac");
    assert!(shown.contains("R00"), "poczatek listy: {shown:?}");
    assert!(!shown.contains("R19"), "koniec poza oknem: {shown:?}");
    assert!(shown.len() < 20, "nie wszystko: {}", shown.len());
}

#[test]
fn scroll_moves_window() {
    let rows = songs(20);
    let b = HostBindings::new()
        .rows("songs", rows)
        .on_int("play", |_| {});
    let loaded = SeqLoader::load_str_with("Main.seq", SRC, &b).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(400.0, 200.0);
    let before = visible_titles(&h);
    assert!(before.contains("R00"));

    for _ in 0..6 {
        h.wheel(200.0, 100.0, -400.0);
        h.tick();
    }
    h.layout(400.0, 200.0);
    let after = visible_titles(&h);
    assert_ne!(before, after, "okno sie przesunelo");
    assert!(!after.contains("R00"), "poczatek za nami: {after:?}");
}

#[test]
fn click_visible_row_carries_id() {
    let rows = songs(20);
    let got = Rc::new(Cell::new(-1i64));
    let g = got.clone();
    let b = HostBindings::new()
        .rows("songs", rows)
        .on_int("play", move |id| g.set(id));
    let loaded = SeqLoader::load_str_with("Main.seq", SRC, &b).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(400.0, 200.0);

    h.click(200.0, 24.0);
    h.tick();
    assert_eq!(got.get(), 0, "klik w widoczny wiersz niesie id");
}

#[test]
fn missing_items_is_rejected() {
    let src = "app A {\n  title: \"A\"\n  size: (400, 200)\n  window M {\n    VirtualList(key: id) {\n      Text(\"x\")\n    }\n  }\n}\n";
    let app = parse("Main.seq", src).unwrap();
    check("Main.seq", &app).unwrap();
    let err = match SeqLoader::load_str("Main.seq", src) {
        Err(e) => e,
        Ok(_) => panic!("oczekiwano bledu"),
    };
    assert!(err.message.contains("items"), "{err}");
}

#[test]
fn static_needs_host() {
    let src = "app A {\n  title: \"A\"\n  size: (400, 200)\n  window M {\n    VirtualList(items: songs, key: id) {\n      Text(\"x\")\n    }\n  }\n}\n";
    let err = match SeqLoader::load_str("Main.seq", src) {
        Err(e) => e,
        Ok(_) => panic!("oczekiwano bledu"),
    };
    assert!(err.message.contains("hosta"), "{err}");
}
