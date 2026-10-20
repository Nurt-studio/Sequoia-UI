#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_seq::{Visibility, parse, parse_seq_file};

const LIB: &str = r#"
import "./card.seq"

pub component Card(title: String) {
  Column { Text(text: title) children }
}
component Helper(title: String) {
  Text(text: title)
}
"#;

#[test]
fn import_and_visibility_parse() {
    let seq = parse_seq_file("ui/widgets.seq", LIB).expect("the library is parsing");
    assert!(seq.app.is_none(), "biblioteka bez app");
    assert_eq!(seq.imports.len(), 1);
    assert_eq!(seq.imports[0].path, "./card.seq");
    assert_eq!(seq.imports[0].line, 2);
    assert_eq!(seq.components.len(), 2);
    let card = &seq.components[0];
    assert_eq!(card.visibility, Visibility::Pub);
    assert_eq!(card.file, "ui/widgets.seq");
    assert_eq!(seq.components[1].visibility, Visibility::Priv);
}

#[test]
fn app_file_with_import_parses() {
    let src = "import \"./ui/card.seq\"\napp A {\n  title: \"A\"\n  size: (10, 10)\n  window M {\n    Text(\"x\")\n  }\n}\ncomponent Local(t: String) {\n  Text(text: t)\n}\n";
    let seq = parse_seq_file("Main.seq", src).expect("parsuje");
    assert!(seq.app.is_some());
    assert_eq!(seq.imports.len(), 1);
    assert_eq!(seq.components.len(), 1);
    assert_eq!(seq.components[0].file, "Main.seq");
}

#[test]
fn parse_still_requires_app() {
    let err = parse("ui/widgets.seq", LIB).unwrap_err();
    assert!(err.message.contains("brak bloku"), "{err}");
}

#[test]
fn import_inside_app_rejected() {
    let src = "app A {\n  title: \"A\"\n  size: (10, 10)\n  import \"./x.seq\"\n  window M {\n    Text(\"x\")\n  }\n}\n";
    let err = parse_seq_file("Main.seq", src).unwrap_err();
    assert!(err.message.contains("poziomie pliku"), "{err}");
    assert_eq!(err.line, 4);
}

#[test]
fn pub_without_component_rejected() {
    let err = parse_seq_file("Main.seq", "pub app A {}\n").unwrap_err();
    assert!(err.message.contains("tylko przed `component`"), "{err}");
}

#[test]
fn pub_inside_app_rejected() {
    let src = "app A {\n  title: \"A\"\n  size: (10, 10)\n  pub component X() {\n    Text(\"x\")\n  }\n  window M {\n    Text(\"x\")\n  }\n}\n";
    let err = parse_seq_file("Main.seq", src).unwrap_err();
    assert!(err.message.contains("poza blokiem `app`"), "{err}");
}

#[test]
fn unknown_top_level_rejected() {
    let err = parse_seq_file("Main.seq", "module Foo {}\n").unwrap_err();
    assert!(
        err.message.contains("`app`, `component` albo `import`"),
        "{err}"
    );
}

#[test]
fn empty_file_rejected() {
    let err = parse_seq_file("Main.seq", "// tylko komentarz\n").unwrap_err();
    assert!(err.message.contains("pusty plik"), "{err}");
}
