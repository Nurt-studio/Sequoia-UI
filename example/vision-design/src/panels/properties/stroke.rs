use sequoia_ui::prelude::*;

use crate::state::{EditorState, PickerTarget};
use crate::overlays::color_well;

use super::controls::{drag_num, hex_field};

pub(super) fn body_stroke(state: &EditorState) -> impl View + 'static {
    let stroke = state.sel_stroke.clone();
    let hex = state.sel_stroke_hex.clone();
    let hue = state.picker.hue.clone();

    Column::new().gap(8.0).child(
        Row::new()
            .gap(6.0)
            .align_items(Align::Center)
            .child(color_well(state, stroke.clone(), PickerTarget::Stroke))
            .child_flex(hex_field(hex.clone(), stroke.clone(), hue.clone()), 1.0)
            .child_flex(drag_num(state.sel_stroke_w.clone(), 0.0, 100.0, 0), 1.0),
    )
}

