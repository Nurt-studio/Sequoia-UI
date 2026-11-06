//! "Code" tab: preview + full editing of Svelte code with reverse parsing.
//!
//! **Preview** mode draws syntax-highlighted code (`components::code_view`) live
//! from the buffer. **Edit** mode hands the field over to the framework `TextArea`
//! (cursor, selection, Tab, auto-indent). "Apply" parses the buffer (`from_svelte`)
//! and replaces the document tree with it.

use sequoia_ui::prelude::*;

use crate::components::code_view::code_view;
use crate::components::segmented::segment;
use crate::state::{EditorState, Tab};
use crate::theme::colors;

const CODE_BG: Color = Color::hex(0x141416);

#[derive(Clone)]
struct CodePanel {
    state: EditorState,
}

impl View for CodePanel {
    fn body(&self) -> impl View + 'static {
        let st = self.state.clone();
        let editing = st.code_editing.get();

        Column::new()
            .fill_size()
            .background(colors::active().surface)
            // header: status dot + title
            .child(self.status_row())
            // Preview / Edit toggle
            .child(self.mode_row())
            // actions
            .child(self.action_row())
            .child(self.file_row())
            // content
            .child(if editing {
                AnyView::new(self.editor_area())
            } else {
                AnyView::new(self.preview_area())
            })
            // footer: error / hint
            .child(self.footer())
    }
}

impl CodePanel {
    fn status_row(&self) -> impl View + 'static {
        let st = self.state.clone();
        Column::new()
            .fill_width()
            .padding(Edges::xy(12.0, 10.0))
            .border_bottom(1.0, colors::active().stroke_soft)
            .child({
                let st2 = st.clone();
                Row::new()
                    .fill_width()
                    .gap(6.0)
                    .align_items(Align::Center)
                    .child(
                        Text::new("Svelte code")
                            .size(13.0)
                            .semibold()
                            .color(colors::active().text),
                    )
                    .child(Spacer::flex(1.0))
                    .child(status_dot(&st2))
                    .child(status_label(&st2))
            })
            .child(
                Text::new("source of truth: the document tree")
                    .size(10.0)
                    .color(colors::active().icon_inactive),
            )
    }

    fn mode_row(&self) -> impl View + 'static {
        let st = self.state.clone();
        let editing = st.code_editing.get();
        Container::new()
            .fill_width()
            .padding(Edges::xy(8.0, 6.0))
            .child(
                Row::new()
                    .gap(2.0)
                    .child(segment("Preview", !editing, {
                        let st = st.clone();
                        move || st.code_editing.set(false)
                    }))
                    .child(segment("Edit", editing, move || st.code_editing.set(true))),
            )
    }

    fn action_row(&self) -> impl View + 'static {
        let st = self.state.clone();
        let st_revert = st.clone();
        Row::new()
            .fill_width()
            .padding(Edges::xy(8.0, 0.0))
            .gap(4.0)
            .child_flex(
                small_btn("Apply", colors::active().sel, move || {
                    st.apply_code_buffer()
                }),
                1.0,
            )
            .child_flex(
                small_btn("Cancel", Color::hex(0x2E2E30), move || {
                    st_revert.revert_code_buffer()
                }),
                1.0,
            )
            .child_flex(copy_btn(&self.state.clone()), 1.0)
    }

    fn file_row(&self) -> impl View + 'static {
        let st = self.state.clone();
        let st_json = st.clone();
        Row::new()
            .fill_width()
            .padding(Edges::xy(8.0, 4.0))
            .gap(4.0)
            .child_flex(tiny_btn("Save .svelte", move || st.save_svelte_file()), 1.0)
            .child_flex(
                tiny_btn("Save .json", move || st_json.save_json_file()),
                1.0,
            )
    }

    fn preview_area(&self) -> impl View + 'static {
        let st = self.state.clone();
        Container::new()
            .fill_size()
            .border_top(1.0, colors::active().stroke_soft)
            .border_bottom(1.0, colors::active().stroke_soft)
            .child(code_view(st.code_buffer.clone()))
    }

    fn editor_area(&self) -> impl View + 'static {
        let st = self.state.clone();
        let st2 = st.clone();
        Container::new()
            .fill_size()
            .border_top(1.0, colors::active().stroke_soft)
            .border_bottom(1.0, colors::active().stroke_soft)
            .background(CODE_BG)
            .child(
                TextArea::new(st.code_buffer.clone())
                    .font_size(11.0)
                    .color(Color::hex(0xE8E8E8))
                    .background(CODE_BG)
                    .border(0.0, Color::TRANSPARENT)
                    .radius(0.0)
                    .padding(Edges::xy(8.0, 8.0))
                    .line_height_factor(1.4)
                    .highlight_current_line(true)
                    .current_line(Color::hexa(0xFFFFFF0A))
                    .accept_tab(true)
                    .auto_indent(true)
                    .width(Length::Fill(1.0))
                    .height(Length::Fill(1.0))
                    .on_edit_end(move |text: String| {
                        // live validation after leaving the field — without overwriting the document
                        match crate::codegen::from_svelte(&text) {
                            Ok(_) => st2.code_error.set(None),
                            Err(e) => st2.code_error.set(Some(e)),
                        }
                    }),
            )
    }

    fn footer(&self) -> impl View + 'static {
        let st = self.state.clone();
        let editing = st.code_editing.get();
        let error = st.code_error.get();
        Column::new()
            .fill_width()
            .child(if let Some(err) = error {
                AnyView::new(
                    Container::new()
                        .fill_width()
                        .padding(Edges::xy(8.0, 6.0))
                        .background(Color::hexa(0xFA4D5622))
                        .border_top(1.0, colors::active().danger)
                        .child(
                            Text::new(short_line(&err))
                                .size(10.0)
                                .color(colors::active().danger),
                        ),
                )
            } else {
                AnyView::new(Unit)
            })
            .child(
                Container::new()
                    .fill_width()
                    .padding(Edges::xy(8.0, 6.0))
                    .child(
                    Text::new(if editing {
                        "Edit the code and press \"Apply\" to update the canvas."
                    } else {
                        "Code buffer preview. Switch to \"Edit\" to change the document as text."
                    })
                    .size(10.0)
                    .color(colors::active().icon_inactive),
                ),
            )
    }
}

fn status_dot(st: &EditorState) -> impl View + 'static {
    let error = st.code_error.get();
    let dirty = st.code_dirty.get();
    let color = if error.is_some() {
        colors::active().danger
    } else if dirty {
        colors::active().warn
    } else {
        colors::active().ok
    };
    Container::new()
        .width(7.0)
        .height(7.0)
        .radius(2.0)
        .background(color)
}

fn status_label(st: &EditorState) -> impl View + 'static {
    let st2 = st.clone();
    Text::with(move || {
        let err = st2.code_error.get();
        if err.is_some() {
            "syntax error".to_string()
        } else if st2.code_dirty.get() {
            "buffer changed".to_string()
        } else {
            "synchronized".to_string()
        }
    })
    .size(10.0)
    .font("IBM Plex Mono")
    .color(colors::active().icon_inactive)
}

/// Text action button (fills the cell).
fn small_btn(label: &'static str, bg: Color, on_click: impl Fn() + 'static) -> impl View + 'static {
    Container::new()
        .height(24.0)
        .center()
        .radius(6.0)
        .background(bg)
        .hover(Color::hex(0x3E3E40))
        .on_click(on_click)
        .child(Text::new(label).size(11.0).semibold().color(Color::WHITE))
}

/// "Copy" button — needs an environment for the system clipboard.
fn copy_btn(state: &EditorState) -> impl View + 'static {
    let st = state.clone();
    Container::new()
        .height(24.0)
        .center()
        .radius(6.0)
        .background(Color::hex(0x2E2E30))
        .hover(Color::hex(0x3E3E40))
        .on_click_env(move |env: &Environment| st.copy_code_to_clipboard(env))
        .child(Text::new("Copy").size(11.0).semibold().color(Color::WHITE))
}

/// Small file button.
fn tiny_btn(label: &'static str, on_click: impl Fn() + 'static) -> impl View + 'static {
    Container::new()
        .height(20.0)
        .center()
        .radius(5.0)
        .background(Color::hex(0x232325))
        .hover(Color::hex(0x2E2E30))
        .on_click(on_click)
        .child(
            Text::new(label)
                .size(10.0)
                .font("IBM Plex Mono")
                .color(colors::active().icon_active),
        )
}

fn short_line(s: &str) -> String {
    let first = s.lines().next().unwrap_or(s);
    if first.chars().count() > 64 {
        format!("{}…", first.chars().take(64).collect::<String>())
    } else {
        first.to_string()
    }
}

/// Right panel content: the Code tab bypasses ScrollView (it handles scrolling itself).
pub(crate) fn code_body(state: &EditorState) -> impl View + 'static {
    CodePanel {
        state: state.clone(),
    }
}

/// Helper used by properties/mod.rs to detect the active tab.
pub(crate) fn is_code_tab(state: &EditorState) -> bool {
    state.active_tab.get() == Tab::Code
}
