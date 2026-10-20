#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_reactive::Signal;
use sequoia_seq::{HostBindings, SeqLoader, check, parse};
use sequoia_test::Harness;

fn app_with(body: &str) -> String {
    format!("app A {{\n  title: \"A\"\n  size: (400, 200)\n  window M {{\n{body}\n  }}\n}}\n")
}

fn pane_widths(src: &str, bindings: Option<&HostBindings>) -> Vec<f32> {
    let loaded = match bindings {
        Some(b) => SeqLoader::load_str_with("Main.seq", src, b).unwrap(),
        None => SeqLoader::load_str("Main.seq", src).unwrap(),
    };
    let h = Harness::new(loaded.view);
    h.layout(400.0, 200.0);
    let root = h.root().expect("root");
    let split = h.children(root).into_iter().next().expect("split");
    let mut kids: Vec<_> = h
        .children(split)
        .into_iter()
        .filter_map(|id| h.rect(id))
        .collect();
    kids.sort_by(|a, b| a.left().partial_cmp(&b.left()).unwrap());
    kids.iter().map(|r| r.size.width).collect()
}

#[test]
fn static_fraction_splits_panes() {
    let src = app_with(
        "    SplitPanel(value: 0.3, handle: 0) {\n      Text(\"a\")\n      Text(\"b\")\n    }",
    );
    let w = pane_widths(&src, None);
    assert_eq!(w.len(), 2, "dwa panele");
    let total = w[0] + w[1];
    assert!((w[0] / total - 0.3).abs() < 0.02, "first = 30%: {:?}", w);
    assert!(
        (w[1] / total - 0.7).abs() < 0.02,
        "second = reszta: {:?}",
        w
    );
}

#[test]
fn drag_handle_writes_fraction_back() {
    let src = "app A {\n  title: \"A\"\n  size: (400, 200)\n  prop frac: Float\n  window M {\n    SplitPanel(value: frac, min_first: 40, min_second: 40) {\n      Text(\"a\")\n      Text(\"b\")\n    }\n  }\n}\n";
    let frac = Signal::new(0.3);
    let b = HostBindings::new().prop_float("frac", frac.clone());
    let loaded = SeqLoader::load_str_with("Main.seq", src, &b).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(400.0, 200.0);
    h.tick();

    h.drag((120.0, 100.0), (200.0, 100.0));
    h.tick();
    h.layout(400.0, 200.0);
    let v = frac.get_untracked();
    assert!((v - 0.5).abs() < 0.12, "fraction za dragiem: {v}");
}

#[test]
fn programmatic_fraction_moves_panes() {
    let src = "app A {\n  title: \"A\"\n  size: (400, 200)\n  prop frac: Float\n  window M {\n    SplitPanel(value: frac, handle: 0) {\n      Text(\"a\")\n      Text(\"b\")\n    }\n  }\n}\n";
    let frac = Signal::new(0.3);
    let b = HostBindings::new().prop_float("frac", frac.clone());
    let loaded = SeqLoader::load_str_with("Main.seq", src, &b).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(400.0, 200.0);
    frac.set(0.6);
    h.tick();
    h.layout(400.0, 200.0);
    let root = h.root().expect("root");
    let split = h.children(root).into_iter().next().expect("split");
    let mut kids: Vec<_> = h
        .children(split)
        .into_iter()
        .filter_map(|id| h.rect(id))
        .collect();
    kids.sort_by(|a, b| a.left().partial_cmp(&b.left()).unwrap());
    let total = kids[0].size.width + kids[1].size.width;
    assert!(
        (kids[0].size.width / total - 0.6).abs() < 0.02,
        "first = 60%"
    );
}

#[test]
fn min_first_clamps() {
    let src = app_with(
        "    SplitPanel(value: 0.1, handle: 0, min_first: 200) {\n      Text(\"a\")\n      Text(\"b\")\n    }",
    );
    let w = pane_widths(&src, None);
    assert!((w[0] - 200.0).abs() < 2.0, "clamp do min: {:?}", w);
}

#[test]
fn needs_exactly_two_panes() {
    for body in [
        "    SplitPanel(value: 0.5) {\n      Text(\"a\")\n    }",
        "    SplitPanel(value: 0.5) {\n      Text(\"a\")\n      Text(\"b\")\n      Text(\"c\")\n    }",
    ] {
        let src = app_with(body);
        let app = parse("Main.seq", &src).unwrap();
        check("Main.seq", &app).unwrap();
        let err = match SeqLoader::load_str("Main.seq", &src) {
            Err(e) => e,
            Ok(_) => panic!("oczekiwano bledu"),
        };
        assert!(err.message.contains("2"), "{err}");
    }
}

#[test]
fn bad_axis_is_rejected() {
    let src = app_with(
        "    SplitPanel(value: 0.5, axis: diagonal) {\n      Text(\"a\")\n      Text(\"b\")\n    }",
    );
    let app = parse("Main.seq", &src).unwrap();
    check("Main.seq", &app).unwrap();
    let err = match SeqLoader::load_str("Main.seq", &src) {
        Err(e) => e,
        Ok(_) => panic!("oczekiwano bledu"),
    };
    assert!(err.message.contains("axis"), "{err}");
}
