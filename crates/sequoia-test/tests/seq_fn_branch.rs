#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_reactive::Signal;
use sequoia_seq::{FieldSignal, HostBindings, SeqLoader, SeqRow};
use sequoia_test::Harness;

#[derive(Clone)]
struct Node {
    id: i64,
    label: Signal<String>,
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl SeqRow for Node {
    fn id(&self) -> i64 {
        self.id
    }

    fn fields(&self) -> Vec<(String, FieldSignal)> {
        vec![("label".to_string(), FieldSignal::Str(self.label.clone()))]
    }
}

const SRC: &str = "app G {\n  title: \"G\"\n  size: (400, 500)\n  prop result: Int\n  prop note: String\n  prop flag: Bool\n\n  fn klass(n: Int) -> String {\n    if (n < 0) {\n      return \"neg\";\n    } else if (n == 0) {\n      return \"zero\";\n    } else {\n      return \"pos\";\n    }\n  }\n  fn apply(n: Int) { note = klass(n); }\n\n  fn tag(t: String) { result = parse_tag(t); }\n\n  fn kind(n: Int) {\n    match n {\n      0 => { note = \"zero\"; }\n      1 => { note = \"one\"; }\n      _ => { note = \"other\"; }\n    }\n  }\n\n  fn count() {\n    mut c = 0;\n    for n in nodes {\n      c = c + 1;\n    }\n    result = c;\n  }\n\n  fn find(prefix: String) {\n    mut c = 0;\n    for n in nodes {\n      if (starts_with(n.label, prefix)) {\n        c = c + 1;\n      }\n    }\n    result = c;\n  }\n\n  fn first_big() -> Int {\n    for n in nodes {\n      if (n.id > 1) {\n        return n.id;\n      }\n    }\n    return -1;\n  }\n  fn first() { result = first_big(); }\n\n  fn chk(a: Int, b: Int) -> Bool { return a > 0 && !(b > 10); }\n  fn chk2(a: Int, b: Int) -> Bool { return a > 0 || b > 10; }\n  fn logic() { flag = chk(2, 3); }\n  fn logic2() { flag = chk2(-1, 30); }\n\n  window M {\n    Column(gap: 0) {\n      Button(\"neg\", height: px(36), on_click: { apply(-3) })\n      Button(\"zero\", height: px(36), on_click: { apply(0) })\n      Button(\"pos\", height: px(36), on_click: { apply(4) })\n      Button(\"tag\", height: px(36), on_click: { tag(\"out:7\") })\n      Button(\"tagbad\", height: px(36), on_click: { tag(\"zzz\") })\n      Button(\"kind1\", height: px(36), on_click: { kind(1) })\n      Button(\"kind9\", height: px(36), on_click: { kind(9) })\n      Button(\"count\", height: px(36), on_click: { count() })\n      Button(\"find\", height: px(36), on_click: { find(\"node:\") })\n      Button(\"findout\", height: px(36), on_click: { find(\"out:\") })\n      Button(\"first\", height: px(36), on_click: { first() })\n      Button(\"logic\", height: px(36), on_click: { logic() })\n      Button(\"logic2\", height: px(36), on_click: { logic2() })\n    }\n  }\n}\n";

struct Fixture {
    h: Harness,
    result: Signal<i64>,
    note: Signal<String>,
    flag: Signal<bool>,
}

fn fixture() -> Fixture {
    let result = Signal::new(0);
    let note = Signal::new(String::new());
    let flag = Signal::new(false);
    let rows = Signal::new(vec![
        Node {
            id: 0,
            label: Signal::new("node:0".to_string()),
        },
        Node {
            id: 1,
            label: Signal::new("out:1".to_string()),
        },
        Node {
            id: 2,
            label: Signal::new("node:2".to_string()),
        },
    ]);
    let b = HostBindings::new()
        .prop_int("result", result.clone())
        .prop_str("note", note.clone())
        .prop_bool("flag", flag.clone())
        .rows("nodes", rows);
    let loaded = SeqLoader::load_str_with("Branch.seq", SRC, &b).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(400.0, 500.0);
    h.paint();
    h.tick();
    Fixture {
        h,
        result,
        note,
        flag,
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

#[test]
fn if_else_chain_returns_strings() {
    let f = fixture();
    click_button(&f.h, 0);
    assert_eq!(f.note.get_untracked(), "neg");
    click_button(&f.h, 1);
    assert_eq!(f.note.get_untracked(), "zero", "else if");
    click_button(&f.h, 2);
    assert_eq!(f.note.get_untracked(), "pos", "else");
}

#[test]
fn match_arms_pick_by_value() {
    let f = fixture();
    click_button(&f.h, 5);
    assert_eq!(f.note.get_untracked(), "one");
    click_button(&f.h, 6);
    assert_eq!(f.note.get_untracked(), "other", "branch `_`");
}

#[test]
fn parse_tag_reads_tail() {
    let f = fixture();
    click_button(&f.h, 3);
    assert_eq!(f.result.get_untracked(), 7);
    click_button(&f.h, 4);
    assert_eq!(f.result.get_untracked(), -1, "brak liczby → -1");
}

#[test]
fn for_counts_rows() {
    let f = fixture();
    click_button(&f.h, 7);
    assert_eq!(f.result.get_untracked(), 3);
}

#[test]
fn for_reads_row_fields() {
    let f = fixture();
    click_button(&f.h, 8);
    assert_eq!(
        f.result.get_untracked(),
        2,
        "starts_with(n.label, \"node:\")"
    );
    click_button(&f.h, 9);
    assert_eq!(f.result.get_untracked(), 1);
}

#[test]
fn early_return_from_loop() {
    let f = fixture();
    click_button(&f.h, 10);
    assert_eq!(f.result.get_untracked(), 2, "pierwsze id > 1");
}

#[test]
fn and_or_not() {
    let f = fixture();
    click_button(&f.h, 11);
    assert!(f.flag.get_untracked(), "2 > 0 && !(3 > 10)");
    click_button(&f.h, 12);
    assert!(f.flag.get_untracked(), "-1 > 0 || 30 > 10");
}
