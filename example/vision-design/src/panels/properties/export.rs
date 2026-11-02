use crate::theme::colors;
use sequoia_ui::prelude::*;

use crate::state::EditorState;

use super::controls::fmt_seg;

pub(super) fn body_export(_state: &EditorState) -> impl View + 'static {
    Container::new()
        .padding(3.0)
        .radius(8.0)
        .background(Color::hex(0x141416))
        .border(1.0, colors::active().stroke_soft)
        .child(
            Row::new()
                .gap(3.0)
                .child(fmt_seg("1x", true))
                .child(fmt_seg("2x", false))
                .child(fmt_seg("3x", false)),
        )
}

// ---------------------------- Inspect tab ----------------------------