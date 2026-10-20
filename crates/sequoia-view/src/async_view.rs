use std::rc::Rc;

use sequoia_async::{AsyncState, Resource};
use sequoia_input::{CursorIcon, Event, PointerButton, PointerEvent};
use sequoia_layout::LayoutStyle;
use sequoia_style::VisualState;

use crate::any::AnyView;
use crate::view::{EventCtx, View};
use crate::views::{Column, Text, Unit};

type BuildFn = Rc<dyn Fn() -> AnyView>;

pub struct AsyncView<T: Clone + PartialEq + 'static> {
    resource: Resource<T>,
    loading: BuildFn,
    data: Rc<dyn Fn(&T) -> AnyView>,
    error: Rc<dyn Fn(&str) -> AnyView>,
    idle: Option<BuildFn>,
    cancelled: Option<BuildFn>,
    retry_label: Option<String>,
    on_retry: Option<Rc<dyn Fn()>>,
    progress: Option<sequoia_reactive::Signal<f32>>,
}

impl<T: Clone + PartialEq + 'static> Clone for AsyncView<T> {
    fn clone(&self) -> Self {
        Self {
            resource: self.resource.clone(),
            loading: self.loading.clone(),
            data: self.data.clone(),
            error: self.error.clone(),
            idle: self.idle.clone(),
            cancelled: self.cancelled.clone(),
            retry_label: self.retry_label.clone(),
            on_retry: self.on_retry.clone(),
            progress: self.progress.clone(),
        }
    }
}

impl<T: Clone + PartialEq + 'static> AsyncView<T> {
    pub fn new(resource: Resource<T>) -> Self {
        Self {
            resource,
            loading: Rc::new(|| AnyView::new(Text::new("Landing…").size(12.0))),
            data: Rc::new(|_| AnyView::new(Unit)),
            error: Rc::new(|e: &str| AnyView::new(Text::new(format!("Error: {e}")).size(12.0))),
            idle: None,
            cancelled: None,
            retry_label: None,
            on_retry: None,
            progress: None,
        }
    }

    pub fn loading<V: View + 'static>(mut self, f: impl Fn() -> V + 'static) -> Self {
        self.loading = Rc::new(move || AnyView::new(f()));
        self
    }

    pub fn data<V: View + 'static>(mut self, f: impl Fn(&T) -> V + 'static) -> Self {
        self.data = Rc::new(move |v| AnyView::new(f(v)));
        self
    }

    pub fn error<V: View + 'static>(mut self, f: impl Fn(&str) -> V + 'static) -> Self {
        self.error = Rc::new(move |e| AnyView::new(f(e)));
        self
    }

    pub fn idle<V: View + 'static>(mut self, f: impl Fn() -> V + 'static) -> Self {
        self.idle = Some(Rc::new(move || AnyView::new(f())));
        self
    }

    pub fn cancelled<V: View + 'static>(mut self, f: impl Fn() -> V + 'static) -> Self {
        self.cancelled = Some(Rc::new(move || AnyView::new(f())));
        self
    }

    pub fn retry(mut self, label: impl Into<String>, on_retry: impl Fn() + 'static) -> Self {
        self.retry_label = Some(label.into());
        self.on_retry = Some(Rc::new(on_retry));
        self
    }

    pub fn progress(mut self, fraction: sequoia_reactive::Signal<f32>) -> Self {
        self.progress = Some(fraction);
        self
    }

    pub fn resource(&self) -> Resource<T> {
        self.resource.clone()
    }

    fn loading_with_progress(&self) -> AnyView {
        let body = (self.loading)();
        match &self.progress {
            Some(frac) => {
                let bar = BarView { frac: frac.clone() };
                AnyView::new(Column::new().fill_size().gap(8.0).child(bar).child(body))
            }
            None => body,
        }
    }
}

impl<T: Clone + PartialEq + 'static> View for AsyncView<T> {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn children_override(&self) -> Option<Vec<AnyView>> {
        let state = self.resource.state().get();
        let view = match &state {
            AsyncState::Idle => match &self.idle {
                Some(f) => f(),
                None => self.loading_with_progress(),
            },
            AsyncState::Loading => self.loading_with_progress(),
            AsyncState::Ready(v) => (self.data)(v),
            AsyncState::Failed(e) => {
                let err = (self.error)(e);
                match (&self.retry_label, &self.on_retry) {
                    (Some(label), Some(cb)) => {
                        let btn = RetryBtn {
                            label: label.clone(),
                            on_retry: cb.clone(),
                        };
                        AnyView::new(Column::new().gap(8.0).child(err).child(btn))
                    }
                    _ => err,
                }
            }
            AsyncState::Cancelled => match &self.cancelled {
                Some(f) => f(),
                None => match &self.idle {
                    Some(f) => f(),
                    None => self.loading_with_progress(),
                },
            },
            _ => self.loading_with_progress(),
        };
        Some(vec![view])
    }

    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle::default()
    }
}

#[derive(Clone)]
struct BarView {
    frac: sequoia_reactive::Signal<f32>,
}

impl View for BarView {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: sequoia_core::Length::Fill(1.0),
            height: sequoia_core::Length::Px(6.0),
            ..LayoutStyle::default()
        }
    }

    fn paint_self(&self, ctx: &mut crate::view::PaintCtx) {
        let _ = self.frac.get_untracked();
        let theme = ctx.theme();
        let r = ctx.rect;
        ctx.quad(r, theme.surface, r.size.height * 0.5);
        let w = r.size.width * self.frac.get_untracked().clamp(0.0, 1.0);
        if w > 0.5 {
            ctx.quad(
                sequoia_core::Rect::from_xywh(r.left(), r.top(), w, r.size.height),
                theme.accent,
                r.size.height * 0.5,
            );
        }
    }
}

#[derive(Clone)]
struct RetryBtn {
    label: String,
    on_retry: Rc<dyn Fn()>,
}

impl View for RetryBtn {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn children_override(&self) -> Option<Vec<AnyView>> {
        Some(vec![AnyView::new(
            Text::new(format!("↻ {}", self.label)).size(12.0).bold(),
        )])
    }

    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle::default()
    }

    fn has_visual_state(&self) -> bool {
        true
    }

    fn cursor_icon(&self, _state: VisualState) -> CursorIcon {
        CursorIcon::Pointer
    }

    fn on_event(&self, event: &Event, ctx: &mut EventCtx) {
        match event {
            Event::Pointer(PointerEvent::Up {
                button: PointerButton::Left,
                pos,
                ..
            }) if ctx.rect.contains(*pos) => {
                (self.on_retry)();
                ctx.handled();
            }
            _ => {}
        }
    }
}
