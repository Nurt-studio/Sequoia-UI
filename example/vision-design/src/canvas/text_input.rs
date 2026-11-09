//! Inline text editing on the canvas: keystrokes edit the selected text layer
//! in place, without a floating input widget.

use sequoia_ui::prelude::*;

use crate::state::EditorState;

/// Handles a key while a text layer is being edited. Returns `true` when the
/// key was consumed by the editor.
pub(crate) fn handle_key(state: &EditorState, ev: &KeyEvent) -> bool {
    // Present mode (Phase 4): keys drive `On Key` rules; Esc exits.
    if state.present.on.get_untracked() {
        if ev
            .modifiers
            .intersects(Modifiers::CONTROL | Modifiers::ALT | Modifiers::SUPER)
        {
            return false;
        }
        match ev.key {
            Key::ESCAPE => {
                state.stop_present();
                return true;
            }
            Key::ENTER => {
                state.present_key("Enter");
                return true;
            }
            _ => {}
        }
        if let Some(c) = ev.key.char()
            && !c.is_control()
        {
            state.present_key(&c.to_string());
            return true;
        }
        return false;
    }
    if !state.editing_text.get_untracked() {
        return false;
    }
    // Command shortcuts keep working (undo, zoom, …); they are not text.
    if ev
        .modifiers
        .intersects(Modifiers::CONTROL | Modifiers::ALT | Modifiers::SUPER)
    {
        return false;
    }

    match ev.key {
        // Esc and Enter leave the editor and keep what was typed, like Figma.
        Key::ESCAPE | Key::ENTER => {
            state.end_text_edit(true);
            true
        }
        Key::BACKSPACE => {
            state.text_backspace();
            true
        }
        Key::DELETE => {
            state.text_delete_forward();
            true
        }
        Key::ARROW_LEFT => {
            state.text_move_caret(false);
            true
        }
        Key::ARROW_RIGHT => {
            state.text_move_caret(true);
            true
        }
        Key::HOME => {
            state.text_caret_home();
            true
        }
        Key::END => {
            state.text_caret_end();
            true
        }
        Key::SPACE => {
            state.text_insert(" ");
            true
        }
        _ => match ev.key.char() {
            Some(c) if !c.is_control() => {
                state.text_insert(&c.to_string());
                true
            }
            _ => false,
        },
    }
}

/// Handles composition events so IME input (dead keys, CJK) lands at the caret.
pub(crate) fn handle_ime(state: &EditorState, ev: &ImeEvent) -> bool {
    if !state.editing_text.get_untracked() {
        return false;
    }
    match ev {
        ImeEvent::Commit(text) => state.text_insert(text),
        // The preedit is not shown inline yet; consume it so it does not leak
        // into other widgets.
        _ => {}
    }
    true
}
