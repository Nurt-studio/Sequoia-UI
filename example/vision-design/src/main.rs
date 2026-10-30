#![allow(dead_code)]

mod app;
mod colors;
mod state;
mod views;

use sequoia_ui::prelude::*;

fn main() {
    let state = state::EditorState::demo();
    App::new()
        .title("Vision Design")
        .size(1440.0, 900.0)
        .theme(app::editor_theme())
        .icons_dir(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/icons"))
        .default_font("IBM Plex Sans")
        .accelerator(Key::from_char('v'), Modifiers::NONE, {
            let t = state.tool.clone();
            move || t.set("V")
        })
        .accelerator(Key::from_char('h'), Modifiers::NONE, {
            let t = state.tool.clone();
            move || t.set("H")
        })
        .accelerator(Key::from_char('f'), Modifiers::NONE, {
            let t = state.tool.clone();
            move || t.set("F")
        })
        .accelerator(Key::from_char('r'), Modifiers::NONE, {
            let t = state.tool.clone();
            move || t.set("R")
        })
        .accelerator(Key::from_char('o'), Modifiers::NONE, {
            let t = state.tool.clone();
            move || t.set("O")
        })
        .accelerator(Key::from_char('t'), Modifiers::NONE, {
            let t = state.tool.clone();
            move || t.set("T")
        })
        .accelerator(Key::from_char('k'), Modifiers::CONTROL, {
            let st = state.clone();
            move || {
                st.cmd_k_open.update(|v| *v = !*v);
                st.cmdk_query.set(String::new());
            }
        })
        .accelerator(Key::from_char('/'), Modifiers::SHIFT, {
            let st = state.clone();
            move || st.help_open.update(|v| *v = !*v)
        })
        .accelerator(Key::ESCAPE, Modifiers::NONE, {
            let st = state.clone();
            move || {
                if st.editing_text.get_untracked() {
                    st.end_text_edit(false); // cancel
                } else if st.editing_layer.get_untracked().is_some() {
                    st.end_layer_rename(false); // cancel
                } else if st.cmd_k_open.get_untracked() {
                    st.cmd_k_open.set(false);
                } else if st.help_open.get_untracked() {
                    st.help_open.set(false);
                } else {
                    st.tool.set("V");
                }
            }
        })
        .accelerator(Key::F2, Modifiers::NONE, {
            let st = state.clone();
            move || {
                if let Some(id) = st.selected_layer.get_untracked() {
                    st.open_layer_rename(id);
                }
            }
        })
        .accelerator(Key::DELETE, Modifiers::NONE, {
            let st = state.clone();
            move || st.delete_selected()
        })
        .accelerator(Key::from_char('c'), Modifiers::CONTROL, {
            let st = state.clone();
            move || st.copy_selected()
        })
        .accelerator(Key::from_char('v'), Modifiers::CONTROL, {
            let st = state.clone();
            move || st.paste()
        })
        .accelerator(Key::from_char('d'), Modifiers::CONTROL, {
            let st = state.clone();
            move || st.duplicate_selected()
        })
        .accelerator(
            Key::from_char('z'),
            Modifiers::CONTROL | Modifiers::SHIFT,
            {
                let st = state.clone();
                move || st.redo()
            },
        )
        .accelerator(Key::from_char('z'), Modifiers::CONTROL, {
            let st = state.clone();
            move || st.undo()
        })
        .accelerator(Key::from_char('y'), Modifiers::CONTROL, {
            let st = state.clone();
            move || st.redo()
        })
        .accelerator(Key::from_char('1'), Modifiers::SHIFT, {
            let st = state.clone();
            move || st.zoom_to_fit()
        })
        .accelerator(Key::from_char('2'), Modifiers::SHIFT, {
            let st = state.clone();
            move || st.zoom_to_selection()
        })
        .accelerator(Key::from_char('0'), Modifiers::SHIFT, {
            let st = state.clone();
            move || st.reset_view()
        })
        .root(app::root(state))
        .run()
        .expect("failed to start");
}
