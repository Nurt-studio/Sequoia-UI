use super::model::*;
use sequoia_ui::prelude::*;
use std::cell::RefCell;
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
    Connect {
        from_node: u64,
        from_side: PortSide,
        from_port: usize,
        cur_wx: f32,
        cur_wy: f32,
    },
    Pan {
        start_sx: f32,
        start_sy: f32,
        orig_pan_x: f32,
        orig_pan_y: f32,
    },
}

#[derive(Clone)]
pub(crate) struct NodeGraphState {
    pub nodes: Signal<Vec<GraphNode>>,
    pub connections: Signal<Vec<GraphConnection>>,
    pub selected_node: Signal<Option<u64>>,
    pub selected_conn: Signal<Option<usize>>,
    pub zoom: Signal<f32>,
    pub pan_x: Signal<f32>,
    pub pan_y: Signal<f32>,
    pub repaint: Signal<u64>,
    pub drag: Rc<RefCell<Option<GraphDrag>>>,
    pub next_id: Rc<RefCell<u64>>,
}

impl NodeGraphState {
    pub(crate) fn new() -> Self {
        Self {
            nodes: Signal::new(Vec::new()),
            connections: Signal::new(Vec::new()),
            selected_node: Signal::new(None),
            selected_conn: Signal::new(None),
            zoom: Signal::new(1.0),
            pan_x: Signal::new(0.0),
            pan_y: Signal::new(0.0),
            repaint: Signal::new(0),
            drag: Rc::new(RefCell::new(None)),
            next_id: Rc::new(RefCell::new(1)),
        }
    }

    /// Sample graph (Input → Mix → Output)
    pub(crate) fn demo() -> Self {
        let s = Self::new();
        *s.next_id.borrow_mut() = 4;

        s.nodes.set(vec![
            GraphNode {
                id: 1,
                title: "Texture Input".into(),
                kind: NodeKind::Input,
                x: 80.0,
                y: 120.0,
                inputs: vec![],
                outputs: vec!["RGB".into(), "Alpha".into()],
            },
            GraphNode {
                id: 2,
                title: "Mix".into(),
                kind: NodeKind::Process,
                x: 380.0,
                y: 100.0,
                inputs: vec!["A".into(), "B".into(), "Factor".into()],
                outputs: vec!["Result".into()],
            },
            GraphNode {
                id: 3,
                title: "Output".into(),
                kind: NodeKind::Output,
                x: 660.0,
                y: 140.0,
                inputs: vec!["Color".into(), "Opacity".into()],
                outputs: vec![],
            },
        ]);

        s.connections.set(vec![
            GraphConnection {
                from_node: 1,
                from_port: 0,
                to_node: 2,
                to_port: 0,
            },
            GraphConnection {
                from_node: 2,
                from_port: 0,
                to_node: 3,
                to_port: 0,
            },
        ]);

        s
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

    pub(crate) fn delete_selected(&self) {
        if let Some(conn_idx) = self.selected_conn.get_untracked() {
            self.connections.update(|cs| {
                if conn_idx < cs.len() {
                    cs.remove(conn_idx);
                }
            });
            self.selected_conn.set(None);
            self.bump();
            return;
        }
        if let Some(nid) = self.selected_node.get_untracked() {
            self.nodes.update(|ns| ns.retain(|n| n.id != nid));
            self.connections
                .update(|cs| cs.retain(|c| c.from_node != nid && c.to_node != nid));
            self.selected_node.set(None);
            self.bump();
        }
    }
}
