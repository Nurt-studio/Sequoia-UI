use crate::core::layer::Layer;
use super::EditorState;

impl EditorState {
    pub(crate) fn push_history(&self) {
        let current = self.layers.get_untracked();
        self.push_snapshot_unchecked(current);
    }

    pub(crate) fn push_snapshot(&self, prev: Vec<Layer>) {
        if prev == self.layers.get_untracked() {
            return;
        }
        self.push_snapshot_unchecked(prev);
    }

    fn push_snapshot_unchecked(&self, prev: Vec<Layer>) {
        self.history.borrow_mut().push(prev);
        self.future.borrow_mut().clear();
        if self.history.borrow().len() > 80 {
            self.history.borrow_mut().remove(0);
        }
        self.can_undo.set(true);
        self.can_redo.set(false);
    }

    pub(crate) fn undo(&self) {
        if let Some(prev) = self.history.borrow_mut().pop() {
            self.future.borrow_mut().push(self.layers.get_untracked());
            self.layers.set(prev);
            self.can_undo.set(!self.history.borrow().is_empty());
            self.can_redo.set(true);
            self.fix_selection_after_history();
            self.propagate_layout();
            self.toast("Undo");
        } else {
            self.toast_warn("Nothing to undo");
        }
    }

    pub(crate) fn redo(&self) {
        if let Some(next) = self.future.borrow_mut().pop() {
            self.history.borrow_mut().push(self.layers.get_untracked());
            self.layers.set(next);
            self.can_undo.set(true);
            self.can_redo.set(!self.future.borrow().is_empty());
            self.fix_selection_after_history();
            self.propagate_layout();
            self.toast("Redo");
        } else {
            self.toast_warn("Nothing to redo");
        }
    }

    fn fix_selection_after_history(&self) {
        if let Some(id) = self.selected_layer.get_untracked()
            && !self.layers.get_untracked().iter().any(|l| l.id == id)
        {
            self.selected_layer.set(None);
        }
    }

}
