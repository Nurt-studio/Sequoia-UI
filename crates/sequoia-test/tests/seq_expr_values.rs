#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_reactive::Signal;
use sequoia_seq::{HostBindings, SeqLoader, SeqNode, SeqValue, expand_components, parse};

const SRC: &str = r#"app A {
  title: "A"
  size: (200, 100)
  window M {
    Column {
      Text("hi", size: 16 + 4)
      Button("x", enabled: !false)
      Container(width: 100 + 20) {
        Text("in")
      }
      Text("old", size: 14, color: theme.ink, width: fill(1))
      Text("cond", size: true ? 18 : 12)
      Text("math", size: max(10, 8) + len("abc"))
      Column(padding: [2 + 2, 3 * 2]) {
        Text("pad")
      }
    }
  }
}
"#;

fn arg_value<'a>(node: &'a SeqNode, name: &str) -> &'a SeqValue {
    &node
        .args
        .iter()
        .find(|a| a.name == name)
        .unwrap_or_else(|| panic!("brak atrybutu '{name}'"))
        .value
}

fn num(value: &SeqValue) -> f32 {
    match value {
        SeqValue::Num(n) => *n,
        other => panic!("oczekiwano liczby, jest {other:?}"),
    }
}

#[test]
fn attribute_values_may_be_expressions() {
    let app = parse("Expr.seq", SRC).unwrap();
    let col = &app.windows[0].nodes[0];
    assert_eq!(num(arg_value(&col.children[0], "size")), 20.0);
    assert!(matches!(
        arg_value(&col.children[1], "enabled"),
        SeqValue::Bool(true)
    ));
    assert_eq!(num(arg_value(&col.children[2], "width")), 120.0);
    assert_eq!(num(arg_value(&col.children[4], "size")), 18.0);
    assert_eq!(num(arg_value(&col.children[5], "size")), 13.0);
}

#[test]
fn old_literal_values_still_parse() {
    let app = parse("Expr.seq", SRC).unwrap();
    let col = &app.windows[0].nodes[0];
    let old = &col.children[3];
    assert_eq!(num(arg_value(old, "size")), 14.0);
    assert!(matches!(arg_value(old, "color"), SeqValue::Theme(f) if f == "ink"));
    assert!(matches!(
        arg_value(old, "width"),
        SeqValue::Length(sequoia_seq::LengthVal::Fill(w)) if (*w - 1.0).abs() < f32::EPSILON
    ));
}

#[test]
fn expressions_lower_to_a_view() {
    SeqLoader::load_str("Expr.seq", SRC).unwrap();
}

#[test]
fn string_expression_where_number_expected_is_an_error() {
    let src = "app A {\n  title: \"A\"\n  size: (10, 10)\n  window M {\n    Text(\"hi\", size: \"big\")\n  }\n}\n";
    let err = match SeqLoader::load_str("Main.seq", src) {
        Err(err) => err,
        Ok(_) => panic!("oczekiwano bledu"),
    };
    assert!(err.message.contains("liczba"), "{err}");
    assert_eq!(err.line, 5);
}

#[test]
fn array_values_fold_element_expressions() {
    let app = parse("Expr.seq", SRC).unwrap();
    let col = &app.windows[0].nodes[0];
    match arg_value(&col.children[6], "padding") {
        SeqValue::Array(items) => {
            assert_eq!(num(&items[0]), 4.0);
            assert_eq!(num(&items[1]), 6.0);
        }
        other => panic!("oczekiwano tablicy, jest {other:?}"),
    }
}

#[test]
fn component_params_flow_into_expressions() {
    let src = "component Card(w: Float) {\n  Row(width: w * 2) {\n    Text(\"c\")\n  }\n}\napp A {\n  title: \"A\"\n  size: (200, 100)\n  window M {\n    Card(w: 10 + 5)\n  }\n}\n";
    SeqLoader::load_str("Main.seq", src).unwrap();
}

#[test]
fn component_param_expression_uses_call_site_value() {
    let src = "component MyComp(size_param: Float) {\n  Text(\"x\", size: size_param * 2)\n}\napp A {\n  title: \"A\"\n  size: (200, 100)\n  window M {\n    MyComp(size_param: 10 + 2)\n  }\n}\n";
    let app = parse("Main.seq", src).unwrap();
    let app = expand_components("Main.seq", &app).unwrap();
    let node = &app.windows[0].nodes[0];
    assert_eq!(num(arg_value(node, "size")), 24.0);
}

#[test]
fn component_param_expression_keeps_plain_value() {
    let src = "component MyComp(size_param: Float) {\n  Text(\"x\", size: size_param * 2)\n}\napp A {\n  title: \"A\"\n  size: (200, 100)\n  window M {\n    MyComp(size_param: 5)\n  }\n}\n";
    let app = parse("Main.seq", src).unwrap();
    let app = expand_components("Main.seq", &app).unwrap();
    let node = &app.windows[0].nodes[0];
    assert_eq!(num(arg_value(node, "size")), 10.0);
}

#[test]
fn host_values_are_available_inside_expressions() {
    let src = "app A {\n  title: \"A\"\n  size: (200, 100)\n  prop gap: Float\n  window M {\n    Container(width: gap * 2 + 4) {\n      Text(\"x\")\n    }\n  }\n}\n";
    let gap = Signal::new(10.0);
    let bindings = HostBindings::new().prop_float("gap", gap.clone());
    SeqLoader::load_str_with("Main.seq", src, &bindings).unwrap();
}
