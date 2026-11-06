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

    /// Ids of every visible, unlocked layer fully inside the rectangle.
    ///
    /// The enclosing frame is skipped because it contains the marquee rather
    /// than lying inside it, so dragging over a frame's children does not add
    /// the frame itself to the selection.
    pub(crate) fn commit(&self, signals: &CanvasSignals) -> Vec<u64> {
        let x1 = self.start_wx.min(self.cur_wx);
        let y1 = self.start_wy.min(self.cur_wy);
        let x2 = self.start_wx.max(self.cur_wx);
        let y2 = self.start_wy.max(self.cur_wy);
        // Borrow instead of cloning the layer list: with thousands of layers
        // `get_untracked()` alone would copy every name and text string.
        signals.layers.with_untracked(|layers| {
            layers
                .iter()
                .filter(|l| {
                    l.visible
                        && !l.locked
                        && l.x >= x1
                        && l.y >= y1
                        && l.x + l.w <= x2
                        && l.y + l.h <= y2
                })
                .map(|l| l.id)
                .collect()
        })
    }
}
