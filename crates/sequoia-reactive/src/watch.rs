use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use crate::runtime::{DepSlot, FrameGuard, Observer};

#[derive(Default)]
pub struct ReadSet {
    deps: Vec<DepSlot>,
}

impl ReadSet {
    pub fn empty() -> Self {
        Self { deps: Vec::new() }
    }

    /// Whether both sets depend on the same signals in the same order.
    ///
    /// Rebuilding a view normally reads the same signals again. Callers use
    /// this to skip unregistering and re-registering every observer, which is
    /// O(observers) per signal and makes one coarse update O(n^2).
    pub fn same_deps(&self, other: &ReadSet) -> bool {
        self.deps.len() == other.deps.len()
            && self
                .deps
                .iter()
                .zip(other.deps.iter())
                .all(|(a, b)| a.0 == b.0)
    }

    pub fn len(&self) -> usize {
        self.deps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.deps.is_empty()
    }

    pub fn union(mut self, other: ReadSet) -> Self {
        for slot in other.deps {
            if !self.deps.iter().any(|(id, _, _)| *id == slot.0) {
                self.deps.push(slot);
            }
        }
        self
    }

    /// Registers `watcher` as an observer of every dependency.
    pub fn bind(&self, watcher: &Watcher) {
        let w: Rc<dyn Observer> = watcher.node.clone();
        let weak = Rc::downgrade(&w);
        for (_, dep, _) in &self.deps {
            if let Some(d) = dep.upgrade() {
                d.register_observer(weak.clone());
            }
        }
    }

    pub fn unbind(&self, watcher: &Watcher) {
        let id = watcher.node.id();
        for (_, dep, _) in &self.deps {
            if let Some(d) = dep.upgrade() {
                d.unregister_observer(id);
            }
        }
    }
}

pub(crate) struct WatcherNode {
    callback: RefCell<Option<Box<dyn Fn()>>>,
    /// Set while the callback runs. A callback that writes a signal it watches
    /// would otherwise re-enter itself until the stack runs out.
    dispatching: Cell<bool>,
}

#[derive(Clone)]
pub struct Watcher {
    pub(crate) node: Rc<WatcherNode>,
}

impl Watcher {
    pub fn new(callback: impl Fn() + 'static) -> Self {
        let node = Rc::new(WatcherNode {
            callback: RefCell::new(Some(Box::new(callback))),
            dispatching: Cell::new(false),
        });
        Self { node }
    }
}

impl Observer for WatcherNode {
    fn id(&self) -> usize {
        self as *const _ as usize
    }

    fn mark_dirty(&self) -> bool {
        if self.dispatching.replace(true) {
            log::warn!(
                "sequoia-reactive: watcher wrote a signal it observes — \
                 skipping the re-entrant dispatch"
            );
            return false;
        }
        struct ResetGuard<'a>(&'a Cell<bool>);
        impl Drop for ResetGuard<'_> {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }
        let _guard = ResetGuard(&self.dispatching);
        let callback = self.callback.borrow();
        let Some(f) = callback.as_ref() else {
            return false;
        };
        // Panic isolation: one broken watcher must not stop the remaining
        // observers in `propagate` from being notified.
        if std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).is_err() {
            log::error!("sequoia-reactive: watcher callback panicked — propagation continues");
        }
        false
    }

    fn dependents(&self) -> Vec<Weak<dyn Observer>> {
        Vec::new()
    }
}

pub fn capture_reads<R>(f: impl FnOnce() -> R) -> (R, ReadSet) {
    let mut deps = Vec::new();
    let result = {
        let _frame = FrameGuard::new_capture(&mut deps);
        f()
    };
    (result, ReadSet { deps })
}
