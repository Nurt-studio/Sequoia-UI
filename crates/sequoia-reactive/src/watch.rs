use std::cell::RefCell;
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
}

#[derive(Clone)]
pub struct Watcher {
    pub(crate) node: Rc<WatcherNode>,
}

impl Watcher {
    pub fn new(callback: impl Fn() + 'static) -> Self {
        let node = Rc::new(WatcherNode {
            callback: RefCell::new(Some(Box::new(callback))),
        });
        Self { node }
    }
}

impl Observer for WatcherNode {
    fn id(&self) -> usize {
        self as *const _ as usize
    }

    fn mark_dirty(&self) -> bool {
        if let Some(f) = self.callback.borrow().as_ref() {
            f();
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
