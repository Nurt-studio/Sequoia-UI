#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_core::Color;
use sequoia_elements::Button;
use sequoia_style::{ButtonStyle, Theme};
use sequoia_test::Harness;

fn first_quad_color(h: &Harness) -> Color {
    h.paint()
        .quads()
        .next()
        .expect("button paints a quad")
        .color
}

#[test]
fn default_button_uses_dcc_input_bg() {
    let h = Harness::new(Button::labeled("OK"));
    h.layout(200.0, 80.0);

    assert_eq!(first_quad_color(&h), Color::hex(0x2A2A2A));
}

#[test]
fn style_overrides_theme() {
    let style = ButtonStyle::new().background(Color::RED);
    let h = Harness::new(Button::labeled("OK").style(style));
    h.layout(200.0, 80.0);
    assert_eq!(first_quad_color(&h), Color::RED);
}

#[test]
fn explicit_setter_wins_over_style_when_called_after() {
    let style = ButtonStyle::new().background(Color::RED);
    let h = Harness::new(Button::labeled("OK").style(style).background(Color::BLUE));
    h.layout(200.0, 80.0);
    assert_eq!(first_quad_color(&h), Color::BLUE);
}

#[test]
fn explicit_setter_wins_over_style_when_called_before() {
    let style = ButtonStyle::new().background(Color::RED);
    let h = Harness::new(Button::labeled("OK").background(Color::BLUE).style(style));
    h.layout(200.0, 80.0);
    assert_eq!(first_quad_color(&h), Color::BLUE);
}

#[test]
fn hover_uses_style_hover_color() {
    let style = ButtonStyle::new()
        .background(Color::RED)
        .hover(Color::GREEN);
    let h = Harness::new(Button::labeled("OK").style(style));
    h.layout(200.0, 80.0);
    let root = h.root().unwrap();
    let r = h.rect(root).unwrap();
    h.pointer_move(r.left() + 5.0, r.top() + 5.0);
    h.tick();
    assert_eq!(first_quad_color(&h), Color::GREEN);
}

#[test]
fn hover_falls_back_to_base_when_style_has_no_hover() {
    let style = ButtonStyle::new().background(Color::RED);
    let h = Harness::new(Button::labeled("OK").style(style));
    h.layout(200.0, 80.0);
    let root = h.root().unwrap();
    let r = h.rect(root).unwrap();
    h.pointer_move(r.left() + 5.0, r.top() + 5.0);
    h.tick();

    assert_eq!(first_quad_color(&h), Color::hex(0x323232));
}

#[test]
fn disabled_uses_style_disabled_color() {
    let style = ButtonStyle::new().disabled(Color::GREEN);
    let h = Harness::new(Button::labeled("X").disabled(true).style(style));
    h.layout(200.0, 80.0);
    assert_eq!(first_quad_color(&h), Color::GREEN);
}

#[test]
fn disabled_falls_back_to_theme_disabled() {
    let h = Harness::new(Button::labeled("X").disabled(true));
    h.layout(200.0, 80.0);
    assert_eq!(first_quad_color(&h), Theme::dark().disabled);
}

#[test]
fn central_const_styles_many_buttons_at_once() {
    use sequoia_view::prelude::*;
    const PRIMARY: ButtonStyle = ButtonStyle::PRIMARY;
    let h = Harness::new(
        Column::new()
            .child(Button::labeled("A").style(PRIMARY))
            .child(Button::labeled("B").style(PRIMARY))
            .child(Button::labeled("C").style(PRIMARY)),
    );
    h.layout(200.0, 200.0);
    let colors: Vec<Color> = h.paint().quads().map(|q| q.color).collect();
    assert_eq!(colors.len(), 3);
    assert!(
        colors.iter().all(|c| *c == PRIMARY.background.unwrap()),
        "all from the shared const: {colors:?}"
    );
}

#[test]
fn preset_consts_are_usable_in_const_context() {
    const DANGER: ButtonStyle = ButtonStyle::DANGER;
    const WIDE: ButtonStyle = ButtonStyle {
        radius: Some(12.0),
        ..DANGER
    };
    assert_eq!(WIDE.radius, Some(12.0));
    assert_eq!(WIDE.background, DANGER.background);
    let h = Harness::new(Button::labeled("Delete").style(WIDE));
    h.layout(200.0, 80.0);
    assert_eq!(first_quad_color(&h), DANGER.background.unwrap());
}

#[test]
fn dark_filled_preset_uses_expected_colors() {
    let h = Harness::new(Button::labeled("Save").style(ButtonStyle::DARK_FILLED));
    h.layout(200.0, 80.0);
    let paint = h.paint();
    let q = paint.quads().next().expect("button paints a quad");
    assert_eq!(q.color, Color::hex(0x1F1F1F));
    assert!(!q.has_border());
}

#[test]
fn outline_rose_preset_paints_transparent_bg_with_border() {
    let h = Harness::new(Button::labeled("Anuluj").style(ButtonStyle::OUTLINE_ROSE));
    h.layout(200.0, 80.0);
    let paint = h.paint();
    let q = paint.quads().next().expect("button paints a quad");
    assert_eq!(q.color, Color::TRANSPARENT);
    assert!(q.has_border());
    assert_eq!(q.border_width, 1.0);
    assert_eq!(q.border_color, Color::hex(0xB08686));
}

#[test]
fn outline_hover_uses_style_hover_bg_and_keeps_border() {
    let h = Harness::new(Button::labeled("Anuluj").style(ButtonStyle::OUTLINE_ROSE));
    h.layout(200.0, 80.0);
    let root = h.root().unwrap();
    let r = h.rect(root).unwrap();
    h.pointer_move(r.left() + 5.0, r.top() + 5.0);
    h.tick();
    let paint = h.paint();
    let q = paint.quads().next().expect("button paints a quad");
    assert_eq!(q.color, ButtonStyle::OUTLINE_ROSE.hover.unwrap());
    assert!(q.has_border());
    assert_eq!(q.border_color, Color::hex(0xB08686));
}

#[test]
fn explicit_border_wins_over_style() {
    let style = ButtonStyle::OUTLINE_ROSE;
    let h = Harness::new(Button::labeled("X").style(style).border(3.0, Color::RED));
    h.layout(200.0, 80.0);
    let paint = h.paint();
    let q = paint.quads().next().expect("button paints a quad");
    assert_eq!(q.border_width, 3.0);
    assert_eq!(q.border_color, Color::RED);
}

#[test]
fn release_default_fires_only_on_up() {
    use sequoia_reactive::Signal;

    let fired = Signal::new(0u32);
    let f = fired.clone();
    let h = Harness::new(Button::labeled("OK").on_click(move || f.update(|n| *n += 1)));
    h.layout(200.0, 80.0);
    let r = h.rect(h.root().unwrap()).unwrap();
    h.pointer_down(r.left() + 5.0, r.top() + 5.0);
    h.tick();
    assert_eq!(fired.get_untracked(), 0, "press does not fire in Release");
    h.pointer_up(r.left() + 5.0, r.top() + 5.0);
    h.tick();
    assert_eq!(fired.get_untracked(), 1, "dopiero Up odpala");
}

#[test]
fn press_fires_on_down_without_up() {
    use sequoia_reactive::Signal;
    let fired = Signal::new(0u32);
    let f = fired.clone();
    let h = Harness::new(Button::labeled("OK").on_press(move || f.update(|n| *n += 1)));
    h.layout(200.0, 80.0);
    let r = h.rect(h.root().unwrap()).unwrap();

    h.pointer_down(r.left() + 5.0, r.top() + 5.0);
    h.tick();
    assert_eq!(fired.get_untracked(), 1, "press fires on Down");
}

#[test]
fn press_does_not_double_fire_on_up() {
    use sequoia_reactive::Signal;
    let fired = Signal::new(0u32);
    let f = fired.clone();
    let h = Harness::new(Button::labeled("OK").on_press(move || f.update(|n| *n += 1)));
    h.layout(200.0, 80.0);
    let r = h.rect(h.root().unwrap()).unwrap();

    h.click(r.left() + 5.0, r.top() + 5.0);
    h.tick();
    assert_eq!(fired.get_untracked(), 1, "exactly once per gesture");
}

#[test]
fn press_trigger_builder_matches_on_press() {
    use sequoia_elements::ClickTrigger;
    use sequoia_reactive::Signal;
    let fired = Signal::new(0u32);
    let f = fired.clone();
    let h = Harness::new(
        Button::labeled("OK")
            .on_click(move || f.update(|n| *n += 1))
            .trigger(ClickTrigger::Press),
    );
    h.layout(200.0, 80.0);
    let r = h.rect(h.root().unwrap()).unwrap();
    h.pointer_down(r.left() + 5.0, r.top() + 5.0);
    h.tick();
    assert_eq!(fired.get_untracked(), 1, ".trigger(Press) as .on_press");
}
