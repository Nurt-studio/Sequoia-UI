#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_seq::{SeqLoader, check, parse};
use sequoia_test::Harness;

fn app_with(body: &str) -> String {
    format!("app A {{\n  title: \"A\"\n  size: (400, 300)\n  window M {{\n{body}\n  }}\n}}\n")
}

fn loads(src: &str) {
    let loaded = match SeqLoader::load_str("Main.seq", src) {
        Ok(l) => l,
        Err(e) => panic!("oczekiwano load ok: {e}"),
    };
    let h = Harness::new(loaded.view);
    h.layout(400.0, 300.0);

    let _ = h.paint();
}

fn load_err(src: &str) -> sequoia_seq::SeqError {
    match SeqLoader::load_str("Main.seq", src) {
        Ok(_) => panic!("oczekiwano bledu"),
        Err(e) => e,
    }
}

#[test]
fn image_path_paints_without_crash() {
    loads(&app_with(
        "    Image(path: \"photo.png\", width: px(120), height: px(80))",
    ));
}

#[test]
fn image_label_as_path() {
    loads(&app_with(
        "    Image(\"photo.png\", width: px(40), height: px(40))",
    ));
}

#[test]
fn image_requires_path() {
    let err = load_err(&app_with("    Image(width: px(40))"));
    assert!(err.message.contains("Image wymaga path"), "{err}");
}

#[test]
fn image_unknown_prop_reports_line() {
    let src = app_with("    Image(path: \"a.png\", foo: 1)");
    let app = parse("Main.seq", &src).unwrap();
    let err = check("Main.seq", &app).unwrap_err();
    assert!(err.message.contains("foo"), "{err}");
}

#[test]
fn text_inline_weights_paint() {
    for w in ["bold", "semi", "medium", "normal", "light", "black"] {
        loads(&app_with(&format!("    Text(\"x\", {w}: true)")));
    }
}

#[test]
fn text_two_weights_is_error() {
    let src = app_with("    Text(\"x\", bold: true, light: true)");
    let app = parse("Main.seq", &src).unwrap();
    let err = check("Main.seq", &app).unwrap_err();
    assert!(err.message.contains("jedna flaga wagi"), "{err}");
}

#[test]
fn text_wrap_bool_paints() {
    loads(&app_with("    Text(\"x\", wrap: true)"));
    loads(&app_with("    Text(\"x\", wrap: false)"));
}

#[test]
fn container_per_side_borders_paint() {
    let src = app_with(
        "    Container(width: px(120), height: px(60), border_left: [4, #FF0000], border_top: [2, theme.border]) {\n      Text(\"x\")\n    }",
    );
    let loaded = match SeqLoader::load_str("Main.seq", &src) {
        Ok(l) => l,
        Err(e) => panic!("oczekiwano load ok: {e}"),
    };
    let h = Harness::new(loaded.view);
    h.layout(400.0, 300.0);
    let qs: Vec<_> = h.paint().quads().cloned().collect();
    assert!(!qs.is_empty(), "ramka boczna maluje");
}

#[test]
fn panel_per_side_borders_paint() {
    let src = app_with("    Panel(width: px(120), height: px(60), border_x: [3, #00FF00])");
    let loaded = match SeqLoader::load_str("Main.seq", &src) {
        Ok(l) => l,
        Err(e) => panic!("oczekiwano load ok: {e}"),
    };
    let h = Harness::new(loaded.view);
    h.layout(400.0, 300.0);
    let qs: Vec<_> = h.paint().quads().cloned().collect();
    assert!(!qs.is_empty(), "ramka Panel maluje");
}

#[test]
fn container_per_side_bad_shape_reports_line() {
    let err = load_err(&app_with("    Container(border_left: 4)"));
    assert!(err.message.contains("border_left"), "{err}");
}
