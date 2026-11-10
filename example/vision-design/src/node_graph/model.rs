use sequoia_ui::prelude::*;

// ── dimensions ───────────────────────────────────────────
pub(crate) const NODE_W: f32 = 200.0;
pub(crate) const HEADER_H: f32 = 42.0;
pub(crate) const PORT_ROW_H: f32 = 28.0;
pub(crate) const PORT_R: f32 = 5.0;
/// Socket hit radius — a bit larger than the drawn dot so grabbing a port
/// does not require pixel-perfect aim.
pub(crate) const PORT_HIT_R: f32 = 9.0;
pub(crate) const NODE_PAD_B: f32 = 12.0;
pub(crate) const GRID_STEP: f32 = 24.0;

// ── types ────────────────────────────────────────────────
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum PortSide {
    Input,
    Output,
}

/// Kinds of the behavior graph (Phase 3). A rule renders as a **Trigger**
/// node, its steps as **Action** nodes, and every state variable as a
/// **State** node; edges are execution (trigger → action → action) and
/// data (state ⇄ action reads/writes).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(crate) enum NodeKind {
    Trigger,
    State,
    Action,
}

impl NodeKind {
    pub(crate) fn header_color(self) -> Color {
        match self {
            NodeKind::Trigger => Color::hex(0x2D6A4F),
            NodeKind::State => Color::hex(0x1B4965),
            NodeKind::Action => Color::hex(0x6A3D2D),
        }
    }
    pub(crate) fn accent(self) -> Color {
        match self {
            NodeKind::Trigger => Color::hex(0x42BE65),
            NodeKind::State => Color::hex(0x0D99FF),
            NodeKind::Action => Color::hex(0xF1C21B),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GraphNode {
    pub id: u64,
    /// Stable identity across rebuilds ("t:19:0", "a:19:0:1", "s:open") —
    /// dragged positions survive a reload through this key.
    pub key: String,
    pub title: String,
    /// Secondary line in the header (element name, type + initial…).
    pub subtitle: Option<String>,
    pub kind: NodeKind,
    pub x: f32,
    pub y: f32,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    /// Element in the document this node was built from — clicking the node
    /// selects it in the editor.
    pub element: Option<u64>,
    /// Chain distance from the trigger (actions only) — drives the column
    /// in the auto-layout.
    pub depth: usize,
    /// `true` when the node takes part in a dependency cycle (DAG check).
    pub in_cycle: bool,
}

impl GraphNode {
    pub(crate) fn height(&self) -> f32 {
        let rows = self.inputs.len().max(self.outputs.len()).max(1);
        HEADER_H + (rows as f32) * PORT_ROW_H + NODE_PAD_B
    }

    pub(crate) fn port_pos(&self, side: PortSide, index: usize) -> Point {
        let y = self.y + HEADER_H + PORT_ROW_H * (index as f32 + 0.5);
        match side {
            PortSide::Input => Point::new(self.x, y),
            PortSide::Output => Point::new(self.x + NODE_W, y),
        }
    }

    /// The socket under a world position, if any. Ports are checked before
    /// nodes in the pointer handler so grabbing a socket starts a wire drag
    /// instead of moving the node.
    pub(crate) fn port_at(&self, wx: f32, wy: f32) -> Option<(PortSide, usize)> {
        for side in [PortSide::Input, PortSide::Output] {
            let count = match side {
                PortSide::Input => self.inputs.len(),
                PortSide::Output => self.outputs.len(),
            };
            for i in 0..count {
                let p = self.port_pos(side, i);
                if (wx - p.x).abs() <= PORT_HIT_R && (wy - p.y).abs() <= PORT_HIT_R {
                    return Some((side, i));
                }
            }
        }
        None
    }

    pub(crate) fn contains(&self, wx: f32, wy: f32) -> bool {
        wx >= self.x && wx <= self.x + NODE_W && wy >= self.y && wy <= self.y + self.height()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GraphConnection {
    pub from_node: u64,
    pub from_port: usize,
    pub to_node: u64,
    pub to_port: usize,
}

impl GraphConnection {
    /// Execution edges (trigger/action → next action) vs data edges
    /// (state → action read, action → state write). Derived from the
    /// port indexes both sides agree on: port 0 is always execution.
    pub(crate) fn is_data(&self, nodes: &[GraphNode]) -> bool {
        let from_data = nodes
            .iter()
            .find(|n| n.id == self.from_node)
            .map(|n| n.kind != NodeKind::Trigger && self.from_port > 0)
            .unwrap_or(false);
        let to_data = nodes
            .iter()
            .find(|n| n.id == self.to_node)
            .map(|_n| self.to_port > 0)
            .unwrap_or(false);
        from_data || to_data
    }
}

// The graph is generated from the document (`build.rs`); the sockets are
// live: dragging from one to another rewires the underlying rules
// (execution order, variable reads/writes) and the view rebuilds from the
// document (design doc §4.4).
