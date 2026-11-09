//! Builds the behavior graph of one scope (Phase 3, design doc §4.4).
//!
//! `graph_for` turns the declarative `Behavior` model into nodes and edges:
//!
//! * every rule becomes a **Trigger** node (`On Click`, …),
//! * every action step becomes an **Action** node, chained in execution
//!   order — `Delay` expands into its own node with the nested steps
//!   hanging off its `after` output,
//! * every state variable becomes a **State** node; data edges connect
//!   writes (`Toggle`/`Set`/`Show`/`Hide` through the target's binding)
//!   and reads (`Set x = y`).
//!
//! The graph "only sees the children of its own component": nested masters
//! and instances are black boxes, exactly like in codegen. After building,
//! the dependency graph is checked for cycles — the only loops the closed
//! dictionary can express go through state variables, and a loop means two
//! rules keep re-assigning each other's variables.

use super::model::{GraphConnection, GraphNode, NODE_W, NodeKind};
#[cfg(test)]
use crate::model::behavior::Trigger;
use crate::model::behavior::{self, Action, Severity, ValueExpr};
use crate::model::svelte_ast::SvelteNode;
use crate::model::tree_ops;

#[derive(Clone, PartialEq)]
pub(crate) struct GraphIssue {
    pub severity: Severity,
    pub message: String,
}

pub(crate) struct BehaviorGraph {
    pub scope_id: u64,
    pub scope_name: String,
    pub nodes: Vec<GraphNode>,
    pub connections: Vec<GraphConnection>,
    pub issues: Vec<GraphIssue>,
}

impl BehaviorGraph {
    /// (errors, warnings) badge counts for the toolbar.
    pub(crate) fn counts(&self) -> (usize, usize) {
        let e = self
            .issues
            .iter()
            .filter(|i| i.severity == Severity::Error)
            .count();
        let w = self
            .issues
            .iter()
            .filter(|i| i.severity == Severity::Warn)
            .count();
        (e, w)
    }
}

/// Builds the graph for the scope that contains `focus_id` (the nearest
/// master around the selection, or the whole document). Missing ids fall
/// back to the document scope.
pub(crate) fn graph_for(root: &SvelteNode, focus_id: u64) -> BehaviorGraph {
    let scope = behavior::scope_root(root, focus_id);
    let scope_id = scope.id();
    let scope_name = scope.name().to_string();

    let mut b = Builder {
        root,
        scope,
        nodes: Vec::new(),
        connections: Vec::new(),
        issues: Vec::new(),
        next_id: 1,
    };

    // 1) State nodes: declared variables of the scope owner first, in
    //    declaration order; references to anything else create dangling
    //    nodes (so a typo is visible as a wire into nowhere).
    if let Some(SvelteNode::Element(se)) = tree_ops::find(root, scope_id) {
        if let Some(be) = &se.behavior {
            for v in &be.vars {
                b.state_node(&v.name, Some(v));
            }
        }
    }

    // 2) Trigger/action chains per element, document order.
    for el in b.scope_elements() {
        let Some(behavior) = &el.behavior else {
            continue;
        };
        for (ri, rule) in behavior.rules.iter().enumerate() {
            let t = b.push_node(
                format!("t:{}:{}", el.id, ri),
                rule.trigger.label(),
                Some(el.name.clone()),
                NodeKind::Trigger,
                vec![],
                vec!["".to_string()],
                Some(el.id),
                0,
            );
            b.emit_chain(&rule.actions, el, ri, 0, t, 0, "");
        }
    }

    // 3) Layout, cycles, aggregated validation.
    b.layout();
    b.check_cycles();
    b.collect_issues();
    b.finish(scope_id, scope_name)
}

// ─────────────────────────────────────────────────────────── builder

struct Builder<'a> {
    root: &'a SvelteNode,
    scope: &'a SvelteNode,
    nodes: Vec<GraphNode>,
    connections: Vec<GraphConnection>,
    issues: Vec<GraphIssue>,
    next_id: u64,
}

/// One step in an action chain: its node plus the data ports it declared.
struct Step {
    node: usize,
    reads: Vec<String>,
    writes: Vec<String>,
}

impl<'a> Builder<'a> {
    // ── node helpers ──

    fn push_node(
        &mut self,
        key: String,
        title: String,
        subtitle: Option<String>,
        kind: NodeKind,
        inputs: Vec<String>,
        outputs: Vec<String>,
        element: Option<u64>,
        depth: usize,
    ) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        self.nodes.push(GraphNode {
            id,
            key,
            title,
            subtitle,
            kind,
            x: 0.0,
            y: 0.0,
            inputs,
            outputs,
            element,
            depth,
            in_cycle: false,
        });
        self.nodes.len() - 1
    }

    /// Index of the State node for `name`; creates a dangling one on first
    /// reference when the variable was not declared in this scope.
    fn state_idx(&mut self, name: &str) -> usize {
        if let Some(i) = self.nodes.iter().position(|n| n.key == format!("s:{name}")) {
            return i;
        }
        self.state_node(name, None)
    }

    fn state_node(&mut self, name: &str, declared: Option<&crate::model::behavior::Var>) -> usize {
        let subtitle = match declared {
            Some(v) => Some(format!("{} = {}", v.ty.as_str(), v.initial.label())),
            None => Some("(not declared)".to_string()),
        };
        self.push_node(
            format!("s:{name}"),
            name.to_string(),
            subtitle,
            NodeKind::State,
            vec!["set".to_string()],
            vec!["read".to_string()],
            Some(self.scope.id()),
            0,
        )
    }

    // ── chains ──

    /// Emits `actions` as a chain starting at depth `depth`; `exec_from`
    /// is the (node, port) the first step hangs on. `key_path` is the
    /// nested-address prefix ("" at the top level, ":0" inside a Delay).
    fn emit_chain(
        &mut self,
        actions: &[Action],
        el: &crate::model::svelte_ast::Element,
        ri: usize,
        depth: usize,
        exec_from: usize,
        exec_port: usize,
        key_path: &str,
    ) {
        let mut from = exec_from;
        let mut from_port = exec_port;
        for (ai, action) in actions.iter().enumerate() {
            let step = self.action_node(action, el, ri, ai, depth + ai, key_path);
            self.connections.push(GraphConnection {
                from_node: self.nodes[from].id,
                from_port,
                to_node: self.nodes[step.node].id,
                to_port: 0,
            });
            self.emit_data_edges(&step);
            if let Action::Delay(_, nested) = action {
                // The nested steps run when the timer fires.
                self.emit_chain(
                    nested,
                    el,
                    ri,
                    depth + ai + 1,
                    step.node,
                    0,
                    &format!("{key_path}:{ai}"),
                );
            }
            from = step.node;
            from_port = 0;
        }
    }

    /// Creates the Action node for one step and returns its data ports.
    fn action_node(
        &mut self,
        action: &Action,
        el: &crate::model::svelte_ast::Element,
        ri: usize,
        ai: usize,
        depth: usize,
        key_path: &str,
    ) -> Step {
        let title = action.label();
        let key = format!("a:{}:{}:{}{key_path}", el.id, ri, ai);
        // Port 0 is execution ("then"); further ports are data.
        let mut reads: Vec<String> = Vec::new();
        let mut writes: Vec<String> = Vec::new();
        match action {
            Action::ToggleVar(v) => writes.push(v.clone()),
            Action::SetVar(v, expr) => {
                writes.push(v.clone());
                if let ValueExpr::Var(src) = expr {
                    reads.push(src.clone());
                }
            }
            Action::Show { target, .. } | Action::Hide { target, .. } => {
                // Show/Hide assign the target's binding variable — the same
                // bridge codegen uses. No binding → no wire (validation
                // already warns in the Interact tab).
                if let Some(v) = behavior::binding_var_of(self.scope, target) {
                    writes.push(v);
                }
            }
            Action::Emit(_) | Action::Delay(..) => {}
        }
        let mut inputs = vec![String::new()];
        inputs.extend(reads.iter().map(|r| format!("↷ {r}")));
        let mut outputs = vec![String::new()];
        outputs.extend(writes.iter().map(|w| format!("⇒ {w}")));
        let node = self.push_node(
            key,
            title,
            Some(el.name.clone()),
            NodeKind::Action,
            inputs,
            outputs,
            Some(el.id),
            depth,
        );
        Step {
            node,
            reads,
            writes,
        }
    }

    fn emit_data_edges(&mut self, step: &Step) {
        let node_id = self.nodes[step.node].id;
        for (pi, r) in step.reads.iter().enumerate() {
            let s = self.state_idx(r);
            self.connections.push(GraphConnection {
                from_node: self.nodes[s].id,
                from_port: 0,
                to_node: node_id,
                to_port: 1 + pi,
            });
        }
        for (pi, w) in step.writes.iter().enumerate() {
            let s = self.state_idx(w);
            self.connections.push(GraphConnection {
                from_node: node_id,
                from_port: 1 + pi,
                to_node: self.nodes[s].id,
                to_port: 0,
            });
        }
    }

    // ── scope walk ──

    /// Elements of the scope in document order; nested masters and
    /// instances are black boxes (§4.4) — their subtrees are skipped.
    fn scope_elements(&self) -> Vec<&'a crate::model::svelte_ast::Element> {
        let mut out = Vec::new();
        self.walk(self.scope, true, &mut out);
        out
    }

    fn walk(
        &self,
        node: &'a SvelteNode,
        is_root: bool,
        out: &mut Vec<&'a crate::model::svelte_ast::Element>,
    ) {
        if let SvelteNode::Element(e) = node {
            // Component/instance boundaries stop the descent (their interior
            // has its own graph), but the boundary element itself still
            // contributes: an `On Event` rule on an instance is the parent's
            // handler for the child's event (design doc §4.6, Phase 4).
            let boundary = !is_root && (e.component || e.instance);
            if e.behavior.is_some() || e.binding.is_some() {
                out.push(e);
            }
            if boundary {
                return;
            }
        }
        for c in node.children() {
            self.walk(c, false, out);
        }
    }

    // ── layout ──

    /// Columns: states · triggers · action chain depth. Simple vertical
    /// stacking per column; dragged positions are re-applied by the
    /// window state afterwards (keyed by `GraphNode::key`).
    fn layout(&mut self) {
        const X0: f32 = 80.0;
        // GAP_X keeps the whole Trigger → Action chain inside the canvas
        // between the browser and properties panels (window 1100 − 488 px
        // of panels → ~612 px of canvas ≈ 3 columns + margins).
        const GAP_X: f32 = 60.0;
        const STEP_X: f32 = NODE_W + 60.0;
        const MAX_COL: usize = 15;
        let col_x = |col: usize| match col {
            0 => X0,
            1 => X0 + NODE_W + GAP_X,
            c => X0 + 2.0 * (NODE_W + GAP_X) + (c - 2) as f32 * STEP_X,
        };
        let mut cursors = [80.0f32; MAX_COL + 1];
        for i in 0..self.nodes.len() {
            let col = match self.nodes[i].kind {
                NodeKind::State => 0,
                NodeKind::Trigger => 1,
                NodeKind::Action => 2 + self.nodes[i].depth.min(MAX_COL - 2),
            };
            let h = self.nodes[i].height();
            let y = cursors[col];
            cursors[col] = y + h + 26.0;
            self.nodes[i].x = col_x(col);
            self.nodes[i].y = y;
        }
    }

    // ── DAG validation ──

    fn check_cycles(&mut self) {
        // Data edges only (state ⇄ action): execution chains are built
        // linearly per rule and cannot loop back. A data cycle means rules
        // keep re-assigning each other's variables — the spreadsheet-style
        // "circular reference". Same-action read+write (`Set x = x`) stays
        // exempt (length-2 cycles are dropped by `find_cycles`).
        let data: Vec<GraphConnection> = self
            .connections
            .iter()
            .filter(|c| c.is_data(&self.nodes))
            .cloned()
            .collect();
        let cycles = find_cycles(&self.nodes, &data);
        let mut in_cycle: Vec<u64> = Vec::new();
        for cycle in &cycles {
            for id in cycle {
                if !in_cycle.contains(id) {
                    in_cycle.push(*id);
                }
            }
            let names: Vec<String> = cycle
                .iter()
                .filter_map(|id| self.nodes.iter().find(|n| n.id == *id))
                .map(|n| node_label(n))
                .collect();
            let first = names.first().cloned().unwrap_or_default();
            self.issues.push(GraphIssue {
                severity: Severity::Error,
                message: format!("Cycle: {} → {first}", names.join(" → ")),
            });
        }
        for n in &mut self.nodes {
            if in_cycle.contains(&n.id) {
                n.in_cycle = true;
            }
        }
    }

    fn collect_issues(&mut self) {
        // Model validation of every element in scope, prefixed with the
        // element name so the toolbar badges stay actionable.
        for el in self.scope_elements() {
            for issue in behavior::validate(self.root, el.id) {
                self.issues.push(GraphIssue {
                    severity: issue.severity,
                    message: format!("[{}] {}", el.name, issue.message),
                });
            }
        }
    }

    fn finish(self, scope_id: u64, scope_name: String) -> BehaviorGraph {
        BehaviorGraph {
            scope_id,
            scope_name,
            nodes: self.nodes,
            connections: self.connections,
            issues: self.issues,
        }
    }
}

/// `On Click · Btn` — the label used in cycle reports.
fn node_label(n: &GraphNode) -> String {
    match &n.subtitle {
        Some(s) if n.kind == NodeKind::Trigger || n.kind == NodeKind::Action => {
            format!("{} ({s})", n.title)
        }
        _ => n.title.clone(),
    }
}

/// All elementary cycles of length ≥ 3 in the **data** subgraph. Length-2
/// cycles are always the benign `Set x = x` pattern (one action reading and
/// writing the same state) and are deliberately ignored; anything longer
/// means variables keep being re-assigned around a loop.
fn find_cycles(nodes: &[GraphNode], conns: &[GraphConnection]) -> Vec<Vec<u64>> {
    let succ = |id: u64| -> Vec<u64> {
        conns
            .iter()
            .filter(|c| c.from_node == id)
            .map(|c| c.to_node)
            .collect()
    };
    let mut seen: std::collections::HashSet<Vec<u64>> = std::collections::HashSet::new();
    let mut out = Vec::new();
    for start in nodes {
        let start = start.id;
        let mut stack: Vec<(u64, Vec<u64>)> =
            succ(start).into_iter().map(|s| (s, vec![start])).collect();
        while let Some((cur, path)) = stack.pop() {
            if cur == start {
                let mut cycle = path;
                let rot = canonical_rotation(&cycle);
                cycle.rotate_left(rot);
                if cycle.len() >= 3 && seen.insert(cycle.clone()) {
                    out.push(cycle);
                }
                continue;
            }
            if path.contains(&cur) || path.len() >= nodes.len() {
                continue;
            }
            for s in succ(cur) {
                let mut p = path.clone();
                p.push(cur);
                stack.push((s, p));
            }
        }
    }
    out
}

/// Rotates the cycle so it starts with its smallest id — the canonical
/// form used for deduplication.
fn canonical_rotation(cycle: &[u64]) -> usize {
    cycle
        .iter()
        .enumerate()
        .min_by_key(|(i, v)| (**v, *i))
        .map(|(i, _)| i)
        .unwrap_or(0)
}

// ──────────────────────────────────────────────────────────── tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::behavior::{Behavior, Binding, Rule, Transition, Var, VarType};
    use crate::model::svelte_ast::{new_button, new_frame};
    /// Page(100){ master Card(200){ Btn(300) }, Panel(400) }.
    fn doc() -> SvelteNode {
        let mut btn = new_button(300, 0.0, 0.0);
        if let SvelteNode::Element(e) = &mut btn {
            e.name = "Btn".into();
        }
        let mut panel = new_frame(400, 0.0, 0.0, 120.0, 60.0);
        if let SvelteNode::Element(e) = &mut panel {
            e.name = "Panel".into();
        }
        let mut card = new_frame(200, 0.0, 0.0, 200.0, 80.0);
        if let SvelteNode::Element(e) = &mut card {
            e.component = true;
            e.name = "Card".into();
            e.children = vec![btn];
        }
        let mut root = new_frame(100, 0.0, 0.0, 600.0, 400.0);
        if let SvelteNode::Element(e) = &mut root {
            e.name = "Page".into();
            e.children = vec![card, panel];
        }
        root
    }

    fn el_mut(root: &mut SvelteNode, id: u64) -> &mut crate::model::svelte_ast::Element {
        match tree_ops::find_mut(root, id) {
            Some(SvelteNode::Element(e)) => e,
            _ => panic!("element {id} missing"),
        }
    }

    fn node<'g>(g: &'g BehaviorGraph, key: &str) -> &'g GraphNode {
        g.nodes.iter().find(|n| n.key == key).unwrap_or_else(|| {
            panic!(
                "node {key} missing: {:?}",
                g.nodes.iter().map(|n| &n.key).collect::<Vec<_>>()
            )
        })
    }

    fn wired(g: &BehaviorGraph, from: &str, to: &str) -> bool {
        let f = node(g, from).id;
        let t = node(g, to).id;
        g.connections
            .iter()
            .any(|c| c.from_node == f && c.to_node == t)
    }

    #[test]
    fn graph_maps_rules_to_trigger_state_action() {
        let mut root = doc();
        el_mut(&mut root, 200).behavior = Some(Behavior {
            vars: vec![
                Var::new("open", VarType::Bool),
                Var::new("count", VarType::Number),
            ],
            rules: vec![],
        });
        el_mut(&mut root, 300).behavior = Some(Behavior {
            vars: vec![],
            rules: vec![
                Rule {
                    trigger: Trigger::OnClick,
                    actions: vec![
                        Action::ToggleVar("open".into()),
                        Action::Emit("confirm".into()),
                    ],
                },
                Rule {
                    trigger: Trigger::OnKey("Enter".into()),
                    actions: vec![Action::SetVar(
                        "count".into(),
                        ValueExpr::Value(crate::model::behavior::Value::Number(3.0)),
                    )],
                },
            ],
        });
        let g = graph_for(&root, 300);

        assert_eq!(g.scope_name, "Card");
        let mut kinds: Vec<(String, NodeKind)> =
            g.nodes.iter().map(|n| (n.key.clone(), n.kind)).collect();
        kinds.sort();
        assert_eq!(
            kinds,
            vec![
                ("a:300:0:0".into(), NodeKind::Action),
                ("a:300:0:1".into(), NodeKind::Action),
                ("a:300:1:0".into(), NodeKind::Action),
                ("s:count".into(), NodeKind::State),
                ("s:open".into(), NodeKind::State),
                ("t:300:0".into(), NodeKind::Trigger),
                ("t:300:1".into(), NodeKind::Trigger),
            ]
        );
        // Execution chain + data edges.
        assert!(wired(&g, "t:300:0", "a:300:0:0"));
        assert!(wired(&g, "a:300:0:0", "a:300:0:1"));
        assert!(wired(&g, "t:300:1", "a:300:1:0"));
        assert!(wired(&g, "a:300:0:0", "s:open"), "Toggle writes open");
        assert!(wired(&g, "a:300:1:0", "s:count"), "Set writes count");
        // Emit has no data edge.
        assert!(!wired(&g, "a:300:0:1", "s:open"));
        assert!(
            g.issues.is_empty(),
            "{:?}",
            g.issues.iter().map(|i| &i.message).collect::<Vec<_>>()
        );
        assert_eq!(g.counts(), (0, 0));
    }

    #[test]
    fn set_can_read_another_var() {
        let mut root = doc();
        el_mut(&mut root, 200).behavior = Some(Behavior {
            vars: vec![
                Var::new("a", VarType::Number),
                Var::new("b", VarType::Number),
            ],
            rules: vec![],
        });
        el_mut(&mut root, 300).behavior = Some(Behavior {
            vars: vec![],
            rules: vec![Rule {
                trigger: Trigger::OnMount,
                actions: vec![Action::SetVar("a".into(), ValueExpr::Var("b".into()))],
            }],
        });
        let g = graph_for(&root, 200);
        assert!(wired(&g, "s:b", "a:300:0:0"), "read edge b → action");
        assert!(wired(&g, "a:300:0:0", "s:a"), "write edge action → a");
    }

    #[test]
    fn show_targets_the_binding_var() {
        let mut root = doc();
        el_mut(&mut root, 100).behavior = Some(Behavior {
            vars: vec![Var::new("open", VarType::Bool)],
            rules: vec![],
        });
        el_mut(&mut root, 400).binding = Some(Binding {
            var: "open".into(),
            transition: Some(Transition::Slide { ms: 200 }),
        });
        el_mut(&mut root, 400).behavior = Some(Behavior {
            vars: vec![],
            rules: vec![Rule {
                trigger: Trigger::OnClick,
                actions: vec![Action::Show {
                    target: "Panel".into(),
                    transition: None,
                }],
            }],
        });
        // Panel carries the rule itself; scope = document root.
        let g = graph_for(&root, 400);
        assert_eq!(g.scope_id, 100);
        assert!(
            wired(&g, "a:400:0:0", "s:open"),
            "Show writes the binding var"
        );
    }

    #[test]
    fn show_without_binding_creates_no_wire() {
        let mut root = doc();
        el_mut(&mut root, 400).behavior = Some(Behavior {
            vars: vec![],
            rules: vec![Rule {
                trigger: Trigger::OnClick,
                actions: vec![Action::Show {
                    target: "Panel".into(),
                    transition: None,
                }],
            }],
        });
        let g = graph_for(&root, 400);
        // No variable is involved at all — no State nodes, no data wires.
        assert!(
            !g.nodes.iter().any(|n| n.kind == NodeKind::State),
            "no state nodes: {:?}",
            g.nodes.iter().map(|n| &n.key).collect::<Vec<_>>()
        );
        assert!(!g.connections.iter().any(|c| c.is_data(&g.nodes)));
        // Validation of the element is aggregated into the graph issues.
        assert!(
            g.issues
                .iter()
                .any(|i| i.message.contains("Panel") && i.message.contains("Binding"))
        );
    }

    #[test]
    fn delay_expands_into_nested_chain() {
        let mut root = doc();
        el_mut(&mut root, 100).behavior = Some(Behavior {
            vars: vec![Var::new("open", VarType::Bool)],
            rules: vec![],
        });
        el_mut(&mut root, 400).binding = Some(Binding {
            var: "open".into(),
            transition: None,
        });
        el_mut(&mut root, 400).behavior = Some(Behavior {
            vars: vec![],
            rules: vec![Rule {
                trigger: Trigger::OnClick,
                actions: vec![
                    Action::Delay(200, vec![Action::ToggleVar("open".into())]),
                    Action::Hide {
                        target: "Panel".into(),
                        transition: None,
                    },
                ],
            }],
        });
        let g = graph_for(&root, 400);
        let delay = node(&g, "a:400:0:0");
        assert_eq!(delay.title, "Delay 200 ms");
        let nested = node(&g, "a:400:0:0:0");
        assert_eq!(nested.title, "Toggle open");
        assert!(wired(&g, "a:400:0:0", "a:400:0:0:0"), "delay fires nested");
        assert!(
            wired(&g, "a:400:0:0", "a:400:0:1"),
            "chain continues after delay"
        );
        assert!(wired(&g, "a:400:0:0:0", "s:open"));
        assert!(wired(&g, "a:400:0:1", "s:open"));
    }

    #[test]
    fn cycle_detection_flags_cross_assignments() {
        let mut root = doc();
        el_mut(&mut root, 200).behavior = Some(Behavior {
            vars: vec![
                Var::new("a", VarType::Number),
                Var::new("b", VarType::Number),
            ],
            rules: vec![],
        });
        el_mut(&mut root, 300).behavior = Some(Behavior {
            vars: vec![],
            rules: vec![Rule {
                trigger: Trigger::OnMount,
                actions: vec![
                    Action::SetVar("a".into(), ValueExpr::Var("b".into())),
                    Action::SetVar("b".into(), ValueExpr::Var("a".into())),
                ],
            }],
        });
        let g = graph_for(&root, 200);
        assert_eq!(
            g.counts().0,
            1,
            "one cycle error: {:?}",
            g.issues.iter().map(|i| &i.message).collect::<Vec<_>>()
        );
        assert!(g.issues[0].message.starts_with("Cycle: "));
        for key in ["a:300:0:0", "a:300:0:1", "s:a", "s:b"] {
            assert!(node(&g, key).in_cycle, "{key} must be in cycle");
        }
        assert!(
            !node(&g, "t:300:0").in_cycle,
            "trigger is not part of the loop"
        );
    }

    #[test]
    fn exec_chains_alone_do_not_create_cycles() {
        // Reading b and later writing b within one rule is a plain swap —
        // execution edges are linear, so no cycle is reported.
        let mut root = doc();
        el_mut(&mut root, 200).behavior = Some(Behavior {
            vars: vec![
                Var::new("a", VarType::Number),
                Var::new("b", VarType::Number),
            ],
            rules: vec![],
        });
        el_mut(&mut root, 300).behavior = Some(Behavior {
            vars: vec![],
            rules: vec![Rule {
                trigger: Trigger::OnMount,
                actions: vec![
                    Action::SetVar("a".into(), ValueExpr::Var("b".into())),
                    Action::ToggleVar("b".into()),
                ],
            }],
        });
        let g = graph_for(&root, 200);
        assert!(
            !g.issues.iter().any(|i| i.message.starts_with("Cycle:")),
            "{:?}",
            g.issues.iter().map(|i| &i.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn self_assignment_is_not_a_cycle() {
        let mut root = doc();
        el_mut(&mut root, 200).behavior = Some(Behavior {
            vars: vec![Var::new("a", VarType::Number)],
            rules: vec![],
        });
        el_mut(&mut root, 300).behavior = Some(Behavior {
            vars: vec![],
            rules: vec![Rule {
                trigger: Trigger::OnMount,
                actions: vec![Action::SetVar("a".into(), ValueExpr::Var("a".into()))],
            }],
        });
        let g = graph_for(&root, 200);
        assert!(
            !g.issues.iter().any(|i| i.message.starts_with("Cycle:")),
            "Set a = a is a benign no-op"
        );
        assert!(!node(&g, "a:300:0:0").in_cycle);
    }

    #[test]
    fn nested_masters_and_instances_are_black_boxes() {
        let mut root = doc();
        // Master Inner(500){ Btn(600) } inside Card; a rule lives on Btn(600).
        let mut inner_btn = new_button(600, 0.0, 0.0);
        if let SvelteNode::Element(e) = &mut inner_btn {
            e.name = "InnerBtn".into();
            e.behavior = Some(Behavior {
                vars: vec![],
                rules: vec![Rule {
                    trigger: Trigger::OnClick,
                    actions: vec![],
                }],
            });
        }
        let mut inner = new_frame(500, 0.0, 0.0, 100.0, 40.0);
        if let SvelteNode::Element(e) = &mut inner {
            e.component = true;
            e.name = "Inner".into();
            e.children = vec![inner_btn];
        }
        el_mut(&mut root, 200).children.push(inner);
        // An instance on the page: rules of a copy stay out as well.
        let mut inst = new_frame(700, 0.0, 0.0, 100.0, 40.0);
        if let SvelteNode::Element(e) = &mut inst {
            e.instance = true;
            e.name = "Inner".into();
            e.behavior = Some(Behavior {
                vars: vec![],
                rules: vec![Rule {
                    trigger: Trigger::OnClick,
                    actions: vec![],
                }],
            });
        }
        el_mut(&mut root, 100).children.push(inst);

        let g = graph_for(&root, 200);
        assert!(
            !g.nodes.iter().any(|n| n.key.starts_with("t:600")),
            "Inner's rules belong to the Inner scope"
        );
        assert!(
            !g.nodes.iter().any(|n| n.key.starts_with("t:700")),
            "instances are black boxes"
        );

        // But the Inner scope itself shows its own rule.
        let gi = graph_for(&root, 600);
        assert_eq!(gi.scope_name, "Inner");
        assert!(gi.nodes.iter().any(|n| n.key == "t:600:0"));
    }

    #[test]
    fn dangling_reference_is_visible_in_the_graph() {
        let mut root = doc();
        el_mut(&mut root, 300).behavior = Some(Behavior {
            vars: vec![],
            rules: vec![Rule {
                trigger: Trigger::OnClick,
                actions: vec![Action::ToggleVar("oops".into())],
            }],
        });
        let g = graph_for(&root, 300);
        let s = node(&g, "s:oops");
        assert_eq!(s.subtitle.as_deref(), Some("(not declared)"));
        assert!(wired(&g, "a:300:0:0", "s:oops"));
        // …and the model validation reports it.
        assert!(
            g.issues
                .iter()
                .any(|i| i.severity == Severity::Error && i.message.contains("oops"))
        );
    }
}
