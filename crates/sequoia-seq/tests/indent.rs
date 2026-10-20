#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_reactive::Signal;
use sequoia_seq::{HostBindings, SeqLoader, check, parse};
use sequoia_test::Harness;

fn app_with(body: &str) -> String {
    format!("app A {{\n  title: \"A\"\n  size: (400, 100)\n  window M {{\n{body}\n  }}\n}}\n")
}

fn second_text_x(src: &str) -> f32 {
    let loaded = SeqLoader::load_str("Main.seq", src).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(400.0, 100.0);
    let root = h.root().expect("root");
    let row = h.children(root).into_iter().next().expect("wiersz");
    let kids = h.children(row);
    assert_eq!(kids.len(), 2, "dwa teksty w wierszu");
    h.rect(kids[1]).expect("rect").left()
}

#[test]
fn indent_left_overrides_gap() {
    let plain = app_with("    Row(gap: 8) {\n      Text(\"a\")\n      Text(\"b\")\n    }");
    let indented =
        app_with("    Row(gap: 8) {\n      Text(\"a\")\n      Text(\"b\", indent_left: 24)\n    }");
    let x0 = second_text_x(&plain);
    let x1 = second_text_x(&indented);
    assert!(
        (x1 - x0 - 16.0).abs() < 0.5,
        "indent 24 zamiast gap 8 → +16px (x0={x0}, x1={x1})"
    );
}

#[test]
fn indent_forms_paint_everywhere() {
    let src = app_with(
        "    Column(indent: 2) {\n      Container(indent_x: 3) {\n        Text(\"a\", indent: 1)\n      }\n      Panel(indent_y: 4)\n      Stack(indent_top: 1, indent_bottom: 2) {\n        Text(\"b\")\n      }\n      Grid(columns: [fill(1)], indent_left: 3, indent_right: 3) {\n        Text(\"c\")\n      }\n      Wrap(indent: 2) {\n        Text(\"d\")\n      }\n      Button(\"ok\", indent: 1)\n      Icon(name: \"x\", indent: 1)\n      Spacer(flex: 1, indent_y: 2)\n    }",
    );
    let loaded = SeqLoader::load_str("Main.seq", &src).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(400.0, 300.0);
    assert!(!h.paint().is_empty());
}

#[test]
fn indent_from_prop_snapshot() {
    let src = "app A {\n  title: \"A\"\n  size: (400, 100)\n  prop w: Float\n  window M {\n    Row(gap: 8) {\n      Text(\"a\")\n      Text(\"b\", indent_left: w)\n    }\n  }\n}\n";
    let b = HostBindings::new().prop_float("w", Signal::new(24.0));
    let loaded = SeqLoader::load_str_with("Main.seq", src, &b).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(400.0, 100.0);
    assert!(!h.paint().is_empty());
}

#[test]
fn indent_rejected_on_canvas_shape() {
    let src = app_with(
        "    Canvas(width: px(200), height: px(100)) {\n      Rect(x: 0, y: 0, w: 10, h: 10, fill: #FF0000, indent_left: 4)\n    }",
    );
    let app = parse("Main.seq", &src).unwrap();
    let err = check("Main.seq", &app).unwrap_err();
    assert!(err.message.contains("indent_left"), "{err}");
}
