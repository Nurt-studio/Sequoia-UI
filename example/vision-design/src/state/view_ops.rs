use sequoia_ui::prelude::{Environment, SharedClipboard};

use super::EditorState;

impl EditorState {
    pub(crate) fn propagate_layout(&self) {
        // Layout is derived from the tree (FlatNode) — no mutation required.
    }

    pub(crate) fn zoom_to_fit(&self) {
        let flat = self.flat_snapshot();
        if flat.is_empty() {
            self.toast_warn("Nothing to display");
            return;
        }
        let mut x1 = f32::INFINITY;
        let mut y1 = f32::INFINITY;
        let mut x2 = f32::NEG_INFINITY;
        let mut y2 = f32::NEG_INFINITY;
        for f in &flat {
            if !f.visible {
                continue;
            }
            x1 = x1.min(f.rect.left());
            y1 = y1.min(f.rect.top());
            x2 = x2.max(f.rect.left() + f.rect.size.width);
            y2 = y2.max(f.rect.top() + f.rect.size.height);
        }
        let w = (x2 - x1).max(1.0);
        let h = (y2 - y1).max(1.0);
        let vw = self.viewport_w.get_untracked();
        let vh = self.viewport_h.get_untracked();
        if vw <= 0.0 || vh <= 0.0 {
            return;
        }
        let pad = 80.0;
        let z = ((vw - pad) / w).min((vh - pad) / h).clamp(0.1, 4.0);
        let px = x1 - (vw / z - w) * 0.5;
        let py = y1 - (vh / z - h) * 0.5;
        self.zoom.set(z);
        self.pan_x.set(px);
        self.pan_y.set(py);
    }

    pub(crate) fn zoom_to_selection(&self) {
        let Some(id) = self.selected_layer.get_untracked() else {
            self.toast_warn("Nothing selected");
            return;
        };
        let flat = self.flat_snapshot();
        let Some(f) = flat.iter().find(|f| f.id == id) else {
            return;
        };
        let vw = self.viewport_w.get_untracked();
        let vh = self.viewport_h.get_untracked();
        if vw <= 0.0 || vh <= 0.0 {
            return;
        }
        let pad = 120.0;
        let z = ((vw - pad) / f.rect.size.width)
            .min((vh - pad) / f.rect.size.height)
            .clamp(0.1, 4.0);
        let cx = f.rect.left() + f.rect.size.width * 0.5;
        let cy = f.rect.top() + f.rect.size.height * 0.5;
        self.zoom.set(z);
        self.pan_x.set(cx - (vw / z) * 0.5);
        self.pan_y.set(cy - (vh / z) * 0.5);
    }

    pub(crate) fn reset_view(&self) {
        self.zoom.set(1.0);
        self.pan_x.set(0.0);
        self.pan_y.set(0.0);
    }

    pub(crate) fn toggle_grid(&self) {
        self.grid_on.update(|v| *v = !*v);
    }

    pub(crate) fn export_svg(&self) {
        let svg = crate::document::export::to_svg(&self.layers.get_untracked());
        match std::fs::write("export.svg", &svg) {
            Ok(_) => self.toast("Saved export.svg"),
            Err(_) => self.toast_error("Could not save export.svg"),
        }
    }

    // ─────────────────────────────── Code tab ───────────────────────────────

    /// Opens the "Code" tab in the right panel.
    pub(crate) fn open_code_tab(&self) {
        self.active_tab.set(crate::state::Tab::Code);
    }

    /// Parses the editor buffer and replaces the document with it.
    pub(crate) fn apply_code_buffer(&self) {
        let src = self.code_buffer.get_untracked();
        match crate::codegen::from_svelte(&src) {
            Ok(new_root) => {
                self.push_history();
                self.root.set(new_root);
                // The normalized code (generated from the AST) goes back into the buffer.
                let normalized = crate::codegen::to_svelte(&self.root.get_untracked());
                self.code_buffer.set(normalized.clone());
                self.code_source.set(normalized);
                self.code_error.set(None);
                self.code_dirty.set(false);
                self.toast("Code applied - canvas updated");
            }
            Err(e) => {
                self.code_error.set(Some(e));
                self.toast_error("Parse error - fix the code and try again");
            }
        }
    }

    /// Reverts to code generated from the document (discards editor changes).
    pub(crate) fn revert_code_buffer(&self) {
        let src = self.code_source.get_untracked();
        self.code_buffer.set(src);
        self.code_error.set(None);
        self.toast("Code restored from document");
    }

    /// Copies the current Svelte code to the system clipboard.
    pub(crate) fn copy_code_to_clipboard(&self, env: &Environment) {
        let code = self.code_buffer.get_untracked();
        if code.is_empty() {
            self.toast_warn("Code is empty");
            return;
        }
        if let Some(clip) = env.get::<SharedClipboard>() {
            clip.0.set_text(&code);
            self.toast("Svelte code copied to clipboard");
        } else {
            self.toast_warn("System clipboard unavailable");
        }
    }

    /// Saves the current Svelte code to the `component.svelte` file.
    pub(crate) fn save_svelte_file(&self) {
        let code = self.code_buffer.get_untracked();
        match std::fs::write("component.svelte", &code) {
            Ok(_) => self.toast("Saved component.svelte"),
            Err(_) => self.toast_error("Could not save component.svelte"),
        }
    }

    // ─────────────────────────────── JSON ───────────────────────────────

    /// Exports the document (AST) as JSON to the system clipboard.
    pub(crate) fn export_json_clipboard(&self, env: &Environment) {
        let json = crate::document::json::to_json(&self.root.get_untracked());
        if let Some(clip) = env.get::<SharedClipboard>() {
            clip.0.set_text(&json);
            self.toast("JSON document copied to clipboard");
        } else {
            self.toast_warn("System clipboard unavailable");
        }
    }

    /// Saves the JSON document to the `vision-design.json` file.
    pub(crate) fn save_json_file(&self) {
        let json = crate::document::json::to_json(&self.root.get_untracked());
        match std::fs::write("vision-design.json", &json) {
            Ok(_) => self.toast("Saved vision-design.json"),
            Err(_) => self.toast_error("Could not save vision-design.json"),
        }
    }

    /// Loads a JSON document from the system clipboard.
    pub(crate) fn import_json_clipboard(&self, env: &Environment) {
        let Some(clip) = env.get::<SharedClipboard>() else {
            self.toast_warn("System clipboard unavailable");
            return;
        };
        let Some(text) = clip.0.get_text() else {
            self.toast_warn("Clipboard is empty");
            return;
        };
        self.import_json_text(&text);
    }

    /// Parses JSON text and replaces the document with it.
    pub(crate) fn import_json_text(&self, text: &str) {
        match crate::document::json::from_json(text) {
            Ok(new_root) => {
                self.push_history();
                self.root.set(new_root);
                self.selected_layer.set(None);
                self.active_tab.set(crate::state::Tab::Design);
                self.toast("Imported JSON document");
            }
            Err(e) => {
                self.toast_error(format!("JSON import error: {e}"));
            }
        }
    }

    // ─────────────────────────────── Auto layout ───────────────────────────────

    /// Cycles the auto layout mode of the selected frame: none → row → column → grid → none.
    pub(crate) fn auto_layout_selection(&self) {
        use crate::model::svelte_ast::{Display, FlexDir};
        let Some(id) = self.selected_layer.get_untracked() else {
            self.toast_warn("Nothing selected");
            return;
        };
        let mut msg = String::new();
        self.push_history();
        self.root.update(|r| {
            let Some(rr) = r else { return };
            if let Some(crate::model::svelte_ast::SvelteNode::Element(e)) =
                crate::model::tree_ops::find_mut(rr, id)
            {
                match e.style.display {
                    Display::None => {
                        e.style.display = Display::Flex;
                        e.style.flex_dir = FlexDir::Row;
                        msg = "Auto layout: row (H)".into();
                    }
                    Display::Flex if e.style.flex_dir == FlexDir::Row => {
                        e.style.display = Display::Flex;
                        e.style.flex_dir = FlexDir::Col;
                        msg = "Auto layout: column (V)".into();
                    }
                    Display::Flex => {
                        e.style.display = Display::Grid;
                        if e.style.grid_cols.is_none() {
                            e.style.grid_cols = Some(2);
                        }
                        msg = "Auto layout: grid (G, 2 columns)".into();
                    }
                    Display::Grid => {
                        e.style.display = Display::None;
                        e.style.grid_cols = None;
                        msg = "Auto layout disabled".into();
                    }
                }
            }
        });
        if msg.is_empty() {
            self.toast_warn("Selected layer is not a frame");
        } else {
            self.toast(msg);
        }
    }

    pub(crate) fn apply_code_edit(&self, new_code: String) {
        match crate::codegen::from_svelte(&new_code) {
            Ok(new_root) => {
                self.push_history();
                self.root.set(new_root);
                self.code_error.set(None);
                self.code_source.set(new_code);
                self.toast("Code updated");
            }
            Err(e) => {
                self.code_error.set(Some(e));
                self.toast_error("Parse error");
            }
        }
    }
}