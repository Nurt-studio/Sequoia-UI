use std::rc::Rc;

use sequoia_layout::LayoutStyle;
use sequoia_reactive::Signal;

use crate::any::AnyView;
use crate::runtime::{ElementId, Runtime, RuntimeError};
use crate::view::View;
use crate::views::Unit;

#[derive(Clone)]
pub struct ErrorBoundary {
    child: Rc<dyn Fn() -> AnyView>,
    fallback: Rc<dyn Fn(&RuntimeError) -> AnyView>,
    on_error: Option<Rc<dyn Fn(&RuntimeError)>>,
    failed: Signal<Option<RuntimeError>>,
}

impl ErrorBoundary {
    pub fn new(child: impl Fn() -> AnyView + 'static) -> Self {
        Self {
            child: Rc::new(child),
            fallback: Rc::new(|e: &RuntimeError| {
                AnyView::new(crate::views::Text::new(format!(
                    "Something went wrong ({})",
                    e.message
                )))
            }),
            on_error: None,
            failed: Signal::new(None),
        }
    }

    pub fn wrapping<V: View + Clone + 'static>(child: V) -> Self {
        Self::new(move || AnyView::new(child.clone()))
    }

    pub fn fallback<V: View + 'static>(mut self, f: impl Fn(&RuntimeError) -> V + 'static) -> Self {
        self.fallback = Rc::new(move |e| AnyView::new(f(e)));
        self
    }

    pub fn on_error(mut self, f: impl Fn(&RuntimeError) + 'static) -> Self {
        self.on_error = Some(Rc::new(f));
        self
    }

    pub fn reset(&self) {
        self.failed.set(None);
    }

    pub fn is_failed(&self) -> bool {
        self.failed.with_untracked(|f| f.is_some())
    }

    pub(crate) fn notify(&self, rt: &Runtime, el: ElementId, err: &RuntimeError) -> bool {
        if self.failed.with_untracked(|f| f.is_some()) {
            return true;
        }
        self.failed.set(Some(err.clone()));
        if let Some(cb) = &self.on_error {
            cb(err);
        }
        rt.request_rebuild(el);
        true
    }
}

impl View for ErrorBoundary {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn children_override(&self) -> Option<Vec<AnyView>> {
        match self.failed.get() {
            Some(err) => Some(vec![(self.fallback)(&err)]),
            None => Some(vec![(self.child)()]),
        }
    }

    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle::default()
    }

    fn layout_style_ctx(&self, rt: &Runtime, el: ElementId) -> LayoutStyle {
        let _ = rt.errors_seq().get();
        if self.failed.with_untracked(|f| f.is_none())
            && let Some(err) = rt
                .errors()
                .into_iter()
                .rev()
                .find(|e| rt.is_descendant_of(e.element, el))
        {
            self.failed.set(Some(err.clone()));
            if let Some(cb) = &self.on_error {
                cb(&err);
            }
        }
        LayoutStyle::default()
    }
}
