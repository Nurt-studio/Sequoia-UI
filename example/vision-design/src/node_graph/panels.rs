//! Side panels of the node editor window: the browser (left) lists scopes,
//! state variables and rules; the right side shows the properties of the
//! selected node.

use super::graph_state::NodeGraphState;
use super::model::{GraphConnection, GraphNode, NodeKind};
use crate::components::search_field::search_field_cancellable;
use crate::model::behavior::{Action, Value, VarType};
use crate::state::PickerOption;
use crate::theme::colors;
use sequoia_ui::prelude::*;

const BROWSER_W: f32 = 236.0;
const PROPS_W: f32 = 252.0;

// ─────────────────────────────── browser (left) ───────────────────────────────

pub(crate) fn browser_panel(graph: &NodeGraphState) -> impl View + 'static {
    BrowserPanel {
        graph: graph.clone(),
    }
}

struct BrowserPanel {
    graph: NodeGraphState,
}

impl View for BrowserPanel {
    fn body(&self) -> impl View + 'static {
        let q = self.graph.browser_query.get().to_lowercase();
        let nodes = self.graph.nodes.get();
        let matches = |n: &GraphNode| q.is_empty() || n.title.to_lowercase().contains(&q);

        let vars: Vec<GraphNode> = nodes
            .iter()
            .filter(|n| n.kind == NodeKind::State && matches(n))
            .cloned()
            .collect();
        let triggers: Vec<GraphNode> = nodes
            .iter()
            .filter(|n| n.kind == NodeKind::Trigger && matches(n))
            .cloned()
            .collect();
        let actions: Vec<GraphNode> = nodes
            .iter()
            .filter(|n| n.kind == NodeKind::Action && matches(n))
            .cloned()
            .collect();
        let vars_empty = vars.is_empty();
        let triggers_empty = triggers.is_empty();
        let actions_empty = actions.is_empty();
        let add_state = self.graph.clone();
        let add_rule = self.graph.clone();
        let add_action = self.graph.clone();

        let mut body = Column::new().gap(2.0).fill_width();
        body = body.child(section_rows(&self.graph, "VARIABLES", vars));
        body = body.child(section_rows(&self.graph, "RULES", triggers));
        body = body.child(section_rows(&self.graph, "ACTIONS", actions));
        if q.is_empty() && nodes.is_empty() {
            body = body.child(
                Container::new().padding(Edges::xy(8.0, 10.0)).child(
                    Text::new("No rules or variables in this scope yet.")
                        .size(11.0)
                        .wrap()
                        .color(colors::active().icon_inactive),
                ),
            );
        }
        if !q.is_empty() && vars_empty && triggers_empty && actions_empty {
            body = body.child(
                Container::new().padding(Edges::xy(8.0, 10.0)).child(
                    Text::new("No matches")
                        .size(11.0)
                        .color(colors::active().icon_inactive),
                ),
            );
        }

        Column::new()
            .width(BROWSER_W)
            .fill_height()
            .background(colors::active().surface)
            .border_right(1.0, colors::active().stroke_soft)
            .child(
                Row::new().fill_width().padding(Edges::xy(10.0, 8.0)).child(
                    Text::new("Browser")
                        .size(11.0)
                        .semibold()
                        .letter_spacing(0.4)
                        .color(colors::active().icon_active),
                ),
            )
            .child(
                Container::new()
                    .fill_width()
                    .margin(Edges::xy(8.0, 0.0))
                    .child(search_field_cancellable(self.graph.browser_query.clone())),
            )
            .child(Spacer::fixed(6.0))
            .child(
                Row::new()
                    .fill_width()
                    .padding(Edges::xy(8.0, 0.0))
                    .gap(4.0)
                    .align_items(Align::Center)
                    .child(tiny_button("+ State", move || {
                        add_state.add_state_node();
                    }))
                    .child(tiny_button("+ Rule", move || {
                        add_rule.add_rule_node();
                    }))
                    .child(picker(
                        &self.graph,
                        "+ Action".into(),
                        action_kind_options(),
                        "",
                        78.0,
                        move |kind| {
                            add_action.add_action_node(kind);
                        },
                    )),
            )
            .child(Spacer::fixed(6.0))
            .child(
                Container::new()
                    .fill_width()
                    .flex(1.0)
                    .child(ScrollView::new(body).vertical()),
            )
    }
}

fn section_rows(graph: &NodeGraphState, title: &str, rows: Vec<GraphNode>) -> AnyView {
    let mut col = Column::new().gap(1.0).fill_width();
    if !rows.is_empty() {
        col = col.child(
            Text::new(title.to_string())
                .size(11.0)
                .font("IBM Plex Mono")
                .letter_spacing(0.8)
                .color(colors::active().icon_inactive),
        );
    }
    for n in rows {
        col = col.child(browser_row(graph, n));
    }
    AnyView::new(
        Container::new()
            .fill_width()
            .padding(Edges::xy(8.0, 6.0))
            .child(col),
    )
}

fn browser_row(graph: &NodeGraphState, node: GraphNode) -> impl View + 'static {
    let selected = graph.selected_node.get() == Some(node.id);
    let g = graph.clone();
    let nid = node.id;
    Container::new()
        .fill_width()
        .padding(Edges::xy(7.0, 4.0))
        .radius(5.0)
        .hover(colors::active().bg_lighter)
        .style(if selected {
            BoxStyle::new()
                .background(colors::active().sel_soft)
                .border(1.0, Color::hexa(0x0D99FF73))
        } else {
            BoxStyle::new()
        })
        .on_click(move || g.select_node(nid))
        .child(
            Row::new()
                .gap(7.0)
                .align_items(Align::Center)
                .child(
                    Container::new()
                        .width(7.0)
                        .height(7.0)
                        .radius(3.5)
                        .background(node.kind.accent()),
                )
                .child_flex(
                    Column::new()
                        .gap(1.0)
                        .child(
                            Text::new(node.title.clone())
                                .size(13.0)
                                .color(colors::active().text)
                                .truncate_fill(),
                        )
                        .child(
                            Text::new(node.subtitle.clone().unwrap_or_default())
                                .size(11.0)
                                .font("IBM Plex Mono")
                                .color(colors::active().icon_inactive)
                                .truncate_fill(),
                        ),
                    1.0,
                ),
        )
}

// ───────────────────────────────── widgets ─────────────────────────────────

fn caption(text: &str) -> impl View + 'static {
    Text::new(text.to_string())
        .size(11.0)
        .font("IBM Plex Mono")
        .letter_spacing(0.6)
        .color(colors::active().icon_inactive)
}

fn tiny_button(label: &str, on_click: impl Fn() + 'static) -> impl View + 'static {
    Container::new()
        .height(22.0)
        .padding_xy(7.0, 0.0)
        .center()
        .radius(6.0)
        .background(colors::active().bg_lighter)
        .border(1.0, colors::active().stroke_soft)
        .hover(colors::active().bg_darker)
        .on_click(on_click)
        .child(
            Text::new(label.to_string())
                .size(11.0)
                .color(Color::hex(0xE8E8E8)),
        )
}

fn text_field(sig: Signal<String>, on_submit: impl Fn(String) + 'static) -> impl View + 'static {
    TextField::new(sig)
        .font_size(13.0)
        .text_color(Color::hex(0xE8E8E8))
        .background(colors::active().bg_darker)
        .border(1.0, colors::active().stroke_soft)
        .radius(6.0)
        .padding(Edges::xy(6.0, 4.0))
        .on_submit(move |text: String| on_submit(text))
}

fn picker(
    graph: &NodeGraphState,
    label: String,
    options: Vec<PickerOption>,
    selected: &str,
    width: f32,
    on_pick: impl Fn(&str) + 'static,
) -> impl View + 'static {
    crate::components::picker_btn::PickerBtn {
        state: graph.editor.clone(),
        label,
        options,
        selected: selected.to_string(),
        on_pick: std::rc::Rc::new(on_pick),
        width,
        menu_width: 190.0,
    }
}

fn field_row(label: &str, control: impl View + 'static) -> impl View + 'static {
    Row::new()
        .fill_width()
        .gap(8.0)
        .align_items(Align::Center)
        .child(
            Container::new().width(64.0).child(
                Text::new(label.to_string())
                    .size(11.0)
                    .color(colors::active().icon_inactive),
            ),
        )
        .child_flex(control, 1.0)
}

fn string_options(values: impl IntoIterator<Item = String>) -> Vec<PickerOption> {
    values
        .into_iter()
        .map(|value| PickerOption {
            label: value.clone(),
            value,
        })
        .collect()
}

fn var_options(graph: &NodeGraphState) -> Vec<PickerOption> {
    graph
        .scope_var_list()
        .into_iter()
        .map(|v| PickerOption {
            label: format!("{} · {}", v.name, v.ty.as_str()),
            value: v.name,
        })
        .collect()
}

fn action_kind_options() -> Vec<PickerOption> {
    [
        ("Toggle variable", "toggle"),
        ("Set variable", "set"),
        ("Show", "show"),
        ("Hide", "hide"),
        ("Emit event", "emit"),
        ("Delay", "delay"),
    ]
    .into_iter()
    .map(|(label, value)| PickerOption {
        label: label.into(),
        value: value.into(),
    })
    .collect()
}

fn trigger_kind_options() -> Vec<PickerOption> {
    [
        ("On Click", "click"),
        ("On Hover In", "hover_in"),
        ("On Hover Out", "hover_out"),
        ("On Mount", "mount"),
        ("On Key", "key"),
        ("On Prop Changed", "prop"),
        ("On Event", "event"),
    ]
    .into_iter()
    .map(|(label, value)| PickerOption {
        label: label.into(),
        value: value.into(),
    })
    .collect()
}

// ──────────────────────────── properties (right) ────────────────────────────

pub(crate) fn properties_panel(graph: &NodeGraphState) -> impl View + 'static {
    PropsPanel {
        graph: graph.clone(),
    }
}

struct PropsPanel {
    graph: NodeGraphState,
}

impl View for PropsPanel {
    fn body(&self) -> impl View + 'static {
        let nodes = self.graph.nodes.get();
        let selected = self
            .graph
            .selected_node
            .get()
            .and_then(|id| nodes.iter().find(|n| n.id == id).cloned());
        let conns = self.graph.connections.get();

        let mut col = Column::new().gap(10.0).fill_width();
        match selected {
            None => {
                col = col.child(
                    Text::new(
                        "Select a node on the canvas or in the browser to see its properties.",
                    )
                    .size(11.0)
                    .wrap()
                    .color(colors::active().icon_inactive),
                );
            }
            Some(node) => {
                let in_n = conns
                    .iter()
                    .filter(|c: &&GraphConnection| c.to_node == node.id)
                    .count();
                let out_n = conns
                    .iter()
                    .filter(|c: &&GraphConnection| c.from_node == node.id)
                    .count();

                col = col.child(
                    Row::new()
                        .gap(8.0)
                        .align_items(Align::Center)
                        .child(
                            Container::new()
                                .padding(Edges::xy(6.0, 2.0))
                                .radius(4.0)
                                .background(node.kind.header_color())
                                .child(
                                    Text::new(kind_label(node.kind).to_string())
                                        .size(10.0)
                                        .font("IBM Plex Mono")
                                        .color(Color::WHITE),
                                ),
                        )
                        .child_flex(
                            Text::new(node.title.clone())
                                .size(13.0)
                                .semibold()
                                .color(Color::hex(0xE8E8E8))
                                .truncate_fill(),
                            1.0,
                        ),
                );

                if let Some(sub) = &node.subtitle {
                    col = col
                        .child(prop_row("Element", sub))
                        .child(prop_row("Key", &node.key));
                } else {
                    col = col.child(prop_row("Key", &node.key));
                }
                col = col.child(prop_row("Connections", &format!("{in_n} in · {out_n} out")));

                if node.in_cycle {
                    col = col.child(
                        Text::new("�? Part of a dependency cycle")
                            .size(11.0)
                            .color(colors::active().danger),
                    );
                }

                col = col.child(match node.kind {
                    NodeKind::State => state_editor(&self.graph),
                    NodeKind::Trigger => trigger_editor(&self.graph),
                    NodeKind::Action => action_editor(&self.graph),
                });

                let del = self.graph.clone();
                col = col.child(tiny_button("Delete node", move || {
                    del.delete_selected_node();
                }));
                col = col.child(
                    Text::new("Delete / Backspace removes the selected node")
                        .size(11.0)
                        .wrap()
                        .color(colors::active().icon_inactive),
                );

                col = col.child(port_list("Inputs", &node.inputs));
                col = col.child(port_list("Outputs", &node.outputs));
            }
        }

        Column::new()
            .width(PROPS_W)
            .fill_height()
            .background(colors::active().surface)
            .border_left(1.0, colors::active().stroke_soft)
            .child(
                Row::new().fill_width().padding(Edges::xy(10.0, 8.0)).child(
                    Text::new("Properties")
                        .size(11.0)
                        .semibold()
                        .letter_spacing(0.4)
                        .color(colors::active().icon_active),
                ),
            )
            .child(
                Container::new().fill_width().flex(1.0).child(
                    ScrollView::new(Container::new().padding(Edges::xy(10.0, 4.0)).child(col))
                        .vertical(),
                ),
            )
    }
}

fn state_editor(graph: &NodeGraphState) -> AnyView {
    let mut col = Column::new().fill_width().gap(6.0).child(caption("STATE"));
    let Some(var) = graph.selected_state() else {
        col = col.child(
            Text::new(
                "This state is referenced but not declared. Add a variable with \
                 + State, or fix the rule in the Interact tab.",
            )
            .size(11.0)
            .wrap()
            .color(colors::active().warn),
        );
        return AnyView::new(col);
    };

    let rename = graph.clone();
    let name_sig = Signal::new(var.name.clone());
    col = col.child(field_row(
        "Name",
        text_field(name_sig, move |text| {
            rename.rename_state(&text);
        }),
    ));

    let retype = graph.clone();
    col = col.child(field_row(
        "Type",
        picker(
            graph,
            var.ty.as_str().into(),
            string_options(["bool", "number", "string"].map(str::to_string)),
            var.ty.as_str(),
            92.0,
            move |value| {
                if let Some(ty) = VarType::from_str(value) {
                    retype.set_state_type(ty);
                }
            },
        ),
    ));

    let initial: AnyView = match &var.initial {
        Value::Bool(b) => {
            let set = graph.clone();
            let selected = b.to_string();
            AnyView::new(picker(
                graph,
                selected.clone(),
                string_options(["false", "true"].map(str::to_string)),
                &selected,
                92.0,
                move |value| {
                    set.set_state_initial_bool(value == "true");
                },
            ))
        }
        Value::Number(n) => {
            let set = graph.clone();
            let sig = Signal::new(n.to_string());
            AnyView::new(text_field(sig, move |text| {
                if let Ok(value) = text.trim().parse::<f32>() {
                    set.set_state_initial_number(value);
                } else {
                    set.editor.toast_warn("Not a number");
                }
            }))
        }
        Value::Str(s) => {
            let set = graph.clone();
            let sig = Signal::new(s.clone());
            AnyView::new(text_field(sig, move |text| {
                set.set_state_initial_str(text);
            }))
        }
    };
    col = col.child(field_row("Initial", initial));
    AnyView::new(col)
}

fn trigger_editor(graph: &NodeGraphState) -> AnyView {
    let mut col = Column::new()
        .fill_width()
        .gap(6.0)
        .child(caption("TRIGGER"));
    let Some(trigger) = graph.selected_trigger() else {
        col = col.child(
            Text::new("This trigger is not in the document anymore.")
                .size(11.0)
                .wrap()
                .color(colors::active().warn),
        );
        return AnyView::new(col);
    };

    let kind = trigger.kind_name();
    let retrigger = graph.clone();
    col = col.child(field_row(
        "Kind",
        picker(
            graph,
            trigger.label(),
            trigger_kind_options(),
            kind,
            150.0,
            move |value| {
                retrigger.set_trigger_kind(value);
            },
        ),
    ));

    if let Some(param) = trigger.param() {
        let set = graph.clone();
        let sig = Signal::new(param.to_string());
        col = col.child(field_row(
            "Param",
            text_field(sig, move |text| {
                set.set_trigger_param(text);
            }),
        ));
    }
    AnyView::new(col)
}

fn action_editor(graph: &NodeGraphState) -> AnyView {
    let mut col = Column::new().fill_width().gap(6.0).child(caption("ACTION"));
    let Some(action) = graph.selected_action() else {
        col = col.child(
            Text::new("This action is not in the document anymore.")
                .size(11.0)
                .wrap()
                .color(colors::active().warn),
        );
        return AnyView::new(col);
    };

    let kind = action.kind_name();
    let retype = graph.clone();
    col = col.child(field_row(
        "Kind",
        picker(
            graph,
            action.label(),
            action_kind_options(),
            kind,
            150.0,
            move |value| {
                retype.set_action_kind(value);
            },
        ),
    ));

    match &action {
        Action::ToggleVar(name) => {
            let set = graph.clone();
            col = col.child(field_row(
                "Variable",
                picker(
                    graph,
                    name.clone(),
                    var_options(graph),
                    name,
                    150.0,
                    move |value| {
                        set.set_action_var(value);
                    },
                ),
            ));
        }
        Action::SetVar(name, expr) => {
            let set = graph.clone();
            col = col.child(field_row(
                "Variable",
                picker(
                    graph,
                    name.clone(),
                    var_options(graph),
                    name,
                    150.0,
                    move |value| {
                        set.set_action_var(value);
                    },
                ),
            ));
            let set = graph.clone();
            let sig = Signal::new(expr.label());
            col = col.child(field_row(
                "Value",
                text_field(sig, move |text| {
                    set.set_action_literal(&text);
                }),
            ));
        }
        Action::Show { target, .. } | Action::Hide { target, .. } => {
            let set = graph.clone();
            col = col.child(field_row(
                "Target",
                picker(
                    graph,
                    target.clone(),
                    string_options(graph.scope_target_names()),
                    target,
                    150.0,
                    move |value| {
                        set.set_action_target_name(value);
                    },
                ),
            ));
        }
        Action::Emit(event) => {
            let set = graph.clone();
            let sig = Signal::new(event.clone());
            col = col.child(field_row(
                "Event",
                text_field(sig, move |text| {
                    set.set_action_event(&text);
                }),
            ));
        }
        Action::Delay(ms, _) => {
            let set = graph.clone();
            let sig = Signal::new(ms.to_string());
            col = col.child(field_row(
                "Wait",
                text_field(sig, move |text| match text.trim().parse::<u32>() {
                    Ok(value) => {
                        set.set_action_delay_ms(value);
                    }
                    Err(_) => set.editor.toast_warn("Not a number"),
                }),
            ));
            col = col.child(
                Text::new("Nested steps stay editable in the Interact tab.")
                    .size(11.0)
                    .wrap()
                    .color(colors::active().icon_inactive),
            );
        }
    }
    AnyView::new(col)
}

fn kind_label(kind: NodeKind) -> &'static str {
    match kind {
        NodeKind::Trigger => "TRIGGER",
        NodeKind::State => "STATE",
        NodeKind::Action => "ACTION",
    }
}

fn prop_row(label: &str, value: &str) -> impl View + 'static {
    let value = value.to_string();
    Row::new()
        .fill_width()
        .gap(8.0)
        .align_items(Align::Center)
        .child(
            Container::new().width(78.0).child(
                Text::new(label.to_string())
                    .size(11.0)
                    .color(colors::active().icon_inactive),
            ),
        )
        .child_flex(
            Text::new(value)
                .size(13.0)
                .font("IBM Plex Mono")
                .color(colors::active().text)
                .truncate_fill(),
            1.0,
        )
}

fn port_list(title: &str, ports: &[String]) -> impl View + 'static {
    let mut col = Column::new().gap(3.0).fill_width();
    col = col.child(
        Text::new(title.to_string())
            .size(11.0)
            .font("IBM Plex Mono")
            .letter_spacing(0.8)
            .color(colors::active().icon_inactive),
    );
    if ports.is_empty() {
        col = col.child(
            Text::new("—")
                .size(11.0)
                .color(colors::active().icon_inactive),
        );
    }
    for (i, label) in ports.iter().enumerate() {
        let desc = if label.is_empty() {
            "execution".to_string()
        } else {
            label.clone()
        };
        col = col.child(
            Row::new()
                .gap(6.0)
                .align_items(Align::Center)
                .child(
                    Container::new()
                        .width(8.0)
                        .height(8.0)
                        .radius(4.0)
                        .background(Color::hex(0x3A3A3E)),
                )
                .child(
                    Text::new(format!("{}  {desc}", i))
                        .size(12.0)
                        .wrap()
                        .font("IBM Plex Mono")
                        .color(colors::active().text),
                ),
        );
    }
    col
}
