
use super::EditorState;
use crate::core::layer::Layer;
use crate::document::auto_layout::apply_auto_layout_frame;

impl EditorState {
    pub(crate) fn propagate_layout(&self) {
        let snapshot = self.layers.get_untracked();
        let frame_ids: Vec<u64> = snapshot
            .iter()
            .filter(|l| l.kind == "FRAME" && l.layout != "NONE")
            .map(|l| l.id)
            .collect();
        if frame_ids.is_empty() {
            return;
        }
        self.layers.update(|ls| {
            for fid in frame_ids {
                apply_auto_layout_frame(fid, ls);
            }
        });
    }

    pub(crate) fn wrap_in_auto_layout(&self) {
        let Some(id) = self.selected_layer.get_untracked() else {
            self.toast_warn("Nothing selected");
            return;
        };
        let Some(frame) = self
            .layers
            .get_untracked()
            .iter()
            .find(|l| l.id == id)
            .cloned()
        else {
            return;
        };
        if frame.kind != "FRAME" {
            self.toast_warn("Select a frame first");
            return;
        }

        self.push_history();
        self.layers.update(|ls| {
            if let Some(l) = ls.iter_mut().find(|l| l.id == id) {
                l.layout = "H";
                if l.gap == 0.0 {
                    l.gap = 12.0;
                }
                if l.pad_l == 0.0 {
                    l.pad_l = 16.0;
                    l.pad_r = 16.0;
                }
                if l.pad_t == 0.0 {
                    l.pad_t = 16.0;
                    l.pad_b = 16.0;
                }
            }
        });
        self.propagate_layout();
        self.toast("Wrapped in auto layout");
    }

    pub(crate) fn zoom_to_fit(&self) {
        let layers = self.layers.get_untracked();
        let visible: Vec<&Layer> = layers.iter().filter(|l| l.visible).collect();
        if visible.is_empty() {
            self.toast_warn("Nothing visible");
            return;
        }
        let mut x1 = f32::INFINITY;
        let mut y1 = f32::INFINITY;
        let mut x2 = f32::NEG_INFINITY;
        let mut y2 = f32::NEG_INFINITY;
        for l in &visible {
            x1 = x1.min(l.x);
            y1 = y1.min(l.y);
            x2 = x2.max(l.x + l.w);
            y2 = y2.max(l.y + l.h);
        }
        let w = (x2 - x1).max(1.0);
        let h = (y2 - y1).max(1.0);
        let vw = self.viewport_w.get_untracked();
        let vh = self.viewport_h.get_untracked();
        if vw <= 0.0 || vh <= 0.0 {
            return;
        }
        let pad = 80.0;
        let z = ((vw - pad) / w).min((vh - pad) / h).clamp(0.1, 4.0);
        let px = x1 - (vw / z - w) * 0.5;
        let py = y1 - (vh / z - h) * 0.5;
        self.zoom.set(z);
        self.pan_x.set(px);
        self.pan_y.set(py);
    }

    pub(crate) fn zoom_to_selection(&self) {
        let Some(id) = self.selected_layer.get_untracked() else {
            self.toast_warn("Nothing selected");
            return;
        };
        let Some(l) = self.layers.get_untracked().into_iter().find(|l| l.id == id) else {
            return;
        };
        let vw = self.viewport_w.get_untracked();
        let vh = self.viewport_h.get_untracked();
        if vw <= 0.0 || vh <= 0.0 {
            return;
        }
        let pad = 120.0;
        let z = ((vw - pad) / l.w).min((vh - pad) / l.h).clamp(0.1, 4.0);
        let cx = l.x + l.w * 0.5;
        let cy = l.y + l.h * 0.5;
        self.zoom.set(z);
        self.pan_x.set(cx - (vw / z) * 0.5);
        self.pan_y.set(cy - (vh / z) * 0.5);
    }

    pub(crate) fn reset_view(&self) {
        self.zoom.set(1.0);
        self.pan_x.set(0.0);
        self.pan_y.set(0.0);
    }

    pub(crate) fn toggle_grid(&self) {
        self.grid_on.update(|v| *v = !*v);
    }

    pub(crate) fn export_svg(&self) {
        let svg = crate::document::export::to_svg(&self.layers.get_untracked());
        match std::fs::write("export.svg", &svg) {
            Ok(_) => self.toast("Exported to export.svg"),
            Err(_) => self.toast_error("Failed to export SVG"),
        }
    }
}
