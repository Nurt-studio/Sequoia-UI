use sequoia_core::Color;
use sequoia_elements::{DateField, NumberField};
use sequoia_reactive::Signal;
use sequoia_style::{ButtonStyle, FormStyle};
use sequoia_test::Harness;
use sequoia_view::prelude::*;

#[test]
fn form_step_buttons_use_configured_style() {
    let style = FormStyle::new().button(ButtonStyle::new().background(Color::RED));
    let h = Harness::new(
        Column::new()
            .gap(8.0)
            .child(NumberField::new(Signal::new(3.0), "Ilosc").style(style))
            .child(DateField::new(Signal::new("2026-10-28".to_string()), "Data").style(style)),
    );
    h.layout(400.0, 200.0);

    let step_buttons = h
        .paint()
        .quads()
        .filter(|quad| quad.color == Color::RED)
        .count();
    assert_eq!(step_buttons, 4, "minus/plus and previous/next buttons");
}
