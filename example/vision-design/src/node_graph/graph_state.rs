//! State of the node graph window. The graph itself is *generated* from the
//! document (`build::graph_for`) — this struct only tracks which scope is
//! shown, remembers dragged positions across rebuilds and forwards the
//! selection to the editor. Rules stay editable in the Interact tab; every
//! document change rebuilds the view (`sync`, design doc §4.4).

use super::build::{BehaviorGraph, GraphIssue, graph_for};
use super::model::{GraphConnection, GraphNode, NodeKind, PortSide};
use crate::model::behavior;
use crate::model::behavior::{Action, Rule, Trigger, Value, ValueExpr, Var, VarType};
use crate::model::svelte_ast::{Element, SvelteNode};
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

/// What a stable graph key points at in the document. The node editor edits
/// these directly, so the graph stays a live view of the behavior model
/// (positions and selection live only in the window).
#[derive(Clone, Debug, PartialEq, Eq)]
enum KeyTarget {
    Trigger {
        el: u64,
        rule: usize,
    },
    Action {
        el: u64,
        rule: usize,
        path: Vec<usize>,
    },
    State {
        name: String,
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

    // ── authoring ────────────────────────────────────────────────
    //
    // The graph edits the same `Behavior` the Interact tab edits. The window
    // never stores a second copy of the program: every operation validates,
    // pushes one history entry, mutates the document and rebuilds the view.

    fn root_untracked(&self) -> Option<SvelteNode> {
        self.editor.root.get_untracked()
    }

    pub(crate) fn scope_owner_id(&self) -> u64 {
        self.scope_id.get_untracked()
    }

    /// Element the next created node is attached to: the selected node's
    /// element, the editor selection when it is inside this scope, or the
    /// scope owner (a master / the page).
    pub(crate) fn target_element(&self) -> Option<u64> {
        let root = self.root_untracked()?;
        let scope = self.scope_id.get_untracked();
        let node_el = self.selected_node.get_untracked().and_then(|id| {
            self.nodes
                .get_untracked()
                .iter()
                .find(|n| n.id == id)
                .and_then(|n| n.element)
        });
        let editor_sel = self.editor.selected_layer.get_untracked();
        for cand in [node_el, editor_sel].into_iter().flatten() {
            if editable_element_in_scope(&root, scope, cand) {
                return Some(cand);
            }
        }
        editable_element_in_scope(&root, scope, scope).then_some(scope)
    }

    /// Variables visible in the shown scope (nearest declaration wins).
    pub(crate) fn scope_var_list(&self) -> Vec<Var> {
        let Some(root) = self.root_untracked() else {
            return Vec::new();
        };
        behavior::scope_vars(&root, self.scope_id.get_untracked())
    }

    /// Show/Hide targets of the scope, in document order.
    pub(crate) fn scope_target_names(&self) -> Vec<String> {
        let Some(root) = self.root_untracked() else {
            return Vec::new();
        };
        let scope = self.scope_id.get_untracked();
        find_in_subtree(&root, scope, scope)
            .map(|n| {
                behavior::descendant_targets(n)
                    .into_iter()
                    .map(|(_, name, _)| name)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Events published by the scope owner's component interface.
    pub(crate) fn scope_event_names(&self) -> Vec<String> {
        let Some(root) = self.root_untracked() else {
            return Vec::new();
        };
        let scope = self.scope_id.get_untracked();
        find_in_subtree(&root, scope, scope)
            .and_then(|n| match n {
                SvelteNode::Element(e) => e.interface.as_ref(),
                _ => None,
            })
            .map(|it| it.events.clone())
            .unwrap_or_default()
    }

    /// Declares a fresh boolean variable on the scope owner and selects its
    /// State node. The node graph can grow a program from scratch.
    pub(crate) fn add_state_node(&self) -> bool {
        let Some(root) = self.root_untracked() else {
            self.editor.toast_warn("No document");
            return false;
        };
        let scope = self.scope_id.get_untracked();
        if !matches!(
            find_in_subtree(&root, scope, scope),
            Some(SvelteNode::Element(_))
        ) {
            self.editor.toast_warn("This scope cannot own variables");
            return false;
        }
        let vars = self.scope_var_list();
        let name = next_var_name(&vars, "var");
        self.editor.push_history();
        let mut ok = false;
        self.editor.root.update(|r| {
            let Some(rr) = r else { return };
            let Some(SvelteNode::Element(e)) = tree_ops::find_mut(rr, scope) else {
                return;
            };
            let b = e.behavior.get_or_insert_with(Default::default);
            b.vars.push(Var::new(&name, VarType::Bool));
            ok = true;
        });
        if ok {
            self.rebuild();
            self.select_key(&format!("s:{name}"));
            self.editor.toast(format!("Added state {name}"));
        }
        ok
    }

    /// Adds an `On Click` rule to the selected element (or the scope owner)
    /// and selects the new Trigger node.
    pub(crate) fn add_rule_node(&self) -> bool {
        let Some(el) = self.target_element() else {
            self.editor
                .toast_warn("Select an element in this scope first");
            return false;
        };
        self.editor.push_history();
        let mut rule = None;
        self.editor.root.update(|r| {
            let Some(rr) = r else { return };
            let Some(SvelteNode::Element(e)) = tree_ops::find_mut(rr, el) else {
                return;
            };
            let b = e.behavior.get_or_insert_with(Default::default);
            b.rules.push(Rule::new(Trigger::OnClick));
            rule = Some(b.rules.len() - 1);
        });
        let Some(ri) = rule else { return false };
        self.rebuild();
        self.select_key(&format!("t:{el}:{ri}"));
        self.editor.toast("Added rule On Click");
        true
    }

    /// Adds a default action to the selected rule. With no rule selected it
    /// uses the first rule of the target element, creating `On Click` when
    /// the element has none yet.
    pub(crate) fn add_action_node(&self, kind: &str) -> bool {
        let Some(root) = self.root_untracked() else {
            self.editor.toast_warn("No document");
            return false;
        };
        let (el, ri, needs_rule) = match self.selected_rule() {
            Some((el, ri)) => (el, ri, false),
            None => {
                let Some(el) = self.target_element() else {
                    self.editor
                        .toast_warn("Select an element in this scope first");
                    return false;
                };
                match first_rule_index(&root, el) {
                    Some(ri) => (el, ri, false),
                    None => (el, 0, true),
                }
            }
        };
        let Some((action, declare)) = default_action_for(&root, el, kind) else {
            self.editor.toast_warn("Unknown action kind");
            return false;
        };

        self.editor.push_history();
        let mut insertion = None;
        self.editor.root.update(|r| {
            let Some(rr) = r else { return };
            if let Some((owner, name)) = &declare
                && let Some(SvelteNode::Element(oe)) = tree_ops::find_mut(rr, *owner)
            {
                let b = oe.behavior.get_or_insert_with(Default::default);
                if !b.vars.iter().any(|v| v.name == *name) {
                    b.vars.push(Var::new(name, VarType::Bool));
                }
            }
            let Some(SvelteNode::Element(e)) = tree_ops::find_mut(rr, el) else {
                return;
            };
            let b = e.behavior.get_or_insert_with(Default::default);
            if needs_rule {
                b.rules.push(Rule::new(Trigger::OnClick));
            }
            let Some(rule) = b.rules.get_mut(ri) else {
                return;
            };
            let ai = rule.actions.len();
            rule.actions.push(action.clone());
            insertion = Some(ai);
        });
        let Some(ai) = insertion else { return false };
        self.rebuild();
        self.select_key(&format!("a:{el}:{ri}:{ai}"));
        self.editor
            .toast(format!("Added action {}", action.label()));
        true
    }

    fn selected_key_target(&self) -> Option<KeyTarget> {
        let id = self.selected_node.get_untracked()?;
        let key = self
            .nodes
            .get_untracked()
            .iter()
            .find(|n| n.id == id)
            .map(|n| n.key.clone())?;
        parse_key(&key)
    }

    fn selected_rule(&self) -> Option<(u64, usize)> {
        match self.selected_key_target()? {
            KeyTarget::Trigger { el, rule } => Some((el, rule)),
            KeyTarget::Action { el, rule, .. } => Some((el, rule)),
            KeyTarget::State { .. } => None,
        }
    }

    /// The trigger currently selected, read from the document.
    pub(crate) fn selected_trigger(&self) -> Option<Trigger> {
        let KeyTarget::Trigger { el, rule } = self.selected_key_target()? else {
            return None;
        };
        let root = self.root_untracked()?;
        find_element(&root, el)?
            .behavior
            .as_ref()?
            .rules
            .get(rule)
            .map(|r| r.trigger.clone())
    }

    /// The variable behind the selected State node, if it is declared by the
    /// shown scope's owner.
    pub(crate) fn selected_state(&self) -> Option<Var> {
        let KeyTarget::State { name } = self.selected_key_target()? else {
            return None;
        };
        let root = self.root_untracked()?;
        find_element(&root, self.scope_id.get_untracked())?
            .behavior
            .as_ref()?
            .vars
            .iter()
            .find(|v| v.name == name)
            .cloned()
    }

    /// The action behind the selected Action node (nested Delay steps too).
    pub(crate) fn selected_action(&self) -> Option<Action> {
        let KeyTarget::Action { el, rule, path } = self.selected_key_target()? else {
            return None;
        };
        let root = self.root_untracked()?;
        let r = find_element(&root, el)?
            .behavior
            .as_ref()?
            .rules
            .get(rule)?;
        action_at_path(r, &path).cloned()
    }

    /// Deletes the selected Trigger/Action/State node and its document
    /// counterpart. Variables are removed through the editor helper, so
    /// dangling action references are pruned across the whole scope.
    pub(crate) fn delete_selected_node(&self) -> bool {
        let Some(target) = self.selected_key_target() else {
            self.editor.toast_warn("Select a node first");
            return false;
        };
        let key = self.selected_node_key();
        match target {
            KeyTarget::State { name } => {
                if self.selected_state().is_none() {
                    self.editor
                        .toast_warn("This state is not declared in this scope");
                    return false;
                }
                let scope = self.scope_id.get_untracked();
                let prev = self.editor.selected_layer.get_untracked();
                self.editor.selected_layer.set(Some(scope));
                self.editor.remove_var(&name);
                self.editor.selected_layer.set(prev);
                if let Some(k) = key {
                    self.positions.borrow_mut().remove(&k);
                }
                self.selected_node.set(None);
                self.rebuild();
                self.editor.toast(format!("Removed state {name}"));
                true
            }
            KeyTarget::Trigger { el, rule } => {
                if !self.rule_exists(el, rule) {
                    return false;
                }
                self.editor.push_history();
                let mut ok = false;
                self.editor.root.update(|r| {
                    let Some(rr) = r else { return };
                    let Some(SvelteNode::Element(e)) = tree_ops::find_mut(rr, el) else {
                        return;
                    };
                    if let Some(b) = &mut e.behavior
                        && rule < b.rules.len()
                    {
                        b.rules.remove(rule);
                        ok = true;
                        if b.is_empty() {
                            e.behavior = None;
                        }
                    }
                });
                if ok {
                    if let Some(k) = key {
                        self.positions.borrow_mut().remove(&k);
                    }
                    self.selected_node.set(None);
                    self.rebuild();
                    self.editor.toast("Removed rule");
                }
                ok
            }
            KeyTarget::Action { el, rule, path } => {
                if !self.rule_exists(el, rule) {
                    return false;
                }
                self.editor.push_history();
                let mut ok = false;
                self.editor.root.update(|r| {
                    let Some(rr) = r else { return };
                    let Some(SvelteNode::Element(e)) = tree_ops::find_mut(rr, el) else {
                        return;
                    };
                    if let Some(b) = &mut e.behavior
                        && let Some(rl) = b.rules.get_mut(rule)
                    {
                        ok = remove_action_at(&mut rl.actions, &path);
                        if b.is_empty() {
                            e.behavior = None;
                        }
                    }
                });
                if ok {
                    if let Some(k) = key {
                        self.positions.borrow_mut().remove(&k);
                    }
                    self.selected_node.set(None);
                    self.rebuild();
                    self.editor.toast("Removed action");
                }
                ok
            }
        }
    }

    fn selected_node_key(&self) -> Option<String> {
        let id = self.selected_node.get_untracked()?;
        self.nodes
            .get_untracked()
            .iter()
            .find(|n| n.id == id)
            .map(|n| n.key.clone())
    }

    // ── editing the selected node ────────────────────────────────

    pub(crate) fn set_trigger_kind(&self, kind: &str) -> bool {
        let Some(KeyTarget::Trigger { el, rule }) = self.selected_key_target() else {
            return false;
        };
        let prev = self.selected_trigger();
        let Some(trigger) = trigger_from_kind(kind, None, prev.as_ref()) else {
            return false;
        };
        self.edit_rule(el, rule, |r| r.trigger = trigger)
    }

    pub(crate) fn set_trigger_param(&self, param: String) -> bool {
        let Some(KeyTarget::Trigger { el, rule }) = self.selected_key_target() else {
            return false;
        };
        let Some(prev) = self.selected_trigger() else {
            return false;
        };
        let trigger = match prev {
            Trigger::OnKey(_) => Trigger::OnKey(param),
            Trigger::OnPropChanged(_) => Trigger::OnPropChanged(param),
            Trigger::OnEvent(_) => Trigger::OnEvent(param),
            _ => return false,
        };
        self.edit_rule(el, rule, |r| r.trigger = trigger)
    }

    pub(crate) fn rename_state(&self, new: &str) -> bool {
        let Some(KeyTarget::State { name }) = self.selected_key_target() else {
            return false;
        };
        if name == new {
            return true;
        }
        if !behavior::is_valid_var_name(new) {
            self.editor
                .toast_warn("Variable name must be a JS identifier");
            return false;
        }
        let names = self.scope_var_list();
        if names.iter().any(|v| v.name == new) {
            self.editor
                .toast_warn("A variable with this name already exists in scope");
            return false;
        }
        let scope = self.scope_id.get_untracked();
        let prev = self.editor.selected_layer.get_untracked();
        self.editor.selected_layer.set(Some(scope));
        self.editor.rename_var(&name, new);
        self.editor.selected_layer.set(prev);
        if let Some((x, y)) = self.positions.borrow_mut().remove(&format!("s:{name}")) {
            self.positions
                .borrow_mut()
                .insert(format!("s:{new}"), (x, y));
        }
        self.rebuild();
        self.select_key(&format!("s:{new}"));
        true
    }

    pub(crate) fn set_state_type(&self, ty: VarType) -> bool {
        self.edit_selected_state(|v| {
            v.ty = ty;
            v.initial = ty.default_value();
        })
    }

    pub(crate) fn set_state_initial_bool(&self, val: bool) -> bool {
        self.edit_selected_state(|v| {
            if v.ty == VarType::Bool {
                v.initial = Value::Bool(val);
            }
        })
    }

    pub(crate) fn set_state_initial_number(&self, val: f32) -> bool {
        self.edit_selected_state(|v| {
            if v.ty == VarType::Number {
                v.initial = Value::Number(val);
            }
        })
    }

    pub(crate) fn set_state_initial_str(&self, val: String) -> bool {
        self.edit_selected_state(|v| {
            if v.ty == VarType::String {
                v.initial = Value::Str(val);
            }
        })
    }

    pub(crate) fn set_action_kind(&self, kind: &str) -> bool {
        let Some(KeyTarget::Action { el, .. }) = self.selected_key_target() else {
            return false;
        };
        let Some(root) = self.root_untracked() else {
            return false;
        };
        let Some((action, declare)) = default_action_for(&root, el, kind) else {
            return false;
        };
        self.replace_selected_action_with(action, declare)
    }

    pub(crate) fn set_action_var(&self, var: &str) -> bool {
        self.edit_selected_action(|a| match a {
            Action::ToggleVar(v) => *v = var.to_string(),
            Action::SetVar(v, _) => *v = var.to_string(),
            _ => {}
        })
    }

    pub(crate) fn set_action_target_name(&self, target: &str) -> bool {
        self.edit_selected_action(|a| match a {
            Action::Show { target: t, .. } | Action::Hide { target: t, .. } => {
                *t = target.to_string()
            }
            _ => {}
        })
    }

    pub(crate) fn set_action_event(&self, event: &str) -> bool {
        self.edit_selected_action(|a| {
            if let Action::Emit(e) = a {
                *e = event.to_string();
            }
        })
    }

    pub(crate) fn set_action_delay_ms(&self, ms: u32) -> bool {
        self.edit_selected_action(|a| {
            if let Action::Delay(v, _) = a {
                *v = ms;
            }
        })
    }

    /// Rewrites the right-hand side of a `Set` action. The value is parsed
    /// against the target variable's type.
    pub(crate) fn set_action_literal(&self, text: &str) -> bool {
        let Some(KeyTarget::Action { el, .. }) = self.selected_key_target() else {
            return false;
        };
        let Some(root) = self.root_untracked() else {
            return false;
        };
        let Some(Action::SetVar(var, _)) = self.selected_action() else {
            return false;
        };
        let ty = behavior::scope_vars(&root, el)
            .into_iter()
            .find(|v| v.name == var)
            .map(|v| v.ty);
        let value = match ty {
            Some(VarType::Bool) => match text.trim() {
                "true" => Value::Bool(true),
                "false" => Value::Bool(false),
                _ => {
                    self.editor.toast_warn("Expected true or false");
                    return false;
                }
            },
            Some(VarType::Number) => match text.trim().parse::<f32>() {
                Ok(n) => Value::Number(n),
                Err(_) => {
                    self.editor.toast_warn("Not a number");
                    return false;
                }
            },
            Some(VarType::String) => Value::Str(text.to_string()),
            None => {
                self.editor.toast_warn("Unknown variable type");
                return false;
            }
        };
        self.edit_selected_action(|a| {
            if let Action::SetVar(_, expr) = a {
                *expr = ValueExpr::Value(value.clone());
            }
        })
    }

    fn replace_selected_action(&self, action: Action) -> bool {
        self.replace_selected_action_with(action, None)
    }

    fn replace_selected_action_with(&self, action: Action, declare: Option<(u64, String)>) -> bool {
        let Some(KeyTarget::Action { el, rule, path }) = self.selected_key_target() else {
            return false;
        };
        if !self.rule_exists(el, rule) {
            return false;
        }
        self.editor.push_history();
        let mut ok = false;
        self.editor.root.update(|r| {
            let Some(rr) = r else { return };
            if let Some((owner, name)) = &declare
                && let Some(SvelteNode::Element(oe)) = tree_ops::find_mut(rr, *owner)
            {
                let b = oe.behavior.get_or_insert_with(Default::default);
                if !b.vars.iter().any(|v| v.name == *name) {
                    b.vars.push(Var::new(name, VarType::Bool));
                }
            }
            let Some(SvelteNode::Element(e)) = tree_ops::find_mut(rr, el) else {
                return;
            };
            if let Some(b) = &mut e.behavior
                && let Some(rl) = b.rules.get_mut(rule)
                && let Some(slot) = action_at_path_mut(rl, &path)
            {
                *slot = action.clone();
                ok = true;
            }
        });
        if ok {
            self.rebuild();
            self.editor.toast("Action updated");
        }
        ok
    }

    fn edit_selected_action(&self, f: impl FnOnce(&mut Action)) -> bool {
        let Some(KeyTarget::Action { el, rule, path }) = self.selected_key_target() else {
            return false;
        };
        if !self.rule_exists(el, rule) {
            return false;
        }
        self.editor.push_history();
        let mut ok = false;
        self.editor.root.update(|r| {
            let Some(rr) = r else { return };
            let Some(SvelteNode::Element(e)) = tree_ops::find_mut(rr, el) else {
                return;
            };
            if let Some(b) = &mut e.behavior
                && let Some(rl) = b.rules.get_mut(rule)
                && let Some(slot) = action_at_path_mut(rl, &path)
            {
                f(slot);
                ok = true;
            }
        });
        if ok {
            self.rebuild();
            self.editor.toast("Action updated");
        }
        ok
    }

    fn edit_selected_state(&self, f: impl FnOnce(&mut Var)) -> bool {
        let Some(KeyTarget::State { name }) = self.selected_key_target() else {
            return false;
        };
        let scope = self.scope_id.get_untracked();
        self.editor.push_history();
        let mut ok = false;
        self.editor.root.update(|r| {
            let Some(rr) = r else { return };
            let Some(SvelteNode::Element(e)) = tree_ops::find_mut(rr, scope) else {
                return;
            };
            if let Some(b) = &mut e.behavior
                && let Some(v) = b.vars.iter_mut().find(|v| v.name == name)
            {
                f(v);
                ok = true;
            }
        });
        if ok {
            self.rebuild();
        }
        ok
    }

    fn edit_rule(&self, el: u64, rule: usize, f: impl FnOnce(&mut Rule)) -> bool {
        if !self.rule_exists(el, rule) {
            return false;
        }
        self.editor.push_history();
        let mut ok = false;
        self.editor.root.update(|r| {
            let Some(rr) = r else { return };
            let Some(SvelteNode::Element(e)) = tree_ops::find_mut(rr, el) else {
                return;
            };
            if let Some(b) = &mut e.behavior
                && let Some(rl) = b.rules.get_mut(rule)
            {
                f(rl);
                ok = true;
            }
        });
        if ok {
            self.rebuild();
        }
        ok
    }

    fn rule_exists(&self, el: u64, rule: usize) -> bool {
        self.root_untracked()
            .as_ref()
            .and_then(|r| find_element(r, el))
            .and_then(|e| e.behavior.as_ref())
            .is_some_and(|b| rule < b.rules.len())
    }

    fn select_key(&self, key: &str) {
        if let Some(id) = self
            .nodes
            .get_untracked()
            .iter()
            .find(|n| n.key == key)
            .map(|n| n.id)
        {
            self.select_node(id);
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

fn find_in_subtree(root: &SvelteNode, scope: u64, id: u64) -> Option<&SvelteNode> {
    tree_ops::find(tree_ops::find(root, scope)?, id)
}

fn find_element(root: &SvelteNode, id: u64) -> Option<&Element> {
    match tree_ops::find(root, id)? {
        SvelteNode::Element(e) => Some(e),
        _ => None,
    }
}

fn editable_element_in_scope(root: &SvelteNode, scope: u64, id: u64) -> bool {
    matches!(
        find_in_subtree(root, scope, id),
        Some(SvelteNode::Element(e)) if !e.instance
    )
}

fn parse_key(key: &str) -> Option<KeyTarget> {
    let parts: Vec<&str> = key.split(':').collect();
    match parts.first().copied()? {
        "t" if parts.len() == 3 => Some(KeyTarget::Trigger {
            el: parts[1].parse().ok()?,
            rule: parts[2].parse().ok()?,
        }),
        "a" if parts.len() >= 4 => Some(KeyTarget::Action {
            el: parts[1].parse().ok()?,
            rule: parts[2].parse().ok()?,
            path: parts[3..]
                .iter()
                .map(|p| p.parse().ok())
                .collect::<Option<Vec<_>>>()?,
        }),
        "s" if parts.len() >= 2 => Some(KeyTarget::State {
            name: parts[1..].join(":"),
        }),
        _ => None,
    }
}

fn first_rule_index(root: &SvelteNode, el: u64) -> Option<usize> {
    find_element(root, el)?
        .behavior
        .as_ref()?
        .rules
        .first()
        .map(|_| 0)
}

fn next_var_name(vars: &[Var], prefix: &str) -> String {
    (1..)
        .map(|i| format!("{prefix}{i}"))
        .find(|n| !vars.iter().any(|v| &v.name == n))
        .unwrap()
}

fn trigger_from_kind(kind: &str, param: Option<&str>, prev: Option<&Trigger>) -> Option<Trigger> {
    let old = param
        .map(str::to_string)
        .or_else(|| prev.and_then(|t| t.param().map(str::to_string)));
    Some(match kind {
        "click" => Trigger::OnClick,
        "hover_in" => Trigger::OnHoverIn,
        "hover_out" => Trigger::OnHoverOut,
        "mount" => Trigger::OnMount,
        "key" => Trigger::OnKey(old.unwrap_or_else(|| "Enter".into())),
        "prop" => Trigger::OnPropChanged(old.unwrap_or_else(|| "title".into())),
        "event" => Trigger::OnEvent(old.unwrap_or_else(|| "selected".into())),
        _ => return None,
    })
}

/// A sensible first action of `kind` for the scope containing `el`, plus an
/// optional variable declaration the action needs to reference.
fn default_action_for(
    root: &SvelteNode,
    el: u64,
    kind: &str,
) -> Option<(Action, Option<(u64, String)>)> {
    let vars = behavior::scope_vars(root, el);
    let owner = behavior::var_owner_id(root, el);
    let mut declare = None;
    let action = match kind {
        "toggle" => {
            let name = match vars.iter().find(|v| v.ty == VarType::Bool) {
                Some(v) => v.name.clone(),
                None => {
                    let name = next_var_name(&vars, "flag");
                    declare = Some((owner, name.clone()));
                    name
                }
            };
            Action::ToggleVar(name)
        }
        "set" => {
            let var = match vars.first() {
                Some(v) => v.clone(),
                None => {
                    let name = next_var_name(&vars, "value");
                    declare = Some((owner, name.clone()));
                    Var::new(&name, VarType::Bool)
                }
            };
            Action::SetVar(var.name, ValueExpr::Value(var.initial))
        }
        "show" | "hide" => {
            let scope = behavior::scope_root(root, el);
            let targets = behavior::descendant_targets(scope);
            let target = targets
                .iter()
                .find(|(_, _, bound)| *bound)
                .or_else(|| targets.iter().find(|(id, _, _)| *id != scope.id()))
                .or_else(|| targets.first())
                .map(|(_, name, _)| name.clone())
                .unwrap_or_else(|| "Panel".into());
            if kind == "show" {
                Action::Show {
                    target,
                    transition: None,
                }
            } else {
                Action::Hide {
                    target,
                    transition: None,
                }
            }
        }
        "emit" => {
            let event = match behavior::scope_root(root, el) {
                SvelteNode::Element(e) => {
                    e.interface.as_ref().and_then(|i| i.events.first().cloned())
                }
                _ => None,
            }
            .unwrap_or_else(|| "confirm".into());
            Action::Emit(event)
        }
        "delay" => Action::Delay(200, Vec::new()),
        _ => return None,
    };
    Some((action, declare))
}

fn action_at_path<'a>(rule: &'a Rule, path: &[usize]) -> Option<&'a Action> {
    fn walk<'a>(list: &'a [Action], path: &[usize]) -> Option<&'a Action> {
        let (&i, rest) = path.split_first()?;
        let action = list.get(i)?;
        if rest.is_empty() {
            Some(action)
        } else if let Action::Delay(_, nested) = action {
            walk(nested, rest)
        } else {
            None
        }
    }
    walk(&rule.actions, path)
}

fn action_at_path_mut<'a>(rule: &'a mut Rule, path: &[usize]) -> Option<&'a mut Action> {
    fn walk<'a>(list: &'a mut [Action], path: &[usize]) -> Option<&'a mut Action> {
        let (&i, rest) = path.split_first()?;
        let action = list.get_mut(i)?;
        if rest.is_empty() {
            Some(action)
        } else if let Action::Delay(_, nested) = action {
            walk(nested, rest)
        } else {
            None
        }
    }
    walk(&mut rule.actions, path)
}

fn remove_action_at(actions: &mut Vec<Action>, path: &[usize]) -> bool {
    let Some((&i, rest)) = path.split_first() else {
        return false;
    };
    if i >= actions.len() {
        return false;
    }
    if rest.is_empty() {
        actions.remove(i);
        true
    } else if let Action::Delay(_, nested) = &mut actions[i] {
        remove_action_at(nested, rest)
    } else {
        false
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
