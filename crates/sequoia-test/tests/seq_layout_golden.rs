#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_reactive::Signal;
use sequoia_seq::{HostBindings, SeqLoader};
use sequoia_test::Harness;
use sequoia_view::ElementId;

const NESTED_GOLDEN: &str = "\
(0.0,0.0 400.0x300.0)
  (0.0,0.0 200.0x120.0)
    (10.0,10.0 30.0x32.0)
      (10.0,10.0 30.0x20.0)
      (10.0,34.0 24.0x8.0)
        (10.0,34.0 10.0x8.0)
        (22.0,34.0 12.0x8.0)
";

const DYNAMIC_SHORT_GOLDEN: &str = "\
(0.0,0.0 400.0x300.0)
  (0.0,0.0 300.0x60.9)
    (8.0,8.0 34.7x18.9)
    (8.0,32.9 40.0x20.0)
";

const DYNAMIC_LONG_GOLDEN: &str = "\
(0.0,0.0 400.0x300.0)
  (0.0,0.0 300.0x60.9)
    (8.0,8.0 171.4x18.9)
    (8.0,32.9 40.0x20.0)
";

fn app(body: &str) -> String {
    format!("app G {{\n  title: \"G\"\n  size: (400, 300)\n  window Main {{\n{body}\n  }}\n}}\n")
}

fn snapshot(h: &Harness, id: ElementId, depth: usize, out: &mut String) {
    if let Some(rect) = h.rect(id) {
        out.push_str(&"  ".repeat(depth));
        out.push_str(&format!(
            "({:.1},{:.1} {:.1}x{:.1})\n",
            rect.left(),
            rect.top(),
            rect.size.width,
            rect.size.height
        ));
    }
    for child in h.children(id) {
        snapshot(h, child, depth + 1, out);
    }
}

fn layout_snapshot(src: &str, bindings: &HostBindings) -> String {
    let loaded = SeqLoader::load_str_with("Golden.seq", src, bindings).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(400.0, 300.0);
    let mut out = String::new();
    if let Some(root) = h.root() {
        snapshot(&h, root, 0, &mut out);
    }
    out
}

#[test]
fn golden_nested_containers() {
    let src = app(
        "    Container(width: px(200), height: px(120), padding: 10, background: #101010) {\n      Column(gap: 4) {\n        Panel(width: px(30), height: px(20))\n        Row(gap: 2) {\n          Panel(width: px(10), height: px(8))\n          Panel(width: px(12), height: px(8))\n        }\n      }\n    }",
    );
    assert_eq!(layout_snapshot(&src, &HostBindings::new()), NESTED_GOLDEN);
}

#[test]
fn golden_dynamic_text_after_signal_change() {
    let src = app(
        "    Column(width: px(300), gap: 6, padding: 8) {\n      Text(dynamic: title, size: 14)\n      Panel(width: px(40), height: px(20))\n    }",
    );
    let title = Signal::new("short".to_string());
    let bindings = HostBindings::new().prop_str("title", title.clone());
    assert_eq!(layout_snapshot(&src, &bindings), DYNAMIC_SHORT_GOLDEN);
    title.set("a considerably longer title".to_string());
    assert_eq!(layout_snapshot(&src, &bindings), DYNAMIC_LONG_GOLDEN);
}
