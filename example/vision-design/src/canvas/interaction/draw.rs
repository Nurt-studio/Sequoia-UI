use sequoia_ui::prelude::*;

use super::CanvasSignals;
use crate::core::geometry::snap;
use crate::core::hit_test::find_parent_frame;
use crate::core::layer::{next_id, pick_hex_for, Layer};

/// Drawing a new shape (F/R/O) before committing.
pub(crate) struct DrawSession {
    pub(crate) tool: &'static str,
    pub(crate) start_wx: f32,
    pub(crate) start_wy: f32,
    pub(crate) cur_wx: f32,
    pub(crate) cur_wy: f32,
    layers_before: Vec<Layer>,
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
            layers_before: signals.layers.get_untracked(),
        }
    }

    pub(crate) fn update(&mut self, wx: f32, wy: f32, grid_snap: bool) {
        self.cur_wx = if grid_snap { snap(wx) } else { wx };
        self.cur_wy = if grid_snap { snap(wy) } else { wy };
    }

    /// Creates a layer from the drawing rect (if it is large enough).
    pub(crate) fn commit(self, signals: &CanvasSignals) {
        let x = self.start_wx.min(self.cur_wx);
        let y = self.start_wy.min(self.cur_wy);
        let w = (self.cur_wx - self.start_wx).abs();
        let h = (self.cur_wy - self.start_wy).abs();

        if w >= 4.0 && h >= 4.0 {
            let (kind, icon, name, radius) = match self.tool {
                "F" => ("FRAME", "tool/frame", "Frame", 0.0),
                "R" => ("RECTANGLE", "tool/rect", "Rectangle", 8.0),
                "O" => ("ELLIPSE", "tool/ellipse", "Ellipse", 60.0),
                _ => ("RECTANGLE", "tool/rect", "Shape", 0.0),
            };
            let (hex, fill) = pick_hex_for(kind);
            let layers = signals.layers.get_untracked();
            let id = next_id(&layers);
            let parent = if kind == "FRAME" {
                None
            } else {
                find_parent_frame(&layers, x, y)
            };
            let level = if parent.is_some() { 1 } else { 0 };
            let layer = Layer {
                id,
                name: name.into(),
                icon,
                kind,
                level,
                parent,
                visible: true,
                locked: false,
                component: false,
                instance: false,
                x,
                y,
                w,
                h,
                opacity: 100.0,
                radius,
                fill,
                fill_hex: hex.into(),
                stroke: Color::TRANSPARENT,
                stroke_hex: "#00000000".into(),
                stroke_width: 0.0,
                text: String::new(),
                font_size: 14.0,
                layout: "NONE",
                gap: 0.0,
                pad_l: 0.0,
                pad_t: 0.0,
                pad_r: 0.0,
                pad_b: 0.0,
                align_h: "start",
                align_v: "start",
                position_mode: "RELATIVE",
                pin_top: false,
                pin_right: false,
                pin_bottom: false,
                pin_left: false,
                self_pad_top: 0.0,
                self_pad_right: 0.0,
                self_pad_bottom: 0.0,
                self_pad_left: 0.0,
            };
            signals.layers.update(|ls| ls.push(layer));
            signals.selected.set(Some(id));
            signals.tool.set("V");
        }

        signals.state.push_snapshot(self.layers_before);
    }
}

/// Text tool click: inserts a TEXT layer and enters editing.
pub(crate) fn create_text(signals: &CanvasSignals, wx: f32, wy: f32, grid_snap: bool) {
    let layers = signals.layers.get_untracked();
    let id = next_id(&layers);
    let (hex, fill) = pick_hex_for("TEXT");
    let x = if grid_snap { snap(wx) } else { wx };
    let y = if grid_snap { snap(wy) } else { wy };
    let parent = find_parent_frame(&layers, x, y);
    let level = if parent.is_some() { 1 } else { 0 };

    signals.state.push_history();
    let layer = Layer {
        id,
        name: "Text".into(),
        icon: "tool/text",
        kind: "TEXT",
        level,
        parent,
        visible: true,
        locked: false,
        component: false,
        instance: false,
        x,
        y,
        w: 200.0,
        h: 32.0,
        opacity: 100.0,
        radius: 0.0,
        fill,
        fill_hex: hex.into(),
        stroke: Color::TRANSPARENT,
        stroke_hex: "#00000000".into(),
        stroke_width: 0.0,
        text: "Text".into(),
        font_size: 16.0,
        layout: "NONE",
        gap: 0.0,
        pad_l: 0.0,
        pad_t: 0.0,
        pad_r: 0.0,
        pad_b: 0.0,
        align_h: "start",
        align_v: "start",
        position_mode: "RELATIVE",
        pin_top: false,
        pin_right: false,
        pin_bottom: false,
        pin_left: false,
        self_pad_top: 0.0,
        self_pad_right: 0.0,
        self_pad_bottom: 0.0,
        self_pad_left: 0.0,
    };
    signals.layers.update(|ls| ls.push(layer));
    signals.selected.set(Some(id));
    signals.tool.set("V");
    signals.state.start_text_edit(false);
}
