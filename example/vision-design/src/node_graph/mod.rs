pub(crate) mod canvas;
pub(crate) mod graph_state;
pub(crate) mod model;

use crate::state::EditorState;
use crate::theme::colors;
use canvas::GraphCanvas;
use graph_state::NodeGraphState;
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
        let st_del = self.graph.clone();

        Stack::new()
            .fill_size()
            .background(colors::active().bg_darker)
            // toolbar
            .child(
                Column::new()
                    .fill_size()
                    .child(
                        Row::new()
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
                            )
                            .child(Spacer::flex(1.0))
                            // Delete button
                            .child(
                                Container::new()
                                    .padding(Edges::xy(8.0, 4.0))
                                    .radius(5.0)
                                    .hover(colors::active().bg_lighter)
                                    .on_click(move || st_del.delete_selected())
                                    .child(
                                        Text::new("Delete")
                                            .size(11.0)
                                            .color(colors::active().danger),
                                    ),
                            )
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
                                            .size(11.0)
                                            .color(colors::active().icon_active),
                                    ),
                            ),
                    )
                    // canvas
                    .child(GraphCanvas {
                        st: self.graph.clone(),
                    }),
            )
    }
}

/// Creates a ready-to-open WindowSpec
pub(crate) fn node_graph_spec(editor: EditorState) -> sequoia_ui::prelude::WindowSpec {
    let graph = NodeGraphState::demo();
    WindowSpec::new("Node Graph", NodeGraphView { editor, graph }).size(1100.0, 720.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sequoia_ui::core::{Point, RenderItem, RenderList};
    use sequoia_ui::input::{Modifiers, PointerButton};
    use sequoia_ui::text::TextSystem;
    use sequoia_ui::view::{ElementId, Runtime};

    #[test]
    fn demo_graph_has_visible_content() {
        let graph = NodeGraphState::demo();
        assert_eq!(graph.nodes.get_untracked().len(), 3);
        assert_eq!(graph.connections.get_untracked().len(), 2);

        let rt = Runtime::new();
        rt.set_root_fill(true);
        rt.mount(NodeGraphView {
            editor: EditorState::demo(),
            graph,
        });
        let mut text = TextSystem::new();
        rt.layout(Rect::from_xywh(0.0, 0.0, 1100.0, 720.0), &mut text);
        let mut list = RenderList::new();
        rt.paint(&mut list, &mut text);

        assert!(rt.errors().is_empty(), "{:?}", rt.errors());
        assert!(!list.is_empty(), "node graph rendered nothing");

        let root = rt.root().expect("root");
        let canvas = find_type(&rt, root, "GraphCanvas").expect("GraphCanvas");
        let rect = rt.rect_of(canvas).expect("canvas rect");
        assert!(
            rect.size.height > 100.0,
            "GraphCanvas has collapsed bounds: {rect:?}"
        );

        let drawn = list
            .items()
            .iter()
            .filter(|item| match item {
                RenderItem::Quad(quad) => quad.rect.top() >= 36.0,
                _ => false,
            })
            .count();
        assert!(drawn > 100, "graph area only drew {drawn} quads");
    }

    fn mounted() -> (Runtime, NodeGraphState) {
        let graph = NodeGraphState::demo();
        let probe = graph.clone();
        let rt = Runtime::new();
        rt.set_root_fill(true);
        rt.mount(NodeGraphView {
            editor: EditorState::demo(),
            graph,
        });
        let mut text = TextSystem::new();
        rt.layout(Rect::from_xywh(0.0, 0.0, 1100.0, 720.0), &mut text);
        (rt, probe)
    }

    #[test]
    fn drag_from_output_to_input_connects() {
        let (rt, graph) = mounted();
        let mut text = TextSystem::new();

        rt.pointer_down(
            Point::new(260.0, 222.0),
            PointerButton::Left,
            Modifiers::NONE,
            &mut text,
        );
        rt.pointer_move(Point::new(660.0, 242.0), &mut text);
        rt.pointer_up(
            Point::new(660.0, 242.0),
            PointerButton::Left,
            Modifiers::NONE,
            &mut text,
        );

        let conns = graph.connections.get_untracked();
        assert_eq!(conns.len(), 3, "{conns:?}");
        assert!(
            conns.iter().any(|c| {
                c.from_node == 1 && c.from_port == 1 && c.to_node == 3 && c.to_port == 1
            })
        );
    }

    #[test]
    fn drag_from_input_to_output_connects_backwards() {
        let (rt, graph) = mounted();
        let mut text = TextSystem::new();

        rt.pointer_down(
            Point::new(660.0, 242.0),
            PointerButton::Left,
            Modifiers::NONE,
            &mut text,
        );
        rt.pointer_move(Point::new(260.0, 222.0), &mut text);
        rt.pointer_up(
            Point::new(260.0, 222.0),
            PointerButton::Left,
            Modifiers::NONE,
            &mut text,
        );

        let conns = graph.connections.get_untracked();
        assert_eq!(conns.len(), 3, "{conns:?}");
        assert!(
            conns.iter().any(|c| {
                c.from_node == 1 && c.from_port == 1 && c.to_node == 3 && c.to_port == 1
            })
        );
    }

    #[test]
    fn dragging_empty_canvas_pans_immediately() {
        let (rt, graph) = mounted();
        let mut text = TextSystem::new();

        rt.pointer_down(
            Point::new(900.0, 600.0),
            PointerButton::Left,
            Modifiers::NONE,
            &mut text,
        );
        rt.pointer_move(Point::new(950.0, 630.0), &mut text);

        assert_eq!(graph.pan_x.get_untracked(), -50.0);
        assert_eq!(graph.pan_y.get_untracked(), -30.0);
    }

    fn find_type(rt: &Runtime, id: ElementId, suffix: &str) -> Option<ElementId> {
        if rt
            .type_name_of(id)
            .is_some_and(|name| name.ends_with(suffix))
        {
            return Some(id);
        }
        for child in rt.children(id) {
            if let Some(found) = find_type(rt, child, suffix) {
                return Some(found);
            }
        }
        None
    }
}
