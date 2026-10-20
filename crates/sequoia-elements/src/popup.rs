use sequoia_core::Edges;
use sequoia_reactive::Signal;
use sequoia_view::{EventCtx, OverlayEntry, OverlayId, PaintCtx};

#[derive(Clone)]
pub struct PopupState {
    open: Signal<bool>,
    entry: Signal<Option<OverlayId>>,
    margin: Edges,
}

impl PopupState {
    pub fn new(open: Signal<bool>) -> Self {
        Self {
            open,
            entry: Signal::new(None),
            margin: Edges::ZERO,
        }
    }

    pub fn is_open(&self) -> bool {
        self.open.get_untracked()
    }

    pub fn entry_id(&self) -> Option<OverlayId> {
        self.entry.get_untracked()
    }

    pub fn open(&self) {
        self.open.set(true);
    }

    pub fn close(&self) {
        self.open.set(false);
    }

    pub fn toggle(&self) {
        self.open.set(!self.open.get_untracked());
    }

    pub fn subscribe(&self) {
        let _ = self.open.get();
    }

    pub fn sync_show(&self, ctx: &mut PaintCtx, make: impl FnOnce() -> OverlayEntry) {
        if !self.open.get_untracked() {
            if let Some(id) = self.entry.get_untracked() {
                ctx.hide_overlay(id);
                self.entry.set(None);
            }
            return;
        }
        if let Some(id) = self.entry.get_untracked() {
            if !ctx.has_overlay(id) {
                self.entry.set(None);
                self.open.set(false);
            }
            return;
        }
        let id = ctx.show_overlay(make());
        self.entry.set(Some(id));
    }

    pub fn toggle_in(&self, ctx: &mut EventCtx) {
        if self.open.get_untracked() {
            self.close_in(ctx);
        } else {
            self.open();
        }
        ctx.request_focus();
        ctx.handled();
    }

    pub fn close_in(&self, ctx: &mut EventCtx) -> bool {
        let was = self.open.get_untracked();
        self.open.set(false);
        if let Some(id) = self.entry.get_untracked() {
            ctx.hide_overlay(id);
            self.entry.set(None);
        }
        if was {
            ctx.handled();
        }
        was
    }

    pub fn forget(&self) {
        self.entry.set(None);
    }

    pub fn margin(mut self, margin: impl Into<Edges>) -> Self {
        let m = margin.into();
        self.margin = Edges::only(
            sanitize_margin(m.left),
            sanitize_margin(m.top),
            sanitize_margin(m.right),
            sanitize_margin(m.bottom),
        );
        self
    }

    pub fn margin_xy(self, x: f32, y: f32) -> Self {
        self.margin(Edges::xy(x, y))
    }

    pub fn margin_x(mut self, x: f32) -> Self {
        let x = sanitize_margin(x);
        self.margin.left = x;
        self.margin.right = x;
        self
    }

    pub fn margin_y(mut self, y: f32) -> Self {
        let y = sanitize_margin(y);
        self.margin.top = y;
        self.margin.bottom = y;
        self
    }

    pub fn margin_left(mut self, v: f32) -> Self {
        self.margin.left = sanitize_margin(v);
        self
    }

    pub fn margin_right(mut self, v: f32) -> Self {
        self.margin.right = sanitize_margin(v);
        self
    }

    pub fn margin_top(mut self, v: f32) -> Self {
        self.margin.top = sanitize_margin(v);
        self
    }

    pub fn margin_bottom(mut self, v: f32) -> Self {
        self.margin.bottom = sanitize_margin(v);
        self
    }
}

fn sanitize_margin(v: f32) -> f32 {
    if v.is_finite() { v.max(0.0) } else { 0.0 }
}
