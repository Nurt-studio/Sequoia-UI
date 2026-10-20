#![forbid(unsafe_code)]

use std::cell::Cell;
use std::rc::Rc;

use sequoia_reactive::Signal;

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum AsyncState<T: Clone + PartialEq> {
    Idle,

    Loading,

    Ready(T),

    Failed(String),

    Cancelled,
}

impl<T: Clone + PartialEq> AsyncState<T> {
    pub fn is_loading(&self) -> bool {
        matches!(self, Self::Loading)
    }

    pub fn data(&self) -> Option<&T> {
        match self {
            Self::Ready(v) => Some(v),
            _ => None,
        }
    }

    pub fn error(&self) -> Option<&str> {
        match self {
            Self::Failed(e) => Some(e),
            _ => None,
        }
    }
}

#[derive(Clone)]
pub struct Resource<T: Clone + PartialEq + 'static> {
    state: Signal<AsyncState<T>>,
    generation: Rc<Cell<u64>>,
}

impl<T: Clone + PartialEq + 'static> Resource<T> {
    pub fn new() -> Self {
        Self {
            state: Signal::new(AsyncState::Idle),
            generation: Rc::new(Cell::new(0)),
        }
    }

    pub fn with_initial(initial: T) -> Self {
        Self {
            state: Signal::new(AsyncState::Ready(initial)),
            generation: Rc::new(Cell::new(0)),
        }
    }

    pub fn state(&self) -> Signal<AsyncState<T>> {
        self.state.clone()
    }

    pub fn generation(&self) -> u64 {
        self.generation.get()
    }

    pub fn begin_loading(&self) -> u64 {
        let next = self.generation.get() + 1;
        self.generation.set(next);
        self.state.set(AsyncState::Loading);
        next
    }

    pub fn complete(&self, generation: u64, value: T) {
        if generation != self.generation.get() {
            return;
        }
        self.state.set(AsyncState::Ready(value));
    }

    pub fn fail(&self, generation: u64, error: impl Into<String>) {
        if generation != self.generation.get() {
            return;
        }
        self.state.set(AsyncState::Failed(error.into()));
    }

    pub fn cancel(&self) {
        self.generation.set(self.generation.get() + 1);
        self.state.set(AsyncState::Cancelled);
    }

    pub fn reset(&self) {
        self.generation.set(self.generation.get() + 1);
        self.state.set(AsyncState::Idle);
    }

    pub fn is_loading(&self) -> bool {
        self.state.get_untracked().is_loading()
    }

    pub fn data_untracked(&self) -> Option<T> {
        match self.state.get_untracked() {
            AsyncState::Ready(v) => Some(v),
            _ => None,
        }
    }
}

impl<T: Clone + PartialEq + 'static> Default for Resource<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone + PartialEq + 'static> std::fmt::Debug for Resource<T>
where
    T: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Resource")
            .field("state", &self.state.get_untracked())
            .field("generation", &self.generation.get())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_generation_is_ignored() {
        let r: Resource<String> = Resource::new();
        let old = r.begin_loading();
        let fresh = r.begin_loading();
        r.complete(old, "stale".into());
        assert!(r.state.get_untracked().is_loading(), "stale ignored");
        r.complete(fresh, "fresh".into());
        assert_eq!(r.data_untracked().as_deref(), Some("fresh"));
    }

    #[test]
    fn cancel_invalidates_pending() {
        let r: Resource<u32> = Resource::new();
        let loading = r.begin_loading();
        r.cancel();
        assert_eq!(r.state.get_untracked(), AsyncState::Cancelled);
        r.complete(loading, 1);
        assert_eq!(r.state.get_untracked(), AsyncState::Cancelled);
    }

    #[test]
    fn fail_and_reset_flow() {
        let r: Resource<u32> = Resource::new();
        let loading = r.begin_loading();
        r.fail(loading, "boom");
        assert_eq!(r.state.get_untracked(), AsyncState::Failed("boom".into()));
        r.reset();
        assert_eq!(r.state.get_untracked(), AsyncState::Idle);
    }
}
