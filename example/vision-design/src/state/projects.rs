//! Open projects: one tab per document, switching and closing.

use super::EditorState;
use crate::model::svelte_ast::SvelteNode;

/// One open project tab. The editor's live signals mirror the active tab;
/// the inactive ones keep their own document, selection, view and history.
#[derive(Clone, PartialEq)]
pub(crate) struct ProjectTab {
    pub(crate) id: u64,
    pub(crate) name: String,
    pub(crate) root: Option<SvelteNode>,
    pub(crate) selected: Option<u64>,
    pub(crate) zoom: f32,
    pub(crate) pan_x: f32,
    pub(crate) pan_y: f32,
    pub(crate) history: Vec<Option<SvelteNode>>,
    pub(crate) future: Vec<Option<SvelteNode>>,
}

impl ProjectTab {
    pub(crate) fn new(id: u64, name: impl Into<String>, root: Option<SvelteNode>) -> Self {
        Self {
            id,
            name: name.into(),
            root,
            selected: None,
            zoom: 1.0,
            pan_x: 0.0,
            pan_y: 0.0,
            history: Vec::new(),
            future: Vec::new(),
        }
    }
}

impl EditorState {
    /// Stores the live document back into its tab (before switching away).
    pub(crate) fn snapshot_active_project(&self) {
        let id = self.active_project.get_untracked();
        let root = self.root.get_untracked();
        let selected = self.selected_layer.get_untracked();
        let zoom = self.zoom.get_untracked();
        let pan_x = self.pan_x.get_untracked();
        let pan_y = self.pan_y.get_untracked();
        let history = self.history.borrow().clone();
        let future = self.future.borrow().clone();
        self.projects.update(|tabs| {
            let Some(tab) = tabs.iter_mut().find(|t| t.id == id) else {
                return;
            };
            tab.root = root;
            tab.selected = selected;
            tab.zoom = zoom;
            tab.pan_x = pan_x;
            tab.pan_y = pan_y;
            tab.history = history;
            tab.future = future;
        });
    }

    /// Restores a tab into the live editor signals.
    fn apply_project(&self, tab: &ProjectTab) {
        self.root.set(tab.root.clone());
        self.selected_layer.set(tab.selected);
        self.zoom.set(tab.zoom);
        self.pan_x.set(tab.pan_x);
        self.pan_y.set(tab.pan_y);
        self.code_dirty.set(false);
        self.doc_name.set(tab.name.clone());
        *self.history.borrow_mut() = tab.history.clone();
        *self.future.borrow_mut() = tab.future.clone();
        self.can_undo.set(!tab.history.is_empty());
        self.can_redo.set(!tab.future.is_empty());
    }

    pub(crate) fn switch_project(&self, id: u64) {
        if self.active_project.get_untracked() == id {
            return;
        }
        let target = self
            .projects
            .with_untracked(|tabs| tabs.iter().find(|t| t.id == id).cloned());
        let Some(target) = target else {
            return;
        };
        self.snapshot_active_project();
        self.active_project.set(id);
        self.apply_project(&target);
    }

    pub(crate) fn new_project(&self) {
        self.snapshot_active_project();
        let id = self.next_project_id.get();
        self.next_project_id.set(id + 1);
        let tab = ProjectTab::new(id, format!("Untitled {id}"), None);
        self.projects.update(|tabs| tabs.push(tab.clone()));
        self.active_project.set(id);
        self.apply_project(&tab);
        self.toast("New project");
    }

    pub(crate) fn close_project(&self, id: u64) {
        let active = self.active_project.get_untracked();
        if id == active {
            self.snapshot_active_project();
        }
        let index = self
            .projects
            .with_untracked(|tabs| tabs.iter().position(|t| t.id == id));
        let Some(index) = index else {
            return;
        };
        self.projects.update(|tabs| tabs.retain(|t| t.id != id));

        if self.projects.with_untracked(Vec::len) == 0 {
            self.new_project();
            return;
        }
        if id != active {
            return;
        }

        let next = self
            .projects
            .with_untracked(|tabs| tabs[index.min(tabs.len().saturating_sub(1))].clone());
        self.active_project.set(next.id);
        self.apply_project(&next);
    }
}
