//! State of the node graph window. The graph itself is *generated* from the
//! document (`build::graph_for`) — this struct only tracks which scope is
//! shown, remembers dragged positions across rebuilds and forwards the
//! selection to the editor. Rules stay editable in the Interact tab; every
//! document change rebuilds the view (`sync`, design doc §4.4).

use super::build::{BehaviorGraph, GraphIssue, graph_for};
use super::model::{GraphConnection, GraphNode, NodeKind, PortSide};
use crate::model::behavior;
use crate::model::behavior::{Action, ValueExpr};
use crate::model::svelte_ast::SvelteNode;
use crate::model::tree_ops;
use crate::state::EditorState;
use sequoia_ui::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Clone)]
pub(crate) enum GraphDrag {
    MoveNode {
        id: u64,
        start_wx: f32,
        start_wy: f32,
        orig_x: f32,
        orig_y: f32,
    },
    Pan {
        start_sx: f32,
        start_sy: f32,
        orig_pan_x: f32,
        orig_pan_y: f32,
    },
    /// Dragging a wire out of a socket; released over a compatible port it
    /// rewires the underlying rules (see `finish_wire`).
    Wire {
        node: u64,
        side: PortSide,
        port: usize,
    },
}

#[derive(Clone)]
pub(crate) struct NodeGraphState {
    pub editor: EditorState,
    /// Scope shown (master id or document root).
    pub scope_id: Signal<u64>,
    pub scope_name: Signal<String>,
    pub nodes: Signal<Vec<GraphNode>>,
    pub connections: Signal<Vec<GraphConnection>>,
    pub issues: Signal<Vec<GraphIssue>>,
    pub selected_node: Signal<Option<u64>>,
    pub zoom: Signal<f32>,
    pub pan_x: Signal<f32>,
    pub pan_y: Signal<f32>,
    pub repaint: Signal<u64>,
    pub drag: Rc<RefCell<Option<GraphDrag>>>,
    /// Wire preview endpoint (screen-space world coords) while dragging.
    pub wire_mouse: Rc<RefCell<Point>>,
    /// Compatible port under the cursor during a wire drag (highlighted).
    pub wire_hover: Rc<RefCell<Option<(u64, PortSide, usize)>>>,
    /// Browser panel filter (left side of the node editor).
    pub browser_query: Signal<String>,
    /// Manual positions by stable node key — they survive rebuilds.
    pub positions: Rc<RefCell<HashMap<String, (f32, f32)>>>,
    /// Rebuild guard: (history len, future len, scope id) of the last build.
    last_built: Rc<RefCell<Option<(usize, usize, u64)>>>,
}

impl NodeGraphState {
    pub(crate) fn new(editor: EditorState, scope_hint: Option<u64>) -> Self {
        let scope_id = match scope_hint {
            Some(id) => id,
            None => {
                let root = editor.root.get_untracked();
                match root.as_ref() {
                    Some(r) => match editor.selected_layer.get_untracked() {
                        Some(sel) => behavior::scope_root(r, sel).id(),
                        None => r.id(),
                    },
                    None => 0,
                }
            }
        };
        let s = Self {
            editor,
            scope_id: Signal::new(scope_id),
            scope_name: Signal::new(String::new()),
            nodes: Signal::new(Vec::new()),
            connections: Signal::new(Vec::new()),
            issues: Signal::new(Vec::new()),
            selected_node: Signal::new(None),
            zoom: Signal::new(1.0),
            pan_x: Signal::new(0.0),
            pan_y: Signal::new(0.0),
            repaint: Signal::new(0),
            drag: Rc::new(RefCell::new(None)),
            wire_mouse: Rc::new(RefCell::new(Point::new(0.0, 0.0))),
            wire_hover: Rc::new(RefCell::new(None)),
            browser_query: Signal::new(String::new()),
            positions: Rc::new(RefCell::new(HashMap::new())),
            last_built: Rc::new(RefCell::new(None)),
        };
        s.rebuild();
        s
    }

    /// Rebuilds the graph from the document, re-applying dragged positions.
    pub(crate) fn rebuild(&self) {
        let Some(root) = self.editor.root.get_untracked() else {
            return;
        };
        let g: BehaviorGraph = graph_for(&root, self.scope_id.get_untracked());
        self.scope_name.set(g.scope_name.clone());
        let mut nodes = g.nodes;
        {
            let pos = self.positions.borrow();
            for n in &mut nodes {
                if let Some((x, y)) = pos.get(&n.key) {
                    n.x = *x;
                    n.y = *y;
                }
            }
        }
        // Keep the selection when the same node still exists after the
        // rebuild (keys are stable across document edits).
        let old_nodes = self.nodes.get_untracked();
        let sel_key = self
            .selected_node
            .get_untracked()
            .and_then(|id| old_nodes.iter().find(|n| n.id == id))
            .map(|n| n.key.clone());
        self.selected_node.set(
            sel_key
                .and_then(|k| nodes.iter().find(|n| n.key == k))
                .map(|n| n.id),
        );
        self.nodes.set(nodes);
        self.connections.set(g.connections);
        self.issues.set(g.issues);
        self.bump();
    }

    /// Pull-based rebuild for the canvas track closure: cheap stamp check
    /// (history length + scope), full rebuild only on real changes.
    pub(crate) fn sync(&self) {
        let scope = self.scope_id.get_untracked();
        let stamp = (
            self.editor.history.borrow().len(),
            self.editor.future.borrow().len(),
            scope,
        );
        let mut last = self.last_built.borrow_mut();
        if last.as_ref() == Some(&stamp) {
            return;
        }
        *last = Some(stamp);
        drop(last);
        self.rebuild();
    }

    pub(crate) fn set_scope(&self, id: u64) {
        self.scope_id.set(id);
        self.selected_node.set(None);
        self.rebuild();
    }

    /// Scope choices for the toolbar chips: the document root plus every
    /// master component, document order.
    pub(crate) fn scope_options(&self) -> Vec<(u64, String)> {
        let Some(root) = self.editor.root.get_untracked() else {
            return Vec::new();
        };
        let mut out = Vec::new();
        collect_scopes(&root, &mut out);
        out
    }

    pub(crate) fn bump(&self) {
        self.repaint.update(|v| *v = v.wrapping_add(1));
    }

    pub(crate) fn screen_to_world(&self, sx: f32, sy: f32) -> (f32, f32) {
        let z = self.zoom.get_untracked();
        let px = self.pan_x.get_untracked();
        let py = self.pan_y.get_untracked();
        (sx / z + px, sy / z + py)
    }

    pub(crate) fn hit_node(&self, wx: f32, wy: f32) -> Option<u64> {
        let nodes = self.nodes.get_untracked();
        for n in nodes.iter().rev() {
            if n.contains(wx, wy) {
                return Some(n.id);
            }
        }
        None
    }

    /// The socket under a world position — checked before `hit_node` so a
    /// press on a port starts a wire drag instead of moving the node.
    pub(crate) fn hit_port(&self, wx: f32, wy: f32) -> Option<(u64, PortSide, usize)> {
        let nodes = self.nodes.get_untracked();
        for n in nodes.iter().rev() {
            if let Some((side, port)) = n.port_at(wx, wy) {
                return Some((n.id, side, port));
            }
        }
        None
    }

    /// World position of one socket (for the wire preview).
    pub(crate) fn port_world_pos(&self, node: u64, side: PortSide, port: usize) -> Option<Point> {
        self.nodes
            .get_untracked()
            .iter()
            .find(|n| n.id == node)
            .map(|n| n.port_pos(side, port))
    }

    /// Applies a finished wire drag: validates the socket pair and rewrites
    /// the rules the graph was generated from. The rebuild then draws the
    /// new connection — the graph stays a faithful view of the document.
    pub(crate) fn finish_wire(&self, from: (u64, PortSide, usize), to: (u64, PortSide, usize)) {
        let (src, dst) = match (from.1, to.1) {
            (PortSide::Output, PortSide::Input) => (from, to),
            (PortSide::Input, PortSide::Output) => (to, from),
            _ => {
                self.editor
                    .toast_warn("Connect an output socket to an input socket");
                return;
            }
        };
        if src.0 == dst.0 {
            self.editor.toast_warn("A node cannot connect to itself");
            return;
        }
        let nodes = self.nodes.get_untracked();
        let (Some(sn), Some(dn)) = (
            nodes.iter().find(|n| n.id == src.0),
            nodes.iter().find(|n| n.id == dst.0),
        ) else {
            return;
        };
        match (sn.kind, dn.kind) {
            // State "read" → action "↷ var" input: the value step reads it.
            (NodeKind::State, NodeKind::Action) => {
                if dst.2 == 0 {
                    self.editor
                        .toast_warn("Port ○ is execution — drop on a ↷ data input");
                    return;
                }
                let Some(var) = sn.key.strip_prefix("s:").map(str::to_string) else {
                    return;
                };
                let Some((el, ri, ai)) = parse_action_key(&dn.key) else {
                    self.editor
                        .toast_warn("Nested delay steps are wired in the Interact tab");
                    return;
                };
                self.set_action_source(el, ri, ai, &var);
            }
            // Action "⇒ var" output → state "set" input: retarget the write.
            (NodeKind::Action, NodeKind::State) => {
                if src.2 == 0 {
                    self.editor
                        .toast_warn("Data wires join a ⇒ output with the state's set input");
                    return;
                }
                let Some(var) = dn.key.strip_prefix("s:").map(str::to_string) else {
                    return;
                };
                let Some((el, ri, ai)) = parse_action_key(&sn.key) else {
                    self.editor
                        .toast_warn("Nested delay steps are wired in the Interact tab");
                    return;
                };
                self.set_action_target(el, ri, ai, &var);
            }
            // Execution: the dropped action runs right after the source.
            (NodeKind::Trigger, NodeKind::Action) | (NodeKind::Action, NodeKind::Action) => {
                if src.2 != 0 || dst.2 != 0 {
                    self.editor
                        .toast_warn("Execution wires join the ○ port-0 sockets");
                    return;
                }
                let s_addr = match sn.kind {
                    NodeKind::Trigger => parse_trigger_key(&sn.key).map(|(el, ri)| (el, ri, None)),
                    _ => parse_action_key(&sn.key).map(|(el, ri, ai)| (el, ri, Some(ai))),
                };
                let Some((el_s, ri_s, ai_s)) = s_addr else {
                    self.editor
                        .toast_warn("Nested delay steps are wired in the Interact tab");
                    return;
                };
                let Some((el_d, ri_d, ai_d)) = parse_action_key(&dn.key) else {
                    self.editor
                        .toast_warn("Nested delay steps are wired in the Interact tab");
                    return;
                };
                if el_s != el_d {
                    self.editor.toast_warn("A rule chain lives on one element");
                    return;
                }
                self.move_action_after(el_s, ri_s, ai_s, ri_d, ai_d);
            }
            _ => {
                self.editor.toast_warn("These sockets cannot be connected");
            }
        }
    }

    /// State → Set action: the action's value becomes a reference to `var`.
    fn set_action_source(&self, el: u64, ri: usize, ai: usize, var: &str) {
        let valid = self
            .editor
            .root
            .get_untracked()
            .as_ref()
            .and_then(|r| tree_ops::find(r, el))
            .and_then(|n| match n {
                SvelteNode::Element(e) => Some(e),
                _ => None,
            })
            .and_then(|e| e.behavior.as_ref())
            .and_then(|b| b.rules.get(ri))
            .and_then(|r| r.actions.get(ai))
            .map(|a| matches!(a, Action::SetVar(_, _)))
            .unwrap_or(false);
        if !valid {
            self.editor
                .toast_warn("Only a \"Set value\" action has a variable input");
            return;
        }
        self.editor.push_history();
        self.editor.root.update(|r| {
            let Some(rr) = r else { return };
            let Some(SvelteNode::Element(e)) = tree_ops::find_mut(rr, el) else {
                return;
            };
            let Some(b) = &mut e.behavior else { return };
            let Some(rule) = b.rules.get_mut(ri) else {
                return;
            };
            let target = match rule.actions.get(ai) {
                Some(Action::SetVar(v, _)) => v.clone(),
                _ => return,
            };
            rule.actions[ai] = Action::SetVar(target, ValueExpr::Var(var.to_string()));
        });
        self.editor
            .toast("Wired — the action now reads the variable");
    }

    /// Action ⇒ output → state: the step writes to `var` instead.
    fn set_action_target(&self, el: u64, ri: usize, ai: usize, var: &str) {
        let action = self
            .editor
            .root
            .get_untracked()
            .as_ref()
            .and_then(|r| tree_ops::find(r, el))
            .and_then(|n| match n {
                SvelteNode::Element(e) => Some(e),
                _ => None,
            })
            .and_then(|e| e.behavior.as_ref())
            .and_then(|b| b.rules.get(ri))
            .and_then(|r| r.actions.get(ai))
            .cloned();
        let Some(action) = action else { return };
        if matches!(action, Action::Emit(_) | Action::Delay(..)) {
            self.editor
                .toast_warn("This step has no variable to rewire");
            return;
        }
        self.editor.push_history();
        let mut ok = false;
        self.editor.root.update(|r| {
            let Some(rr) = r else { return };
            match action {
                Action::ToggleVar(_) | Action::SetVar(_, _) => {
                    if let Some(SvelteNode::Element(e)) = tree_ops::find_mut(rr, el)
                        && let Some(b) = &mut e.behavior
                        && let Some(rule) = b.rules.get_mut(ri)
                        && ai < rule.actions.len()
                    {
                        match &mut rule.actions[ai] {
                            Action::ToggleVar(v) => *v = var.to_string(),
                            Action::SetVar(v, _) => *v = var.to_string(),
                            _ => {}
                        }
                        ok = true;
                    }
                }
                // Show/Hide write through the TARGET element's binding.
                // `target` stores the element NAME, so resolve its id first.
                Action::Show { target, .. } | Action::Hide { target, .. } => {
                    if let Some(tid) = find_id_by_name(rr, &target)
                        && let Some(SvelteNode::Element(te)) = tree_ops::find_mut(rr, tid)
                        && let Some(bi) = &mut te.binding
                    {
                        bi.var = var.to_string();
                        ok = true;
                    }
                }
                _ => {}
            }
        });
        if ok {
            self.editor.toast("Wired — the step writes the variable");
        } else {
            self.editor
                .toast_warn("This step's target cannot be rewired");
        }
    }

    /// Execution wire: the destination action runs right after the source
    /// (port 0 of a trigger = first step of its rule).
    fn move_action_after(
        &self,
        el: u64,
        ri_s: usize,
        ai_s: Option<usize>,
        ri_d: usize,
        ai_d: usize,
    ) {
        // Already in place?
        match (ai_s, ri_s == ri_d) {
            (None, true) if ai_d == 0 => return,
            (Some(sa), true) if ai_d == sa || ai_d == sa + 1 => return,
            _ => {}
        }
        self.editor.push_history();
        let mut ok = false;
        self.editor.root.update(|r| {
            let Some(rr) = r else { return };
            if let Some(SvelteNode::Element(e)) = tree_ops::find_mut(rr, el)
                && let Some(b) = &mut e.behavior
            {
                let Some(rule_d) = b.rules.get_mut(ri_d) else {
                    return;
                };
                if ai_d >= rule_d.actions.len() {
                    return;
                }
                let step = rule_d.actions.remove(ai_d);
                let insert_at = match ai_s {
                    None => 0,
                    Some(sa) => {
                        if ri_s == ri_d && sa > ai_d {
                            sa
                        } else {
                            sa + 1
                        }
                    }
                };
                let Some(rule_s) = b.rules.get_mut(ri_s) else {
                    // Source rule vanished mid-edit — put the step back.
                    if let Some(rd) = b.rules.get_mut(ri_d) {
                        rd.actions.insert(ai_d.min(rd.actions.len()), step);
                    }
                    return;
                };
                rule_s
                    .actions
                    .insert(insert_at.min(rule_s.actions.len()), step);
                ok = true;
            }
        });
        if ok {
            self.editor.toast("Wired — the step runs after the source");
        }
    }

    /// Clicking a node selects the element it was built from in the editor,
    /// so the Interact tab can fix the rule right away.
    pub(crate) fn select_node(&self, id: u64) {
        self.selected_node.set(Some(id));
        if let Some(el) = self
            .nodes
            .get_untracked()
            .iter()
            .find(|n| n.id == id)
            .and_then(|n| n.element)
        {
            self.editor.selected_layer.set(Some(el));
        }
        self.bump();
    }

    fn node_key_of(&self, id: u64) -> Option<String> {
        self.nodes
            .get_untracked()
            .iter()
            .find(|n| n.id == id)
            .map(|n| n.key.clone())
    }

    /// Applies a node drag: live position update + remember by key.
    pub(crate) fn move_node(&self, id: u64, x: f32, y: f32) {
        self.nodes.update(|ns| {
            if let Some(n) = ns.iter_mut().find(|n| n.id == id) {
                n.x = x;
                n.y = y;
            }
        });
        if let Some(key) = self.node_key_of(id) {
            self.positions.borrow_mut().insert(key, (x, y));
        }
        self.bump();
    }
}

fn collect_scopes(node: &SvelteNode, out: &mut Vec<(u64, String)>) {
    if let SvelteNode::Element(e) = node {
        // The document root is always a scope (page-level rules); masters
        // join it as their own scopes.
        if e.component || out.is_empty() {
            out.push((e.id, e.name.clone()));
        }
    }
    for c in node.children() {
        collect_scopes(c, out);
    }
}

// ── node key parsing ─────────────────────────────────────────

/// Depth-first lookup of an element id by its NAME. `Action::Show`/`Hide`
/// store the target as a name string (that's what the Interact tab offers),
/// so rewiring has to resolve it back to an id before `find_mut` can touch
/// the binding. The first match wins.
fn find_id_by_name(node: &SvelteNode, name: &str) -> Option<u64> {
    if let SvelteNode::Element(e) = node
        && e.name == name
    {
        return Some(e.id);
    }
    for c in node.children() {
        if let Some(id) = find_id_by_name(c, name) {
            return Some(id);
        }
    }
    None
}

/// `"a:{el}:{ri}:{ai}"` → (element, rule, action). Keys with a nested path
/// (steps inside a Delay) are longer and rejected — the Interact tab edits
/// those.
fn parse_action_key(key: &str) -> Option<(u64, usize, usize)> {
    let parts: Vec<&str> = key.split(':').collect();
    if parts.len() != 4 || parts[0] != "a" {
        return None;
    }
    Some((
        parts[1].parse().ok()?,
        parts[2].parse().ok()?,
        parts[3].parse().ok()?,
    ))
}

/// `"t:{el}:{ri}"` → (element, rule).
fn parse_trigger_key(key: &str) -> Option<(u64, usize)> {
    let parts: Vec<&str> = key.split(':').collect();
    if parts.len() != 3 || parts[0] != "t" {
        return None;
    }
    Some((parts[1].parse().ok()?, parts[2].parse().ok()?))
}
