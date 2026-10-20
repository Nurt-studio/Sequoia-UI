#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_reactive::Signal;
use sequoia_seq::{HostBindings, SeqLoader};
use sequoia_test::Harness;

const SRC: &str = "app B {\n  title: \"B\"\n  size: (400, 700)\n  prop result: Float\n  prop note: String\n  prop flag: Bool\n\n  fn calc(a: Int, b: Int) -> Int { return a * 2 + b; }\n  fn calcbtn() { result = calc(2, 3); }\n  fn prec() { result = 2 + 3 * 4; }\n  fn intdiv() { result = 7 / 2; }\n  fn fltdiv() { result = 7.0 / 2.0; }\n  fn rem() { result = 7 % 2; }\n  fn neg() { result = -5 + 1; }\n  fn strcat() { note = \"a\" + \"b\"; }\n  fn strip(s: String) { note = strip_prefix(s, \"node:\"); }\n  fn method(s: String) { flag = s.starts_with(\"out:\"); }\n  fn pair(a: Float, b: Float) -> (Float, Float) { return ((a + 1.0), (b - 1.0)); }\n  fn tup(a: Float, b: Float) { let p = pair(a, b); result = p.0 * p.1; }\n  fn tupbtn() { tup(3.0, 4.0); }\n  fn clampit(x: Float) { result = clamp(x, 0.0, 10.0); }\n  fn powers() { result = powf(2.0, 3.0); }\n  fn extrema() { result = max(min(9, 4), len(\"abcd\")); }\n  fn tune(v: Float) { result = v; }\n  fn logic(n: Int) { flag = n > 0 && !(n > 10); }\n  fn logic2(n: Int) { flag = n > 0 && !(n > 10); }\n\n  window M {\n    Column(gap: 0) {\n      Button(\"calc\", height: px(36), on_click: { calcbtn() })\n      Button(\"prec\", height: px(36), on_click: { prec() })\n      Button(\"intdiv\", height: px(36), on_click: { intdiv() })\n      Button(\"fltdiv\", height: px(36), on_click: { fltdiv() })\n      Button(\"rem\", height: px(36), on_click: { rem() })\n      Button(\"neg\", height: px(36), on_click: { neg() })\n      Button(\"strcat\", height: px(36), on_click: { strcat() })\n      Button(\"strip\", height: px(36), on_click: { strip(\"node:7\") })\n      Button(\"method\", height: px(36), on_click: { method(\"out:3\") })\n      Button(\"tup\", height: px(36), on_click: { tupbtn() })\n      Button(\"clampit\", height: px(36), on_click: { clampit(15.0) })\n      Button(\"powers\", height: px(36), on_click: { powers() })\n      Button(\"extrema\", height: px(36), on_click: { extrema() })\n      Button(\"tune\", height: px(36), on_click: { tune(2) })\n      Button(\"logic\", height: px(36), on_click: { logic(5) })\n      Button(\"logic2\", height: px(36), on_click: { logic2(15) })\n    }\n  }\n}\n";

struct Fixture {
    h: Harness,
    result: Signal<f64>,
    note: Signal<String>,
    flag: Signal<bool>,
}

fn fixture() -> Fixture {
    let result = Signal::new(0.0);
    let note = Signal::new(String::new());
    let flag = Signal::new(false);
    let b = HostBindings::new()
        .prop_float("result", result.clone())
        .prop_str("note", note.clone())
        .prop_bool("flag", flag.clone());
    let loaded = SeqLoader::load_str_with("Basic.seq", SRC, &b).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(400.0, 700.0);
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
fn arithmetic_and_precedence() {
    let f = fixture();
    click_button(&f.h, 0);
    assert_eq!(f.result.get_untracked(), 7.0, "a * 2 + b");
    click_button(&f.h, 1);
    assert_eq!(f.result.get_untracked(), 14.0, "2 + 3 * 4");
    click_button(&f.h, 2);
    assert_eq!(
        f.result.get_untracked(),
        3.0,
        "Int / Int = integer division"
    );
    click_button(&f.h, 3);
    assert_eq!(f.result.get_untracked(), 3.5, "Float / Float");
    click_button(&f.h, 4);
    assert_eq!(f.result.get_untracked(), 1.0, "7 % 2");
    click_button(&f.h, 5);
    assert_eq!(f.result.get_untracked(), -4.0, "-5 + 1");
}

#[test]
fn strings_and_methods() {
    let f = fixture();
    click_button(&f.h, 6);
    assert_eq!(f.note.get_untracked(), "ab", "String + String");
    click_button(&f.h, 7);
    assert_eq!(f.note.get_untracked(), "7", "strip_prefix");
    click_button(&f.h, 8);
    assert!(f.flag.get_untracked(), "s.starts_with(...) — cukier metody");
}

#[test]
fn tuples_and_builtins() {
    let f = fixture();
    click_button(&f.h, 9);
    assert_eq!(f.result.get_untracked(), 12.0, "krotka: p.0 * p.1");
    click_button(&f.h, 10);
    assert_eq!(f.result.get_untracked(), 10.0, "clamp");
    click_button(&f.h, 11);
    assert_eq!(f.result.get_untracked(), 8.0, "powf");
    click_button(&f.h, 12);
    assert_eq!(f.result.get_untracked(), 4.0, "max(min(...), len(...))");
}

#[test]
fn int_literal_coerces_to_float_param() {
    let f = fixture();
    click_button(&f.h, 13);
    assert_eq!(f.result.get_untracked(), 2.0, "tune(2) → Float");
}

#[test]
fn boolean_logic() {
    let f = fixture();
    click_button(&f.h, 14);
    assert!(f.flag.get_untracked(), "5 > 0 && !(5 > 10)");
    click_button(&f.h, 15);
    assert!(!f.flag.get_untracked(), "15 > 0 && !(15 > 10)");
}

#[test]
fn unknown_type_in_signature_is_rejected() {
    let src = "app A {\n  title: \"A\"\n  size: (10, 10)\n  fn f(x: Liczba) { }\n  window M {\n    Text(\"x\")\n  }\n}\n";
    let err = match SeqLoader::load_str("Main.seq", src) {
        Err(e) => e,
        Ok(_) => panic!("oczekiwano bledu"),
    };
    assert!(err.message.contains("nieznany typ parametru"), "{err}");
    assert_eq!(err.line, 4, "linia parametru: {err}");
}
