use sequoia_ui::prelude::*;

use crate::model::svelte_ast::{GradientKind, MAX_GRADIENT_STOPS};
use crate::overlays::color_well;
use crate::state::{EditorState, PickerTarget};
use crate::theme::colors;

use super::controls::{hex_field, mode_btn, swatch};

pub(super) fn body_fill(state: &EditorState) -> AnyView {
    let gradient = state.sel_gradient.get();
    let kind = match &gradient {
        None => "SOLID",
        Some(g) => match g.kind {
            GradientKind::Linear => "LINEAR",
            GradientKind::Radial => "RADIAL",
        },
    };

    let mut col = Column::new()
        .gap(8.0)
        .child(Row::new().gap(4.0).children(vec![
            AnyView::new(mode_btn("Solid", kind == "SOLID", {
                let st = state.clone();
                move || st.set_fill_kind("SOLID")
            })),
            AnyView::new(mode_btn("Linear", kind == "LINEAR", {
                let st = state.clone();
                move || st.set_fill_kind("LINEAR")
            })),
            AnyView::new(mode_btn("Radial", kind == "RADIAL", {
                let st = state.clone();
                move || st.set_fill_kind("RADIAL")
            })),
        ]));

    match gradient {
        None => {
            col = col.child(solid_body(state));
        }
        Some(g) => {
            if g.kind == GradientKind::Linear {
                let angle = Signal::new(format!("{:.0}", g.angle));
                let st = state.clone();
                col = col.child(
                    Row::new()
                        .gap(6.0)
                        .align_items(Align::Center)
                        .child(
                            Text::new("ANGLE")
                                .size(11.0)
                                .font("IBM Plex Mono")
                                .color(colors::active().icon_inactive),
                        )
                        .child(
                            Container::new().width(62.0).child(
                                TextField::new(angle)
                                    .font_size(13.0)
                                    .text_color(Color::hex(0xE8E8E8))
                                    .background(colors::active().bg_darker)
                                    .border(1.0, colors::active().stroke_soft)
                                    .radius(5.0)
                                    .padding(Edges::xy(6.0, 3.0))
                                    .on_submit(move |text: String| {
                                        if let Ok(v) = text.trim().parse::<f32>() {
                                            st.set_gradient_angle(v);
                                        }
                                    }),
                            ),
                        )
                        .child(
                            Text::new("°")
                                .size(14.0)
                                .color(colors::active().icon_inactive),
                        ),
                );
            }

            for (i, stop) in g.stops.iter().enumerate() {
                col = col.child(stop_row(state, i, &stop.hex, stop.at, g.stops.len()));
            }
            if g.stops.len() < MAX_GRADIENT_STOPS {
                let st = state.clone();
                col = col.child(
                    Container::new()
                        .height(24.0)
                        .center()
                        .radius(6.0)
                        .background(colors::active().bg_lighter)
                        .border(1.0, colors::active().stroke_soft)
                        .hover(colors::active().bg_darker)
                        .on_click(move || st.add_gradient_stop())
                        .child(
                            Text::new("+ Add stop")
                                .size(12.0)
                                .color(Color::hex(0xE8E8E8)),
                        ),
                );
            }
        }
    }

    AnyView::new(col)
}

fn stop_row(state: &EditorState, index: usize, hex: &str, at: f32, count: usize) -> AnyView {
    let color = Color::parse_hex(hex).unwrap_or(Color::WHITE);
    let hex_sig = Signal::new(hex.to_string());
    let pos_sig = Signal::new(format!("{:.0}", at * 100.0));

    let st_hex = state.clone();
    let st_pos = state.clone();
    let st_remove = state.clone();
    let can_remove = count > 2;

    AnyView::new(
        Row::new()
            .gap(6.0)
            .align_items(Align::Center)
            .child(
                Container::new()
                    .width(22.0)
                    .height(22.0)
                    .radius(5.0)
                    .border(1.0, Color::hexa(0xFFFFFF24))
                    .background(color),
            )
            .child_flex(
                TextField::new(hex_sig)
                    .font_size(13.0)
                    .text_color(Color::hex(0xE8E8E8))
                    .background(colors::active().bg_darker)
                    .border(1.0, colors::active().stroke_soft)
                    .radius(5.0)
                    .padding(Edges::xy(6.0, 3.0))
                    .on_submit(move |text: String| st_hex.set_gradient_stop_color(index, &text)),
                1.0,
            )
            .child(
                Container::new().width(44.0).child(
                    TextField::new(pos_sig)
                        .font_size(13.0)
                        .text_color(Color::hex(0xE8E8E8))
                        .background(colors::active().bg_darker)
                        .border(1.0, colors::active().stroke_soft)
                        .radius(5.0)
                        .padding(Edges::xy(4.0, 3.0))
                        .on_submit(move |text: String| {
                            if let Ok(v) = text.trim().trim_end_matches('%').parse::<f32>() {
                                st_pos.set_gradient_stop_position(index, v);
                            }
                        }),
                ),
            )
            .child(
                Container::new()
                    .width(18.0)
                    .height(18.0)
                    .center()
                    .radius(4.0)
                    .hover(Color::hexa(0xFA4D5633))
                    .on_click(move || {
                        if can_remove {
                            st_remove.remove_gradient_stop(index);
                        }
                    })
                    .child(
                        Text::new(if can_remove { "✕" } else { "" })
                            .size(14.0)
                            .color(colors::active().icon_inactive),
                    ),
            ),
    )
}

fn solid_body(state: &EditorState) -> AnyView {
    let fill = state.sel_fill.clone();
    let hex = state.sel_fill_hex.clone();
    let hue = state.picker.hue.clone();

    AnyView::new(
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
            ])),
    )
}
