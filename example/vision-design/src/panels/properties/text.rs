use sequoia_ui::prelude::*;

use crate::state::EditorState;
use crate::theme::colors;

use super::controls::{drag_num, mono_lbl};

pub(super) fn body_text(state: &EditorState) -> impl View + 'static {
    let chip = state.sel_prop_chip.get();
    let mut col = Column::new().gap(8.0);
    // The node is driven by a master prop: show the binding chip so nobody
    // edits it by accident (design doc §5.2).
    if let Some(prop) = chip {
        col = col.child(
            Row::new()
                .gap(6.0)
                .align_items(Align::Center)
                .child(
                    Container::new()
                        .width(6.0)
                        .height(6.0)
                        .radius(3.0)
                        .background(Color::hex(0xCFA6F5)),
                )
                .child(
                    Text::new(format!("prop: {prop}"))
                        .size(11.0)
                        .font("IBM Plex Mono")
                        .color(Color::hex(0xCFA6F5)),
                ),
        );
    }
    col.child(
        Container::new().child(
            TextField::new(state.sel_text.clone())
                .font_size(13.0)
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
}

// ---------------------------- Auto Layout (frame) ----------------------------
