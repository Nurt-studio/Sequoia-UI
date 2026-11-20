#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_core::Color;
use sequoia_seq::{SeqLoader, check, check_all, lower, parse, parse_all};
use sequoia_test::Harness;

const DEMO: &str = r#"
app Demo {
  title: "Demo"
  size: (400, 300)
  theme: dark
  window Main {
    Column(padding: 12, gap: 8, background: theme.bg) {
      Text("Hi", size: 14, color: theme.ink)
      Row(gap: 8) {
        Icon(name: "check", size: 16, color: theme.accent)
        Button("OK", style: primary)
      }
      Spacer(flex: 1)
    }
  }
}
"#;

#[test]
fn parses_app_meta() {
    let app = parse("Main.seq", DEMO).unwrap();
    assert_eq!(app.title, "Demo");
    assert_eq!(app.size, (400.0, 300.0));
    assert_eq!(app.windows.len(), 1);
    assert_eq!(app.windows[0].name, "Main");
}

#[test]
fn lowers_and_paints() {
    let loaded = SeqLoader::load_str("Main.seq", DEMO).unwrap();
    assert_eq!(loaded.meta.title, "Demo");
    assert_eq!(loaded.meta.size, (400.0, 300.0));
    let h = Harness::new(loaded.view);
    h.layout(400.0, 300.0);
    let list = h.paint();
    assert!(!list.is_empty(), "statyczne drzewo maluje");

    let bg = list.quads().next().expect("column background").color;
    assert_eq!(bg, Color::rgb8(0x19, 0x19, 0x19));
}

#[test]
fn unknown_component_reports_line() {
    let src = "app A {\n  title: \"A\"\n  size: (10, 10)\n  window M {\n    Col {}\n  }\n}\n";
    let app = parse("Main.seq", src).unwrap();
    let err = check("Main.seq", &app).unwrap_err();
    assert_eq!(err.line, 5, "linia Col: {err}");
}

#[test]
fn unknown_prop_reports_line() {
    let src = "app A {\n  title: \"A\"\n  size: (10, 10)\n  window M {\n    Text(\"x\", spacing: 8)\n  }\n}\n";
    let app = parse("Main.seq", src).unwrap();
    let err = check("Main.seq", &app).unwrap_err();
    assert!(err.message.contains("spacing"), "nazwa propki: {err}");
    assert_eq!(err.line, 5);
}

#[test]
fn callback_needs_host() {
    let src = "app A {\n  title: \"A\"\n  size: (10, 10)\n  window M {\n    Button(\"Go\", on_click: { go() })\n  }\n}\n";
    let err = match SeqLoader::load_str("Main.seq", src) {
        Err(e) => e,
        Ok(_) => panic!("expected error"),
    };
    assert!(err.message.contains("inkrement 2"), "{err}");
    assert_eq!(err.line, 5);
}

#[test]
fn dynamic_needs_host() {
    let src = "app A {\n  title: \"A\"\n  size: (10, 10)\n  window M {\n    Text(dynamic: title)\n  }\n}\n";
    let err = match SeqLoader::load_str("Main.seq", src) {
        Err(e) => e,
        Ok(_) => panic!("expected error"),
    };
    assert!(err.message.contains("inkrement 2"), "{err}");
}

#[test]
fn prop_block_needs_host() {
    let src = "app A {\n  title: \"A\"\n  size: (10, 10)\n  prop n: Int\n  window M {\n    Text(\"x\")\n  }\n}\n";
    let err = match SeqLoader::load_str("Main.seq", src) {
        Err(e) => e,
        Ok(_) => panic!("expected error"),
    };
    assert!(err.message.contains("prop"), "{err}");
    assert_eq!(err.line, 4);
}

#[test]
fn bad_hex_is_rejected() {
    let src = "app A {\n  title: \"A\"\n  size: (10, 10)\n  window M {\n    Text(\"x\", color: #FFF)\n  }\n}\n";
    let err = parse("Main.seq", src).unwrap_err();
    assert!(err.message.contains("#RRGGBB"), "{err}");
    assert_eq!(err.line, 5);
}

#[test]
fn missing_title_reports_app_line() {
    let src = "app A {\n  size: (10, 10)\n  window M {\n    Text(\"x\")\n  }\n}\n";
    let err = parse("Main.seq", src).unwrap_err();
    assert!(err.message.contains("title"), "{err}");
    assert_eq!(err.line, 1);
}

#[test]
fn lower_without_check_still_guards() {
    let src = "app A {\n  title: \"A\"\n  size: (10, 10)\n  window M {\n    Text(\"x\", spacing: 8)\n  }\n}\n";
    let app = parse("Main.seq", src).unwrap();
    let err = match lower("Main.seq", &app) {
        Err(e) => e,
        Ok(_) => panic!("expected error"),
    };
    assert!(err.message.contains("spacing"), "{err}");
}

#[test]
fn deep_nesting_is_refused_not_overflow() {
    let mut body = String::new();
    for _ in 0..10_000 {
        body.push_str("Column { ");
    }
    body.push_str("Text(\"x\") ");
    for _ in 0..10_000 {
        body.push_str("} ");
    }
    let src = format!(
        "app A {{\n  title: \"A\"\n  size: (10, 10)\n  window M {{\n    {body}\n  }}\n}}\n"
    );
    let err = parse("Main.seq", &src).unwrap_err();
    assert!(err.message.contains("nesting"), "{err}");

    let mut ok_body = String::new();
    for _ in 0..100 {
        ok_body.push_str("Column { ");
    }
    ok_body.push_str("Text(\"x\") ");
    for _ in 0..100 {
        ok_body.push_str("} ");
    }
    let ok_src = format!(
        "app A {{\n  title: \"A\"\n  size: (10, 10)\n  window M {{\n    {ok_body}\n  }}\n}}\n"
    );
    let app = parse("Main.seq", &ok_src).unwrap();
    check("Main.seq", &app).unwrap();
}

#[test]
fn attr_expr_type_checking() {
    let src_ok = "app A {\n  title: \"A\"\n  size: (10, 10)\n  window M {\n    Text(\"hi\", size: 16 + 4)\n  }\n}\n";
    let app = parse("Main.seq", src_ok).unwrap();
    check("Main.seq", &app).unwrap();

    let src_bool = "app A {\n  title: \"A\"\n  size: (10, 10)\n  window M {\n    Text(\"hi\", size: true)\n  }\n}\n";
    let app = parse("Main.seq", src_bool).unwrap();
    let err = check("Main.seq", &app).unwrap_err();
    assert!(err.message.contains("expected Float, is Bool"), "{err}");

    let src_str = "app A {\n  title: \"A\"\n  size: (10, 10)\n  window M {\n    Text(\"hi\", size: \"big\")\n  }\n}\n";
    let app = parse("Main.seq", src_str).unwrap();
    assert!(check("Main.seq", &app).is_err());

    let src_prop = "app A {\n  title: \"A\"\n  size: (10, 10)\n  prop count: Int\n  window M {\n    If(when: count > 0) {\n      Text(\"yes\")\n    }\n  }\n}\n";
    let app = parse("Main.seq", src_prop).unwrap();
    check("Main.seq", &app).unwrap();
}

#[test]
fn parse_all_collects_multiple_errors_in_one_log() {
    let src = "app A {\n  title \"A\"\n  siz: (10, 10)\n  wndow M {\n    Text(\"x\")\n  }\n}\n";
    let (_, errors) = parse_all("Main.seq", src);
    assert!(
        errors.len() >= 2,
        "expected multiple parse errors: {errors:?}"
    );

    let err = parse("Main.seq", src).unwrap_err();
    let log = format!("{err}");
    assert!(err.related().len() >= 2, "expected related errors: {log}");
    assert!(log.contains("title"), "{log}");
    assert!(log.lines().count() >= 2, "{log}");
}

#[test]
fn check_all_collects_errors_from_multiple_nodes() {
    let src = "app A {\n  title: \"A\"\n  size: (10, 10)\n  window M {\n    Col {}\n    Rowr {}\n  }\n}\n";
    let (app, parse_errors) = parse_all("Main.seq", src);
    assert!(parse_errors.is_empty(), "{parse_errors:?}");
    let app = app.unwrap();
    let errors = check_all("Main.seq", &app);
    assert!(
        errors.len() >= 2,
        "expected multiple check errors: {errors:?}"
    );

    let err = check("Main.seq", &app).unwrap_err();
    assert!(
        format!("{err}").lines().count() >= 2,
        "check must print all errors: {err}"
    );
}
