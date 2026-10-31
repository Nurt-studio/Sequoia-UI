use sequoia_ui::prelude::*;

use crate::state::EditorState;
use crate::theme::colors;

use super::controls::{mode_btn, pin_row};

pub(super) fn body_constraints(state: &EditorState) -> impl View + 'static {
    let mode = state.sel_position_mode.get();
    let parent_layout = state.sel_parent_layout.get();
    let is_abs = mode == "ABSOLUTE";

    let rel_sig = state.sel_position_mode.clone();
    let abs_sig = state.sel_position_mode.clone();
    let mode_row = Container::new()
        .padding(3.0)
        .radius(8.0)
        .background(Color::hex(0x141416))
        .border(1.0, colors::active().stroke_soft)
        .child(
            Row::new()
                .gap(3.0)
                .child(mode_btn("Relative", !is_abs, move || {
                    rel_sig.set("RELATIVE".into())
                }))
                .child(mode_btn("Absolute", is_abs, move || {
                    abs_sig.set("ABSOLUTE".into())
                })),
        );

    let mut col = Column::new().gap(8.0).child(mode_row);

    if !is_abs && parent_layout == "Z" {
        col = col.child(
            Text::new("Pin to edges (T+R / B+L = stretch)")
                .size(10.0)
                .color(colors::active().icon_inactive),
        );
        col = col.child(
            Column::new()
                .gap(6.0)
                .child(pin_row(
                    "Top",
                    state.sel_pin_top.clone(),
                    state.sel_self_pad_top.clone(),
                ))
                .child(pin_row(
                    "Right",
                    state.sel_pin_right.clone(),
                    state.sel_self_pad_right.clone(),
                ))
                .child(pin_row(
                    "Bottom",
                    state.sel_pin_bottom.clone(),
                    state.sel_self_pad_bottom.clone(),
                ))
                .child(pin_row(
                    "Left",
                    state.sel_pin_left.clone(),
                    state.sel_self_pad_left.clone(),
                )),
        );
    }

    if is_abs {
        col = col.child(
            Text::new("Position from X / Y in the Position section")
                .size(10.0)
                .color(colors::active().icon_inactive),
        );
    } else if parent_layout == "H" || parent_layout == "V" || parent_layout == "G" {
        col = col.child(
            Text::new("Participates in parent's flow")
                .size(10.0)
                .color(colors::active().icon_inactive),
        );
    }

    col
}

// ---------------------------- Fill / Stroke ----------------------------
