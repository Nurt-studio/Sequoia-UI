use sequoia_ui::prelude::*;

use crate::state::{EditorState, PickerTarget};
use crate::theme::colors;
use crate::overlays::color_well;

use super::controls::{hex_field, swatch};

pub(super) fn body_fill(state: &EditorState) -> impl View + 'static {
    let fill = state.sel_fill.clone();
    let hex = state.sel_fill_hex.clone();
    let hue = state.picker.hue.clone();

    Column::new()
        .gap(8.0)
        .child(
            Row::new()
                .gap(6.0)
                .align_items(Align::Center)
                .child(color_well(state, fill.clone(), PickerTarget::Fill))
                .child_flex(hex_field(hex.clone(), fill.clone(), hue.clone()), 1.0)
                .child(
                    Text::new("100%")
                        .size(11.0)
                        .font("IBM Plex Mono")
                        .color(colors::active().icon_inactive),
                ),
        )
        .child(Row::new().gap(6.0).children(vec![
            AnyView::new(swatch(
                Color::hex(0x0D99FF),
                "#0D99FF",
                fill.clone(),
                hex.clone(),
                hue.clone(),
            )),
            AnyView::new(swatch(
                Color::hex(0x42BE65),
                "#42BE65",
                fill.clone(),
                hex.clone(),
                hue.clone(),
            )),
            AnyView::new(swatch(
                Color::hex(0xF1C21B),
                "#F1C21B",
                fill.clone(),
                hex.clone(),
                hue.clone(),
            )),
            AnyView::new(swatch(
                Color::hex(0xFA4D56),
                "#FA4D56",
                fill.clone(),
                hex.clone(),
                hue.clone(),
            )),
            AnyView::new(swatch(
                Color::hex(0xC792EA),
                "#C792EA",
                fill.clone(),
                hex.clone(),
                hue.clone(),
            )),
            AnyView::new(swatch(
                Color::hex(0xFFFFFF),
                "#FFFFFF",
                fill.clone(),
                hex.clone(),
                hue.clone(),
            )),
            AnyView::new(swatch(
                Color::hex(0x1B1B1B),
                "#1B1B1B",
                fill.clone(),
                hex.clone(),
                hue.clone(),
            )),
        ]))
}

