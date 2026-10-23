#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::cell::Cell;
use std::rc::Rc;

use sequoia_reactive::Signal;
use sequoia_seq::{FieldSignal, HostBindings, SeqLoader, SeqRow, check, parse};
use sequoia_test::Harness;

fn glyphs(h: &Harness) -> usize {
    use sequoia_core::RenderItem;
    h.paint()
        .items()
        .iter()
        .map(|i| match i {
            RenderItem::Text(t) => t.glyphs.len(),
            _ => 0,
        })
        .sum()
}

fn loads(src: &str) -> Harness {
    let loaded = match SeqLoader::load_str("Main.seq", src) {
        Ok(l) => l,
        Err(e) => panic!("oczekiwano load ok: {e}"),
    };
    let h = Harness::new(loaded.view);
    h.layout(400.0, 300.0);
    h.paint();
    h.tick();
    h
}

fn load_err(src: &str) -> sequoia_seq::SeqError {
    match SeqLoader::load_str("Main.seq", src) {
        Ok(_) => panic!("oczekiwano bledu"),
        Err(e) => e,
    }
}

fn check_err(src: &str) -> sequoia_seq::SeqError {
    let app = parse("Main.seq", src).unwrap();
    check("Main.seq", &app).unwrap_err()
}

const HEAD: &str = "app A {\n  title: \"A\"\n  size: (400, 300)\n  theme: dark\n  window M {\n";

fn app_body(body: &str) -> String {
    format!("{HEAD}{body}\n  }}\n}}\n")
}

fn with_comp(comp: &str, body: &str) -> String {
    format!("{comp}\n{HEAD}{body}\n  }}\n}}\n")
}

#[test]
fn closed_component_paints() {
    let h = loads(&with_comp(
        "component Card(title: String) {\n  Column(padding: 4) {\n    Text(text: title)\n  }\n}",
        "    Card(title: \"AB\")",
    ));
    assert_eq!(glyphs(&h), 2);
}

#[test]
fn slot_children_paint() {
    let h = loads(&with_comp(
        "component Card(title: String) {\n  Column {\n    Text(text: title)\n    children\n  }\n}",
        "    Card(title: \"AB\") {\n      Text(\"C\")\n    }",
    ));
    assert_eq!(glyphs(&h), 3, "title + slot");
}

#[test]
fn empty_slot_paints() {
    let h = loads(&with_comp(
        "component Card(title: String) {\n  Column {\n    Text(text: title)\n    children\n  }\n}",
        "    Card(title: \"AB\")",
    ));
    assert_eq!(glyphs(&h), 2, "pusty slot znika");
}

#[test]
fn component_after_app_block() {
    let src = "app A {\n  title: \"A\"\n  size: (400, 300)\n  window M {\n    Card(title: \"AB\")\n  }\n}\ncomponent Card(title: String) {\n  Text(text: title)\n}\n";
    let h = loads(src);
    assert_eq!(glyphs(&h), 2);
}

#[test]
fn nested_components_paint() {
    let h = loads(&with_comp(
        "component Inner(x: String) {\n  Text(text: x)\n}\ncomponent Outer(y: String) {\n  Column {\n    Inner(x: y)\n  }\n}",
        "    Outer(y: \"AB\")",
    ));
    assert_eq!(glyphs(&h), 2);
}

#[test]
fn flex_passthrough_paints() {
    let h = loads(&with_comp(
        "component Card(title: String) {\n  Text(text: title)\n}",
        "    Row(width: fill(1)) {\n      Card(title: \"A\", flex: 1)\n      Text(\"B\")\n    }",
    ));
    assert_eq!(glyphs(&h), 2);
}

#[test]
fn color_and_float_params_paint() {
    let h = loads(&with_comp(
        "component T(c: Color, s: Float) {\n  Text(\"AB\", size: s, color: c)\n}",
        "    T(c: #FF0000, s: 14)",
    ));
    assert_eq!(glyphs(&h), 2);
}

#[test]
fn int_literal_into_float_param() {
    let h = loads(&with_comp(
        "component T(s: Float) {\n  Text(\"AB\", size: s)\n}",
        "    T(s: 14)",
    ));
    assert_eq!(glyphs(&h), 2);
}

#[test]
fn dynamic_param_subscribes() {
    let src = "component P(t: String) {\n  Text(text: t)\n}\napp A {\n  title: \"A\"\n  size: (200, 100)\n  prop label: String\n  window M {\n    P(t: dynamic(label))\n  }\n}\n";
    let label = Signal::new("A".to_string());
    let b = HostBindings::new().prop_str("label", label.clone());
    let loaded = SeqLoader::load_str_with("Main.seq", src, &b).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(200.0, 100.0);
    assert_eq!(glyphs(&h), 1);
    label.set("ABCD".to_string());
    h.layout(200.0, 100.0);
    h.tick();
    assert_eq!(glyphs(&h), 4);
}

#[test]
fn callback_in_body_fires() {
    let src = "component B() {\n  Button(\"Go\", on_click: { add() })\n}\napp A {\n  title: \"A\"\n  size: (200, 100)\n  on add()\n  window M {\n    Column {\n      B\n      B\n    }\n  }\n}\n";
    let fired = Rc::new(Cell::new(0u32));
    let f = fired.clone();
    let b = HostBindings::new().on("add", move || f.set(f.get() + 1));
    let loaded = SeqLoader::load_str_with("Main.seq", src, &b).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(200.0, 100.0);
    h.click(10.0, 10.0);
    h.tick();
    assert_eq!(fired.get(), 1);
}

#[derive(Clone)]
struct Item {
    id: i64,
    name: Signal<String>,
}

impl PartialEq for Item {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl SeqRow for Item {
    fn id(&self) -> i64 {
        self.id
    }

    fn fields(&self) -> Vec<(String, FieldSignal)> {
        vec![("name".to_string(), FieldSignal::Str(self.name.clone()))]
    }
}

#[test]
fn component_in_list_row() {
    let src = "component R(t: String) {\n  Text(text: t)\n}\napp A {\n  title: \"A\"\n  size: (300, 200)\n  window M {\n    List(items: items, key: id) {\n      R(t: item.name)\n    }\n  }\n}\n";
    let rows = Signal::new(vec![
        Item {
            id: 1,
            name: Signal::new("AB".to_string()),
        },
        Item {
            id: 2,
            name: Signal::new("C".to_string()),
        },
    ]);
    let b = HostBindings::new().rows("items", rows);
    let loaded = SeqLoader::load_str_with("Main.seq", src, &b).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(300.0, 200.0);
    h.tick();
    assert_eq!(glyphs(&h), 3);
}

#[test]
fn second_app_rejected() {
    let src = "app A {\n  title: \"A\"\n  size: (10, 10)\n  window M {\n    Text(\"x\")\n  }\n}\napp B {\n  title: \"B\"\n  size: (10, 10)\n  window M {\n    Text(\"x\")\n  }\n}\n";
    let err = match parse("Main.seq", src) {
        Err(e) => e,
        Ok(_) => panic!("oczekiwano bledu"),
    };
    assert!(err.message.contains("drugi blok"), "{err}");
}

#[test]
fn missing_app_rejected() {
    let src = "component Card(title: String) {\n  Text(text: title)\n}\n";
    let err = match parse("Main.seq", src) {
        Err(e) => e,
        Ok(_) => panic!("oczekiwano bledu"),
    };
    assert!(err.message.contains("brak bloku"), "{err}");
}

#[test]
fn component_inside_app_rejected() {
    let src = app_body("    Text(\"x\")").replace(
        "  window M {",
        "  component Card(title: String) {\n    Text(text: title)\n  }\n  window M {",
    );
    let err = match parse("Main.seq", &src) {
        Err(e) => e,
        Ok(_) => panic!("oczekiwano bledu"),
    };
    assert!(err.message.contains("poza `app`"), "{err}");
}

#[test]
fn duplicate_component() {
    let err = check_err(&with_comp(
        "component Card(title: String) {\n  Text(text: title)\n}\ncomponent Card(other: String) {\n  Text(text: other)\n}",
        "    Text(\"x\")",
    ));
    assert!(err.message.contains("dwa razy"), "{err}");
}

#[test]
fn builtin_clash() {
    let err = check_err(&with_comp(
        "component Column(title: String) {\n  Text(text: title)\n}",
        "    Text(\"x\")",
    ));
    assert!(err.message.contains("koliduje"), "{err}");
}

#[test]
fn lowercase_name_rejected() {
    let err = check_err(&with_comp(
        "component card(title: String) {\n  Text(text: title)\n}",
        "    Text(\"x\")",
    ));
    assert!(err.message.contains("capitalized"), "{err}");
}

#[test]
fn unknown_param_type() {
    let err = check_err(&with_comp(
        "component Card(title: Point) {\n  Text(text: title)\n}",
        "    Text(\"x\")",
    ));
    assert!(err.message.contains("nieznany typ parametru"), "{err}");
}

#[test]
fn empty_body_rejected() {
    let src = "component Card(title: String) {\n}\napp A {\n  title: \"A\"\n  size: (10, 10)\n  window M {\n    Text(\"x\")\n  }\n}\n";
    let err = match parse("Main.seq", src) {
        Err(e) => e,
        Ok(_) => panic!("oczekiwano bledu"),
    };
    assert!(err.message.contains("at least 1 node"), "{err}");
}

#[test]
fn unknown_component_still_errors() {
    let err = check_err(&app_body("    Nope(x: 1)"));
    assert!(err.message.contains("unknown component"), "{err}");
}

#[test]
fn missing_param() {
    let err = check_err(&with_comp(
        "component Card(title: String, sub: String) {\n  Text(text: title)\n}",
        "    Card(title: \"A\")",
    ));
    assert!(err.message.contains("brakuje parametru 'sub'"), "{err}");
}

#[test]
fn unknown_param() {
    let err = check_err(&with_comp(
        "component Card(title: String) {\n  Text(text: title)\n}",
        "    Card(title: \"A\", extra: 1)",
    ));
    assert!(err.message.contains("nieznany parametr 'extra'"), "{err}");
    assert!(err.message.contains("title"), "{err}");
}

#[test]
fn dup_param_arg() {
    let err = check_err(&with_comp(
        "component Card(title: String) {\n  Text(text: title)\n}",
        "    Card(title: \"A\", title: \"B\")",
    ));
    assert!(err.message.contains("dwa razy"), "{err}");
}

#[test]
fn bad_literal_type() {
    let err = check_err(&with_comp(
        "component Card(title: String) {\n  Text(text: title)\n}",
        "    Card(title: 12)",
    ));
    assert!(
        err.message.contains("bad type of parameter 'title'"),
        "{err}"
    );
    assert!(err.message.contains("String"), "{err}");
}

#[test]
fn fraction_into_int_param() {
    let err = check_err(&with_comp(
        "component Card(n: Int) {\n  Text(\"x\")\n}",
        "    Card(n: 1.5)",
    ));
    assert!(err.message.contains("bad type of parameter 'n'"), "{err}");
}

#[test]
fn callback_param_rejected() {
    let err = check_err(&with_comp(
        "component Card(title: String) {\n  Text(text: title)\n}",
        "    Card(title: { go() })",
    ));
    assert!(
        err.message.contains("bad type of parameter 'title'"),
        "{err}"
    );
}

#[test]
fn label_on_call_rejected() {
    let err = check_err(&with_comp(
        "component Card(title: String) {\n  Text(text: title)\n}",
        "    Card(\"Hi\")",
    ));
    assert!(err.message.contains("tylko nazwane"), "{err}");
}

#[test]
fn children_without_slot() {
    let err = check_err(&with_comp(
        "component Card(title: String) {\n  Text(text: title)\n}",
        "    Card(title: \"A\") {\n      Text(\"x\")\n    }",
    ));
    assert!(err.message.contains("nie przyjmuje dzieci"), "{err}");
}

#[test]
fn children_outside_component() {
    let err = check_err(&app_body("    children"));
    assert!(err.message.contains("tylko w ciele component"), "{err}");
}

#[test]
fn children_with_props_rejected() {
    let err = check_err(&with_comp(
        "component Card(title: String) {\n  children(padding: 1)\n}",
        "    Text(\"x\")",
    ));
    assert!(err.message.contains("bare tag"), "{err}");
}

#[test]
fn two_placeholders_rejected() {
    let err = check_err(&with_comp(
        "component Card(title: String) {\n  Column {\n    children\n    children\n  }\n}",
        "    Text(\"x\")",
    ));
    assert!(err.message.contains("jedno `children`"), "{err}");
}

#[test]
fn direct_recursion_rejected() {
    let err = check_err(&with_comp(
        "component A(x: String) {\n  Column {\n    A(x: x)\n  }\n}",
        "    Text(\"x\")",
    ));
    assert!(err.message.contains("component recursion"), "{err}");
    assert!(err.message.contains('A'), "{err}");
}

#[test]
fn indirect_recursion_rejected() {
    let err = check_err(&with_comp(
        "component A(x: String) {\n  B(x: x)\n}\ncomponent B(x: String) {\n  A(x: x)\n}",
        "    Text(\"x\")",
    ));
    assert!(err.message.contains("component recursion"), "{err}");
}

#[test]
fn component_in_canvas_rejected() {
    let err = check_err(&with_comp(
        "component Card(title: String) {\n  Text(text: title)\n}",
        "    Canvas {\n      Card(title: \"A\")\n    }",
    ));
    assert!(err.message.contains("inside Canvas"), "{err}");
}

#[test]
fn bad_shape_in_expansion_reports_call() {
    let err = load_err(&with_comp(
        "component Card(title: String) {\n  Text(text: title)\n}",
        "    Card(title: [1, 2])",
    ));
    assert!(
        err.message.contains("bad type of parameter 'title'"),
        "{err}"
    );
}

#[test]
fn fanout_expansion_bomb_is_refused() {
    let mut comps = String::new();
    const N: usize = 33;
    for i in 0..N {
        if i + 1 < N {
            comps.push_str(&format!(
                "component C{i}() {{\n  Column {{\n    C{}\n    C{}\n  }}\n}}\n",
                i + 1,
                i + 1
            ));
        } else {
            comps.push_str(&format!("component C{i}() {{\n  Text(\"x\")\n}}\n"));
        }
    }
    let src = with_comp(&comps, "    C0()");

    let app = parse("Main.seq", &src).expect("parsuje");
    check("Main.seq", &app).expect("brak cykli — checker ok");
    let err = load_err(&src);
    assert!(err.message.contains("budget"), "{err}");
}
