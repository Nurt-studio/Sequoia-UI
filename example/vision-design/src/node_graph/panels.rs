//! Side panels of the node editor window: the browser (left) lists scopes,
//! state variables and rules; the right side shows the properties of the
//! selected node.

use super::graph_state::NodeGraphState;
use super::model::{GraphConnection, GraphNode, NodeKind};
use crate::components::search_field::search_field_cancellable;
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

        let mut body = Column::new().gap(2.0).fill_width();
        body = body.child(section_rows(&self.graph, "VARIABLES", vars));
        body = body.child(section_rows(&self.graph, "RULES", triggers));
        body = body.child(section_rows(&self.graph, "ACTIONS", actions));
        if q.is_empty() && nodes.is_empty() {
            body = body.child(
                Container::new().padding(Edges::xy(8.0, 10.0)).child(
                    Text::new("No rules or variables in this scope yet.")
                        .size(11.0)
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
                        .size(12.0)
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
                .size(10.0)
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
                                .size(12.0)
                                .color(colors::active().text)
                                .truncate_fill(),
                        )
                        .child(
                            Text::new(node.subtitle.clone().unwrap_or_default())
                                .size(9.5)
                                .font("IBM Plex Mono")
                                .color(colors::active().icon_inactive)
                                .truncate_fill(),
                        ),
                    1.0,
                ),
        )
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
                                        .size(9.5)
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

                if node.kind == NodeKind::State {
                    if let Some(sub) = &node.subtitle {
                        col = col.child(prop_row("Value", sub));
                    }
                }
                if node.in_cycle {
                    col = col.child(
                        Text::new("⚠ Part of a dependency cycle")
                            .size(10.5)
                            .color(colors::active().danger),
                    );
                }

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
                        .size(12.0)
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
                    .size(10.5)
                    .color(colors::active().icon_inactive),
            ),
        )
        .child_flex(
            Text::new(value)
                .size(11.0)
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
            .size(10.0)
            .font("IBM Plex Mono")
            .letter_spacing(0.8)
            .color(colors::active().icon_inactive),
    );
    if ports.is_empty() {
        col = col.child(
            Text::new("—")
                .size(10.5)
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
                    Text::new(format!("{} · {desc}", i))
                        .size(10.5)
                        .font("IBM Plex Mono")
                        .color(colors::active().text),
                ),
        );
    }
    col
}
