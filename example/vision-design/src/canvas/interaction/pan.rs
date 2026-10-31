use sequoia_ui::prelude::*;

use super::CanvasSignals;

/// View panning (middle button or the "H" tool).
pub(crate) struct PanSession {
    pub(crate) start_screen_x: f32,
    pub(crate) start_screen_y: f32,
    pub(crate) start_pan_x: f32,
    pub(crate) start_pan_y: f32,
}

impl PanSession {
    pub(crate) fn apply(&self, signals: &CanvasSignals, pos: Point, zoom: f32) {
        signals
            .pan_x
            .set(self.start_pan_x - (pos.x - self.start_screen_x) / zoom);
        signals
            .pan_y
            .set(self.start_pan_y - (pos.y - self.start_screen_y) / zoom);
    }
}
