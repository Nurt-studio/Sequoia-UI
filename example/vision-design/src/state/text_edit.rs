use crate::model::svelte_ast::SvelteNode;
use crate::model::tree_ops;

use super::EditorState;

/// Clamps a caret offset to `text` and moves it back to a character boundary.
fn boundary(text: &str, caret: usize) -> usize {
    let mut caret = caret.min(text.len());
    while caret > 0 && !text.is_char_boundary(caret) {
        caret -= 1;
    }
    caret
}

impl EditorState {
    /// Starts editing the selected text layer directly on the canvas.
    ///
    /// The layer is looked up in the document (`root`), NOT in the `layers`
    /// signal: that one is refreshed by an effect, so a text node created by
    /// the T tool in this very frame is not there yet and editing silently
    /// never started ("nothing can be typed").
    pub(crate) fn start_text_edit(&self, snapshot: bool) {
        let Some(id) = self.selected_layer.get_untracked() else {
            return;
        };
        let content = match self
            .root
            .get_untracked()
            .as_ref()
            .and_then(|r| tree_ops::find(r, id))
        {
            Some(SvelteNode::Text(t)) => t.content.clone(),
            _ => return,
        };
        *self.text_edit_orig.borrow_mut() = content.clone();
        if snapshot {
            self.push_history();
        }
        self.text_caret.set(content.len());
        self.editing_text.set(true);
    }

    pub(crate) fn end_text_edit(&self, commit: bool) {
        if !self.editing_text.get_untracked() {
            return;
        }
        self.editing_text.set(false);
        self.text_caret.set(0);

        if !commit {
            let orig = self.text_edit_orig.borrow().clone();
            self.sel_text.set(orig);
            return;
        }

        // Leaving an empty text layer deletes it, the way Figma drops a text
        // box the user emptied.
        if self.sel_text.get_untracked().trim().is_empty()
            && let Some(id) = self.selected_layer.get_untracked()
        {
            self.root.update(|r| {
                tree_ops::remove(r, id);
            });
            self.selected_layer.set(None);
            self.selected_layers.set(Vec::new());
        }
    }

    /// Inserts `s` at the caret and moves it past the inserted text.
    pub(crate) fn text_insert(&self, s: &str) {
        if !self.editing_text.get_untracked() || s.is_empty() {
            return;
        }
        let mut text = self.sel_text.get_untracked();
        let caret = boundary(&text, self.text_caret.get_untracked());
        text.insert_str(caret, s);
        self.sel_text.set(text);
        self.text_caret.set(caret + s.len());
    }

    pub(crate) fn text_backspace(&self) {
        let text = self.sel_text.get_untracked();
        let caret = boundary(&text, self.text_caret.get_untracked());
        let Some(prev) = text[..caret].char_indices().next_back().map(|(i, _)| i) else {
            return;
        };
        let mut next = text;
        next.replace_range(prev..caret, "");
        self.sel_text.set(next);
        self.text_caret.set(prev);
    }

    pub(crate) fn text_delete_forward(&self) {
        let text = self.sel_text.get_untracked();
        let caret = boundary(&text, self.text_caret.get_untracked());
        let Some(ch) = text[caret..].chars().next() else {
            return;
        };
        let mut next = text;
        next.replace_range(caret..caret + ch.len_utf8(), "");
        self.sel_text.set(next);
        self.text_caret.set(caret);
    }

    /// Moves the caret one character left (`false`) or right (`true`).
    pub(crate) fn text_move_caret(&self, forward: bool) {
        let text = self.sel_text.get_untracked();
        let caret = boundary(&text, self.text_caret.get_untracked());
        let next = if forward {
            text[caret..].chars().next().map(|c| caret + c.len_utf8())
        } else {
            text[..caret].char_indices().next_back().map(|(i, _)| i)
        };
        if let Some(next) = next {
            self.text_caret.set(next);
        }
    }

    pub(crate) fn text_caret_home(&self) {
        self.text_caret.set(0);
    }

    pub(crate) fn text_caret_end(&self) {
        self.text_caret.set(self.sel_text.get_untracked().len());
    }

    pub(crate) fn open_layer_rename(&self, id: u64) {
        let Some(l) = self
            .layers
            .get_untracked()
            .iter()
            .find(|l| l.id == id)
            .cloned()
        else {
            return;
        };
        self.rename_buffer.set(l.name);
        self.editing_layer.set(Some(id));
    }

    pub(crate) fn end_layer_rename(&self, commit: bool) {
        let editing = self.editing_layer.get_untracked();
        self.editing_layer.set(None);
        if !commit {
            return;
        }
        let Some(id) = editing else {
            return;
        };
        let name = self.rename_buffer.get_untracked().trim().to_string();
        if name.is_empty() {
            return;
        }
        self.push_history();
        self.layers.update(|ls| {
            if let Some(l) = ls.iter_mut().find(|l| l.id == id) {
                l.name = name;
            }
        });
    }
}
