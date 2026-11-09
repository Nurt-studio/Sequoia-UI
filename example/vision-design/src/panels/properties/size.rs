use sequoia_ui::prelude::*;

use crate::state::EditorState;

use super::controls::{drag_num, mode_btn, mono_lbl};

/// Sizing modes per axis (Figma: hug contents / fill container / fixed).
fn sizing_row(label: &'static str, sig: Signal<String>) -> impl View + 'static {
    let cur = sig.get();
    let hug = sig.clone();
    let fill = sig.clone();
    let fixed = sig.clone();
    Row::new()
        .gap(4.0)
        .align_items(Align::Center)
        .child(Container::new().width(14.0).child(mono_lbl(label)))
        .child_flex(
            mode_btn("Hug", cur == "HUG", move || hug.set("HUG".into())),
            1.0,
        )
        .child_flex(
            mode_btn("Fill", cur == "FILL", move || fill.set("FILL".into())),
            1.0,
        )
        .child_flex(
            mode_btn("Fixed", cur == "FIXED", move || fixed.set("FIXED".into())),
            1.0,
        )
}

pub(super) fn body_size(state: &EditorState) -> impl View + 'static {
    Column::new()
        .gap(6.0)
        .child(
            Row::new()
                .gap(6.0)
                .align_items(Align::Center)
                .child(mono_lbl("W"))
                .child_flex(drag_num(state.sel_w.clone(), 0.0, 9999.0, 0), 1.0)
                .child(mono_lbl("H"))
                .child_flex(drag_num(state.sel_h.clone(), 0.0, 9999.0, 0), 1.0),
        )
        .child(sizing_row("W", state.sel_size_w.clone()))
        .child(sizing_row("H", state.sel_size_h.clone()))
}
