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

    /// Multi-file export: masters → `svelte-project/components/*.svelte`,
    /// the page → `svelte-project/App.svelte`.
    pub(crate) fn save_svelte_project(&self) {
        let root = self.root.get_untracked();
        let files = crate::codegen::export_project(&root);
        let dir = std::path::Path::new("svelte-project");
        let mut ok = true;
        for f in &files {
            let path = dir.join(&f.path);
            match std::fs::create_dir_all(path.parent().unwrap_or(dir))
                .and_then(|_| std::fs::write(&path, &f.code))
            {
                Ok(_) => {}
                Err(_) => ok = false,
            }
        }
        if ok {
            self.toast(format!(
                "Exported {} file{} to {}/",
                files.len(),
                if files.len() == 1 { "" } else { "s" },
                dir.display()
            ));
        } else {
            self.toast_error("Could not write the Svelte project");
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

    /// Applies one auto layout mode (`H`, `V`, `G` or `NONE`) to the selected
    /// frame.
    ///
    /// Applying a flow mode pulls the frame's absolutely positioned children
    /// into that flow — an auto-layout parent skips absolute children, so
    /// without this the layout changed but nothing on the canvas moved ("auto
    /// layout doesn't work"). Turning the layout off bakes the current world
    /// rectangles back into the children, so nothing jumps. Both the panel
    /// buttons and Ctrl+L go through here, so they cannot disagree.
    pub(crate) fn set_layout_mode(&self, mode: &str) {
        use crate::model::layout_engine::is_absolute;
        use crate::model::svelte_ast::{Display, FlexDir, SvelteNode};
        let Some(id) = self.selected_layer.get_untracked() else {
            self.toast_warn("Nothing selected");
            return;
        };
        let is_element = self
            .root
            .get_untracked()
            .as_ref()
            .and_then(|r| crate::model::tree_ops::find(r, id))
            .is_some_and(|n| matches!(n, SvelteNode::Element(_)));
        if !is_element {
            self.toast_warn("Selected layer is not a frame");
            return;
        }

        // World rects of the children BEFORE the mode change — turning the
        // layout off bakes them into `left`/`top` so nothing jumps.
        let world: std::collections::HashMap<u64, (f32, f32)> = self
            .flat_snapshot()
            .into_iter()
            .map(|f| (f.id, (f.rect.left(), f.rect.top())))
            .collect();
        self.push_history();
        self.root.update(|r| {
            let Some(rr) = r else { return };
            let Some(SvelteNode::Element(e)) = crate::model::tree_ops::find_mut(rr, id) else {
                return;
            };
            let was_flow = matches!(e.style.display, Display::Flex | Display::Grid);
            match mode {
                "H" => {
                    e.style.display = Display::Flex;
                    e.style.flex_dir = FlexDir::Row;
                }
                "V" => {
                    e.style.display = Display::Flex;
                    e.style.flex_dir = FlexDir::Col;
                }
                "G" => {
                    e.style.display = Display::Grid;
                    if e.style.grid_cols.is_none() {
                        e.style.grid_cols = Some(2);
                    }
                }
                _ => {
                    e.style.display = Display::None;
                    e.style.grid_cols = None;
                }
            }
            let now_flow = matches!(e.style.display, Display::Flex | Display::Grid);
            if now_flow {
                for c in e.children.iter_mut() {
                    if is_absolute(c) {
                        // Enter the flow: the parent positions it from now on.
                        match c {
                            SvelteNode::Element(ce) => {
                                ce.style.position = crate::model::svelte_ast::Position::Relative;
                                ce.style.left = None;
                                ce.style.top = None;
                                ce.style.right = None;
                                ce.style.bottom = None;
                            }
                            SvelteNode::Text(ct) => ct.absolute = false,
                        }
                    }
                }
            } else if was_flow {
                for c in e.children.iter_mut() {
                    if !is_absolute(c) {
                        // Layout off: keep the current on-screen rectangle.
                        let (x, y) = world.get(&c.id()).copied().unwrap_or((0.0, 0.0));
                        match c {
                            SvelteNode::Element(ce) => {
                                ce.style.position = crate::model::svelte_ast::Position::Absolute;
                                ce.style.left = Some(x);
                                ce.style.top = Some(y);
                            }
                            SvelteNode::Text(ct) => {
                                ct.absolute = true;
                                ct.x = x;
                                ct.y = y;
                            }
                        }
                    }
                }
            }
        });
        self.toast(match mode {
            "H" => "Auto layout: row (H)",
            "V" => "Auto layout: column (V)",
            "G" => "Auto layout: grid (G, 2 columns)",
            _ => "Auto layout disabled",
        });
    }

    /// Cycles the auto layout mode of the selected frame: none → row → column → grid → none.
    pub(crate) fn auto_layout_selection(&self) {
        use crate::model::svelte_ast::{Display, FlexDir, SvelteNode};
        let Some(id) = self.selected_layer.get_untracked() else {
            self.toast_warn("Nothing selected");
            return;
        };
        let root = self.root.get_untracked();
        let Some(SvelteNode::Element(e)) = root
            .as_ref()
            .and_then(|r| crate::model::tree_ops::find(r, id))
        else {
            self.toast_warn("Selected layer is not a frame");
            return;
        };
        let next = match e.style.display {
            Display::None => "H",
            Display::Flex if e.style.flex_dir == FlexDir::Row => "V",
            Display::Flex => "G",
            Display::Grid => "NONE",
        };
        self.set_layout_mode(next);
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
