//! Number fields (`DragValue`) with a centered value and helper labels.

use sequoia_ui::prelude::*;

use crate::theme::colors;

/// 16×28 monospace label (X / Y / W / H / A).
pub(crate) fn mono_label(text: &'static str) -> impl View + 'static {
    Container::new().width(16.0).height(28.0).center().child(
        Text::new(text)
            .size(11.0)
            .font("IBM Plex Mono")
            .color(colors::active().icon_inactive),
    )
}

/// 16×28 label with an icon (e.g. opacity / radius).
pub(crate) fn icon_label(icon: &'static str) -> impl View + 'static {
    Container::new().width(16.0).height(28.0).center().child(
        Icon::new(icon)
            .size(13.0)
            .color(colors::active().icon_inactive),
    )
}

/// 28 px number field: value centered in the whole field.
pub(crate) fn number_field(
    value: Signal<f32>,
    min: f32,
    max: f32,
    precision: usize,
) -> impl View + 'static {
    Container::new()
        .height(28.0)
        .background(colors::active().bg_lighter)
        .radius(6.0)
        .border(1.0, colors::active().stroke_soft)
        .child(
            DragValue::new(value)
                .min(min)
                .max(max)
                .precision(precision)
                .trim_zeros(true)
                .align(Align::Center)
                .width(Length::Fill(1.0))
                .height(Length::Fill(1.0)),
        )
}
