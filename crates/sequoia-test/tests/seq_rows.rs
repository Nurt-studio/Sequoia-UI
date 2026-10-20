#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::cell::Cell;
use std::rc::Rc;

use sequoia_reactive::Signal;
use sequoia_seq::{FieldSignal, HostBindings, SeqLoader, SeqRow};
use sequoia_test::Harness;
use sequoia_view::ElementId;

#[derive(Clone)]
struct Layer {
    id: i64,
    title: Signal<String>,
    visible: Signal<bool>,
}

impl PartialEq for Layer {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl SeqRow for Layer {
    fn id(&self) -> i64 {
        self.id
    }

    fn fields(&self) -> Vec<(String, FieldSignal)> {
        vec![
            ("title".to_string(), FieldSignal::Str(self.title.clone())),
            (
                "visible".to_string(),
                FieldSignal::Bool(self.visible.clone()),
            ),
        ]
    }
}

const SRC: &str = "app A {\n  title: \"A\"\n  size: (300, 200)\n  on toggle(id: Int)\n  window M {\n    List(items: layers, key: id) {\n      Row(gap: 8) {\n        Button(\"o\", on_click: { toggle(item.id) })\n        Text(dynamic: item.title)\n      }\n    }\n  }\n}\n";

type Fixture = (
    Signal<Vec<Layer>>,
    HostBindings,
    Rc<Cell<Vec<i64>>>,
    Signal<String>,
);

fn layers() -> Fixture {
    let a_title = Signal::new("A".to_string());
    let b_title = Signal::new("BB".to_string());
    let rows = Signal::new(vec![
        Layer {
            id: 1,
            title: a_title.clone(),
            visible: Signal::new(true),
        },
        Layer {
            id: 2,
            title: b_title.clone(),
            visible: Signal::new(false),
        },
    ]);
    let fired = Rc::new(Cell::new(Vec::new()));
    let f = fired.clone();
    let b = HostBindings::new()
        .rows("layers", rows.clone())
        .on_int("toggle", move |id| {
            let mut v = f.take();
            v.push(id);
            f.set(v);
        });
    (rows, b, fired, b_title)
}

fn texts_in(h: &Harness, id: ElementId, out: &mut Vec<String>) {
    if let Some(t) = h.text(id) {
        out.push(t);
    }
    for c in h.children(id) {
        texts_in(h, c, out);
    }
}

fn row_texts(h: &Harness) -> Vec<String> {
    let root = h.root().unwrap();
    let list = h.children(root)[0];
    let mut out = Vec::new();
    for row in h.children(list) {
        let mut texts = Vec::new();
        texts_in(h, row, &mut texts);

        let title = texts.into_iter().find(|t| t != "o").unwrap();
        out.push(title);
    }
    out
}

fn row_button(h: &Harness, row_idx: usize) -> ElementId {
    let root = h.root().unwrap();
    let list = h.children(root)[0];
    let row = h.children(list)[row_idx];
    let wrap = h.children(row)[0];
    h.children(wrap)[0]
}

fn click_center(h: &Harness, id: ElementId) {
    let r = h.rect(id).unwrap();
    h.click(r.left() + r.size.width * 0.5, r.top() + r.size.height * 0.5);
}

#[test]
fn renders_rows_with_titles() {
    let (_rows, b, _fired, _t) = layers();
    let loaded = SeqLoader::load_str_with("Main.seq", SRC, &b).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(300.0, 200.0);
    h.paint();
    h.tick();
    assert_eq!(row_texts(&h), vec!["A".to_string(), "BB".to_string()]);
}

#[test]
fn row_click_fires_with_row_id() {
    let (_rows, b, fired, _t) = layers();
    let loaded = SeqLoader::load_str_with("Main.seq", SRC, &b).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(300.0, 200.0);
    h.paint();
    h.tick();
    click_center(&h, row_button(&h, 1));
    h.tick();
    assert_eq!(fired.take(), vec![2]);
}

#[test]
fn field_edit_updates_row_text() {
    let (_rows, b, _fired, b_title) = layers();
    let loaded = SeqLoader::load_str_with("Main.seq", SRC, &b).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(300.0, 200.0);
    h.paint();
    h.tick();
    b_title.set("BBBB".to_string());
    h.layout(300.0, 200.0);
    h.paint();
    h.tick();
    assert_eq!(row_texts(&h), vec!["A".to_string(), "BBBB".to_string()]);
}

#[test]
fn reorder_keeps_keys_click_follows_data() {
    let (rows, b, fired, _t) = layers();
    let loaded = SeqLoader::load_str_with("Main.seq", SRC, &b).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(300.0, 200.0);
    h.paint();
    h.tick();

    let mut v = rows.get_untracked();
    v.swap(0, 1);
    rows.set(v);
    h.layout(300.0, 200.0);
    h.paint();
    h.tick();
    assert_eq!(row_texts(&h), vec!["BB".to_string(), "A".to_string()]);
    click_center(&h, row_button(&h, 0));
    h.tick();
    assert_eq!(fired.take(), vec![2], "click follows data, not position");
}

#[test]
fn unknown_rows_prop_reports_use_line() {
    let src = "app A {\n  title: \"A\"\n  size: (300, 200)\n  window M {\n    List(items: nope, key: id) {\n      Text(\"x\")\n    }\n  }\n}\n";
    let b = HostBindings::new();
    let err = match SeqLoader::load_str_with("Main.seq", src, &b) {
        Err(e) => e,
        Ok(_) => panic!("oczekiwano bledu"),
    };
    assert!(err.message.contains("nope"), "{err}");
    assert_eq!(err.line, 5);
}

#[test]
fn non_id_key_is_rejected() {
    let src = "app A {\n  title: \"A\"\n  size: (300, 200)\n  window M {\n    List(items: layers, key: title) {\n      Text(\"x\")\n    }\n  }\n}\n";
    let (_rows, b, _f, _t) = layers();
    let err = match SeqLoader::load_str_with("Main.seq", src, &b) {
        Err(e) => e,
        Ok(_) => panic!("oczekiwano bledu"),
    };
    assert!(err.message.contains("key"), "{err}");
    assert_eq!(err.line, 5);
}

#[test]
fn static_list_requires_host() {
    let src = "app A {\n  title: \"A\"\n  size: (300, 200)\n  window M {\n    List(items: layers, key: id) {\n      Text(\"x\")\n    }\n  }\n}\n";
    let err = match SeqLoader::load_str("Main.seq", src) {
        Err(e) => e,
        Ok(_) => panic!("oczekiwano bledu"),
    };
    assert!(err.message.contains("rows()"), "{err}");
}

#[test]
fn item_outside_list_is_rejected() {
    let src = "app A {\n  title: \"A\"\n  size: (300, 200)\n  window M {\n    Text(dynamic: item.title)\n  }\n}\n";
    let (_rows, b, _f, _t) = layers();
    let err = match SeqLoader::load_str_with("Main.seq", src, &b) {
        Err(e) => e,
        Ok(_) => panic!("oczekiwano bledu"),
    };
    assert!(err.message.contains("List"), "{err}");
    assert_eq!(err.line, 5);
}

#[test]
fn unknown_row_field_paints_error() {
    let src = "app A {\n  title: \"A\"\n  size: (300, 200)\n  window M {\n    List(items: layers, key: id) {\n      Text(dynamic: item.nope)\n    }\n  }\n}\n";
    let (_rows, b, _f, _t) = layers();
    let loaded = SeqLoader::load_str_with("Main.seq", src, &b).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(300.0, 200.0);
    h.paint();
    h.tick();
    let root = h.root().unwrap();
    let mut texts = Vec::new();
    texts_in(&h, root, &mut texts);
    assert!(
        texts
            .iter()
            .any(|t| t.contains("seq:") && t.contains("nope")),
        "line error appears as text: {texts:?}"
    );
}
