use std::marker::PhantomData;
use std::rc::Rc;

use sequoia_core::{Edges, Length};
use sequoia_layout::LayoutStyle;
use sequoia_reactive::Signal;
use sequoia_view::{AnyView, DragPayload, DropPosition, EventCtx, PaintCtx, View};

pub struct Droppable<V, T> {
    child: AnyView,
    on_drop: Rc<dyn Fn(T, DropPosition)>,
    on_over: Option<Rc<dyn Fn(DropPosition)>>,
    highlight: Option<Signal<bool>>,
    style: LayoutStyle,
    _view: PhantomData<(V, T)>,
}

impl<V: View + 'static, T: Clone + 'static> Droppable<V, T> {
    pub fn new(child: V, on_drop: impl Fn(T, DropPosition) + 'static) -> Self {
        Self {
            child: AnyView::new(child),
            on_drop: Rc::new(on_drop),
            on_over: None,
            highlight: None,
            style: LayoutStyle::default(),
            _view: PhantomData,
        }
    }

    pub fn on_over(mut self, f: impl Fn(DropPosition) + 'static) -> Self {
        self.on_over = Some(Rc::new(f));
        self
    }

    pub fn highlight(mut self, signal: Signal<bool>) -> Self {
        self.highlight = Some(signal);
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
}

impl<V: View + 'static, T: Clone + 'static> View for Droppable<V, T> {
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

    fn accepts_drop(&self, payload: &DragPayload) -> bool {
        payload.is::<T>()
    }

    fn on_drag_over(&self, pos: DropPosition, _ctx: &mut EventCtx) {
        if let Some(f) = &self.on_over {
            f(pos);
        }
    }

    fn on_drop(&self, pos: DropPosition, ctx: &mut EventCtx) {
        let Some(value) = ctx
            .drag_session()
            .and_then(|s| s.payload.get::<T>().cloned())
        else {
            return;
        };
        (self.on_drop)(value, pos);
        ctx.handled();
    }

    fn paint_self(&self, ctx: &mut PaintCtx) {
        if let Some(sig) = &self.highlight {
            let over = ctx.is_drag_target();
            if sig.get_untracked() != over {
                sig.set(over);
            }
        }
    }
}
