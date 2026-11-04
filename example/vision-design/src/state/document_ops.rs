use super::EditorState;
use crate::model::svelte_ast::SvelteNode;
use crate::model::tree_ops;

impl EditorState {
    pub(crate) fn new_document(&self) {
        self.push_history();
        self.root.set(None);
        self.selected_layer.set(None);
        self.zoom.set(1.0);
        self.pan_x.set(0.0);
        self.pan_y.set(0.0);
        self.toast("New document");
    }

    pub(crate) fn duplicate_selected(&self) {
        let Some(id) = self.selected_layer.get_untracked() else {
            self.toast_warn("Nothing selected"); return;
        };
        let root = self.root.get_untracked();
        let Some(node) = root.as_ref().and_then(|r| tree_ops::find(r, id)).cloned() else {
            return;
        };
        self.push_history();
        let mut next = self.next_node_id();
        let copy = reid(&node, &mut next);
        let new_id = copy.id();
        let parent = root.as_ref().and_then(|r| tree_ops::parent_of(r, id)).map(|p| p.id());
        self.root.update(|r| {
            if let Some(pid) = parent {
                tree_ops::insert_child(r, pid, copy);
            } else {
                *r = Some(copy);
            }
        });
        self.selected_layer.set(Some(new_id));
        self.toast("Duplicated");
    }

    pub(crate) fn copy_selected(&self) {
        let Some(id) = self.selected_layer.get_untracked() else {
            self.toast_warn("Nothing selected"); return;
        };
        let root = self.root.get_untracked();
        let Some(node) = root.as_ref().and_then(|r| tree_ops::find(r, id)).cloned() else { return; };
        *self.clipboard.borrow_mut() = vec![node];
        self.toast("Copied");
    }

    pub(crate) fn paste(&self) {
        let clip = self.clipboard.borrow().clone();
        if clip.is_empty() {
            self.toast_warn("Internal clipboard is empty"); return;
        }
        self.push_history();
        let mut next = self.next_node_id();
        let copies: Vec<SvelteNode> = clip.into_iter().map(|n| reid(&n, &mut next)).collect();
        let first_id = copies.first().map(|n| n.id());
        self.root.update(|r| {
            if r.is_none() {
                *r = copies.into_iter().next();
                return;
            }
            for c in copies {
                let _ = tree_ops::insert_child(r, r.as_ref().unwrap().id(), c);
            }
        });
        if let Some(id) = first_id { self.selected_layer.set(Some(id)); }
        self.toast("Pasted");
    }

    pub(crate) fn delete_selected(&self) {
        let Some(id) = self.selected_layer.get_untracked() else {
            self.toast_warn("Nothing selected"); return;
        };
        self.push_history();
        self.root.update(|r| { tree_ops::remove(r, id); });
        self.selected_layer.set(None);
        self.toast("Deleted");
    }

    pub(crate) fn bring_forward(&self) { self.reorder(1); }
    pub(crate) fn send_backward(&self) { self.reorder(-1); }
    pub(crate) fn bring_to_front(&self) { self.reorder_to_end(true); }
    pub(crate) fn send_to_back(&self) { self.reorder_to_end(false); }

    fn reorder(&self, delta: i32) {
        let Some(id) = self.selected_layer.get_untracked() else { return; };
        self.push_history();
        let root = self.root.get_untracked();
        let parent_id = root.as_ref()
            .and_then(|r| tree_ops::parent_of(r, id))
            .map(|p| p.id());
        self.root.update(|r| {
            let Some(rr) = r else { return };
            let container = if let Some(pid) = parent_id {
                tree_ops::find_mut(rr, pid).and_then(|n| n.children_mut())
            } else {
                None
            };
            if let Some(ch) = container {
                if let Some(i) = ch.iter().position(|c| c.id() == id) {
                    let ni = (i as i32 + delta).clamp(0, ch.len() as i32 - 1) as usize;
                    if ni != i {
                        let node = ch.remove(i);
                        ch.insert(ni, node);
                    }
                }
            }
        });
    }

    fn reorder_to_end(&self, to_front: bool) {
        let Some(id) = self.selected_layer.get_untracked() else { return; };
        self.push_history();
        let root = self.root.get_untracked();
        let parent_id = root.as_ref()
            .and_then(|r| tree_ops::parent_of(r, id))
            .map(|p| p.id());
        self.root.update(|r| {
            let Some(rr) = r else { return };
            let container = if let Some(pid) = parent_id {
                tree_ops::find_mut(rr, pid).and_then(|n| n.children_mut())
            } else { None };
            if let Some(ch) = container {
                if let Some(i) = ch.iter().position(|c| c.id() == id) {
                    let node = ch.remove(i);
                    if to_front { ch.push(node); } else { ch.insert(0, node); }
                }
            }
        });
    }

    pub(crate) fn toggle_visibility(&self) {
        let Some(id) = self.selected_layer.get_untracked() else { return; };
        self.root.update(|r| {
            if let Some(rr) = r {
                if let Some(n) = tree_ops::find_mut(rr, id) {
                    match n {
                        SvelteNode::Element(e) => e.visible = !e.visible,
                        SvelteNode::Text(t) => t.visible = !t.visible,
                    }
                }
            }
        });
    }

    pub(crate) fn toggle_lock(&self) {
        let Some(id) = self.selected_layer.get_untracked() else { return; };
        let mut now_locked = false;
        self.root.update(|r| {
            if let Some(rr) = r {
                if let Some(n) = tree_ops::find_mut(rr, id) {
                    match n {
                        SvelteNode::Element(e) => { e.locked = !e.locked; now_locked = e.locked; }
                        SvelteNode::Text(t) => { t.locked = !t.locked; now_locked = t.locked; }
                    }
                }
            }
        });
        self.toast(if now_locked { "Locked" } else { "Unlocked" });
    }

    pub(crate) fn move_layer(&self, id: u64, x: f32, y: f32) {
        self.root.update(|r| {
            if let Some(rr) = r {
                if let Some(n) = tree_ops::find_mut(rr, id) {
                    match n {
                        SvelteNode::Element(e) => {
                            e.style.position = crate::model::svelte_ast::Position::Absolute;
                            e.style.left = Some(x);
                            e.style.top = Some(y);
                        }
                        SvelteNode::Text(_) => {}
                    }
                }
            }
        });
    }

    pub(crate) fn resize_layer(&self, id: u64, x: f32, y: f32, w: f32, h: f32) {
        self.root.update(|r| {
            if let Some(rr) = r {
                if let Some(n) = tree_ops::find_mut(rr, id) {
                    match n {
                        SvelteNode::Element(e) => {
                            e.style.position = crate::model::svelte_ast::Position::Absolute;
                            e.style.left = Some(x);
                            e.style.top = Some(y);
                            e.style.width = crate::model::svelte_ast::Dimension::Fixed(w);
                            e.style.height = crate::model::svelte_ast::Dimension::Fixed(h);
                        }
                        SvelteNode::Text(t) => {
                            t.width = Some(w);
                            t.height = Some(h);
                        }
                    }
                }
            }
        });
    }
}

fn reid(node: &SvelteNode, next: &mut u64) -> SvelteNode {
    let id = *next;
    *next += 1;
    match node {
        SvelteNode::Element(e) => SvelteNode::Element(crate::model::svelte_ast::Element {
            id,
            name: e.name.clone(),
            tag: e.tag,
            style: e.style.clone(),
            children: e.children.iter().map(|c| reid(c, next)).collect(),
            visible: e.visible,
            locked: e.locked,
            component: e.component,
            instance: e.instance,
        }),
        SvelteNode::Text(t) => SvelteNode::Text(crate::model::svelte_ast::TextNode {
            id,
            name: t.name.clone(),
            content: t.content.clone(),
            style: t.style.clone(),
            width: t.width,
            height: t.height,
            visible: t.visible,
            locked: t.locked,
        }),
    }
}