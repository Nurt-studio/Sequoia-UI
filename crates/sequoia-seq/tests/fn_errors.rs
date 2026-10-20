#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_reactive::Signal;
use sequoia_seq::{FieldSignal, HostBindings, SeqError, SeqLoader, SeqRow};
use sequoia_test::Harness;

fn app(fn_src: &str, button: &str) -> String {
    format!(
        "app A {{\n  title: \"A\"\n  size: (400, 200)\n  prop out: Int\n{fn_src}\n  window M {{\n    {button}\n  }}\n}}\n"
    )
}

fn err(src: &str) -> SeqError {
    match SeqLoader::load_str("Err.seq", src) {
        Err(e) => e,
        Ok(_) => panic!("oczekiwano bledu dla:\n{src}"),
    }
}

#[test]
fn unknown_action_in_callback_reports_missing_host() {
    let out = Signal::new(0);
    let b = HostBindings::new().prop_int("out", out);
    let src = app("", "Button(\"x\", on_click: { nope() })");
    let e = match SeqLoader::load_str_with("Err.seq", &src, &b) {
        Err(e) => e,
        Ok(_) => panic!("oczekiwano bledu"),
    };
    assert!(e.message.contains("nieznany callback 'nope'"), "{e}");
}

#[test]
fn call_arity_inside_fn_body() {
    let e = err(&app(
        "  fn f(a: Int) { out = a; }\n  fn g() { f(); }",
        "Button(\"x\", on_click: { g() })",
    ));
    assert!(e.message.contains("expected 1 argument"), "{e}");
}

#[test]
fn callback_arity_vs_signature() {
    let e = err(&app(
        "  fn f(a: Int, b: Int) { out = a + b; }",
        "Button(\"x\", on_click: { f(1) })",
    ));
    assert!(e.message.contains("expected 2 parameters"), "{e}");
}

#[test]
fn callback_arg_type_mismatch() {
    let e = err(&app(
        "  fn f(a: Int) { out = a; }",
        "Button(\"x\", on_click: { f(\"s\") })",
    ));
    assert!(e.message.contains("podaje String"), "{e}");
    assert_eq!(e.line, 7, "linia callbacku: {e}");
}

#[test]
fn assign_to_param_is_rejected() {
    let e = err(&app(
        "  fn f(x: Int) { x = 1; }",
        "Button(\"x\", on_click: { f(1) })",
    ));
    assert!(e.message.contains("niemutowalna"), "{e}");
}

#[test]
fn return_type_mismatch() {
    let e = err(&app(
        "  fn f() -> Int { return \"s\"; }",
        "Button(\"x\", on_click: { f() })",
    ));
    assert!(e.message.contains("oczekiwano Int"), "{e}");
}

#[test]
fn every_path_must_return() {
    let e = err(&app(
        "  fn f() -> Int { if (true) { return 1; } }",
        "Button(\"x\", on_click: { f() })",
    ));
    assert!(e.message.contains("end with `return`"), "{e}");
}

#[test]
fn unknown_variable_in_fn() {
    let e = err(&app(
        "  fn f() { nope = 1; }",
        "Button(\"x\", on_click: { f() })",
    ));
    assert!(e.message.contains("nieznana zmienna 'nope'"), "{e}");
}

#[test]
fn recursion_is_allowed() {
    let out = Signal::new(0);
    let b = HostBindings::new().prop_int("out", out.clone());
    let src = app(
        "  fn f(n: Int) -> Int { if (n <= 0) { return 0; } return n + g(n - 1); }\n  fn g(n: Int) -> Int { if (n <= 0) { return 0; } return n + f(n - 1); }\n  fn bump() { out = f(3); }",
        "Button(\"x\", on_click: { bump() })",
    );
    let loaded = SeqLoader::load_str_with("Ok.seq", &src, &b).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(10.0, 10.0);
    h.paint();

    let _ = h;
}

#[test]
fn builtin_name_shadowing_is_rejected() {
    let e = err(&app(
        "  fn max(a: Int, b: Int) -> Int { return a; }",
        "Button(\"x\", on_click: { max(1, 2) })",
    ));
    assert!(e.message.contains("overrides built-in"), "{e}");
}

#[test]
fn duplicate_fn_is_rejected() {
    let e = err(&app(
        "  fn f() { }\n  fn f() { }",
        "Button(\"x\", on_click: { f() })",
    ));
    assert!(e.message.contains("zdefiniowana dwa razy"), "{e}");
}

#[test]
fn state_collides_with_prop() {
    let src = "app A {\n  title: \"A\"\n  size: (10, 10)\n  prop n: Int\n  state n: Int = 0\n  window M {\n    Text(\"x\")\n  }\n}\n";
    let e = err(src);
    assert!(e.message.contains("koliduje z `prop n`"), "{e}");
}

#[test]
fn match_requires_wildcard() {
    let e = err(&app(
        "  fn f(n: Int) { match n { 0 => { out = 1; } } }",
        "Button(\"x\", on_click: { f(0) })",
    ));
    assert!(e.message.contains("match requires branch `_`"), "{e}");
}

#[test]
fn match_pattern_type_mismatch() {
    let e = err(&app(
        "  fn f(n: Int) { match n { \"a\" => { out = 1; } _ => { out = 2; } } }",
        "Button(\"x\", on_click: { f(0) })",
    ));
    assert!(e.message.contains("nie pasuje"), "{e}");
}

#[test]
fn callseq_outside_callback_is_rejected() {
    let e = err(&app(
        "  fn f() { out = 1; }",
        "Button(\"x\", width: { f() })",
    ));
    assert!(e.message.contains("tylko jako callback"), "{e}");
}

#[test]
fn fn_call_needs_host_in_static_mode() {
    let src = "app A {\n  title: \"A\"\n  size: (400, 200)\n  fn f() { }\n  window M {\n    Button(\"x\", on_click: { f() })\n  }\n}\n";
    let e = err(src);
    assert!(e.message.contains("wymaga hosta Rust (lower_with)"), "{e}");
}

#[test]
fn wheel_shape_mismatch() {
    let e = err(&app(
        "  fn z(x: Float, y: Float, d: Float) { out = 1; }",
        "Canvas(on_wheel: { z(x, y) })",
    ));
    assert!(e.message.contains("on_wheel"), "{e}");
}

#[test]
fn tuple_field_out_of_range() {
    let e = err(&app(
        "  fn f() -> Float { let p = (1.0, 2.0); return p.2; }",
        "Button(\"x\", on_click: { f() })",
    ));
    assert!(e.message.contains("has 2 elements"), "{e}");
}

#[test]
fn fn_name_must_start_lowercase() {
    let e = err(&app(
        "  fn Move() { }",
        "Button(\"x\", on_click: { Move() })",
    ));
    assert!(e.message.contains("lower case"), "{e}");
}

#[test]
fn missing_semicolon_after_let() {
    let e = err(&app(
        "  fn f() { let x = 1 }",
        "Button(\"x\", on_click: { f() })",
    ));
    assert!(e.message.contains("';'"), "{e}");
}

#[test]
fn missing_return_is_rejected_at_runtime_backup() {
    let e = err(&app(
        "  fn f() -> Int { let x = 1; }",
        "Button(\"x\", on_click: { f() })",
    ));
    assert!(e.message.contains("end with `return`"), "{e}");
}

#[derive(Clone)]
struct Row {
    id: i64,
}

impl PartialEq for Row {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl SeqRow for Row {
    fn id(&self) -> i64 {
        self.id
    }

    fn fields(&self) -> Vec<(String, FieldSignal)> {
        Vec::new()
    }
}

const FUEL: &str = "app F {\n  title: \"F\"\n  size: (400, 200)\n  prop out: Int\n\n  fn blow() {\n    mut c = 0;\n    for n in nodes {\n      for m in nodes {\n        c = c + 1;\n      }\n    }\n    out = c;\n  }\n\n  fn rescue() { out = 1; }\n\n  window M {\n    Column(gap: 0) {\n      Button(\"blow\", height: px(36), on_click: { blow() })\n      Button(\"rescue\", height: px(36), on_click: { rescue() })\n    }\n  }\n}\n";

#[test]
fn fuel_limit_aborts_loop_without_hanging() {
    let out = Signal::new(0);
    let rows = Signal::new((0..200).map(|id| Row { id }).collect::<Vec<_>>());
    let b = HostBindings::new()
        .prop_int("out", out.clone())
        .rows("nodes", rows);
    let loaded = SeqLoader::load_str_with("Fuel.seq", FUEL, &b).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(400.0, 200.0);
    h.paint();
    h.tick();

    let click = |i: usize| {
        let root = h.root().unwrap();
        let col = h.children(root)[0];
        let btn = h.children(col)[i];
        let r = h.rect(btn).unwrap();
        h.click(r.left() + r.size.width * 0.5, r.top() + r.size.height * 0.5);
        h.tick();
    };

    click(0);
    assert_eq!(
        out.get_untracked(),
        0,
        "fuel interrupted: `out = c` after loop did not execute"
    );
    click(1);
    assert_eq!(out.get_untracked(), 1, "runtime alive after fuel error");
}
