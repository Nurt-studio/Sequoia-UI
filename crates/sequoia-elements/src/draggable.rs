use std::rc::Rc;

use sequoia_core::{Edges, Length};
use sequoia_input::{CursorIcon, Event, EventPhase, PointerButton, PointerEvent};
use sequoia_layout::LayoutStyle;
use sequoia_style::VisualState;
use sequoia_view::{AnyView, DragEnd, DragOptions, DragPayload, EventCtx, View};

pub struct Draggable<V, T> {
    child: AnyView,
    payload: T,
    threshold: f32,
    button: PointerButton,
    preview: Option<AnyView>,
    on_start: Option<Rc<dyn Fn()>>,
    on_end: Option<Rc<dyn Fn(DragEnd)>>,
    style: LayoutStyle,
    _view: std::marker::PhantomData<V>,
}

impl<V: View + 'static, T: Clone + 'static> Draggable<V, T> {
    pub fn new(child: V, payload: T) -> Self {
        Self {
            child: AnyView::new(child),
            payload,
            threshold: sequoia_view::DRAG_THRESHOLD,
            button: PointerButton::Left,
            preview: None,
            on_start: None,
            on_end: None,
            style: LayoutStyle::default(),
            _view: std::marker::PhantomData,
        }
    }

    pub fn threshold(mut self, px: f32) -> Self {
        self.threshold = px.max(0.0);
        self
    }

    pub fn button(mut self, button: PointerButton) -> Self {
        self.button = button;
        self
    }

    pub fn preview<W: View + 'static>(mut self, preview: W) -> Self {
        self.preview = Some(AnyView::new(preview));
        self
    }

    pub fn on_drag_start(mut self, f: impl Fn() + 'static) -> Self {
        self.on_start = Some(Rc::new(f));
        self
    }

    pub fn on_drag_end(mut self, f: impl Fn(DragEnd) + 'static) -> Self {
        self.on_end = Some(Rc::new(f));
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.style = self.style.width(width.into());
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.style = self.style.height(height.into());
        self
    }

    pub fn min_width(mut self, v: f32) -> Self {
        self.style = self.style.min_width(v);
        self
    }

    pub fn max_width(mut self, v: f32) -> Self {
        self.style = self.style.max_width(v);
        self
    }

    pub fn min_height(mut self, v: f32) -> Self {
        self.style = self.style.min_height(v);
        self
    }

    pub fn max_height(mut self, v: f32) -> Self {
        self.style = self.style.max_height(v);
        self
    }

    pub fn flex(mut self, weight: f32) -> Self {
        self.style = self.style.flex(weight);
        self
    }

    pub fn expand(self) -> Self {
        self.flex(1.0)
    }

    pub fn fill_width(mut self) -> Self {
        self.style = self.style.width(Length::Fill(1.0));
        self
    }

    pub fn fill_height(mut self) -> Self {
        self.style = self.style.height(Length::Fill(1.0));
        self
    }

    pub fn fill_size(self) -> Self {
        self.fill_width().fill_height()
    }

    pub fn margin(mut self, margin: impl Into<Edges>) -> Self {
        self.style = self.style.margin(margin);
        self
    }

    pub fn margin_xy(mut self, x: f32, y: f32) -> Self {
        self.style = self.style.margin_xy(x, y);
        self
    }

    pub fn margin_x(mut self, x: f32) -> Self {
        self.style = self.style.margin_x(x);
        self
    }

    pub fn margin_y(mut self, y: f32) -> Self {
        self.style = self.style.margin_y(y);
        self
    }

    pub fn margin_left(mut self, v: f32) -> Self {
        self.style = self.style.margin_left(v);
        self
    }

    pub fn margin_right(mut self, v: f32) -> Self {
        self.style = self.style.margin_right(v);
        self
    }

    pub fn margin_top(mut self, v: f32) -> Self {
        self.style = self.style.margin_top(v);
        self
    }

    pub fn margin_bottom(mut self, v: f32) -> Self {
        self.style = self.style.margin_bottom(v);
        self
    }

    pub fn drag_payload(&self) -> DragPayload {
        DragPayload::new(self.payload.clone())
    }
}

impl<V: View + 'static, T: Clone + 'static> View for Draggable<V, T> {
    fn body(&self) -> impl View + 'static {
        sequoia_view::Unit
    }

    fn children_override(&self) -> Option<Vec<AnyView>> {
        Some(vec![self.child.clone()])
    }

    fn layout_style(&self) -> LayoutStyle {
        self.style
    }

    fn has_visual_state(&self) -> bool {
        true
    }

    fn captures_pointer(&self) -> bool {
        true
    }

    fn cursor_icon(&self, state: VisualState) -> CursorIcon {
        if state.pressed {
            CursorIcon::Grabbing
        } else {
            CursorIcon::Grab
        }
    }

    fn on_event(&self, event: &Event, ctx: &mut EventCtx) {
        if ctx.phase == EventPhase::Bubble {
            return;
        }
        match event {
            Event::Pointer(PointerEvent::Down { button, pos, .. })
                if *button == self.button && ctx.rect.contains(*pos) =>
            {
                let mut opts = DragOptions::new().threshold(self.threshold);
                if let Some(pv) = self.preview.clone() {
                    opts = opts.preview(pv);
                }
                ctx.begin_drag(self.payload.clone(), opts);
            }
            _ => {}
        }
    }

    fn on_drag_start(&self, _ctx: &mut EventCtx) {
        if let Some(f) = &self.on_start {
            f();
        }
    }

    fn on_drag_end(&self, end: DragEnd, _ctx: &mut EventCtx) {
        if let Some(f) = &self.on_end {
            f(end);
        }
    }
}
