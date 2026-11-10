//! Node graph window (Phase 3, design doc §4.4): a live view of one
//! behavior scope — trigger/action chains, state variables with data wires,
//! and the DAG check that flags dependency cycles. The graph is generated
//! from the document; rules stay editable in the Interact tab.

pub(crate) mod build;
pub(crate) mod canvas;
pub(crate) mod graph_state;
pub(crate) mod model;
pub(crate) mod panels;

use crate::model::behavior::Severity;
use crate::state::EditorState;
use crate::theme::colors;
use canvas::GraphCanvas;
use graph_state::NodeGraphState;
use panels::{browser_panel, properties_panel};
use sequoia_ui::prelude::*;

/// Main view of the node graph window
pub(crate) struct NodeGraphView {
    pub editor: EditorState,
    pub graph: NodeGraphState,
}

impl View for NodeGraphView {
    fn body(&self) -> impl View + 'static {
        colors::set_active(self.editor.palette());

        let graph = self.graph.clone();
        // Tracked reads: the toolbar refreshes on every rebuild.
        let scope_name = self.graph.scope_name.get();
        let scopes = self.graph.scope_options();
        let active_scope = self.graph.scope_id.get_untracked();
        let nodes_n = self.graph.nodes.get().len();
        let wires_n = self.graph.connections.get().len();
        let issues = self.graph.issues.get();
        let errors = issues
            .iter()
            .filter(|i| i.severity == Severity::Error)
            .count();
        let warns = issues
            .iter()
            .filter(|i| i.severity == Severity::Warn)
            .count();

        let mut toolbar = Row::new()
            .fill_width()
            .height(36.0)
            .padding(Edges::xy(10.0, 0.0))
            .gap(8.0)
            .align_items(Align::Center)
            .background(colors::active().surface)
            .border_bottom(1.0, colors::active().stroke_soft)
            .child(
                Text::new("Node Graph")
                    .size(13.0)
                    .semibold()
                    .color(colors::active().text),
            );

        // Scope chips: document root + every master.
        for (id, name) in &scopes {
            let st = self.graph.clone();
            let id = *id;
            let active = id == active_scope;
            toolbar = toolbar.child(
                Container::new()
                    .padding(Edges::xy(8.0, 3.0))
                    .radius(5.0)
                    .background(if active {
                        colors::active().sel
                    } else {
                        Color::TRANSPARENT
                    })
                    .hover(colors::active().bg_lighter)
                    .on_click(move || st.set_scope(id))
                    .child(
                        Text::new(if active {
                            format!("{name} ◇")
                        } else {
                            name.clone()
                        })
                        .size(12.0)
                        .color(if active {
                            Color::WHITE
                        } else {
                            colors::active().icon_inactive
                        }),
                    ),
            );
        }

        toolbar = toolbar
            .child(Spacer::flex(1.0))
            .child(
                Text::new(format!("{scope_name} · {nodes_n} nodes · {wires_n} wires"))
                    .size(11.0)
                    .font("IBM Plex Mono")
                    .color(colors::active().icon_inactive),
            )
            // validation badges (DAG cycles + model issues)
            .child({
                let mut row = Row::new().gap(6.0).align_items(Align::Center);
                if errors > 0 {
                    row = row.child(badge("✕", errors, colors::active().danger));
                }
                if warns > 0 {
                    row = row.child(badge("!", warns, colors::active().warn));
                }
                if errors == 0 && warns == 0 && nodes_n > 0 {
                    row = row.child(
                        Text::new("�? DAG ok")
                            .size(11.0)
                            .font("IBM Plex Mono")
                            .color(colors::active().ok),
                    );
                }
                row
            })
            // Reset View button
            .child(
                Container::new()
                    .padding(Edges::xy(8.0, 4.0))
                    .radius(5.0)
                    .hover(colors::active().bg_lighter)
                    .on_click(move || {
                        graph.zoom.set(1.0);
                        graph.pan_x.set(0.0);
                        graph.pan_y.set(0.0);
                        graph.bump();
                    })
                    .child(
                        Text::new("Reset View")
                            .size(12.0)
                            .color(colors::active().icon_active),
                    ),
            );

        Column::new()
            .fill_size()
            .background(colors::active().bg_darker)
            .child(AnyView::new(toolbar))
            .child(
                Row::new()
                    .fill_size()
                    // Browser (signals, variables, rules) on the left, the
                    // properties of the selected node on the right.
                    .child(browser_panel(&self.graph))
                    .child(GraphCanvas {
                        st: self.graph.clone(),
                    })
                    .child(properties_panel(&self.graph)),
            )
    }
}

fn badge(mark: &str, count: usize, color: Color) -> impl View + 'static {
    Row::new()
        .gap(3.0)
        .align_items(Align::Center)
        .child(
            Container::new()
                .width(16.0)
                .height(16.0)
                .center()
                .radius(8.0)
                .background(color)
                .child(Text::new(mark).size(11.0).color(Color::hex(0x111111))),
        )
        .child(
            Text::new(format!("{count}"))
                .size(11.0)
                .font("IBM Plex Mono")
                .color(colors::active().text),
        )
}

/// Creates a ready-to-open WindowSpec bound to the editor state: the graph
/// shows the scope of the current selection (nearest master, or the page).
pub(crate) fn node_graph_spec(editor: EditorState) -> sequoia_ui::prelude::WindowSpec {
    let graph = NodeGraphState::new(editor.clone(), None);
    WindowSpec::new("Node Graph", NodeGraphView { editor, graph }).size(1100.0, 720.0)
}

#[cfg(test)]
mod tests {
    use super::model::PortSide;
    use super::*;
    use crate::model::behavior::{Action, Behavior, Rule, Trigger, Var, VarType};
    use crate::model::svelte_ast::{SvelteNode, new_button, new_frame};
    use crate::model::tree_ops;
    use sequoia_ui::core::{RenderItem, RenderList};
    use sequoia_ui::input::{Key, Modifiers, PointerButton};
    use sequoia_ui::text::TextSystem;
    use sequoia_ui::view::Runtime;

    /// Page(1){ master Card(2){ Btn(3) }, Panel(4) } with one rule on the
    /// button and the `open` variable on the master — enough to render a
    /// Trigger → Action chain and a State node.
    fn seed_editor() -> EditorState {
        let mut btn = new_button(3, 0.0, 0.0);
        if let SvelteNode::Element(e) = &mut btn {
            e.name = "Btn".into();
            e.behavior = Some(Behavior {
                vars: vec![],
                rules: vec![Rule {
                    trigger: Trigger::OnClick,
                    actions: vec![Action::ToggleVar("open".into())],
                }],
            });
        }
        let mut card = new_frame(2, 0.0, 0.0, 200.0, 80.0);
        if let SvelteNode::Element(e) = &mut card {
            e.component = true;
            e.name = "Card".into();
            e.behavior = Some(Behavior {
                vars: vec![Var::new("open", VarType::Bool)],
                rules: vec![],
            });
            e.children = vec![btn];
        }
        let mut panel = new_frame(4, 0.0, 0.0, 120.0, 60.0);
        if let SvelteNode::Element(e) = &mut panel {
            e.name = "Panel".into();
        }
        let mut root = new_frame(1, 0.0, 0.0, 500.0, 400.0);
        if let SvelteNode::Element(e) = &mut root {
            e.name = "Page".into();
            e.children = vec![card, panel];
        }
        EditorState::with_seed(Some(root), Some(3))
    }

    fn mounted() -> (Runtime, NodeGraphState) {
        let editor = seed_editor();
        let graph = NodeGraphState::new(editor.clone(), None);
        let probe = graph.clone();
        let rt = Runtime::new();
        rt.set_root_fill(true);
        rt.mount(NodeGraphView { editor, graph });
        let mut text = TextSystem::new();
        rt.layout(Rect::from_xywh(0.0, 0.0, 1100.0, 720.0), &mut text);
        (rt, probe)
    }

    #[test]
    fn graph_window_renders_behavior_nodes() {
        let (rt, graph) = mounted();
        // Scope of the selection (Btn 3) is the master Card.
        assert_eq!(graph.scope_name.get(), "Card");
        let nodes = graph.nodes.get();
        let keys: Vec<&str> = nodes.iter().map(|n| n.key.as_str()).collect();
        assert!(keys.contains(&"t:3:0"), "{keys:?}");
        assert!(keys.contains(&"a:3:0:0"), "{keys:?}");
        assert!(keys.contains(&"s:open"), "{keys:?}");
        assert_eq!(graph.connections.get().len(), 2, "exec + data edge");

        let mut text = TextSystem::new();
        let mut list = RenderList::new();
        rt.paint(&mut list, &mut text);
        assert!(rt.errors().is_empty(), "{:?}", rt.errors());
        assert!(!list.is_empty(), "node graph rendered nothing");
        let drawn = list
            .items()
            .iter()
            .filter(|item| match item {
                RenderItem::Quad(quad) => quad.rect.top() >= 36.0,
                _ => false,
            })
            .count();
        assert!(drawn > 50, "graph area only drew {drawn} quads");
    }

    #[test]
    fn scope_chips_switch_scopes() {
        let (_, graph) = mounted();
        let scopes = graph.scope_options();
        assert!(scopes.iter().any(|(id, _)| *id == 1), "{scopes:?}");
        assert!(scopes.iter().any(|(id, _)| *id == 2), "{scopes:?}");
        graph.set_scope(1);
        assert_eq!(graph.scope_id.get(), 1);
        assert_eq!(graph.scope_name.get(), "Page");
        // The document root scope shows its own (empty) graph.
        assert!(graph.nodes.get().is_empty());
    }

    #[test]
    fn clicking_a_node_selects_the_element_in_the_editor() {
        let (_, graph) = mounted();
        let t = graph
            .nodes
            .get()
            .iter()
            .find(|n| n.key == "t:3:0")
            .cloned()
            .expect("trigger node");
        graph.select_node(t.id);
        assert_eq!(graph.selected_node.get(), Some(t.id));
        assert_eq!(graph.editor.selected_layer.get(), Some(3));
    }

    #[test]
    fn dragging_a_node_moves_it_and_survives_rebuild() {
        let (rt, graph) = mounted();
        let mut text = TextSystem::new();
        let t = graph
            .nodes
            .get()
            .iter()
            .find(|n| n.key == "t:3:0")
            .cloned()
            .expect("trigger node");
        // Auto-layout puts triggers in the second column; the pointer takes
        // WINDOW coordinates. The canvas starts after the 36px toolbar and
        // the 236px browser panel.
        const CANVAS_X: f32 = 236.0;
        let start_nodes = graph.nodes.get();
        let start = start_nodes.iter().find(|n| n.id == t.id).unwrap();
        let (sx, sy) = (CANVAS_X + start.x + 60.0, start.y + 12.0 + 36.0);

        rt.pointer_down(
            Point::new(sx, sy),
            PointerButton::Left,
            Modifiers::NONE,
            &mut text,
        );
        rt.pointer_move(Point::new(sx + 90.0, sy + 40.0), &mut text);
        rt.pointer_up(
            Point::new(sx + 90.0, sy + 40.0),
            PointerButton::Left,
            Modifiers::NONE,
            &mut text,
        );

        let moved_nodes = graph.nodes.get();
        let moved = moved_nodes.iter().find(|n| n.id == t.id).unwrap();
        assert!((moved.x - (start.x + 90.0)).abs() < 0.5, "{}", moved.x);
        // A rebuild (document change) keeps the dragged position.
        graph.rebuild();
        let after_nodes = graph.nodes.get();
        let after = after_nodes.iter().find(|n| n.id == t.id).unwrap();
        assert!((after.x - moved.x).abs() < 0.5);
        assert!((after.y - moved.y).abs() < 0.5);
    }

    #[test]
    fn document_edits_rebuild_the_graph() {
        let (_, graph) = mounted();
        graph.set_scope(2);
        assert_eq!(graph.nodes.get().len(), 3);
        // A document change (rule removed) flows into the window through
        // `sync` — history length is the stamp. The selection is Btn(3),
        // so the editor op edits that element's rules.
        graph.editor.remove_rule(0);
        graph.sync();
        assert!(
            !graph.nodes.get().iter().any(|n| n.key == "t:3:0"),
            "rule removed → trigger gone"
        );
    }

    #[test]
    fn instance_on_event_shows_in_the_parent_graph() {
        // Phase 4 (design doc §4.6): an `On Event` rule on an instance is
        // the parent's handler — it must appear in the PARENT's graph even
        // though the instance interior belongs to the master's own graph.
        // Page(1) { Card(2, comp) { Btn(3) }, Panel(4) } — plain rects, so
        // the ids stay unique (new_button would add a text child with id 4).
        let mut btn = crate::model::svelte_ast::new_rect(3, 0.0, 0.0, 80.0, 30.0, 0x0D99FF);
        if let SvelteNode::Element(e) = &mut btn {
            e.behavior = Some(Behavior {
                vars: vec![],
                rules: vec![Rule {
                    trigger: Trigger::OnClick,
                    actions: vec![Action::ToggleVar("open".into())],
                }],
            });
        }
        let mut card = crate::model::svelte_ast::new_frame(2, 0.0, 0.0, 200.0, 80.0);
        if let SvelteNode::Element(e) = &mut card {
            e.component = true;
            e.name = "Card".into();
            e.behavior = Some(Behavior {
                vars: vec![Var::new("open", VarType::Bool)],
                rules: vec![],
            });
            e.children = vec![btn];
        }
        let mut panel = crate::model::svelte_ast::new_frame(4, 0.0, 0.0, 120.0, 60.0);
        if let SvelteNode::Element(e) = &mut panel {
            e.name = "Panel".into();
            // The parent's handler for the child's event.
            e.behavior = Some(Behavior {
                vars: vec![],
                rules: vec![Rule {
                    trigger: Trigger::OnEvent("confirm".into()),
                    actions: vec![Action::ToggleVar("open".into())],
                }],
            });
        }
        let mut root = crate::model::svelte_ast::new_frame(1, 0.0, 0.0, 500.0, 400.0);
        if let SvelteNode::Element(e) = &mut root {
            e.name = "Page".into();
            e.children = vec![card, panel];
        }
        let editor = EditorState::with_seed(Some(root), Some(3));

        let graph = NodeGraphState::new(editor.clone(), None);
        // The selection (3) scopes to Card: Btn's rule is visible there.
        assert!(graph.nodes.get().iter().any(|n| n.key == "t:3:0"));

        // Page scope: the instance's On Event rule is the parent's handler.
        graph.set_scope(1);
        let keys: Vec<String> = graph.nodes.get().iter().map(|n| n.key.clone()).collect();
        assert!(
            keys.iter().any(|k| k == "t:4:0"),
            "On Event trigger of the instance is in the parent graph: {keys:?}"
        );
        assert!(
            !keys.iter().any(|k| k == "t:3:0"),
            "the master's interior stays out of the parent graph: {keys:?}"
        );
    }

    #[test]
    fn horizontal_wheel_pans_the_graph() {
        let (rt, graph) = mounted();
        let mut text = TextSystem::new();
        // The canvas sits between the browser panel and the properties panel.
        rt.wheel(
            Point::new(600.0, 400.0),
            Vec2::new(40.0, 0.0),
            Modifiers::NONE,
            &mut text,
        );
        assert_eq!(graph.pan_x.get(), -40.0);
        assert_eq!(graph.pan_y.get(), 0.0);
    }

    #[test]
    fn dragging_empty_canvas_pans_immediately() {
        let (rt, graph) = mounted();
        let mut text = TextSystem::new();
        rt.pointer_down(
            Point::new(700.0, 500.0),
            PointerButton::Left,
            Modifiers::NONE,
            &mut text,
        );
        rt.pointer_move(Point::new(750.0, 530.0), &mut text);
        assert_eq!(graph.pan_x.get(), -50.0);
        assert_eq!(graph.pan_y.get(), -30.0);
    }

    #[test]
    fn dragging_between_sockets_wires_the_rule() {
        let (rt, graph) = mounted();
        let mut text = TextSystem::new();
        const CANVAS_X: f32 = 236.0;
        // Seeded scope: State (s:open) in column 0, Trigger (t:3:0) in
        // column 1, Action (a:3:0:0) in column 2 — all inside the canvas.
        // 1) Grab the trigger's exec output and drop it on the action's
        //    exec input: the step already follows the trigger, so this is a
        //    no-op and must not corrupt the document.
        let nodes = graph.nodes.get();
        let t = nodes.iter().find(|n| n.key == "t:3:0").cloned().unwrap();
        let a = nodes.iter().find(|n| n.key == "a:3:0:0").cloned().unwrap();
        let out = t.port_pos(PortSide::Output, 0);
        let inp = a.port_pos(PortSide::Input, 0);
        rt.pointer_down(
            Point::new(CANVAS_X + out.x, 36.0 + out.y),
            PointerButton::Left,
            Modifiers::NONE,
            &mut text,
        );
        rt.pointer_move(Point::new(CANVAS_X + inp.x, 36.0 + inp.y), &mut text);
        rt.pointer_up(
            Point::new(CANVAS_X + inp.x, 36.0 + inp.y),
            PointerButton::Left,
            Modifiers::NONE,
            &mut text,
        );
        assert!(graph.nodes.get().iter().any(|n| n.key == "a:3:0:0"));

        // 2) State "read" → the action's EXEC input is invalid: a warning
        //    toast fires and the document stays intact.
        let s = nodes.iter().find(|n| n.key == "s:open").cloned().unwrap();
        let s_out = s.port_pos(PortSide::Output, 0);
        rt.pointer_down(
            Point::new(CANVAS_X + s_out.x, 36.0 + s_out.y),
            PointerButton::Left,
            Modifiers::NONE,
            &mut text,
        );
        rt.pointer_move(Point::new(CANVAS_X + inp.x, 36.0 + inp.y), &mut text);
        rt.pointer_up(
            Point::new(CANVAS_X + inp.x, 36.0 + inp.y),
            PointerButton::Left,
            Modifiers::NONE,
            &mut text,
        );
        assert!(
            graph
                .editor
                .toasts
                .get()
                .iter()
                .any(|t| { matches!(t.kind, crate::state::ToastKind::Warn) })
        );
    }

    #[test]
    fn browser_panel_lists_state_nodes() {
        let (_, graph) = mounted();
        // The browser is driven by the same node list; filtering by name
        // finds the `open` variable.
        graph.browser_query.set("open".into());
        let nodes = graph.nodes.get();
        assert!(nodes.iter().any(|n| n.key == "s:open"));
        graph.browser_query.set("zzz".into());
    }

    fn select_key(graph: &NodeGraphState, key: &str) {
        let node = graph
            .nodes
            .get()
            .iter()
            .find(|n| n.key == key)
            .cloned()
            .unwrap_or_else(|| panic!("missing node {key}"));
        graph.select_node(node.id);
    }

    fn behavior_of(graph: &NodeGraphState, el: u64) -> Option<Behavior> {
        let root = graph.editor.root.get_untracked().unwrap();
        match tree_ops::find(&root, el) {
            Some(SvelteNode::Element(e)) => e.behavior.clone(),
            _ => None,
        }
    }

    #[test]
    fn add_state_rule_and_action_nodes_grows_the_document() {
        let (_, graph) = mounted();
        // State lands on the scope owner (Card), the selected rule's element.
        assert!(graph.add_state_node());
        assert!(
            graph.nodes.get().iter().any(|n| n.key == "s:var1"),
            "new state node appears"
        );
        let card = behavior_of(&graph, 2).expect("Card behavior");
        assert!(card.vars.iter().any(|v| v.name == "var1"));

        // Rule attaches to the currently selected node's element (Card).
        assert!(graph.add_rule_node());
        assert!(graph.nodes.get().iter().any(|n| n.key == "t:2:0"));
        // Default action: Toggle with the first bool already in scope.
        assert!(graph.add_action_node("toggle"));
        assert!(
            graph.nodes.get().iter().any(|n| n.key == "a:2:0:0"),
            "action node hangs off the new rule"
        );
        let card = behavior_of(&graph, 2).unwrap();
        assert_eq!(card.rules[0].actions[0], Action::ToggleVar("open".into()));

        // Add an action to the seeded Btn rule.
        select_key(&graph, "t:3:0");
        assert!(graph.add_action_node("delay"));
        let btn = behavior_of(&graph, 3).unwrap();
        assert_eq!(btn.rules[0].actions.len(), 2);
        assert!(matches!(btn.rules[0].actions[1], Action::Delay(..)));
    }

    #[test]
    fn editing_nodes_updates_the_behavior_model() {
        let (_, graph) = mounted();
        select_key(&graph, "s:open");
        assert!(graph.rename_state("enabled"));
        assert!(graph.nodes.get().iter().any(|n| n.key == "s:enabled"));
        assert_eq!(
            behavior_of(&graph, 3).unwrap().rules[0].actions[0],
            Action::ToggleVar("enabled".into())
        );

        select_key(&graph, "t:3:0");
        assert_eq!(graph.selected_trigger(), Some(Trigger::OnClick));
        assert!(graph.set_trigger_kind("key"));
        assert_eq!(
            graph.selected_trigger(),
            Some(Trigger::OnKey("Enter".into()))
        );
        assert!(graph.set_trigger_param("Space".into()));
        assert_eq!(
            graph.selected_trigger(),
            Some(Trigger::OnKey("Space".into()))
        );

        select_key(&graph, "a:3:0:0");
        assert_eq!(
            graph.selected_action(),
            Some(Action::ToggleVar("enabled".into()))
        );
        assert!(graph.set_action_kind("set"));
        assert_eq!(
            graph.selected_action(),
            Some(Action::SetVar(
                "enabled".into(),
                crate::model::behavior::ValueExpr::Value(crate::model::behavior::Value::Bool(
                    false
                ))
            ))
        );
        assert!(graph.set_action_literal("true"));
        assert_eq!(
            graph.selected_action(),
            Some(Action::SetVar(
                "enabled".into(),
                crate::model::behavior::ValueExpr::Value(crate::model::behavior::Value::Bool(true))
            ))
        );

        select_key(&graph, "s:enabled");
        assert!(graph.set_state_type(VarType::Number));
        assert_eq!(behavior_of(&graph, 2).unwrap().vars[0].ty, VarType::Number);
    }

    #[test]
    fn deleting_nodes_prunes_the_document() {
        let (_, graph) = mounted();
        select_key(&graph, "s:open");
        assert!(graph.delete_selected_node());
        assert!(
            !graph.nodes.get().iter().any(|n| n.key == "s:open"),
            "state node is gone"
        );
        let card = behavior_of(&graph, 2);
        assert!(card.is_none(), "empty Card behavior is dropped");
        let btn = behavior_of(&graph, 3).unwrap();
        assert!(
            btn.rules[0].actions.is_empty(),
            "Toggle open pruned with the variable"
        );

        select_key(&graph, "t:3:0");
        assert!(graph.delete_selected_node());
        assert!(behavior_of(&graph, 3).is_none());
        assert!(!graph.nodes.get().iter().any(|n| n.key == "t:3:0"));
    }

    #[test]
    fn delete_key_removes_the_selected_node() {
        let (rt, graph) = mounted();
        let mut text = TextSystem::new();
        let t = graph
            .nodes
            .get()
            .iter()
            .find(|n| n.key == "t:3:0")
            .cloned()
            .expect("trigger node");
        const CANVAS_X: f32 = 236.0;
        let click = Point::new(CANVAS_X + t.x + 50.0, 36.0 + t.y + 12.0);
        rt.pointer_down(click, PointerButton::Left, Modifiers::NONE, &mut text);
        rt.pointer_up(click, PointerButton::Left, Modifiers::NONE, &mut text);
        assert_eq!(graph.selected_node.get(), Some(t.id));
        rt.key(Key::DELETE, Modifiers::NONE, &mut text);
        assert!(
            !graph.nodes.get().iter().any(|n| n.key == "t:3:0"),
            "Delete removed the selected Trigger node"
        );
        assert!(behavior_of(&graph, 3).is_none());
    }
}
