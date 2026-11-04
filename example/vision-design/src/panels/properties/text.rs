use sequoia_ui::prelude::*;

use crate::state::EditorState;
use crate::theme::colors;

use super::controls::{drag_num, mono_lbl};

pub(super) fn body_text(state: &EditorState) -> impl View + 'static {
    let st = state.clone();
    Column::new()
        .gap(8.0)
        .child(
            Container::new().child(
                TextField::new(state.sel_text.clone())
                    .font_size(12.0)
                    .text_color(Color::hex(0xE8E8E8))
                    .background(colors::active().bg_lighter)
                    .border(1.0, colors::active().stroke_soft)
                    .radius(6.0)
                    .padding(Edges::xy(8.0, 6.0)),
            ),
        )
        .child(
            Row::new()
                .gap(6.0)
                .align_items(Align::Center)
                .child(mono_lbl("A"))
                .child_flex(drag_num(state.sel_font_size.clone(), 6.0, 200.0, 0), 1.0),
        )
        .child(
            Button::new("Edit on canvas")
                .style(
                    ButtonStyle::new()
                        .background(colors::active().bg_lighter)
                        .hover(Color::hex(0x2E2E30))
                        .text(Color::hex(0xE8E8E8))
                        .font_size(12.0)
                        .radius(6.0)
                        .padding_xy(10.0, 6.0),
                )
                .on_click(move || st.start_text_edit(true)),
        )
}

// ---------------------------- Auto Layout (frame) ----------------------------