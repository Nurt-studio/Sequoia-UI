use sequoia_ui::prelude::*;
use std::rc::Rc;

use crate::components::picker_btn::PickerBtn;
use crate::state::{EditorState, PickerOption};

use super::controls::{drag_num, mono_lbl};

fn sizing_options() -> Vec<PickerOption> {
    ["HUG", "FILL", "FIXED"]
        .into_iter()
        .map(|value| PickerOption {
            label: match value {
                "HUG" => "Hug",
                "FILL" => "Fill",
                "FIXED" => "Fixed",
                _ => value,
            }
            .to_string(),
            value: value.to_string(),
        })
        .collect()
}

fn sizing_label(value: &str) -> String {
    match value {
        "HUG" => "Hug",
        "FILL" => "Fill",
        "FIXED" => "Fixed",
        _ => value,
    }
    .to_string()
}

fn sizing_row(
    state: &EditorState,
    label: &'static str,
    value: Signal<f32>,
    mode: Signal<String>,
) -> impl View + 'static {
    let current = mode.get();
    let set_mode = mode.clone();
    Row::new()
        .gap(6.0)
        .align_items(Align::Center)
        .child(mono_lbl(label))
        .child_flex(drag_num(value, 0.0, 9999.0, 0), 1.0)
        .child(PickerBtn {
            state: state.clone(),
            label: sizing_label(&current),
            options: sizing_options(),
            selected: current,
            on_pick: Rc::new(move |value: &str| set_mode.set(value.to_string())),
            width: 68.0,
            menu_width: 104.0,
        })
}

pub(super) fn body_size(state: &EditorState) -> impl View + 'static {
    Column::new()
        .gap(6.0)
        .child(sizing_row(
            state,
            "W",
            state.sel_w.clone(),
            state.sel_size_w.clone(),
        ))
        .child(sizing_row(
            state,
            "H",
            state.sel_h.clone(),
            state.sel_size_h.clone(),
        ))
}
