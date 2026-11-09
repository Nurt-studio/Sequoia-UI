use super::CanvasSignals;
use crate::core::geometry::snap;
use crate::core::hit_test::find_parent_frame;
use crate::model::svelte_ast::{SvelteNode, new_ellipse, new_frame, new_rect, new_text};

/// Drawing a new shape (F/R/O) before committing.
pub(crate) struct DrawSession {
    pub(crate) tool: &'static str,
    pub(crate) start_wx: f32,
    pub(crate) start_wy: f32,
    pub(crate) cur_wx: f32,
    pub(crate) cur_wy: f32,
    pub(crate) before: Option<SvelteNode>,
}

impl DrawSession {
    pub(crate) fn begin(
        signals: &CanvasSignals,
        tool: &'static str,
        wx: f32,
        wy: f32,
        grid_snap: bool,
    ) -> Self {
        let sx = if grid_snap { snap(wx) } else { wx };
        let sy = if grid_snap { snap(wy) } else { wy };
        Self {
            tool,
            start_wx: sx,
            start_wy: sy,
            cur_wx: sx,
            cur_wy: sy,
            before: signals.state.root.get_untracked().clone(),
        }
    }

    pub(crate) fn update(&mut self, wx: f32, wy: f32, grid_snap: bool) {
        self.cur_wx = if grid_snap { snap(wx) } else { wx };
        self.cur_wy = if grid_snap { snap(wy) } else { wy };
    }

    /// Creates a node from the drawing rect (if it is large enough).
    pub(crate) fn commit(mut self, signals: &CanvasSignals) {
        let x = self.start_wx.min(self.cur_wx);
        let y = self.start_wy.min(self.cur_wy);
        let mut w = (self.cur_wx - self.start_wx).abs();
        let mut h = (self.cur_wy - self.start_wy).abs();

        // A plain click (no real drag) still has to produce something —
        // otherwise "I cannot create shapes". Like Figma: 100×100 at the
        // click point.
        if w < 4.0 && h < 4.0 {
            w = 100.0;
            h = 100.0;
        }

        if w >= 4.0 && h >= 4.0 {
            let (kind, hex) = match self.tool {
                "F" => ("FRAME", 0x1B1B1B),
                "O" => ("ELLIPSE", 0x42BE65),
                _ => ("RECTANGLE", 0x0D99FF),
            };
            let id = signals.state.next_node_id();
            let node = if kind == "FRAME" {
                new_frame(id, x, y, w, h)
            } else if kind == "ELLIPSE" {
                new_ellipse(id, x, y, w, h, hex)
            } else {
                new_rect(id, x, y, w, h, hex)
            };
            let target = if kind == "FRAME" {
                0
            } else {
                find_parent_frame(&signals.layers.get_untracked(), x, y).unwrap_or(0)
            };
            signals.state.push_history();
            signals.state.root.update(|r| {
                let _ = crate::model::tree_ops::insert_child(r, target, node);
            });
            signals.selected.set(Some(id));
            signals.tool.set("V");
        }

        signals.state.push_snapshot(self.before.take());
    }
}

/// Text tool click: inserts an empty TEXT node and edits it in place, so the
/// first keystroke becomes the content.
pub(crate) fn create_text(signals: &CanvasSignals, wx: f32, wy: f32, grid_snap: bool) {
    let x = if grid_snap { snap(wx) } else { wx };
    let y = if grid_snap { snap(wy) } else { wy };
    let id = signals.state.next_node_id();
    let mut node = new_text(id, x, y, "");
    if let SvelteNode::Text(t) = &mut node {
        t.absolute = true;
    }
    let target = find_parent_frame(&signals.layers.get_untracked(), x, y).unwrap_or(0);
    signals.state.push_history();
    signals.state.root.update(|r| {
        let _ = crate::model::tree_ops::insert_child(r, target, node);
    });
    signals.selected.set(Some(id));
    signals.tool.set("V");
    signals.state.start_text_edit(false);
}
