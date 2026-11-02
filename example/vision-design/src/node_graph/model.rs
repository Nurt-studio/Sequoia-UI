use sequoia_ui::prelude::*;

// ── dimensions ───────────────────────────────────────────
pub(crate) const NODE_W: f32 = 180.0;
pub(crate) const HEADER_H: f32 = 30.0;
pub(crate) const PORT_ROW_H: f32 = 24.0;
pub(crate) const PORT_R: f32 = 5.0;
pub(crate) const PORT_HIT_R: f32 = 10.0;
pub(crate) const NODE_PAD_B: f32 = 10.0;
pub(crate) const GRID_STEP: f32 = 24.0;

// ── types ────────────────────────────────────────────────
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum PortSide {
    Input,
    Output,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum NodeKind {
    Input,
    Process,
    Output,
}

impl NodeKind {
    pub(crate) fn header_color(self) -> Color {
        match self {
            NodeKind::Input => Color::hex(0x2D6A4F),
            NodeKind::Process => Color::hex(0x1B4965),
            NodeKind::Output => Color::hex(0x6A3D2D),
        }
    }
    pub(crate) fn accent(self) -> Color {
        match self {
            NodeKind::Input => Color::hex(0x42BE65),
            NodeKind::Process => Color::hex(0x0D99FF),
            NodeKind::Output => Color::hex(0xF1C21B),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GraphNode {
    pub id: u64,
    pub title: String,
    pub kind: NodeKind,
    pub x: f32,
    pub y: f32,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
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

// ── port geometry ────────────────────────────────────────
pub(crate) fn find_port(nodes: &[GraphNode], wx: f32, wy: f32) -> Option<(u64, PortSide, usize)> {
    for node in nodes.iter().rev() {
        for (i, _) in node.inputs.iter().enumerate() {
            let p = node.port_pos(PortSide::Input, i);
            if (wx - p.x).abs() < PORT_HIT_R && (wy - p.y).abs() < PORT_HIT_R {
                return Some((node.id, PortSide::Input, i));
            }
        }
        for (i, _) in node.outputs.iter().enumerate() {
            let p = node.port_pos(PortSide::Output, i);
            if (wx - p.x).abs() < PORT_HIT_R && (wy - p.y).abs() < PORT_HIT_R {
                return Some((node.id, PortSide::Output, i));
            }
        }
    }
    None
}