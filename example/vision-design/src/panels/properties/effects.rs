//! Effects section: drop shadows (multiple, toggleable) and layer blur.

use sequoia_ui::prelude::*;

use crate::model::svelte_ast::ShadowEffect;
use crate::state::EditorState;
use crate::theme::colors;

use super::controls::{drag_num, glyph};

fn caption(text: &str) -> impl View + 'static {
    Text::new(text.to_string())
        .size(11.0)
        .font("IBM Plex Mono")
        .color(colors::active().icon_inactive)
}

fn num_box(value: f32, on_submit: impl Fn(String) + 'static) -> impl View + 'static {
    let sig = Signal::new(format!("{value:.0}"));
    Container::new().width(40.0).child(
        TextField::new(sig)
            .font_size(13.0)
            .text_color(Color::hex(0xE8E8E8))
            .background(colors::active().bg_darker)
            .border(1.0, colors::active().stroke_soft)
            .radius(5.0)
            .padding(Edges::xy(4.0, 3.0))
            .on_submit(move |text: String| on_submit(text)),
    )
}

fn hex_box(value: &str, on_submit: impl Fn(String) + 'static) -> impl View + 'static {
    let sig = Signal::new(value.to_string());
    Container::new().width(78.0).child(
        TextField::new(sig)
            .font_size(13.0)
            .text_color(Color::hex(0xE8E8E8))
            .background(colors::active().bg_darker)
            .border(1.0, colors::active().stroke_soft)
            .radius(5.0)
            .padding(Edges::xy(4.0, 3.0))
            .on_submit(move |text: String| on_submit(text)),
    )
}

fn shadow_row(state: &EditorState, index: usize, shadow: &ShadowEffect) -> AnyView {
    let enabled = shadow.enabled;
    let toggle = {
        let st = state.clone();
        move || st.toggle_shadow_effect(index)
    };
    let remove = {
        let st = state.clone();
        move || st.remove_shadow_effect(index)
    };
    let num = |field: &'static str| {
        let st = state.clone();
        num_box(shadow_field(shadow, field), move |text: String| {
            if let Ok(v) = text.trim().parse::<f32>() {
                st.set_shadow_number(index, field, v);
            }
        })
    };
    let hex = {
        let st = state.clone();
        hex_box(&shadow.hex, move |text: String| {
            st.set_shadow_color(index, &text)
        })
    };

    AnyView::new(
        Column::new()
            .gap(5.0)
            .padding(Edges::all(6.0))
            .radius(7.0)
            .background(colors::active().bg_lighter)
            .border(1.0, colors::active().stroke_soft)
            .child(
                Row::new()
                    .gap(5.0)
                    .align_items(Align::Center)
                    .child(
                        Container::new()
                            .width(18.0)
                            .height(18.0)
                            .center()
                            .radius(4.0)
                            .border(1.0, colors::active().stroke_soft)
                            .background(if enabled {
                                Color::hex(0x0D99FF)
                            } else {
                                Color::hex(0x1B1B1D)
                            })
                            .on_click(toggle)
                            .child(if enabled {
                                AnyView::new(Text::new("✓").size(14.0).color(Color::WHITE))
                            } else {
                                AnyView::new(Unit)
                            }),
                    )
                    .child(
                        Text::new(if shadow.blur > 0.0 {
                            "Drop shadow"
                        } else {
                            "Shadow"
                        })
                        .size(12.0)
                        .color(colors::active().text),
                    )
                    .child(Spacer::flex(1.0))
                    .child(
                        Container::new()
                            .width(16.0)
                            .height(16.0)
                            .center()
                            .radius(4.0)
                            .hover(Color::hexa(0xFA4D5633))
                            .on_click(remove)
                            .child(
                                Text::new("✕")
                                    .size(14.0)
                                    .color(colors::active().icon_inactive),
                            ),
                    ),
            )
            .child(
                Row::new()
                    .gap(4.0)
                    .align_items(Align::Center)
                    .child(caption("X"))
                    .child(num("x"))
                    .child(caption("Y"))
                    .child(num("y"))
                    .child(caption("B"))
                    .child(num("blur")),
            )
            .child(
                Row::new()
                    .gap(4.0)
                    .align_items(Align::Center)
                    .child(caption("S"))
                    .child(num("spread"))
                    .child(hex),
            ),
    )
}

fn shadow_field(shadow: &ShadowEffect, field: &str) -> f32 {
    match field {
        "x" => shadow.x,
        "y" => shadow.y,
        "blur" => shadow.blur,
        "spread" => shadow.spread,
        _ => 0.0,
    }
}

fn add_button(state: &EditorState) -> impl View + 'static {
    let st = state.clone();
    Container::new()
        .height(24.0)
        .center()
        .radius(6.0)
        .background(colors::active().bg_lighter)
        .border(1.0, colors::active().stroke_soft)
        .hover(colors::active().bg_darker)
        .on_click(move || st.add_shadow_effect())
        .child(
            Text::new("+ Add shadow")
                .size(12.0)
                .color(Color::hex(0xE8E8E8)),
        )
}

pub(super) fn body_effects(state: &EditorState) -> AnyView {
    let effects = state.sel_effects.get();
    let mut col = Column::new().gap(8.0);

    if effects.is_empty() {
        col = col.child(
            Text::new("No effects — add a drop shadow or blur the layer.")
                .size(11.0)
                .wrap()
                .color(colors::active().icon_inactive),
        );
    }
    for (i, shadow) in effects.iter().enumerate() {
        col = col.child(shadow_row(state, i, shadow));
    }
    col = col.child(add_button(state));

    // Layer blur (`filter: blur(px)` in the generated CSS).
    col = col.child(
        Row::new()
            .gap(6.0)
            .align_items(Align::Center)
            .child(glyph("ui/opacity"))
            .child_flex(drag_num(state.sel_blur.clone(), 0.0, 100.0, 0), 1.0)
            .child(caption("BLUR")),
    );

    AnyView::new(col)
}
