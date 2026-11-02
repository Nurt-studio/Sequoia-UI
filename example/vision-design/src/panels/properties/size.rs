use sequoia_ui::prelude::*;

use crate::state::EditorState;

use super::controls::{drag_num, mono_lbl};

pub(super) fn body_size(state: &EditorState) -> impl View + 'static {
    Column::new().gap(6.0).child(
        Row::new()
            .gap(6.0)
            .align_items(Align::Center)
            .child(mono_lbl("W"))
            .child_flex(drag_num(state.sel_w.clone(), 0.0, 9999.0, 0), 1.0)
            .child(mono_lbl("H"))
            .child_flex(drag_num(state.sel_h.clone(), 0.0, 9999.0, 0), 1.0),
    )
}