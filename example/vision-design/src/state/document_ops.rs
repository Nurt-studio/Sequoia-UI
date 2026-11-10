use super::EditorState;
use crate::model::svelte_ast::SvelteNode;
use crate::model::tree_ops;

impl EditorState {
    pub(crate) fn new_document(&self) {
        self.new_project();
    }

    pub(crate) fn duplicate_selected(&self) {
        let Some(id) = self.selected_layer.get_untracked() else {
            self.toast_warn("Nothing selected");
            return;
        };
        let root = self.root.get_untracked();
        let Some(node) = root.as_ref().and_then(|r| tree_ops::find(r, id)).cloned() else {
            return;
        };
        self.push_history();
        let mut next = self.next_node_id();
        let copy = reid(&node, &mut next);
        let new_id = copy.id();
        let parent = root
            .as_ref()
            .and_then(|r| tree_ops::parent_of(r, id))
            .map(|p| p.id());
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
            self.toast_warn("Nothing selected");
            return;
        };
        let root = self.root.get_untracked();
        let Some(node) = root.as_ref().and_then(|r| tree_ops::find(r, id)).cloned() else {
            return;
        };
        *self.clipboard.borrow_mut() = vec![node];
        self.toast("Copied");
    }

    pub(crate) fn paste(&self) {
        let clip = self.clipboard.borrow().clone();
        if clip.is_empty() {
            self.toast_warn("Internal clipboard is empty");
            return;
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
        if let Some(id) = first_id {
            self.selected_layer.set(Some(id));
        }
        self.toast("Pasted");
    }

    pub(crate) fn delete_selected(&self) {
        let Some(id) = self.selected_layer.get_untracked() else {
            self.toast_warn("Nothing selected");
            return;
        };
        self.push_history();
        self.root.update(|r| {
            tree_ops::remove(r, id);
        });
        self.selected_layer.set(None);
        self.toast("Deleted");
    }

    pub(crate) fn bring_forward(&self) {
        self.reorder(1);
    }
    pub(crate) fn send_backward(&self) {
        self.reorder(-1);
    }
    pub(crate) fn bring_to_front(&self) {
        self.reorder_to_end(true);
    }
    pub(crate) fn send_to_back(&self) {
        self.reorder_to_end(false);
    }

    fn reorder(&self, delta: i32) {
        let Some(id) = self.selected_layer.get_untracked() else {
            return;
        };
        self.push_history();
        let root = self.root.get_untracked();
        let parent_id = root
            .as_ref()
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
        let Some(id) = self.selected_layer.get_untracked() else {
            return;
        };
        self.push_history();
        let root = self.root.get_untracked();
        let parent_id = root
            .as_ref()
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
                    let node = ch.remove(i);
                    if to_front {
                        ch.push(node);
                    } else {
                        ch.insert(0, node);
                    }
                }
            }
        });
    }

    pub(crate) fn toggle_visibility(&self) {
        let Some(id) = self.selected_layer.get_untracked() else {
            return;
        };
        self.toggle_layer_visible(id);
    }

    /// Eye toggle in the Layers panel: flips `visible` in the DOCUMENT.
    ///
    /// The old code mutated the derived `layers` signal, which the layout
    /// effect overwrote on the next rebuild — the change never stuck (and
    /// hidden layers were not even listed). Hidden layers now stay in the
    /// list (dimmed) and the eye actually hides/shows on canvas.
    pub(crate) fn toggle_layer_visible(&self, id: u64) {
        self.push_history();
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
        let Some(id) = self.selected_layer.get_untracked() else {
            return;
        };
        let mut now_locked = false;
        self.root.update(|r| {
            if let Some(rr) = r {
                if let Some(n) = tree_ops::find_mut(rr, id) {
                    match n {
                        SvelteNode::Element(e) => {
                            e.locked = !e.locked;
                            now_locked = e.locked;
                        }
                        SvelteNode::Text(t) => {
                            t.locked = !t.locked;
                            now_locked = t.locked;
                        }
                    }
                }
            }
        });
        self.toast(if now_locked { "Locked" } else { "Unlocked" });
    }

    /// Auto-layout parents position their children, so dragging a flow child
    /// reorders it instead of moving it. Returns `true` when the layer belongs
    /// to an auto-layout parent, i.e. the drag was handled here.
    pub(crate) fn reorder_in_auto_layout(&self, id: u64, wx: f32, wy: f32) -> bool {
        use crate::model::layout_engine::is_absolute;
        use crate::model::svelte_ast::{Display, FlexDir};

        let root = self.root.get_untracked();
        let Some(r) = root.as_ref() else {
            return false;
        };
        // Text is edited inline and its box is the drag target, so it never
        // reorders: dragging always moves it, even inside auto layout.
        if matches!(tree_ops::find(r, id), Some(SvelteNode::Text(_))) {
            return false;
        }
        let Some(SvelteNode::Element(parent)) = tree_ops::parent_of(r, id) else {
            return false;
        };
        let row = parent.style.display == Display::Flex && parent.style.flex_dir == FlexDir::Row;
        let grid = parent.style.display == Display::Grid;
        if parent.style.display != Display::Flex && !grid {
            return false;
        }

        // Only flow children are ordered by the parent; absolute ones move freely.
        let flow: Vec<(usize, u64)> = parent
            .children
            .iter()
            .enumerate()
            .filter(|(_, c)| !is_absolute(c))
            .map(|(i, c)| (i, c.id()))
            .collect();
        let Some(pos) = flow.iter().position(|(_, sid)| *sid == id) else {
            return false;
        };

        let flat = self.flat_snapshot();
        let mut target = 0usize;
        for (_, sid) in &flow {
            if *sid == id {
                continue;
            }
            let Some(rect) = flat.iter().find(|f| f.id == *sid).map(|f| f.rect) else {
                continue;
            };
            let before = if grid {
                let mid_y = rect.top() + rect.size.height * 0.5;
                wy > mid_y + 0.01
                    || (wy >= rect.top()
                        && wy <= rect.top() + rect.size.height
                        && wx > rect.left() + rect.size.width * 0.5)
            } else if row {
                wx > rect.left() + rect.size.width * 0.5
            } else {
                wy > rect.top() + rect.size.height * 0.5
            };
            if before {
                target += 1;
            }
        }
        if target == pos {
            return true;
        }

        let before_id = flow
            .iter()
            .filter(|(_, sid)| *sid != id)
            .nth(target)
            .map(|(_, sid)| *sid);
        self.root.update(|slot| {
            if let Some(rr) = slot.as_mut()
                && let Some(p) = tree_ops::parent_of_mut(rr, id)
                && let Some(node) = tree_ops::take_child(p, id)
            {
                let index = before_id
                    .and_then(|bid| p.children().iter().position(|c| c.id() == bid))
                    .unwrap_or(usize::MAX);
                tree_ops::insert_child_at(p, index, node);
            }
        });
        true
    }

    pub(crate) fn move_layer(&self, id: u64, x: f32, y: f32) {
        self.root.update(|r| {
            if let Some(rr) = r {
                // Flow children of an auto-layout parent are positioned by that
                // parent; the drag handler reorders them instead.
                if owns_child_positions(rr, id) {
                    return;
                }
                if let Some(n) = tree_ops::find_mut(rr, id) {
                    match n {
                        SvelteNode::Element(e) => {
                            e.style.position = crate::model::svelte_ast::Position::Absolute;
                            e.style.left = Some(x);
                            e.style.top = Some(y);
                        }
                        SvelteNode::Text(t) => {
                            t.absolute = true;
                            t.x = x;
                            t.y = y;
                        }
                    }
                }
            }
        });
    }

    pub(crate) fn resize_layer(&self, id: u64, x: f32, y: f32, w: f32, h: f32) {
        self.root.update(|r| {
            if let Some(rr) = r {
                // A flow child keeps its flow placement; resizing only changes
                // its size, like Figma does inside auto layout.
                let in_flow = owns_child_positions(rr, id);
                if let Some(n) = tree_ops::find_mut(rr, id) {
                    match n {
                        SvelteNode::Element(e) => {
                            if !in_flow {
                                e.style.position = crate::model::svelte_ast::Position::Absolute;
                                e.style.left = Some(x);
                                e.style.top = Some(y);
                            }
                            e.style.width = crate::model::svelte_ast::Dimension::Fixed(w);
                            e.style.height = crate::model::svelte_ast::Dimension::Fixed(h);
                        }
                        SvelteNode::Text(t) => {
                            if t.absolute {
                                t.x = x;
                                t.y = y;
                            }
                            t.width = Some(w);
                            t.height = Some(h);
                        }
                    }
                }
            }
        });
    }

    // ───────────────────────── Drag & drop / reparenting ─────────────────────────

    /// `true` when `candidate` lies inside the subtree rooted at `id`.
    pub(crate) fn is_descendant(&self, id: u64, candidate: u64) -> bool {
        let root = self.root.get_untracked();
        let Some(r) = root.as_ref() else {
            return false;
        };
        let mut cursor = tree_ops::find(r, candidate);
        while let Some(c) = cursor {
            if c.id() == id {
                return true;
            }
            cursor = tree_ops::parent_of(r, c.id());
        }
        false
    }

    /// Canvas drag & drop: dropping a layer over another container moves it
    /// there. `sx/sy` is the grab point, `wx/wy` the release point; a click
    /// without a real drag is ignored.
    pub(crate) fn reparent_on_drop(&self, id: u64, sx: f32, sy: f32, wx: f32, wy: f32) {
        if (wx - sx).abs() + (wy - sy).abs() <= 2.0 {
            return;
        }
        let root = self.root.get_untracked();
        let Some(r) = root.as_ref() else {
            return;
        };
        let Some(cur_parent) = tree_ops::parent_of(r, id).map(|p| p.id()) else {
            return;
        };
        // Deepest container under the release point, outside the dragged
        // subtree (a frame cannot be dropped into itself).
        let mut target =
            crate::model::tree_ops::deepest_container_at(&self.flat_snapshot(), wx, wy);
        while let Some(t) = target {
            if t != id && !self.is_descendant(id, t) {
                break;
            }
            target = tree_ops::parent_of(r, t).map(|p| p.id());
        }
        let Some(target) = target.filter(|t| *t != cur_parent && *t != 0) else {
            return;
        };
        self.move_node_into(id, target);
    }

    /// Wraps the current selection in a plain `div` group. Every selected
    /// layer must share a parent; the group is placed at the selection bounds
    /// and the children are re-based to group-local coordinates, so grouping
    /// never moves anything on screen. The group is an ordinary element, so
    /// codegen emits a nested `<div>` for it in the Svelte output.
    pub(crate) fn group_selection(&self) {
        use crate::model::svelte_ast::{
            Dimension, Display, Element, FlexDir, Position, Style, Tag,
        };

        let root = self.root.get_untracked();
        let Some(r) = root.as_ref() else {
            return;
        };
        let mut ids = self.selected_layers.get_untracked();
        if ids.is_empty()
            && let Some(id) = self.selected_layer.get_untracked()
        {
            ids.push(id);
        }
        ids.retain(|id| tree_ops::find(r, *id).is_some());
        ids.sort_unstable();
        ids.dedup();
        if ids.is_empty() {
            self.toast_warn("Select layers to group");
            return;
        }
        let root_id = r.id();
        let parent_id = |id: u64| {
            tree_ops::parent_of(r, id)
                .map(|p| p.id())
                .unwrap_or(root_id)
        };
        let parent_of_all = parent_id(ids[0]);
        if ids.iter().any(|id| parent_id(*id) != parent_of_all) {
            self.toast_warn("Grouping needs layers with the same parent");
            return;
        }
        if ids.contains(&root_id) {
            return;
        }

        let flat = self.flat_snapshot();
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;
        for id in &ids {
            if let Some(f) = flat.iter().find(|f| f.id == *id) {
                min_x = min_x.min(f.rect.left());
                min_y = min_y.min(f.rect.top());
                max_x = max_x.max(f.rect.left() + f.rect.size.width);
                max_y = max_y.max(f.rect.top() + f.rect.size.height);
            }
        }
        if !min_x.is_finite() || !min_y.is_finite() {
            self.toast_warn("Cannot group hidden layers");
            return;
        }
        let (bx, by) = (min_x, min_y);
        let (bw, bh) = ((max_x - min_x).max(1.0), (max_y - min_y).max(1.0));
        let gid = self.next_node_id();

        self.push_history();
        self.root.update(|slot| {
            let Some(rr) = slot.as_mut() else {
                return;
            };
            let Some(parent) = tree_ops::find_mut(rr, parent_of_all) else {
                return;
            };
            let Some(children) = parent.children_mut() else {
                return;
            };
            let first = children
                .iter()
                .position(|c| ids.contains(&c.id()))
                .unwrap_or(0);
            let mut taken = Vec::new();
            let mut i = 0;
            while i < children.len() {
                if ids.contains(&children[i].id()) {
                    let nid = children[i].id();
                    let mut node = children.remove(i);
                    set_absolute_at(&mut node, local_pos(&flat, nid, (bx, by)));
                    taken.push(node);
                } else {
                    i += 1;
                }
            }
            if taken.is_empty() {
                return;
            }
            let mut style = Style::default();
            style.display = Display::Flex;
            style.flex_dir = FlexDir::Col;
            style.position = Position::Absolute;
            style.left = Some(bx);
            style.top = Some(by);
            style.width = Dimension::Fixed(bw);
            style.height = Dimension::Fixed(bh);
            style.opacity = 100.0;
            style.fill = None;
            style.radius = 0.0;
            let group = SvelteNode::Element(Element {
                id: gid,
                name: "Group".into(),
                tag: Tag::Div,
                style,
                children: taken,
                visible: true,
                locked: false,
                component: false,
                instance: false,
                interface: None,
                overrides: Vec::new(),
                slot_content: Vec::new(),
                behavior: None,
                binding: None,
                hover: None,
                prototype: None,
            });
            children.insert(first.min(children.len()), group);
        });
        self.selected_layer.set(Some(gid));
        self.selected_layers.set(vec![gid]);
        self.toast("Grouped");
    }

    /// Dissolves the selected group/frame: its children move up into the
    /// parent at the group's slot, keeping their on-screen positions.
    pub(crate) fn ungroup_selection(&self) {
        let Some(id) = self.selected_layer.get_untracked() else {
            self.toast_warn("Select a group to ungroup");
            return;
        };
        let root = self.root.get_untracked();
        let Some(r) = root.as_ref() else {
            return;
        };
        if id == r.id() {
            self.toast_warn("The page root cannot be ungrouped");
            return;
        }
        let Some(parent_id) = tree_ops::parent_of(r, id).map(|p| p.id()) else {
            return;
        };
        let Some(SvelteNode::Element(group)) = tree_ops::find(r, id) else {
            return;
        };
        if group.children.is_empty() {
            self.toast_warn("Nothing to ungroup");
            return;
        }
        let flat = self.flat_snapshot();
        let origin = flat
            .iter()
            .find(|f| f.id == parent_id)
            .map(|f| (f.rect.left(), f.rect.top()))
            .unwrap_or((0.0, 0.0));
        // Prefer the flat rects (they account for auto layout); when the
        // layout snapshot has not caught up with a group just created, fall
        // back to the group's own offset plus the child's local position.
        let group_rect = flat.iter().find(|f| f.id == id).map(|f| f.rect);
        let group_pos = match (group.style.left, group.style.top) {
            (Some(x), Some(y)) => (x, y),
            _ => group_rect
                .map(|r| (r.left() - origin.0, r.top() - origin.1))
                .unwrap_or((0.0, 0.0)),
        };
        let child_positions: Vec<(u64, (f32, f32))> = group
            .children
            .iter()
            .map(|c| {
                let explicit = match c {
                    SvelteNode::Element(e) => e.style.left.is_some() && e.style.top.is_some(),
                    SvelteNode::Text(_) => true,
                };
                let local = match c {
                    SvelteNode::Element(e) => {
                        (e.style.left.unwrap_or(0.0), e.style.top.unwrap_or(0.0))
                    }
                    SvelteNode::Text(t) => (t.x, t.y),
                };
                let pos = if explicit {
                    (group_pos.0 + local.0, group_pos.1 + local.1)
                } else {
                    local_pos(&flat, c.id(), origin)
                };
                (c.id(), pos)
            })
            .collect();
        let first_child = group.children.first().map(|c| c.id());

        self.push_history();
        self.root.update(|slot| {
            let Some(rr) = slot.as_mut() else {
                return;
            };
            let Some(parent) = tree_ops::find_mut(rr, parent_id) else {
                return;
            };
            let Some(index) = parent.children().iter().position(|c| c.id() == id) else {
                return;
            };
            let Some(group) = tree_ops::take_child(parent, id) else {
                return;
            };
            let SvelteNode::Element(group) = group else {
                return;
            };
            for (i, mut child) in group.children.into_iter().enumerate() {
                let pos = child_positions
                    .iter()
                    .find(|(cid, _)| *cid == child.id())
                    .map(|(_, p)| *p)
                    .unwrap_or((0.0, 0.0));
                set_absolute_at(&mut child, pos);
                tree_ops::insert_child_at(parent, index + i, child);
            }
        });
        if let Some(fid) = first_child {
            self.selected_layer.set(Some(fid));
            self.selected_layers.set(vec![fid]);
        }
        self.toast("Ungrouped");
    }

    /// Moves `id` into `new_parent` (appended last). Auto-layout parents
    /// position flow children themselves, so the node becomes relative;
    /// otherwise it stays absolute at its current world rectangle.
    pub(crate) fn move_node_into(&self, id: u64, new_parent: u64) {
        if id == new_parent || self.is_descendant(id, new_parent) {
            return;
        }
        if let Some(r) = self.root.get_untracked()
            && tree_ops::parent_of(&r, id).map(|p| p.id()) == Some(new_parent)
            && tree_ops::find(&r, new_parent)
                .and_then(|p| p.children().last())
                .map(|c| c.id())
                == Some(id)
        {
            return;
        }
        let world = self
            .flat_snapshot()
            .iter()
            .find(|f| f.id == id)
            .map(|f| (f.rect.left(), f.rect.top()))
            .unwrap_or((0.0, 0.0));
        self.push_history();
        self.root.update(|r| {
            let Some(rr) = r.as_mut() else {
                return;
            };
            reparent_node(rr, id, ReparentPlace::IntoParent(new_parent), world);
        });
    }

    /// Layers-panel drop: `Before(next_id)` inserts the node right before
    /// `next_id` (becoming its sibling); `None` appends at the document root.
    /// Reordering within the same parent only changes the order; crossing
    /// parents repositions like `move_node_into`.
    pub(crate) fn move_layer_before(&self, id: u64, next_id: Option<u64>) {
        if next_id == Some(id) {
            return;
        }
        if let Some(r) = self.root.get_untracked() {
            let parent = tree_ops::parent_of(&r, id)
                .map(|p| p.id())
                .unwrap_or(r.id());
            let siblings = tree_ops::find(&r, parent)
                .map(|p| p.children().to_vec())
                .unwrap_or_default();
            let pos = siblings.iter().position(|c| c.id() == id);
            match next_id {
                Some(nid) => {
                    let same_parent = tree_ops::parent_of(&r, nid)
                        .map(|p| p.id())
                        .unwrap_or(r.id())
                        == parent;
                    let target = siblings.iter().position(|c| c.id() == nid);
                    if same_parent && matches!((pos, target), (Some(p), Some(t)) if t == p + 1) {
                        return;
                    }
                }
                None => {
                    if siblings.last().map(|c| c.id()) == Some(id) {
                        return;
                    }
                }
            }
        }
        let world = self
            .flat_snapshot()
            .iter()
            .find(|f| f.id == id)
            .map(|f| (f.rect.left(), f.rect.top()))
            .unwrap_or((0.0, 0.0));
        self.push_history();
        self.root.update(|r| {
            let Some(rr) = r.as_mut() else {
                return;
            };
            if let Some(nid) = next_id {
                if tree_ops::find(rr, nid).is_none() {
                    return;
                }
            }
            reparent_node(rr, id, ReparentPlace::Before(next_id), world);
        });
    }

    /// Aligns the current selection. `edge` is one of `left`, `center-h`,
    /// `right`, `top`, `center-v` or `bottom`.
    ///
    /// A single flow child of an auto-layout frame changes that frame's
    /// alignment, so it stays in the flow. Everything else (free-positioned
    /// children and marquee selections) is moved to an absolute position
    /// inside its parent, or to the selection's bounding box when several
    /// layers are selected.
    pub(crate) fn align_selection(&self, edge: &str) {
        use crate::model::layout_engine::is_absolute;
        use crate::model::svelte_ast::{Align, Display, FlexDir, Position};
        use sequoia_ui::prelude::Rect;

        let (horizontal, align) = match edge {
            "left" => (true, Align::Start),
            "center-h" => (true, Align::Center),
            "right" => (true, Align::End),
            "top" => (false, Align::Start),
            "center-v" => (false, Align::Center),
            "bottom" => (false, Align::End),
            _ => return,
        };

        let mut ids = self.selected_layers.get_untracked();
        if ids.is_empty()
            && let Some(id) = self.selected_layer.get_untracked()
        {
            ids.push(id);
        }
        if ids.is_empty() {
            self.toast_warn("Nothing selected");
            return;
        }
        ids.sort_unstable();
        ids.dedup();

        let root = self.root.get_untracked();
        let Some(r) = root.as_ref() else {
            return;
        };
        let flat = self.flat_snapshot();
        let mut items: Vec<(u64, Rect, Option<u64>)> = ids
            .into_iter()
            .filter_map(|id| {
                let f = flat.iter().find(|f| f.id == id)?;
                Some((id, f.rect, f.parent))
            })
            .collect();

        // Root layers have no parent frame to align against.
        items.retain(|(id, _, _)| tree_ops::parent_of(r, *id).is_some());
        if items.is_empty() {
            self.toast_warn("Root layer has no parent");
            return;
        }

        // A single flow child is positioned by its auto-layout parent, so
        // align the parent's items instead of taking the child out of flow.
        if let [(id, _, Some(pid))] = items.as_slice() {
            let child_absolute = tree_ops::find(r, *id).is_none_or(is_absolute);
            if !child_absolute
                && let Some(parent) = flat.iter().find(|f| f.id == *pid)
                && let Some(ps) = parent.style()
            {
                let slot = match ps.display {
                    Display::Flex => {
                        let row = ps.flex_dir == FlexDir::Row;
                        if horizontal == row {
                            Some(true)
                        } else {
                            Some(false)
                        }
                    }
                    Display::Grid => Some(horizontal),
                    _ => None,
                };
                if let Some(justify) = slot {
                    self.push_history();
                    self.root.update(|slot| {
                        if let Some(SvelteNode::Element(p)) =
                            slot.as_mut().and_then(|rr| tree_ops::find_mut(rr, *pid))
                        {
                            if justify {
                                p.style.justify_content = align;
                            } else {
                                p.style.align_items = align;
                            }
                        }
                    });
                    return;
                }
            }
        }

        let bounds = if let [(_, rect, Some(pid))] = items.as_slice() {
            flat.iter()
                .find(|f| f.id == *pid)
                .map(flat_content_rect)
                .unwrap_or(*rect)
        } else {
            let mut min_x = f32::MAX;
            let mut min_y = f32::MAX;
            let mut max_x = f32::MIN;
            let mut max_y = f32::MIN;
            for (_, rect, _) in &items {
                min_x = min_x.min(rect.left());
                min_y = min_y.min(rect.top());
                max_x = max_x.max(rect.left() + rect.size.width);
                max_y = max_y.max(rect.top() + rect.size.height);
            }
            Rect::from_xywh(min_x, min_y, max_x - min_x, max_y - min_y)
        };

        let mut changes = Vec::new();
        for (id, rect, _) in &items {
            let x = match edge {
                "left" => bounds.left(),
                "center-h" => bounds.left() + (bounds.size.width - rect.size.width) * 0.5,
                "right" => bounds.left() + bounds.size.width - rect.size.width,
                _ => rect.left(),
            };
            let y = match edge {
                "top" => bounds.top(),
                "center-v" => bounds.top() + (bounds.size.height - rect.size.height) * 0.5,
                "bottom" => bounds.top() + bounds.size.height - rect.size.height,
                _ => rect.top(),
            };
            if (x - rect.left()).abs() > 0.01 || (y - rect.top()).abs() > 0.01 {
                changes.push((*id, x, y));
            }
        }
        if changes.is_empty() {
            return;
        }

        self.push_history();
        self.root.update(|slot| {
            let Some(rr) = slot else { return };
            for (id, x, y) in changes {
                let Some(node) = tree_ops::find_mut(rr, id) else {
                    continue;
                };
                match node {
                    SvelteNode::Element(e) => {
                        e.style.position = Position::Absolute;
                        e.style.left = Some(x);
                        e.style.top = Some(y);
                        e.style.right = None;
                        e.style.bottom = None;
                    }
                    SvelteNode::Text(t) => {
                        t.absolute = true;
                        t.x = x;
                        t.y = y;
                    }
                }
            }
        });
    }

    /// Layers-panel drop: place `id` immediately after `prev_id` (as its
    /// sibling, or at the end of `prev_id`'s parent).
    pub(crate) fn move_layer_after(&self, id: u64, prev_id: u64) {
        if id == prev_id {
            return;
        }
        let root = self.root.get_untracked();
        let Some(r) = root.as_ref() else {
            return;
        };
        let Some(parent_id) = tree_ops::parent_of(r, prev_id).map(|p| p.id()) else {
            return;
        };
        let Some(parent) = tree_ops::find(r, parent_id) else {
            return;
        };
        let next = parent
            .children()
            .iter()
            .skip_while(|c| c.id() != prev_id)
            .nth(1)
            .map(|c| c.id());
        match next {
            Some(next) => self.move_layer_before(id, Some(next)),
            None => self.move_node_into(id, parent_id),
        }
    }
}

/// Where a reparented node should land.
pub(crate) enum ReparentPlace {
    /// Appended as the last child of `parent`.
    IntoParent(u64),
    /// Inserted right before `next` in its parent (document root when `None`).
    Before(Option<u64>),
}

/// Detaches `id` and re-inserts it at `place`. When the parent changes the
/// node is repositioned: flow children of auto-layout parents become
/// relative (the parent places them), everything else keeps its world
/// rectangle as an absolute node. Returns `true` when the tree changed.
fn reparent_node(root: &mut SvelteNode, id: u64, place: ReparentPlace, world: (f32, f32)) -> bool {
    use crate::model::svelte_ast::{Display, Position};

    let old_parent = tree_ops::parent_of(root, id).map(|p| p.id());
    let old_index =
        tree_ops::parent_of(root, id).and_then(|p| p.children().iter().position(|c| c.id() == id));
    let root_id = root.id();

    // Resolve the destination BEFORE detaching. A drop target that lives in
    // the dragged subtree disappears together with the node; resolving it
    // afterwards made the insert fail and silently lost the layer.
    let inside_dragged = |node: u64| {
        let mut cursor = tree_ops::find(root, node);
        while let Some(c) = cursor {
            if c.id() == id {
                return true;
            }
            cursor = tree_ops::parent_of(root, c.id());
        }
        false
    };
    let (parent_id, mut index) = match &place {
        ReparentPlace::IntoParent(pid) => (*pid, usize::MAX),
        // The root row has no parent: dropping before it means "first child of
        // the document", not "before the root element".
        ReparentPlace::Before(Some(next)) if *next == root_id => (root_id, 0),
        ReparentPlace::Before(Some(next)) => match tree_ops::parent_of(root, *next) {
            Some(p) => {
                let idx = p
                    .children()
                    .iter()
                    .position(|c| c.id() == *next)
                    .unwrap_or(usize::MAX);
                (p.id(), idx)
            }
            None => return false,
        },
        ReparentPlace::Before(None) => (root_id, usize::MAX),
    };
    // Refuse to drop a node into its own subtree, or before one of its own
    // descendants (that slot travels with the node).
    if parent_id == id || inside_dragged(parent_id) {
        return false;
    }
    if let ReparentPlace::Before(Some(next)) = &place
        && *next != root_id
        && inside_dragged(*next)
    {
        return false;
    }
    // Detaching the node first shifts a later slot in the same parent left.
    if old_parent == Some(parent_id)
        && let Some(old) = old_index
        && index != usize::MAX
        && old < index
    {
        index -= 1;
    }

    let Some(old_p) = tree_ops::parent_of_mut(root, id) else {
        return false;
    };
    let Some(node) = tree_ops::take_child(old_p, id) else {
        return false;
    };

    let inserted = {
        let Some(p) = tree_ops::find_mut(root, parent_id) else {
            return false;
        };
        tree_ops::insert_child_at(p, index, node)
    };
    if !inserted {
        return false;
    }

    // Positioning only matters when the node changed parents. Root children
    // have no `parent_of` (None), so compare against the root id; otherwise a
    // plain reorder at the top level would pull the layer into the root's
    // auto layout.
    let old_parent_id = old_parent.unwrap_or(root_id);
    if parent_id != old_parent_id {
        let auto = match tree_ops::find(root, parent_id) {
            Some(SvelteNode::Element(e)) => {
                matches!(e.style.display, Display::Flex | Display::Grid)
            }
            _ => false,
        };
        if let Some(n) = tree_ops::find_mut(root, id) {
            match n {
                SvelteNode::Element(e) => {
                    if auto {
                        e.style.position = Position::Relative;
                        e.style.left = None;
                        e.style.top = None;
                    } else {
                        e.style.position = Position::Absolute;
                        e.style.left = Some(world.0);
                        e.style.top = Some(world.1);
                    }
                    e.style.right = None;
                    e.style.bottom = None;
                }
                SvelteNode::Text(t) => {
                    if auto {
                        t.absolute = false;
                    } else {
                        t.absolute = true;
                        t.x = world.0;
                        t.y = world.1;
                    }
                }
            }
        }
    }
    true
}

/// Rect of `id` in the flat snapshot, expressed relative to `origin`.
fn local_pos(
    flat: &[crate::model::layout_engine::FlatNode],
    id: u64,
    origin: (f32, f32),
) -> (f32, f32) {
    flat.iter()
        .find(|f| f.id == id)
        .map(|f| (f.rect.left() - origin.0, f.rect.top() - origin.1))
        .unwrap_or((0.0, 0.0))
}

/// Pins a node at `pos` in its parent, leaving layout flow.
fn set_absolute_at(node: &mut SvelteNode, pos: (f32, f32)) {
    use crate::model::svelte_ast::Position;
    match node {
        SvelteNode::Element(e) => {
            e.style.position = Position::Absolute;
            e.style.left = Some(pos.0);
            e.style.top = Some(pos.1);
            e.style.right = None;
            e.style.bottom = None;
        }
        SvelteNode::Text(t) => {
            t.absolute = true;
            t.x = pos.0;
            t.y = pos.1;
        }
    }
}

/// True when `id` is a flow child of an auto-layout parent, i.e. the parent
/// decides its position.
/// Content box of a laid-out frame (its rectangle minus padding).
fn flat_content_rect(f: &crate::model::layout_engine::FlatNode) -> sequoia_ui::prelude::Rect {
    use sequoia_ui::prelude::Rect;

    let s = f.style();
    let (l, t, r, b) = s
        .map(|s| {
            (
                s.padding.left,
                s.padding.top,
                s.padding.right,
                s.padding.bottom,
            )
        })
        .unwrap_or((0.0, 0.0, 0.0, 0.0));
    Rect::from_xywh(
        f.rect.left() + l,
        f.rect.top() + t,
        (f.rect.size.width - l - r).max(0.0),
        (f.rect.size.height - t - b).max(0.0),
    )
}

fn owns_child_positions(root: &SvelteNode, id: u64) -> bool {
    use crate::model::layout_engine::is_absolute;
    use crate::model::svelte_ast::Display;

    let Some(node) = tree_ops::find(root, id) else {
        return false;
    };
    // Text can always be dragged; it leaves the flow and becomes positioned.
    if matches!(node, SvelteNode::Text(_)) {
        return false;
    }
    let Some(SvelteNode::Element(parent)) = tree_ops::parent_of(root, id) else {
        return false;
    };
    if parent.style.display == Display::None {
        return false;
    }
    !is_absolute(node)
}

fn reid(node: &SvelteNode, next: &mut u64) -> SvelteNode {
    let mut map = std::collections::HashMap::new();
    reid_map(node, &mut map, next)
}

/// Deep clone with fresh ids — used by the slot-content fill (Phase 4): the
/// copy placed in a slot is an independent node tree.
pub(crate) fn clone_with_new_ids(node: &SvelteNode, next: &mut u64) -> SvelteNode {
    reid(node, next)
}

/// Re-id walk that records old→new ids, so a duplicated master keeps its
/// interface targets (and slot containers) pointing at the right nodes.
fn reid_map(
    node: &SvelteNode,
    map: &mut std::collections::HashMap<u64, u64>,
    next: &mut u64,
) -> SvelteNode {
    let id = *next;
    *next += 1;
    map.insert(node.id(), id);
    match node {
        SvelteNode::Element(e) => SvelteNode::Element(crate::model::svelte_ast::Element {
            id,
            name: e.name.clone(),
            tag: e.tag,
            style: e.style.clone(),
            children: e.children.iter().map(|c| reid_map(c, map, next)).collect(),
            visible: e.visible,
            locked: e.locked,
            component: e.component,
            instance: e.instance,
            interface: e.interface.as_ref().map(|it| {
                let mut it = it.clone();
                crate::model::component_ops::remap_interface(&mut it, map);
                it
            }),
            overrides: e.overrides.clone(),
            slot_content: e
                .slot_content
                .iter()
                .map(|sc| crate::model::svelte_ast::SlotContent {
                    slot: sc.slot.clone(),
                    nodes: sc.nodes.iter().map(|n| reid_map(n, map, next)).collect(),
                })
                .collect(),
            behavior: e.behavior.clone(),
            binding: e.binding.clone(),
            hover: e.hover.clone(),
            prototype: e.prototype.clone(),
        }),
        SvelteNode::Text(t) => SvelteNode::Text(crate::model::svelte_ast::TextNode {
            id,
            name: t.name.clone(),
            content: t.content.clone(),
            style: t.style.clone(),
            width: t.width,
            height: t.height,
            x: t.x,
            y: t.y,
            absolute: t.absolute,
            visible: t.visible,
            locked: t.locked,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::svelte_ast::{
        Dimension, Display, Element, FlexDir, Position, Style, Tag, new_frame, new_rect,
    };

    fn row_frame() -> SvelteNode {
        let mut child = new_rect(2, 0.0, 0.0, 100.0, 40.0, 0x0D99FF);
        if let SvelteNode::Element(e) = &mut child {
            e.style.position = Position::Relative;
        }
        SvelteNode::Element(Element {
            id: 1,
            name: "Row".into(),
            tag: Tag::Div,
            style: Style {
                display: Display::Flex,
                flex_dir: FlexDir::Row,
                width: Dimension::Fixed(400.0),
                height: Dimension::Fixed(100.0),
                ..Default::default()
            },
            children: vec![child],
            visible: true,
            locked: false,
            component: false,
            instance: false,
            interface: None,
            overrides: Vec::new(),
            slot_content: Vec::new(),
            behavior: None,
            binding: None,
            hover: None,
            prototype: None,
        })
    }

    #[test]
    fn opacity_of_a_frame_multiplies_into_children() {
        let mut frame = new_frame(1, 0.0, 0.0, 200.0, 100.0);
        let mut child = new_rect(2, 10.0, 10.0, 20.0, 20.0, 0x0D99FF);
        if let (SvelteNode::Element(f), SvelteNode::Element(c)) = (&mut frame, &mut child) {
            f.style.opacity = 50.0;
            c.style.opacity = 50.0;
        }
        let text = SvelteNode::Text(crate::model::svelte_ast::TextNode {
            id: 3,
            name: "Label".into(),
            content: "Hi".into(),
            style: crate::model::svelte_ast::TextStyle {
                opacity: 50.0,
                ..Default::default()
            },
            width: None,
            height: None,
            x: 80.0,
            y: 10.0,
            absolute: false,
            visible: true,
            locked: false,
        });
        if let SvelteNode::Element(f) = &mut frame {
            f.children = vec![child, text];
        }

        let flat = crate::model::layout_engine::compute_layout(&Some(frame));
        let layers = crate::state::layers_from_flat(&flat);
        let frame_op = layers.iter().find(|l| l.id == 1).expect("frame").opacity;
        let child_op = layers.iter().find(|l| l.id == 2).expect("child").opacity;
        let text_op = layers.iter().find(|l| l.id == 3).expect("text").opacity;
        assert_eq!(frame_op, 50.0);
        assert_eq!(child_op, 25.0, "child opacity is relative to its parent");
        assert_eq!(text_op, 25.0, "text opacity is relative to its parent");
    }

    #[test]
    fn reordering_root_children_does_not_change_their_positioning() {
        let mut root = new_frame(1, 0.0, 0.0, 400.0, 200.0);
        if let SvelteNode::Element(e) = &mut root {
            e.children = vec![
                new_rect(2, 10.0, 20.0, 50.0, 40.0, 0x0D99FF),
                new_rect(3, 80.0, 20.0, 50.0, 40.0, 0x42BE65),
            ];
        }
        let state = EditorState::with_seed(Some(root), Some(2));
        state.move_layer_before(2, Some(3));

        let root = state.root.get_untracked().expect("root");
        match tree_ops::find(&root, 2).expect("layer") {
            SvelteNode::Element(e) => {
                assert_eq!(e.style.position, Position::Absolute);
                assert_eq!(e.style.left, Some(10.0));
                assert_eq!(e.style.top, Some(20.0));
            }
            _ => panic!("expected an element"),
        }
    }

    #[test]
    fn dropping_before_the_root_row_keeps_the_layer() {
        let state = EditorState::with_seed(Some(row_frame()), Some(2));
        state.move_layer_before(2, Some(1));

        let root = state.root.get_untracked().expect("root");
        assert!(tree_ops::find(&root, 2).is_some(), "layer lost on drop");
        assert_eq!(root.children().first().map(|c| c.id()), Some(2));
    }

    #[test]
    fn dropping_a_frame_before_its_own_child_keeps_the_frame() {
        let mut outer = new_frame(1, 0.0, 0.0, 400.0, 200.0);
        let mut inner = new_frame(2, 0.0, 0.0, 200.0, 100.0);
        if let SvelteNode::Element(e) = &mut inner {
            e.style.position = Position::Absolute;
            e.children = vec![new_rect(3, 0.0, 0.0, 50.0, 20.0, 0x0D99FF)];
        }
        if let SvelteNode::Element(e) = &mut outer {
            e.style.display = Display::None;
            e.children = vec![inner, new_rect(4, 300.0, 0.0, 50.0, 20.0, 0x42BE65)];
        }
        let state = EditorState::with_seed(Some(outer), Some(2));
        state.move_layer_before(2, Some(3));

        let root = state.root.get_untracked().expect("root");
        assert!(tree_ops::find(&root, 2).is_some(), "frame lost on drop");
        assert!(tree_ops::find(&root, 3).is_some(), "child lost on drop");
    }

    #[test]
    fn resizing_and_nudging_a_flow_child_keeps_it_in_the_auto_layout() {
        let state = EditorState::with_seed(Some(row_frame()), Some(2));

        state.resize_layer(2, 500.0, 500.0, 150.0, 50.0);
        state.move_layer(2, 900.0, 900.0);

        let root = state.root.get_untracked().expect("root");
        match tree_ops::find(&root, 2).expect("child") {
            SvelteNode::Element(e) => {
                assert_eq!(e.style.position, Position::Relative, "stays in the flow");
                assert_eq!(e.style.width, Dimension::Fixed(150.0));
                assert_eq!(e.style.height, Dimension::Fixed(50.0));
            }
            _ => panic!("expected an element"),
        }

        let rect = state
            .flat_snapshot()
            .into_iter()
            .find(|f| f.id == 2)
            .map(|f| f.rect)
            .expect("child laid out");
        assert_eq!(
            (rect.left(), rect.top()),
            (0.0, 0.0),
            "parent still places it"
        );
        assert_eq!(rect.size.width, 150.0);
    }

    #[test]
    fn dropping_after_a_row_places_the_layer_next_to_it() {
        let mut root = new_frame(1, 0.0, 0.0, 400.0, 300.0);
        if let SvelteNode::Element(e) = &mut root {
            e.children = vec![
                new_rect(2, 10.0, 20.0, 50.0, 40.0, 0x0D99FF),
                new_rect(3, 80.0, 20.0, 50.0, 40.0, 0x42BE65),
                new_rect(4, 150.0, 20.0, 50.0, 40.0, 0xF1C21B),
            ];
        }
        let state = EditorState::with_seed(Some(root), Some(2));
        state.move_layer_after(2, 3);
        let root = state.root.get_untracked().expect("root");
        let order: Vec<u64> = root.children().iter().map(|c| c.id()).collect();
        assert_eq!(order, vec![3, 2, 4], "dropped after row 3");
    }

    #[test]
    fn grouping_wraps_siblings_and_ungrouping_restores_them() {
        let mut root = new_frame(1, 0.0, 0.0, 400.0, 300.0);
        if let SvelteNode::Element(e) = &mut root {
            e.children = vec![
                new_rect(2, 10.0, 20.0, 50.0, 40.0, 0x0D99FF),
                new_rect(3, 80.0, 20.0, 50.0, 40.0, 0x42BE65),
            ];
        }
        let state = EditorState::with_seed(Some(root), Some(2));
        state.selected_layers.set(vec![2, 3]);
        state.group_selection();

        let root = state.root.get_untracked().expect("root");
        let gid = state
            .selected_layer
            .get_untracked()
            .expect("group selected");
        match tree_ops::find(&root, gid).expect("group exists") {
            SvelteNode::Element(g) => {
                assert_eq!(g.name, "Group");
                assert_eq!(g.tag, crate::model::svelte_ast::Tag::Div);
                assert_eq!(g.children.len(), 2);
                assert_eq!(g.style.left, Some(10.0));
                assert_eq!(g.style.top, Some(20.0));
                assert_eq!(g.style.width, Dimension::Fixed(120.0));
                match &g.children[0] {
                    SvelteNode::Element(c) => {
                        assert_eq!(c.style.left, Some(0.0), "group-local x");
                        assert_eq!(c.style.top, Some(0.0));
                        assert_eq!(c.style.position, Position::Absolute);
                    }
                    _ => panic!("expected an element child"),
                }
            }
            _ => panic!("expected an element group"),
        }

        state.ungroup_selection();
        let root = state.root.get_untracked().expect("root after ungroup");
        assert!(tree_ops::find(&root, gid).is_none(), "group removed");
        match tree_ops::find(&root, 2).expect("child back") {
            SvelteNode::Element(e) => {
                assert_eq!(e.style.left, Some(10.0), "world position restored");
                assert_eq!(e.style.top, Some(20.0));
            }
            _ => panic!("expected an element"),
        }
    }

    #[test]
    fn aligning_a_free_layer_uses_the_parent_content_box() {
        let mut root = new_frame(1, 0.0, 0.0, 400.0, 300.0);
        if let SvelteNode::Element(e) = &mut root {
            e.style.display = Display::None;
            e.style.padding = sequoia_ui::prelude::Edges::all(10.0);
            e.children = vec![new_rect(2, 30.0, 40.0, 50.0, 40.0, 0x0D99FF)];
        }
        let state = EditorState::with_seed(Some(root), Some(2));

        state.align_selection("right");
        match tree_ops::find(&state.root.get_untracked().expect("root"), 2).expect("child") {
            SvelteNode::Element(e) => {
                assert_eq!(e.style.left, Some(340.0), "400 - 10 - 50");
                assert_eq!(e.style.top, Some(40.0));
                assert_eq!(e.style.position, Position::Absolute);
            }
            _ => panic!("expected an element"),
        }

        state.align_selection("bottom");
        match tree_ops::find(&state.root.get_untracked().expect("root"), 2).expect("child") {
            SvelteNode::Element(e) => {
                assert_eq!(e.style.left, Some(340.0));
                assert_eq!(e.style.top, Some(250.0), "300 - 10 - 40");
            }
            _ => panic!("expected an element"),
        }
    }

    #[test]
    fn aligning_a_flow_child_changes_the_parent_layout() {
        use crate::model::svelte_ast::Align;

        let state = EditorState::with_seed(Some(row_frame()), Some(2));
        state.align_selection("center-v");
        match tree_ops::find(&state.root.get_untracked().expect("root"), 1).expect("parent") {
            SvelteNode::Element(e) => assert_eq!(e.style.align_items, Align::Center),
            _ => panic!("expected the row frame"),
        }

        state.align_selection("right");
        let root = state.root.get_untracked().expect("root");
        match tree_ops::find(&root, 1).expect("parent") {
            SvelteNode::Element(e) => assert_eq!(e.style.justify_content, Align::End),
            _ => panic!("expected the row frame"),
        }
        match tree_ops::find(&root, 2).expect("child") {
            SvelteNode::Element(e) => {
                assert_eq!(e.style.position, Position::Relative, "stays in the flow");
            }
            _ => panic!("expected an element"),
        }
    }

    #[test]
    fn grid_column_signal_updates_the_selected_frame() {
        let mut root = new_frame(1, 0.0, 0.0, 300.0, 200.0);
        if let SvelteNode::Element(e) = &mut root {
            e.style.display = Display::Grid;
            e.style.grid_cols = Some(3);
            e.children = vec![new_rect(2, 0.0, 0.0, 40.0, 30.0, 0x0D99FF)];
        }
        let state = EditorState::with_seed(Some(root), Some(1));
        assert_eq!(state.sel_grid_cols.get_untracked(), 3.0);

        state.sel_grid_cols.set(4.0);

        match tree_ops::find(&state.root.get_untracked().expect("root"), 1).expect("frame") {
            SvelteNode::Element(e) => assert_eq!(e.style.grid_cols, Some(4)),
            _ => panic!("expected the grid frame"),
        }
    }

    #[test]
    fn grid_row_and_align_self_signals_update_the_selection() {
        use crate::model::svelte_ast::Align;
        let mut root = new_frame(1, 0.0, 0.0, 300.0, 200.0);
        if let SvelteNode::Element(e) = &mut root {
            e.style.display = Display::Grid;
            e.style.grid_rows = Some(2);
            e.children = vec![new_rect(2, 0.0, 0.0, 40.0, 30.0, 0x0D99FF)];
        }
        let state = EditorState::with_seed(Some(root), Some(1));
        assert_eq!(state.sel_grid_rows.get_untracked(), 2.0);
        state.sel_grid_rows.set(4.0);
        match tree_ops::find(&state.root.get_untracked().expect("root"), 1).expect("frame") {
            SvelteNode::Element(e) => assert_eq!(e.style.grid_rows, Some(4)),
            _ => panic!("expected the grid frame"),
        }
        state.sel_grid_rows.set(0.0);
        match tree_ops::find(&state.root.get_untracked().expect("root"), 1).expect("frame") {
            SvelteNode::Element(e) => assert_eq!(e.style.grid_rows, None),
            _ => panic!("expected the grid frame"),
        }

        state.selected_layer.set(Some(2));
        assert_eq!(state.sel_align_self.get_untracked(), "auto");
        state.sel_align_self.set("end".into());
        match tree_ops::find(&state.root.get_untracked().expect("root"), 2).expect("child") {
            SvelteNode::Element(e) => assert_eq!(e.style.align_self, Some(Align::End)),
            _ => panic!("expected the child"),
        }
        state.sel_align_self.set("auto".into());
        match tree_ops::find(&state.root.get_untracked().expect("root"), 2).expect("child") {
            SvelteNode::Element(e) => assert_eq!(e.style.align_self, None),
            _ => panic!("expected the child"),
        }
    }

    #[test]
    fn aligning_a_marquee_selection_uses_its_bounding_box() {
        let mut root = new_frame(1, 0.0, 0.0, 400.0, 300.0);
        if let SvelteNode::Element(e) = &mut root {
            e.style.display = Display::None;
            e.children = vec![
                new_rect(2, 10.0, 20.0, 50.0, 40.0, 0x0D99FF),
                new_rect(3, 100.0, 80.0, 30.0, 20.0, 0x42BE65),
            ];
        }
        let state = EditorState::with_seed(Some(root), Some(2));
        state.selected_layers.set(vec![2, 3]);

        state.align_selection("left");
        state.align_selection("top");

        let root = state.root.get_untracked().expect("root");
        for id in [2, 3] {
            match tree_ops::find(&root, id).expect("layer") {
                SvelteNode::Element(e) => {
                    assert_eq!(e.style.left, Some(10.0), "layer {id} left");
                    assert_eq!(e.style.top, Some(20.0), "layer {id} top");
                }
                _ => panic!("expected an element"),
            }
        }
    }
}
