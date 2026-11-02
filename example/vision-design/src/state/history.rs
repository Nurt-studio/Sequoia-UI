use super::EditorState;
use crate::model::svelte_ast::SvelteNode;

impl EditorState {
    pub(crate) fn push_history(&self) {
        let current = self.root.get_untracked();
        self.push_snapshot(current);
    }

    pub(crate) fn push_snapshot(&self, prev: Option<SvelteNode>) {
        if prev == self.root.get_untracked() {
            return;
        }
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
            self.future.borrow_mut().push(self.root.get_untracked());
            self.root.set(prev);
            self.can_undo.set(!self.history.borrow().is_empty());
            self.can_redo.set(true);
            self.fix_selection_after_history();
            self.toast("Cofnięto");
        } else {
            self.toast_warn("Nie ma czego cofnąć");
        }
    }

    pub(crate) fn redo(&self) {
        if let Some(next) = self.future.borrow_mut().pop() {
            self.history.borrow_mut().push(self.root.get_untracked());
            self.root.set(next);
            self.can_undo.set(true);
            self.can_redo.set(!self.future.borrow().is_empty());
            self.fix_selection_after_history();
            self.toast("Ponowiono");
        } else {
            self.toast_warn("Nie ma czego ponowić");
        }
    }

    fn fix_selection_after_history(&self) {
        if let Some(id) = self.selected_layer.get_untracked() {
            let found = self.root.get_untracked()
                .as_ref()
                .and_then(|r| crate::model::tree_ops::find(r, id))
                .is_some();
            if !found {
                self.selected_layer.set(None);
            }
        }
    }
}