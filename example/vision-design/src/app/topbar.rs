//! Topbar: undo/redo, doc-name, zoom, grid toggle, fit, vars, present, share, avatar.
use sequoia_ui::prelude::*;

use crate::components::icon_button::{glyph_button, icon_button, icon_button_env};
use sequoia_ui::view::{Fill, GradientStops};

use crate::state::EditorState;
use crate::theme::colors;

const ZOOM_MIN: f32 = 0.10;
const ZOOM_MAX: f32 = 8.0;

fn theme_toggle(state: &EditorState) -> impl View + 'static {
    let st = state.clone();
    let icon = if state.is_night.get_untracked() {
        "tb/sun"
    } else {
        "tb/moon"
    };

    Container::new()
        .width(26.0)
        .height(26.0)
        .center()
        .radius(6.0)
        .hover(colors::active().bg_lighter)
        .on_click(move || st.is_night.update(|v| *v = !*v))
        .child(
            Icon::new(icon)
                .size(14.0)
                .color(colors::active().icon_active),
        )
}

/// Green "present" (play) button — toggles the on-canvas behavior
/// interpreter (Phase 4, design doc §7).
fn present_btn(state: &EditorState) -> impl View + 'static {
    let on = state.present.on.get();
    let st = state.clone();
    Container::new()
        .width(26.0)
        .height(26.0)
        .center()
        .radius(6.0)
        .hover(colors::active().bg_lighter)
        .style(if on {
            BoxStyle::new()
                .background(Color::hexa(0x4ADE8029))
                .border(1.0, Color::hex(0x4ADE80))
        } else {
            BoxStyle::new()
        })
        .on_click(move || st.toggle_present())
        .child(
            Icon::new("tb/present")
                .size(14.0)
                .color(Color::hex(0x4ADE80)),
        )
}

/// Grid toggle — 3×3 dots icon, lit when the grid is enabled
fn grid_btn(active: bool, on_click: impl Fn() + 'static) -> impl View + 'static {
    let color = if active {
        colors::active().icon_active
    } else {
        colors::active().icon_inactive
    };
    Container::new()
        .width(26.0)
        .height(26.0)
        .center()
        .radius(6.0)
        .hover(colors::active().bg_lighter)
        .style(if active {
            // A lit button must read as "on": tinted background + ring.
            BoxStyle::new()
                .background(colors::active().sel_soft)
                .border(1.0, Color::hexa(0x0D99FF73))
        } else {
            BoxStyle::new()
        })
        .on_click(on_click)
        .child(
            Canvas::new()
                .width(15.0)
                .height(15.0)
                .focusable(false)
                .on_draw(move |p| {
                    for row in 0..3 {
                        for col in 0..3 {
                            let x = 2.5 + (col as f32) * 5.0;
                            let y = 2.5 + (row as f32) * 5.0;
                            p.circle(Point::new(x, y), 1.2, Fill::solid(color));
                        }
                    }
                }),
        )
}

/// .doc-name — click turns it into an input
#[derive(Clone)]
struct DocName {
    name: Signal<String>,
    editing: Signal<bool>,
}

impl View for DocName {
    fn body(&self) -> impl View + 'static {
        if self.editing.get() {
            let e1 = self.editing.clone();
            let e2 = self.editing.clone();
            AnyView::new(
                TextField::new(self.name.clone())
                    .width(200.0)
                    .font_size(13.0)
                    .text_color(Color::WHITE)
                    .background(colors::active().bg_darker)
                    .border(1.0, colors::active().sel)
                    .radius(5.0)
                    .padding(Edges::xy(6.0, 1.0))
                    .on_submit(move |_| e1.set(false))
                    .on_edit_end(move |_| e2.set(false)),
            )
        } else {
            let editing = self.editing.clone();
            let name = self.name.clone();
            AnyView::new(
                Container::new()
                    .padding(Edges::xy(7.0, 3.0))
                    .radius(6.0)
                    .hover(colors::active().bg_lighter)
                    .on_click(move || editing.set(true))
                    .child(
                        Text::with(move || name.get())
                            .size(13.0)
                            .semibold()
                            .color(Color::hex(0xE8E8E8))
                            .letter_spacing(0.1),
                    ),
            )
        }
    }
}

/// .zoom-group — bg-lighter + border stroke-soft, radius 7
#[derive(Clone)]
struct ZoomGroup {
    zoom: Signal<f32>,
}

impl View for ZoomGroup {
    fn body(&self) -> impl View + 'static {
        let z = self.zoom.get();
        let down = self.zoom.clone();
        let up = self.zoom.clone();
        let reset = self.zoom.clone();
        let show = self.zoom.clone();
        Row::new()
            .align_items(Align::Center)
            .background(colors::active().bg_lighter)
            .border(1.0, colors::active().stroke_soft)
            .radius(7.0)
            .child(glyph_button("−", z <= ZOOM_MIN + 1e-4, move || {
                down.update(|v| *v = (*v / 1.25).clamp(ZOOM_MIN, ZOOM_MAX))
            }))
            .child(
                Container::new()
                    .width(48.0)
                    .height(26.0)
                    .center()
                    .hover(Color::hex(0x2E2E30))
                    .on_click(move || reset.set(1.0))
                    .child(
                        Text::with(move || format!("{}%", (show.get() * 100.0).round() as i32))
                            .size(12.0)
                            .font("IBM Plex Mono")
                            .color(colors::active().text),
                    ),
            )
            .child(glyph_button("+", z >= ZOOM_MAX - 1e-4, move || {
                up.update(|v| *v = (*v * 1.25).clamp(ZOOM_MIN, ZOOM_MAX))
            }))
    }
}

/// .btn-primary — 24px, bg #e8e8e8
fn share_btn() -> impl View + 'static {
    Button::new("Share")
        .style(
            ButtonStyle::new()
                .background(Color::hex(0xE8E8E8))
                .hover(Color::hex(0xCFCFCF))
                .text(Color::hex(0x111111))
                .font_size(13.0)
                .font_weight(600)
                .radius(6.0)
                .padding_xy(10.0, 5.0),
        )
        .on_click(|| {})
}

/// .avatar — 24px circle, gradient, initials
fn avatar() -> impl View + 'static {
    Canvas::new()
        .width(24.0)
        .height(24.0)
        .focusable(false)
        .on_draw(|g| {
            g.circle(
                Point::new(12.0, 12.0),
                12.0,
                Fill::horizontal(GradientStops::two(
                    Color::hex(0xAEAEAE),
                    Color::hex(0x4D4D4D),
                )),
            );
            g.text_center(g.rect(), "VD", 9.0, colors::active().bg_darker);
        })
}

#[derive(Clone)]
struct TopbarBar {
    state: EditorState,
}

impl View for TopbarBar {
    fn body(&self) -> impl View + 'static {
        let cu = self.state.can_undo.get();
        let cr = self.state.can_redo.get();
        let grid_on = self.state.grid_on.get();

        let undo_click = {
            let st = self.state.clone();
            move || st.undo()
        };
        let redo_click = {
            let st = self.state.clone();
            move || st.redo()
        };
        let grid_toggle = {
            let g = self.state.grid_on.clone();
            move || g.update(|v| *v = !*v)
        };
        let fit_click = {
            let st = self.state.clone();
            move || st.zoom_to_fit()
        };

        Row::new()
            .height(36.0)
            .fill_width()
            .gap(6.0)
            // The undo/redo + document-name group hugs the left edge — the
            // old 8px bar padding plus a 2px spacer read as a huge gap.
            .padding(Edges::xy(4.0, 0.0))
            .align_items(Align::Center)
            .background(colors::active().surface)
            .border_bottom(1.0, colors::active().stroke_soft)
            .child(
                Row::new()
                    .gap(4.0)
                    .align_items(Align::Center)
                    .child(icon_button("tb/undo", !cu, undo_click))
                    .child(icon_button("tb/redo", !cr, redo_click))
                    .child(DocName {
                        name: self.state.doc_name.clone(),
                        editing: self.state.editing_doc.clone(),
                    }),
            )
            .child(Spacer::flex(1.0))
            .child(
                Row::new()
                    .gap(3.0)
                    .align_items(Align::Center)
                    .child(ZoomGroup {
                        zoom: self.state.zoom.clone(),
                    })
                    .child(grid_btn(grid_on, grid_toggle))
                    .child(icon_button("tb/fit", false, fit_click))
                    .child({
                        let st = self.state.clone();
                        icon_button_env("tb/vars", false, move |env| {
                            st.open_node_graph(env);
                        })
                    })
                    .child(present_btn(&self.state))
                    .child(theme_toggle(&self.state))
                    .child(share_btn())
                    .child(Spacer::fixed(1.0))
                    .child(avatar()),
            )
    }
}

pub(crate) fn topbar(state: &EditorState) -> impl View + 'static {
    TopbarBar {
        state: state.clone(),
    }
}
