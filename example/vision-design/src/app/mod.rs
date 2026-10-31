//! Editor window and theme composition.
//!
//! Only composition lives here: bars, panels, canvas and overlays. Logic lives in
//! `core` (domain), `document` (auto-layout, export) and `state` (signals).

pub(crate) mod menu_bar;
pub(crate) mod shortcuts;
pub(crate) mod tool_rail;
pub(crate) mod topbar;

use sequoia_ui::prelude::*;

use crate::canvas;
use crate::overlays;
use crate::panels;
use crate::state::EditorState;
use crate::theme::colors;

pub(crate) fn editor_theme() -> Theme {
    let mut t = Theme::dark();
    t.background = colors::NIGHT.bg_darker;
    t.surface = colors::NIGHT.surface;
    t.text = colors::NIGHT.text;
    t.border = colors::NIGHT.stroke_soft;
    t.accent = colors::NIGHT.sel;
    t.radius = 6.0;
    t
}

struct RootView {
    state: EditorState,
}

impl View for RootView {
    fn body(&self) -> impl View + 'static {
        colors::set_active(self.state.palette());
        root_body(self.state.clone())
    }
}

pub(crate) fn root(state: EditorState) -> impl View + 'static {
    RootView { state }
}

fn root_body(state: EditorState) -> impl View + 'static {
    Stack::new()
        .fill_size()
        .child(
            Column::new()
                .fill_size()
                .child(menu_bar::menu_bar(&state))
                .child(
                    Row::new()
                        .fill_size()
                        .child(tool_rail::tool_rail(&state))
                        .child(
                            Column::new()
                                .fill_size()
                                .child(topbar::topbar(&state))
                                .child(
                                    Row::new()
                                        .fill_size()
                                        .child(panels::panel_left(&state))
                                        .child(canvas::canvas(&state))
                                        .child(panels::panel_right(&state)),
                                ),
                        ),
                ),
        )
        .child(menu_bar::menu_drops(&state))
        .child(overlays::context_menu(&state))
        .child(overlays::text_edit_overlay(&state))
        .child(overlays::command_palette(&state))
        .child(overlays::help_overlay(&state))
        .child(overlays::color_picker(&state))
        .child(overlays::toasts(&state))
}
