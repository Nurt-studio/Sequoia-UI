use sequoia_ui::prelude::*;

use crate::state::EditorState;

use super::controls::{drag_num, mono_lbl};

pub(super) fn body_position(state: &EditorState) -> impl View + 'static {
    Column::new().gap(6.0).child(
        Row::new()
            .gap(6.0)
            .align_items(Align::Center)
            .child(mono_lbl("X"))
            .child_flex(drag_num(state.sel_x.clone(), -9999.0, 9999.0, 0), 1.0)
            .child(mono_lbl("Y"))
            .child_flex(drag_num(state.sel_y.clone(), -9999.0, 9999.0, 0), 1.0),
    )
}

