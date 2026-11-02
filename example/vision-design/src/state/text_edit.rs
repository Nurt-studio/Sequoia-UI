use super::EditorState;

impl EditorState {
    pub(crate) fn start_text_edit(&self, snapshot: bool) {
        let Some(id) = self.selected_layer.get_untracked() else {
            return;
        };
        let Some(l) = self
            .layers
            .get_untracked()
            .iter()
            .find(|l| l.id == id)
            .cloned()
        else {
            return;
        };
        if l.kind != "TEXT" {
            return;
        }
        let z = self.zoom.get_untracked();
        let px = self.pan_x.get_untracked();
        let py = self.pan_y.get_untracked();
        let sx = 44.0 + (l.x - px) * z;
        let sy = 64.0 + (l.y - py) * z;
        self.text_edit_anchor
            .set((sx, sy, (l.w * z).max(120.0), (l.h * z).max(28.0)));
        *self.text_edit_orig.borrow_mut() = l.text.clone();
        if snapshot {
            self.push_history();
        }
        self.editing_text.set(true);
    }

    pub(crate) fn end_text_edit(&self, commit: bool) {
        if !commit {
            let orig = self.text_edit_orig.borrow().clone();
            if let Some(id) = self.selected_layer.get_untracked() {
                self.layers.update(|ls| {
                    if let Some(l) = ls.iter_mut().find(|l| l.id == id) {
                        l.text = orig;
                    }
                });
            }
        }
        self.editing_text.set(false);
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