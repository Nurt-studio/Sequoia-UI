use super::CanvasSignals;

/// Rectangle selection over empty canvas space.
pub(crate) struct MarqueeSession {
    pub(crate) start_wx: f32,
    pub(crate) start_wy: f32,
    pub(crate) cur_wx: f32,
    pub(crate) cur_wy: f32,
}

impl MarqueeSession {
    pub(crate) fn begin(wx: f32, wy: f32) -> Self {
        Self {
            start_wx: wx,
            start_wy: wy,
            cur_wx: wx,
            cur_wy: wy,
        }
    }

    pub(crate) fn update(&mut self, wx: f32, wy: f32) {
        self.cur_wx = wx;
        self.cur_wy = wy;
    }

    /// Returns the first hit element (`None` if the rectangle is empty).
    pub(crate) fn commit(&self, signals: &CanvasSignals) -> Option<u64> {
        let x1 = self.start_wx.min(self.cur_wx);
        let y1 = self.start_wy.min(self.cur_wy);
        let x2 = self.start_wx.max(self.cur_wx);
        let y2 = self.start_wy.max(self.cur_wy);
        let layers = signals.layers.get_untracked();
        layers
            .iter()
            .filter(|l| l.visible && l.x < x2 && l.x + l.w > x1 && l.y < y2 && l.y + l.h > y1)
            .map(|l| l.id)
            .next()
    }
}