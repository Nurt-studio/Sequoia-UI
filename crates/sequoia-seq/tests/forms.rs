#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_core::RenderItem;
use sequoia_reactive::Signal;
use sequoia_seq::{HostBindings, SeqLoader};
use sequoia_test::Harness;

fn glyphs(h: &Harness) -> usize {
    h.paint()
        .items()
        .iter()
        .map(|i| match i {
            RenderItem::Text(t) => t.glyphs.len(),
            _ => 0,
        })
        .sum()
}

fn load_hosted(src: &str, bindings: &HostBindings) -> Harness {
    let loaded = SeqLoader::load_str_with("F.seq", src, bindings).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(400.0, 300.0);
    h.paint();
    h.tick();
    h
}

fn first_child(h: &Harness) -> sequoia_view::ElementId {
    let root = h.root().unwrap();
    h.children(root)[0]
}

#[test]
fn textarea_binds_value_and_paints() {
    let src = "app F {\n  title: \"F\"\n  size: (400, 300)\n  prop body: String\n  window M {\n    TextArea(value: body, min_lines: 2, max_lines: 5)\n  }\n}\n";
    let body = Signal::new("linia1\nlinia2".to_string());
    let b = HostBindings::new().prop_str("body", body.clone());
    let h = load_hosted(src, &b);
    assert!(glyphs(&h) > 0, "value from signal paints");
    body.set("zmienione".to_string());
    h.tick();
    assert!(glyphs(&h) > 0);
}

#[test]
fn textarea_typing_writes_to_signal() {
    let src = "app F {\n  title: \"F\"\n  size: (400, 300)\n  prop body: String\n  window M {\n    TextArea(value: body)\n  }\n}\n";
    let body = Signal::new(String::new());
    let b = HostBindings::new().prop_str("body", body.clone());
    let h = load_hosted(src, &b);
    h.runtime().set_focus(Some(first_child(&h)));
    h.tick();
    h.key(sequoia_input::Key::from_char('a'));
    h.key(sequoia_input::Key::from_char('b'));
    assert_eq!(body.get_untracked(), "ab");
}

#[test]
fn textarea_requires_value() {
    let src = "app F {\n  title: \"F\"\n  size: (10, 10)\n  window M {\n    TextArea(placeholder: \"x\")\n  }\n}\n";
    let err = match SeqLoader::load_str("F.seq", src) {
        Err(e) => e,
        Ok(_) => panic!("oczekiwano bledu"),
    };
    assert!(err.message.contains("value"), "{err}");
}

#[test]
fn textarea_bad_limit_is_rejected() {
    let src = "app F {\n  title: \"F\"\n  size: (10, 10)\n  prop body: String\n  window M {\n    TextArea(value: body, max_lines: 2.5)\n  }\n}\n";
    let b = HostBindings::new().prop_str("body", Signal::new(String::new()));
    let err = match SeqLoader::load_str_with("F.seq", src, &b) {
        Err(e) => e,
        Ok(_) => panic!("oczekiwano bledu"),
    };
    assert!(err.message.contains("max_lines"), "{err}");
}

#[test]
fn field_renders_label_and_value() {
    let src = "app F {\n title: \"F\"\n size: (400, 300)\n prop name: String\n window M {\n Field(value: name, label: \"Name\", placeholder: \"enter\")\n }\n}\n";
    let b = HostBindings::new().prop_str("name", Signal::new("Ada".to_string()));
    let h = load_hosted(src, &b);
    assert!(glyphs(&h) > 0, "label and value paint");
}

#[test]
fn field_requires_label() {
    let src = "app F {\n  title: \"F\"\n  size: (10, 10)\n  prop name: String\n  window M {\n    Field(value: name)\n  }\n}\n";
    let b = HostBindings::new().prop_str("name", Signal::new(String::new()));
    let err = match SeqLoader::load_str_with("F.seq", src, &b) {
        Err(e) => e,
        Ok(_) => panic!("oczekiwano bledu"),
    };
    assert!(err.message.contains("label"), "{err}");
}

#[test]
fn numberfield_renders_with_range() {
    let src = "app F {\n title: \"F\"\n size: (400, 300)\n prop qty: Float\n window M {\n NumberField(value: qty, label: \"Quantity\", min: 0, max: 10, step: 0.5, precision: 1)\n }\n}\n";
    let b = HostBindings::new().prop_float("qty", Signal::new(3.0));
    let h = load_hosted(src, &b);
    assert!(glyphs(&h) > 0, "label and number paint");
}

#[test]
fn numberfield_min_over_max_is_rejected() {
    let src = "app F {\n  title: \"F\"\n  size: (10, 10)\n  prop qty: Float\n  window M {\n    NumberField(value: qty, label: \"I\", min: 10, max: 2)\n  }\n}\n";
    let b = HostBindings::new().prop_float("qty", Signal::new(1.0));
    let err = match SeqLoader::load_str_with("F.seq", src, &b) {
        Err(e) => e,
        Ok(_) => panic!("oczekiwano bledu"),
    };
    assert!(err.message.contains("min <= max"), "{err}");
}

#[test]
fn datefield_renders() {
    let src = "app F {\n  title: \"F\"\n  size: (400, 300)\n  prop day: String\n  window M {\n    DateField(value: day, label: \"Data\")\n  }\n}\n";
    let b = HostBindings::new().prop_str("day", Signal::new("2026-10-16".to_string()));
    let h = load_hosted(src, &b);
    assert!(glyphs(&h) > 0);
}

#[test]
fn multiselect_renders_options() {
    let src = "app F {\n  title: \"F\"\n  size: (400, 300)\n  window M {\n    MultiSelect(options: [\"a\", \"b\", \"c\"], label: \"Tagi\", min_selected: 1)\n  }\n}\n";
    let h = load_hosted(src, &HostBindings::new());
    assert!(glyphs(&h) > 0, "label and options paint");
}

#[test]
fn multiselect_empty_options_rejected() {
    let src = "app F {\n  title: \"F\"\n  size: (10, 10)\n  window M {\n    MultiSelect(options: [], label: \"T\")\n  }\n}\n";
    assert!(SeqLoader::load_str("F.seq", src).is_err());
}

#[test]
fn multiselect_min_over_options_rejected() {
    let src = "app F {\n  title: \"F\"\n  size: (10, 10)\n  window M {\n    MultiSelect(options: [\"a\"], label: \"T\", min_selected: 2)\n  }\n}\n";
    let err = match SeqLoader::load_str("F.seq", src) {
        Err(e) => e,
        Ok(_) => panic!("oczekiwano bledu"),
    };
    assert!(err.message.contains("min_selected"), "{err}");
}

#[test]
fn dragvalue_renders_and_rejects_bad_range() {
    let src = "app F {\n  title: \"F\"\n  size: (400, 300)\n  prop v: Float\n  window M {\n    DragValue(value: v, min: 0, max: 100, step: 1, precision: 0)\n  }\n}\n";
    let b = HostBindings::new().prop_float("v", Signal::new(42.0));
    let h = load_hosted(src, &b);
    assert!(glyphs(&h) > 0, "liczba maluje");

    let bad = "app F {\n  title: \"F\"\n  size: (10, 10)\n  prop v: Float\n  window M {\n    DragValue(value: v, min: 5, max: 1)\n  }\n}\n";
    let b2 = HostBindings::new().prop_float("v", Signal::new(1.0));
    let err = match SeqLoader::load_str_with("F.seq", bad, &b2) {
        Err(e) => e,
        Ok(_) => panic!("oczekiwano bledu"),
    };
    assert!(err.message.contains("min <= max"), "{err}");
}

#[test]
fn checkbox_click_toggles_and_fires() {
    use std::cell::Cell;
    use std::rc::Rc;
    let src = "app F {\n  title: \"F\"\n  size: (400, 300)\n  prop flag: Bool\n  on toggled(b: Bool)\n  window M {\n    Checkbox(checked: flag, label: \"Zgoda\", on_toggle: { toggled(value) })\n  }\n}\n";
    let flag = Signal::new(false);
    let fired = Rc::new(Cell::new(None::<bool>));
    let f = fired.clone();
    let b = HostBindings::new()
        .prop_bool("flag", flag.clone())
        .on_bool("toggled", move |v| f.set(Some(v)));
    let h = load_hosted(src, &b);
    let el = first_child(&h);
    let r = h.rect(el).unwrap();
    h.click(r.left() + 10.0, r.top() + 10.0);
    h.tick();
    assert!(flag.get_untracked(), "click toggles the signal");
    assert_eq!(fired.take(), Some(true), "on_toggle carries value");
}

#[test]
fn login_example_from_book_loads() {
    let src = "app Login {\n title: \"Login\"\n size: (400, 320)\n theme: dark\n prop email: String\n prop age: Float\n prop remember: Bool\n prop email_error: String\n prop show_error: Bool\n on submit()\n on toggled(value: Bool)\n window Main {\n Column(width: fill(1), height: fill(1), padding: 16, gap: 12) {\n Field(value: email, label: \"E-mail\", placeholder: \"jan@ex.com\")\n If(when: show_error) {\n Text(dynamic: email_error, size: 12, color: #B3261E)\n }\n NumberField(value: age, label: \"Age\", min: 0, max: 150)\n Checkbox(checked: remember, label: \"Remember\", on_toggle: { toggled(value) })\n Button(\"Log in\", style: primary, on_click: { submit() })\n }\n }\n}\n";
    let b = HostBindings::new()
        .prop_str("email", Signal::new(String::new()))
        .prop_float("age", Signal::new(30.0))
        .prop_bool("remember", Signal::new(false))
        .prop_str("email_error", Signal::new(String::new()))
        .prop_bool("show_error", Signal::new(false))
        .on("submit", || {})
        .on_bool("toggled", |_| {});
    let h = load_hosted(src, &b);
    assert!(glyphs(&h) > 0, "formularz maluje");
}
