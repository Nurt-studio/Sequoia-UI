use super::EditorState;

use crate::core::layer::Layer;

impl EditorState {
    pub(crate) fn new_document(&self) {
        self.push_history();
        self.layers.set(Vec::new());
        self.selected_layer.set(None);
        self.zoom.set(1.0);
        self.pan_x.set(0.0);
        self.pan_y.set(0.0);
        self.toast("New document");
    }

    pub(crate) fn duplicate_selected(&self) {
        let Some(id) = self.selected_layer.get_untracked() else {
            self.toast_warn("Nothing selected");
            return;
        };
        let ls = self.layers.get_untracked();
        let Some(orig) = ls.iter().find(|l| l.id == id).cloned() else {
            return;
        };
        self.push_history();

        let is_frame = orig.kind == "FRAME";
        let children: Vec<Layer> = if is_frame {
            ls.iter()
                .filter(|l| l.parent == Some(id))
                .cloned()
                .collect()
        } else {
            vec![]
        };

        let mut next_id = ls.iter().map(|l| l.id).max().unwrap_or(0) + 1;
        let new_root = next_id;
        next_id += 1;

        let mut new_children = Vec::new();
        for mut c in children {
            c.id = next_id;
            c.parent = Some(new_root);
            c.x += 16.0;
            c.y += 16.0;
            new_children.push(c);
            next_id += 1;
        }

        let copy = Layer {
            id: new_root,
            name: format!("{} copy", orig.name),
            x: orig.x + 16.0,
            y: orig.y + 16.0,
            ..orig
        };
        self.layers.update(|ls| {
            ls.push(copy);
            for c in new_children {
                ls.push(c);
            }
        });
        self.selected_layer.set(Some(new_root));
        self.propagate_layout();
        self.toast("Duplicated");
    }

    pub(crate) fn copy_selected(&self) {
        let Some(id) = self.selected_layer.get_untracked() else {
            self.toast_warn("Nothing selected");
            return;
        };
        let ls = self.layers.get_untracked();
        let mut copy = Vec::new();
        if let Some(root) = ls.iter().find(|l| l.id == id).cloned() {
            if root.kind == "FRAME" {
                copy = ls
                    .iter()
                    .filter(|l| l.parent == Some(id))
                    .cloned()
                    .collect();
            }
            copy.insert(0, root);
        }
        if copy.is_empty() {
            return;
        }
        *self.clipboard.borrow_mut() = copy;
        self.toast("Copied");
    }

    pub(crate) fn paste(&self) {
        let clip = self.clipboard.borrow().clone();
        if clip.is_empty() {
            self.toast_warn("Clipboard empty");
            return;
        }
        self.push_history();
        let mut ls = self.layers.get_untracked();
        let mut next = ls.iter().map(|l| l.id).max().unwrap_or(0) + 1;

        let mut remap: std::collections::HashMap<u64, u64> = std::collections::HashMap::new();
        #[allow(clippy::explicit_counter_loop)]
        for l in &clip {
            remap.insert(l.id, next);
            next += 1;
        }

        let mut pasted = Vec::new();
        for mut l in clip {
            l.id = remap[&l.id];
            if let Some(p) = l.parent {
                l.parent = remap.get(&p).copied();
            }
            l.x += 16.0;
            l.y += 16.0;
            pasted.push(l);
        }

        let first_id = pasted.first().map(|l| l.id);
        ls.extend(pasted);
        self.layers.set(ls);
        if let Some(fid) = first_id {
            self.selected_layer.set(Some(fid));
        }
        self.propagate_layout();
        self.toast("Pasted");
    }

    pub(crate) fn delete_selected(&self) {
        let Some(id) = self.selected_layer.get_untracked() else {
            self.toast_warn("Nothing selected");
            return;
        };
        self.push_history();
        let is_frame = self
            .layers
            .get_untracked()
            .iter()
            .find(|l| l.id == id)
            .map(|l| l.kind == "FRAME")
            .unwrap_or(false);
        self.layers.update(|ls| {
            if is_frame {
                ls.retain(|l| l.id != id && l.parent != Some(id));
            } else {
                ls.retain(|l| l.id != id);
            }
        });
        self.selected_layer.set(None);
        self.toast("Deleted");
    }

    pub(crate) fn bring_forward(&self) {
        let Some(id) = self.selected_layer.get_untracked() else {
            return;
        };
        self.push_history();
        self.layers.update(|ls| {
            if let Some(i) = ls.iter().position(|l| l.id == id)
                && i + 1 < ls.len()
            {
                let l = ls.remove(i);
                ls.insert(i + 1, l);
            }
        });
    }

    pub(crate) fn send_backward(&self) {
        let Some(id) = self.selected_layer.get_untracked() else {
            return;
        };
        self.push_history();
        self.layers.update(|ls| {
            if let Some(i) = ls.iter().position(|l| l.id == id)
                && i > 0
            {
                let l = ls.remove(i);
                ls.insert(i - 1, l);
            }
        });
    }

    pub(crate) fn bring_to_front(&self) {
        let Some(id) = self.selected_layer.get_untracked() else {
            return;
        };
        self.push_history();
        self.layers.update(|ls| {
            if let Some(i) = ls.iter().position(|l| l.id == id) {
                let l = ls.remove(i);
                ls.push(l);
            }
        });
    }

    pub(crate) fn send_to_back(&self) {
        let Some(id) = self.selected_layer.get_untracked() else {
            return;
        };
        self.push_history();
        self.layers.update(|ls| {
            if let Some(i) = ls.iter().position(|l| l.id == id) {
                let l = ls.remove(i);
                ls.insert(0, l);
            }
        });
    }

    pub(crate) fn toggle_visibility(&self) {
        let Some(id) = self.selected_layer.get_untracked() else {
            return;
        };
        self.layers.update(|ls| {
            if let Some(l) = ls.iter_mut().find(|l| l.id == id) {
                l.visible = !l.visible;
            }
        });
    }

    pub(crate) fn toggle_lock(&self) {
        let Some(id) = self.selected_layer.get_untracked() else {
            return;
        };
        self.layers.update(|ls| {
            if let Some(l) = ls.iter_mut().find(|l| l.id == id) {
                l.locked = !l.locked;
            }
        });
        if let Some(l) = self.layers.get_untracked().iter().find(|l| l.id == id) {
            self.toast(if l.locked { "Locked" } else { "Unlocked" });
        }
    }
}
