use std::cell::RefCell;
use std::time::Duration;

use crate::history::History;
use crate::runtime::{batch, transaction};
use crate::signal::Signal;

pub const DEFAULT_COALESCE_WINDOW: Duration = Duration::from_millis(800);

pub struct UndoSignal<T> {
    signal: Signal<T>,
    history: RefCell<History<T>>,
    coalesce_window: Duration,
}

impl<T: Clone + PartialEq + 'static> UndoSignal<T> {
    pub fn new(value: T, limit: usize) -> Self {
        Self {
            signal: Signal::new(value),
            history: RefCell::new(History::new(limit)),
            coalesce_window: DEFAULT_COALESCE_WINDOW,
        }
    }

    pub fn with_coalesce_window(mut self, window: Duration) -> Self {
        self.coalesce_window = window;
        self
    }

    pub fn signal(&self) -> Signal<T> {
        self.signal.clone()
    }

    pub fn get(&self) -> T {
        self.signal.get()
    }

    pub fn get_untracked(&self) -> T {
        self.signal.get_untracked()
    }

    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        self.signal.with(f)
    }

    pub fn with_untracked<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        self.signal.with_untracked(f)
    }

    pub fn set(&self, value: T) {
        let old = self.signal.get_untracked();
        if old == value {
            return;
        }
        self.history.borrow_mut().checkpoint(old);
        batch(|| self.signal.set(value));
    }

    pub fn update(&self, f: impl FnOnce(&mut T)) {
        self.transact_impl(None, None, f);
    }

    pub fn transact_named(&self, tag: &str, f: impl FnOnce(&mut T)) {
        self.transact_impl(Some(tag), None, f);
    }

    pub fn transact_coalesced(&self, tag: &str, f: impl FnOnce(&mut T)) {
        let window = self.coalesce_window;
        self.transact_impl(Some(tag), Some(window), f);
    }

    pub fn transact(&self, tag: &str, window: Duration, f: impl FnOnce(&mut T)) {
        self.transact_impl(Some(tag), Some(window), f);
    }

    fn transact_impl(&self, tag: Option<&str>, window: Option<Duration>, f: impl FnOnce(&mut T)) {
        let old = self.signal.get_untracked();
        let run = || {
            batch(|| self.signal.update(f));
        };

        match window {
            Some(w) => self.history.borrow_mut().checkpoint_coalesced(old, w),
            None => self.history.borrow_mut().checkpoint(old),
        }
        match tag {
            Some(t) => transaction(t, run),
            None => run(),
        }
    }

    pub fn undo(&self) -> bool {
        let current = self.signal.get_untracked();
        let prev = self.history.borrow_mut().undo(current);
        match prev {
            Some(v) => {
                batch(|| self.signal.set(v));
                true
            }
            None => false,
        }
    }

    pub fn redo(&self) -> bool {
        let current = self.signal.get_untracked();
        let next = self.history.borrow_mut().redo(current);
        match next {
            Some(v) => {
                batch(|| self.signal.set(v));
                true
            }
            None => false,
        }
    }

    pub fn can_undo(&self) -> bool {
        self.history.borrow().can_undo()
    }

    pub fn can_redo(&self) -> bool {
        self.history.borrow().can_redo()
    }

    pub fn depths(&self) -> (usize, usize) {
        self.history.borrow().depths()
    }

    pub fn clear_history(&self) {
        self.history.borrow_mut().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_undo_redo_roundtrip() {
        let doc = UndoSignal::new("a".to_string(), 60);
        doc.set("b".to_string());
        assert!(doc.can_undo());
        assert!(doc.undo());
        assert_eq!(doc.get_untracked(), "a");
        assert!(doc.redo());
        assert_eq!(doc.get_untracked(), "b");
    }

    #[test]
    fn coalesced_typing_makes_single_step() {
        let doc = UndoSignal::new(String::new(), 60).with_coalesce_window(Duration::from_secs(60));
        doc.transact_coalesced("typing", |s| s.push('a'));
        doc.transact_coalesced("typing", |s| s.push('b'));
        doc.transact_coalesced("typing", |s| s.push('c'));
        assert_eq!(doc.get_untracked(), "abc");
        assert_eq!(doc.depths().0, 1, "szybkie pisanie = 1 wpis undo");
        assert!(doc.undo());
        assert_eq!(doc.get_untracked(), "");
    }

    #[test]
    fn limit_zero_disables_history() {
        let doc = UndoSignal::new(1, 0);
        doc.set(2);
        assert!(!doc.can_undo());
        assert_eq!(doc.get_untracked(), 2);
    }
}
