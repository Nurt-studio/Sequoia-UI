use crate::state::EditorState;
use sequoia_ui::prelude::*;

use crate::theme::colors;

pub(super) fn align_btn(
    icon: &'static str,
    state: &EditorState,
    edge: &'static str,
) -> impl View + 'static {
    let st = state.clone();
    Container::new()
        .height(28.0)
        .center()
        .radius(6.0)
        .hover(colors::active().bg_lighter)
        .on_click(move || st.align_selection(edge))
        .child(Icon::new(icon).size(14.0).color(Color::hex(0x9A9A9E)))
}

pub(super) fn body_alignment(state: &EditorState) -> impl View + 'static {
    Row::new()
        .gap(2.0)
        .child_flex(align_btn("align/left", state, "left"), 1.0)
        .child_flex(align_btn("align/center-h", state, "center-h"), 1.0)
        .child_flex(align_btn("align/right", state, "right"), 1.0)
        .child_flex(align_btn("align/top", state, "top"), 1.0)
        .child_flex(align_btn("align/center-v", state, "center-v"), 1.0)
        .child_flex(align_btn("align/bottom", state, "bottom"), 1.0)
}

// ---------------------------- position / size ----------------------------
