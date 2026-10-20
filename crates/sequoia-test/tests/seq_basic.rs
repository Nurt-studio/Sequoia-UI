#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_core::Color;
use sequoia_seq::{SeqLoader, check, lower, parse};
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
        Ok(_) => panic!("oczekiwano bledu"),
    };
    assert!(err.message.contains("inkrement 2"), "{err}");
    assert_eq!(err.line, 5);
}

#[test]
fn dynamic_needs_host() {
    let src = "app A {\n  title: \"A\"\n  size: (10, 10)\n  window M {\n    Text(dynamic: title)\n  }\n}\n";
    let err = match SeqLoader::load_str("Main.seq", src) {
        Err(e) => e,
        Ok(_) => panic!("oczekiwano bledu"),
    };
    assert!(err.message.contains("inkrement 2"), "{err}");
}

#[test]
fn prop_block_needs_host() {
    let src = "app A {\n  title: \"A\"\n  size: (10, 10)\n  prop n: Int\n  window M {\n    Text(\"x\")\n  }\n}\n";
    let err = match SeqLoader::load_str("Main.seq", src) {
        Err(e) => e,
        Ok(_) => panic!("oczekiwano bledu"),
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
        Ok(_) => panic!("oczekiwano bledu"),
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
