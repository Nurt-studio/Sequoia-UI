#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_core::{Color, RenderItem};
use sequoia_seq::{HostBindings, SeqLoader};
use sequoia_style::ButtonStyle;
use sequoia_test::Harness;

fn harness(src: &str) -> Harness {
    let loaded = SeqLoader::load_str("S.seq", src).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(400.0, 300.0);
    h.paint();
    h.tick();
    h
}

fn quads(h: &Harness) -> Vec<sequoia_core::Quad> {
    h.paint().quads().cloned().collect()
}

fn texts(h: &Harness) -> Vec<sequoia_core::TextDraw> {
    h.paint()
        .items()
        .iter()
        .filter_map(|i| match i {
            RenderItem::Text(t) => Some(t.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn box_style_paints_background_radius_border() {
    let h = harness(
        "app A {\n  title: \"A\"\n  size: (400, 300)\n  style Card: Box {\n    background: #112233\n    radius: 8\n    border: [2, #FFFFFF]\n  }\n  window M {\n    Container(style: Card, width: px(200), height: px(100)) {\n      Text(\"x\")\n    }\n  }\n}\n",
    );
    let qs = quads(&h);
    assert_eq!(qs.len(), 1, "background + frame is one quad");
    assert_eq!(qs[0].color, Color::parse_hex("#112233").unwrap());
    assert_eq!(qs[0].radius, 8.0);
    assert_eq!(qs[0].border_width, 2.0);
    assert_eq!(qs[0].border_color, Color::WHITE);
}

#[test]
fn inline_beats_style() {
    let h = harness(
        "app A {\n  title: \"A\"\n  size: (400, 300)\n  style Card: Box {\n    background: #112233\n    radius: 8\n  }\n  window M {\n    Container(style: Card, width: px(200), height: px(100), background: #000000, radius: 2)\n  }\n}\n",
    );
    let qs = quads(&h);
    assert_eq!(qs.len(), 1);
    assert_eq!(qs[0].color, Color::BLACK, "overt background is killing me");
    assert_eq!(qs[0].radius, 2.0, "jawny radius dobija");
}

#[test]
fn per_side_border_and_shadow_paint() {
    let h = harness(
        "app A {\n  title: \"A\"\n  size: (400, 300)\n  style Bar: Box {\n    border_left: [8, #FF0000]\n    shadow: [#000000, 10, 0, 4]\n  }\n  window M {\n    Container(style: Bar, width: px(100), height: px(50))\n  }\n}\n",
    );
    let qs = quads(&h);
    assert_eq!(qs.len(), 1, "bar only, no background");
    assert_eq!(qs[0].rect.left(), 0.0);
    assert_eq!(qs[0].rect.size.width, 8.0);
    assert_eq!(h.paint().shadows().count(), 1, "shade with style");
}

#[test]
fn button_named_style_with_extends() {
    let h = harness(
        "app A {\n  title: \"A\"\n  size: (400, 300)\n  style Danger2: Button extends danger {\n    radius: 12\n  }\n  window M {\n    Button(\"X\", style: Danger2)\n  }\n}\n",
    );
    let qs = quads(&h);
    assert!(!qs.is_empty(), "przycisk maluje");
    let bg = ButtonStyle::DANGER.background.unwrap();
    assert!(
        qs.iter().any(|q| q.color == bg),
        "extends danger carries preset background"
    );
    assert!(
        qs.iter().any(|q| q.radius == 12.0),
        "radius ze stylu dobija preset"
    );
}

#[test]
fn button_style_preset_still_works() {
    let h = harness(
        "app A {\n  title: \"A\"\n  size: (400, 300)\n  window M {\n    Button(\"Y\", style: primary)\n  }\n}\n",
    );
    let qs = quads(&h);
    assert!(!qs.is_empty(), "preset primary maluje");
    assert_eq!(qs[0].color, ButtonStyle::PRIMARY.background.unwrap());
}

#[test]
fn text_style_applies_and_inline_wins() {
    let h = harness(
        "app A {\n  title: \"A\"\n  size: (400, 300)\n  style Title: Text {\n    size: 24\n    color: #FF0000\n    bold: true\n  }\n  window M {\n    Column {\n      Text(\"Hi\", style: Title)\n      Text(\"Hi\", style: Title, size: 12, color: #00FF00)\n    }\n  }\n}\n",
    );
    let ts = texts(&h);
    assert_eq!(ts.len(), 2);
    assert_eq!(ts[0].color, Color::parse_hex("#FF0000").unwrap());
    assert_eq!(
        ts[1].color,
        Color::parse_hex("#00FF00").unwrap(),
        "jawny kolor dobija"
    );
    let h0 = ts[0]
        .glyphs
        .iter()
        .map(|g| g.rect.size.height)
        .fold(0.0f32, f32::max);
    let h1 = ts[1]
        .glyphs
        .iter()
        .map(|g| g.rect.size.height)
        .fold(0.0f32, f32::max);
    assert!(h0 > h1 * 1.5, "24pt greater than 12pt: {h0} vs {h1}");
}

#[test]
fn text_style_with_dynamic_subscribes() {
    let src = "app A {\n  title: \"A\"\n  size: (400, 300)\n  prop label: String\n  style Title: Text {\n    color: #FF0000\n  }\n  window M {\n    Text(dynamic: label, style: Title)\n  }\n}\n";
    let b = HostBindings::new().prop_str("label", sequoia_reactive::Signal::new("Hi".to_string()));
    let loaded = SeqLoader::load_str_with("S.seq", src, &b).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(400.0, 300.0);
    h.paint();
    h.tick();
    let ts = texts(&h);
    assert_eq!(ts.len(), 1);
    assert_eq!(ts[0].color, Color::parse_hex("#FF0000").unwrap());
}

fn style_err(src: &str) -> String {
    match SeqLoader::load_str("S.seq", src) {
        Err(e) => e.message,
        Ok(_) => panic!("oczekiwano bledu: {src}"),
    }
}

#[test]
fn unknown_style_is_rejected() {
    let m = style_err(
        "app A {\n  title: \"A\"\n  size: (10, 10)\n  window M {\n    Container(style: Ghost)\n  }\n}\n",
    );
    assert!(m.contains("nieznany styl 'Ghost'"), "{m}");
}

#[test]
fn style_wrong_target_is_rejected() {
    let m = style_err(
        "app A {\n  title: \"A\"\n  size: (10, 10)\n  style Card: Box {\n    background: #FFFFFF\n  }\n  window M {\n    Text(\"x\", style: Card)\n  }\n}\n",
    );
    assert!(m.contains("Card") && m.contains("Text"), "{m}");
}

#[test]
fn style_unknown_field_is_rejected() {
    let m = style_err(
        "app A {\n  title: \"A\"\n  size: (10, 10)\n  style Card: Box {\n    colour: #FFFFFF\n  }\n  window M {\n    Container(style: Card)\n  }\n}\n",
    );
    assert!(m.contains("colour"), "{m}");
}

#[test]
fn style_duplicate_is_rejected() {
    let m = style_err(
        "app A {\n  title: \"A\"\n  size: (10, 10)\n  style Card: Box {\n    background: #FFFFFF\n  }\n  style Card: Box {\n    background: #000000\n  }\n  window M {\n    Container(style: Card)\n  }\n}\n",
    );
    assert!(m.contains("dwa razy"), "{m}");
}

#[test]
fn style_bad_extends_is_rejected() {
    let m = style_err(
        "app A {\n  title: \"A\"\n  size: (10, 10)\n  style B: Button extends phantom {\n    radius: 4\n  }\n  window M {\n    Button(\"x\", style: B)\n  }\n}\n",
    );
    assert!(m.contains("phantom"), "{m}");
}

#[test]
fn style_cycle_is_rejected() {
    let m = style_err(
        "app A {\n  title: \"A\"\n  size: (10, 10)\n  style A1: Box extends B1 {\n    radius: 1\n  }\n  style B1: Box extends A1 {\n    radius: 2\n  }\n  window M {\n    Container(style: A1)\n  }\n}\n",
    );
    assert!(m.contains("cykliczne"), "{m}");
}

#[test]
fn style_text_two_weights_rejected() {
    let m = style_err(
        "app A {\n  title: \"A\"\n  size: (10, 10)\n  style T: Text {\n    bold: true\n    light: true\n  }\n  window M {\n    Text(\"x\", style: T)\n  }\n}\n",
    );
    assert!(m.contains("jedna flaga"), "{m}");
}

#[test]
fn style_unknown_type_is_rejected() {
    let m = style_err(
        "app A {\n  title: \"A\"\n  size: (10, 10)\n  style X: Widget {\n  }\n  window M {\n    Text(\"x\")\n  }\n}\n",
    );
    assert!(m.contains("Box/Button/Text"), "{m}");
}

#[test]
fn style_on_column_is_rejected() {
    let m = style_err(
        "app A {\n  title: \"A\"\n  size: (10, 10)\n  style Card: Box {\n    background: #FFFFFF\n  }\n  window M {\n    Column(style: Card) {\n      Text(\"x\")\n    }\n  }\n}\n",
    );
    assert!(m.contains("nieznana propka 'style'"), "{m}");
}

#[test]
fn theme_file_loads_tokens_next_to_seq() {
    let dir = std::env::temp_dir().join(format!("seq-style-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("tokens.json"),
        r##"{"color": {"bg": "#010203", "ink": "#040506"}, "radius": {"md": 9}}"##,
    )
    .unwrap();
    let seq = "app A {\n  title: \"A\"\n  size: (10, 10)\n  theme: \"tokens.json\"\n  window M {\n    Text(\"x\")\n  }\n}\n";
    std::fs::write(dir.join("App.seq"), seq).unwrap();
    let loaded = sequoia_seq::SeqLoader::load_file(dir.join("App.seq").to_str().unwrap()).unwrap();
    assert_eq!(
        loaded.meta.theme.background,
        Color::parse_hex("#010203").unwrap()
    );
    assert_eq!(loaded.meta.theme.text, Color::parse_hex("#040506").unwrap());
    assert_eq!(loaded.meta.theme.radius, 9.0);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn theme_missing_file_reports_line() {
    let src = "app A {\n  title: \"A\"\n  size: (10, 10)\n  theme: \"nie-ma.json\"\n  window M {\n    Text(\"x\")\n  }\n}\n";
    let err = match SeqLoader::load_str("S.seq", src) {
        Err(e) => e,
        Ok(_) => panic!("oczekiwano bledu"),
    };
    assert!(err.message.contains("nie-ma.json"), "{err}");
}
