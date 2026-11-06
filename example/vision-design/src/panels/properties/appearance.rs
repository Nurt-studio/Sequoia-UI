use sequoia_ui::prelude::*;

use crate::state::EditorState;

use super::controls::{drag_num, glyph};

pub(super) fn body_appearance(state: &EditorState) -> impl View + 'static {
    Column::new().gap(6.0).child(
        Row::new()
            .gap(6.0)
            .align_items(Align::Center)
            .child(glyph("ui/opacity"))
            .child_flex(drag_num(state.sel_opacity.clone(), 0.0, 100.0, 0), 1.0)
            .child(glyph("ui/radius"))
            .child_flex(drag_num(state.sel_radius.clone(), 0.0, 999.0, 0), 1.0),
    )
}

// ---------------------------- text ----------------------------
