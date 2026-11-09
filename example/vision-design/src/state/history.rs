use super::EditorState;
use crate::model::svelte_ast::SvelteNode;

impl EditorState {
    /// Snapshots the PRE-mutation tree. Called before every editor
    /// mutation, so the value differs from the post-mutation root and must
    /// not go through `push_snapshot`'s no-op dedupe — otherwise the undo
    /// stack (and the node-graph rebuild stamp, which is the history
    /// length) would never move.
    pub(crate) fn push_history(&self) {
        self.record(self.root.get_untracked());
    }

    /// Pushes an explicitly captured older state (e.g. the drag-session
    /// `before` snapshot taken at pointer-down). Skips when the captured
    /// state equals the current one (click without a drag).
    pub(crate) fn push_snapshot(&self, prev: Option<SvelteNode>) {
        if prev == self.root.get_untracked() {
            return;
        }
        self.record(prev);
    }

    fn record(&self, prev: Option<SvelteNode>) {
        self.history.borrow_mut().push(prev);
        self.future.borrow_mut().clear();
        if self.history.borrow().len() > 40 {
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
            self.toast("Undone");
        } else {
            self.toast_warn("Nothing to undo");
        }
    }

    pub(crate) fn redo(&self) {
        if let Some(next) = self.future.borrow_mut().pop() {
            self.history.borrow_mut().push(self.root.get_untracked());
            self.root.set(next);
            self.can_undo.set(true);
            self.can_redo.set(!self.future.borrow().is_empty());
            self.fix_selection_after_history();
            self.toast("Redone");
        } else {
            self.toast_warn("Nothing to redo");
        }
    }

    fn fix_selection_after_history(&self) {
        if let Some(id) = self.selected_layer.get_untracked() {
            let found = self
                .root
                .get_untracked()
                .as_ref()
                .and_then(|r| crate::model::tree_ops::find(r, id))
                .is_some();
            if !found {
                self.selected_layer.set(None);
            }
        }
    }
}
