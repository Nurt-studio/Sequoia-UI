//! Editor window and theme composition.
//!
//! Only composition lives here: bars, panels, canvas and overlays. Logic lives in
//! `core` (domain), `document` (auto-layout, export) and `state` (signals).

pub(crate) mod menu_bar;
pub(crate) mod shortcuts;
pub(crate) mod tool_rail;
pub(crate) mod topbar;
pub(crate) mod window_frame;

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
    t.font_size = 13.0;
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
    let centered = state.resize_handles_centered.get();
    let left_handle = ResizeHandle::new(state.left_panel_w.clone())
        .min(180.0)
        .max(440.0)
        .hit_width(8.0)
        .gutter(colors::active().surface)
        .idle_width(1.0)
        .idle_color(colors::active().stroke)
        .active_width(2.0)
        .active_color(colors::active().sel);
    let left_handle = if centered {
        left_handle.line_at_center()
    } else {
        left_handle.line_at_end()
    };
    let right_handle = ResizeHandle::new(state.right_panel_w.clone())
        .min(200.0)
        .max(480.0)
        .hit_width(8.0)
        .invert(true)
        .gutter(colors::active().surface)
        .idle_width(1.0)
        .idle_color(colors::active().stroke)
        .active_width(2.0)
        .active_color(colors::active().sel);
    let right_handle = if centered {
        right_handle.line_at_center()
    } else {
        right_handle.line_at_start()
    };

    Stack::new()
        .fill_size()
        .child(
            Column::new()
                .fill_size()
                .child(menu_bar::title_bar(&state))
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
                                        .child(left_handle)
                                        .child(canvas::canvas(&state))
                                        .child(right_handle)
                                        .child(panels::panel_right(&state)),
                                ),
                        ),
                ),
        )
        .child(panels::layer_drag_overlay(&state))
        .child(menu_bar::menu_drops(&state))
        .child(overlays::context_menu(&state))
        .child(overlays::command_palette(&state))
        .child(overlays::help_overlay(&state))
        .child(overlays::settings_overlay(&state))
        .child(overlays::color_picker(&state))
        .child(overlays::picker_menu(&state))
        .child(overlays::toasts(&state))
        .child(window_frame::resize_borders())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sequoia_test::Harness;
    use sequoia_ui::view::ElementId;

    fn resize_handles(h: &Harness) -> Vec<ElementId> {
        fn walk(h: &Harness, id: ElementId, out: &mut Vec<ElementId>) {
            if h.view::<ResizeHandle>(id).is_some() {
                out.push(id);
            }
            for child in h.children(id) {
                walk(h, child, out);
            }
        }
        let mut out = Vec::new();
        if let Some(root) = h.root() {
            walk(h, root, &mut out);
        }
        out
    }

    fn center(h: &Harness, id: ElementId) -> (f32, f32) {
        let r = h.rect(id).expect("handle rect");
        (r.left() + r.size.width * 0.5, r.top() + r.size.height * 0.5)
    }

    #[test]
    fn window_edges_are_resizable() {
        let env = Environment::new();
        env.provide(Windows::new());
        let h = Harness::new(root(EditorState::demo()));
        h.set_environment(env);
        h.layout(1000.0, 700.0);

        h.pointer_move(998.0, 350.0);
        assert_eq!(h.cursor_icon(), CursorIcon::ColResize);
        h.pointer_move(2.0, 350.0);
        assert_eq!(h.cursor_icon(), CursorIcon::ColResize);
        h.pointer_move(500.0, 2.0);
        assert_eq!(h.cursor_icon(), CursorIcon::RowResize);
        h.pointer_move(500.0, 698.0);
        assert_eq!(h.cursor_icon(), CursorIcon::RowResize);
        h.pointer_move(2.0, 2.0);
        assert_eq!(h.cursor_icon(), CursorIcon::NwseResize);
        h.pointer_move(998.0, 698.0);
        assert_eq!(h.cursor_icon(), CursorIcon::NwseResize);

        h.pointer_move(500.0, 350.0);
        assert_eq!(h.cursor_icon(), CursorIcon::Default);
    }

    #[test]
    fn resize_handle_line_switches_between_edge_and_center() {
        let state = EditorState::demo();
        let h = Harness::new(root(state.clone()));
        h.layout(1000.0, 700.0);

        let handles = resize_handles(&h);
        let handle = h.rect(handles[0]).expect("left handle");
        let line_color = colors::active().stroke;
        let line_x = |h: &Harness| {
            h.paint()
                .quads()
                .find(|q| {
                    q.color == line_color
                        && (q.rect.size.width - 1.0).abs() < 0.01
                        && q.rect.left() >= handle.left()
                        && q.rect.right() <= handle.right()
                        && q.rect.size.height >= handle.size.height - 0.5
                })
                .map(|q| q.rect.left())
                .expect("resize line")
        };

        let edge = line_x(&h);
        assert!((edge - (handle.left() + 7.0)).abs() < 0.5);

        state.resize_handles_centered.set(true);
        h.tick();
        h.layout(1000.0, 700.0);
        let centered = line_x(&h);
        assert!((centered - (handle.left() + 3.5)).abs() < 0.5);
    }

    #[test]
    fn side_panels_resize_with_the_handles() {
        let state = EditorState::demo();
        let h = Harness::new(root(state.clone()));
        h.layout(1000.0, 700.0);

        let handles = resize_handles(&h);
        assert_eq!(handles.len(), 2, "a handle on each side of the canvas");
        let left_before = h.rect(handles[0]).expect("left handle").left();
        let (x, y) = center(&h, handles[0]);

        h.drag((x, y), (x + 40.0, y));
        h.tick();
        h.layout(1000.0, 700.0);

        assert_eq!(state.left_panel_w.get_untracked(), 268.0);
        let handles = resize_handles(&h);
        let left_after = h.rect(handles[0]).expect("left handle").left();
        assert!(
            (left_after - (left_before + 40.0)).abs() < 0.5,
            "handle follows the wider panel: {left_before} -> {left_after}"
        );

        let right_before = h.rect(handles[1]).expect("right handle").left();
        let (x, y) = center(&h, handles[1]);
        h.drag((x, y), (x - 40.0, y));
        h.tick();
        h.layout(1000.0, 700.0);

        assert_eq!(state.right_panel_w.get_untracked(), 278.0);
        let handles = resize_handles(&h);
        let right_after = h.rect(handles[1]).expect("right handle").left();
        assert!(
            (right_after - (right_before - 40.0)).abs() < 0.5,
            "right panel grows leftwards: {right_before} -> {right_after}"
        );
    }

    #[test]
    fn debug_settings_buttons() {
        let state = EditorState::demo();
        state.settings_open.set(true);
        let h = Harness::new(root(state.clone()));
        h.layout(1000.0, 700.0);
        let list = h.paint();
        let active = colors::active().sel;
        let inactive = colors::active().bg_lighter;
        for q in list.quads() {
            if q.color == active || q.color == inactive {
                println!(
                    "color={:?} l={:.1} t={:.1} w={:.1} h={:.1} r={:.1}",
                    q.color,
                    q.rect.left(),
                    q.rect.top(),
                    q.rect.size.width,
                    q.rect.size.height,
                    q.radius
                );
            }
        }
    }
}
