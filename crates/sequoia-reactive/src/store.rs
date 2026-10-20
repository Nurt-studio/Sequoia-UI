use std::cell::RefCell;
use std::rc::Rc;

use crate::{Signal, transaction};

pub(crate) const MAX_HISTORY: usize = 100;

struct History<T> {
    undo: Vec<T>,
    redo: Vec<T>,

    depth: usize,

    snapshot: Option<T>,
}

impl<T> Default for History<T> {
    fn default() -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
            depth: 0,
            snapshot: None,
        }
    }
}

fn push_bounded<T>(stack: &mut Vec<T>, value: T) {
    if stack.len() >= MAX_HISTORY {
        stack.remove(0);
    }
    stack.push(value);
}

pub struct Store<T> {
    signal: Signal<T>,
    history: Rc<RefCell<History<T>>>,
}

impl<T> Clone for Store<T> {
    fn clone(&self) -> Self {
        Self {
            signal: self.signal.clone(),
            history: Rc::clone(&self.history),
        }
    }
}

impl<T: 'static> Store<T> {
    pub fn new(value: T) -> Self {
        Self {
            signal: Signal::new(value),
            history: Rc::new(RefCell::new(History::default())),
        }
    }

    pub fn id(&self) -> usize {
        self.signal.id()
    }

    pub fn version(&self) -> u64 {
        self.signal.version()
    }

    pub fn can_undo(&self) -> bool {
        !self.history.borrow().undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.history.borrow().redo.is_empty()
    }

    pub fn clear_history(&self) {
        let mut h = self.history.borrow_mut();
        h.undo.clear();
        h.redo.clear();
    }
}

impl<T: Clone + PartialEq + 'static> Store<T> {
    pub fn get(&self) -> T {
        self.signal.get()
    }

    pub fn get_untracked(&self) -> T {
        self.signal.get_untracked()
    }

    pub fn set(&self, value: T) {
        if self.history.borrow().depth > 0 {
            self.signal.set(value);
            return;
        }
        let current = self.signal.get_untracked();
        if current == value {
            return;
        }
        {
            let mut h = self.history.borrow_mut();
            push_bounded(&mut h.undo, current);
            h.redo.clear();
        }
        self.signal.set(value);
    }

    pub fn update(&self, f: impl FnOnce(&mut T)) {
        let mut next = self.signal.get_untracked();
        f(&mut next);
        self.set(next);
    }

    pub fn transact<R>(&self, name: &str, f: impl FnOnce() -> R) -> R {
        self.begin_txn();
        struct Guard<'a, T: Clone + PartialEq + 'static> {
            store: &'a Store<T>,
        }
        impl<'a, T: Clone + PartialEq + 'static> Drop for Guard<'a, T> {
            fn drop(&mut self) {
                self.store.end_txn();
            }
        }
        let _guard = Guard { store: self };
        transaction(name, f)
    }

    fn begin_txn(&self) {
        let mut h = self.history.borrow_mut();
        if h.depth == 0 {
            h.snapshot = Some(self.signal.get_untracked());
        }
        h.depth += 1;
    }

    fn end_txn(&self) {
        let snapshot = {
            let mut h = self.history.borrow_mut();
            if h.depth == 0 {
                return;
            }
            h.depth -= 1;
            if h.depth > 0 {
                return;
            }
            h.snapshot.take()
        };
        if let Some(before) = snapshot
            && before != self.signal.get_untracked()
        {
            let mut h = self.history.borrow_mut();
            push_bounded(&mut h.undo, before);
            h.redo.clear();
        }
    }

    pub fn undo(&self) -> bool {
        let prev = self.history.borrow_mut().undo.pop();
        let Some(prev) = prev else {
            return false;
        };
        let current = self.signal.get_untracked();
        push_bounded(&mut self.history.borrow_mut().redo, current);
        self.signal.set(prev);
        true
    }

    pub fn redo(&self) -> bool {
        let next = self.history.borrow_mut().redo.pop();
        let Some(next) = next else {
            return false;
        };
        let current = self.signal.get_untracked();
        push_bounded(&mut self.history.borrow_mut().undo, current);
        self.signal.set(next);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::current_transaction;

    #[test]
    fn set_undo_redo_roundtrip() {
        let s = Store::new(0);
        assert!(!s.can_undo() && !s.can_redo());
        s.set(1);
        s.set(2);
        assert!(s.can_undo() && !s.can_redo());
        assert!(s.undo());
        assert_eq!(s.get_untracked(), 1);
        assert!(s.can_undo() && s.can_redo());
        assert!(s.undo());
        assert_eq!(s.get_untracked(), 0);
        assert!(!s.can_undo() && s.can_redo());
        assert!(!s.undo(), "empty history = false");
        assert!(s.redo());
        assert_eq!(s.get_untracked(), 1);
        assert!(s.redo());
        assert_eq!(s.get_untracked(), 2);
        assert!(!s.redo(), "empty redo = false");
    }

    #[test]
    fn equal_set_records_nothing() {
        let s = Store::new(1);
        let v0 = s.version();
        s.set(1);
        assert_eq!(s.version(), v0, "equal value = no-op");
        assert!(!s.can_undo(), "no-op records no step");
    }

    #[test]
    fn update_is_a_single_step() {
        let s = Store::new(vec![1]);
        s.update(|v| {
            v.push(2);
            v.push(3);
        });
        assert_eq!(s.get_untracked(), vec![1, 2, 3]);
        assert!(s.undo());
        assert_eq!(s.get_untracked(), vec![1]);
        assert!(!s.can_undo(), "update is a single step");
    }

    #[test]
    fn new_edit_clears_redo() {
        let s = Store::new(0);
        s.set(1);
        assert!(s.undo());
        assert!(s.can_redo());
        s.set(2);
        assert!(!s.can_redo(), "a new write clears redo");
        assert_eq!(s.get_untracked(), 2);
    }

    #[test]
    fn transact_groups_into_one_step() {
        let s = Store::new(0);
        s.transact("Move Nodes", || {
            assert_eq!(
                current_transaction().as_deref(),
                Some("Move Nodes"),
                "name visible in diagnostics"
            );
            s.set(1);
            s.set(2);
            s.update(|v| *v += 10);
        });
        assert_eq!(s.get_untracked(), 12);
        assert!(s.undo(), "a transaction undoes in one step");
        assert_eq!(s.get_untracked(), 0);
        assert!(!s.can_undo(), "the whole transaction is one step");
    }

    #[test]
    fn empty_transact_records_nothing() {
        let s = Store::new(0);
        s.transact("noop", || {
            s.set(0);
        });
        assert!(!s.can_undo());
    }

    #[test]
    fn history_is_capped() {
        let s = Store::new(0);
        for v in 1..=(MAX_HISTORY as i32 + 5) {
            s.set(v);
        }
        let mut undos = 0;
        while s.undo() {
            undos += 1;
        }
        assert_eq!(undos, MAX_HISTORY);
        assert_eq!(s.get_untracked(), 5);
    }

    #[test]
    fn clones_share_signal_and_history() {
        let s = Store::new(0);
        let c = s.clone();
        s.set(7);
        assert_eq!(c.get_untracked(), 7);
        assert!(c.undo(), "history shared by clones");
        assert_eq!(s.get_untracked(), 0);
    }
}
