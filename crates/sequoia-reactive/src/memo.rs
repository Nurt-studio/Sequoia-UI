use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use crate::runtime::{
    DepSlot, Dependency, FlagGuard, FrameGuard, Observer, observers_in, propagate,
    register_observer_in, track_read, unregister_all, unregister_observer_in, untracked,
};

pub struct Memo<T> {
    inner: Rc<MemoNode<T>>,
}

impl<T> Clone for Memo<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Rc::clone(&self.inner),
        }
    }
}

struct MemoNode<T> {
    compute: RefCell<Option<Box<dyn Fn() -> T>>>,
    value: RefCell<Option<T>>,
    version: Cell<u64>,
    dirty: Cell<bool>,

    retry: Cell<bool>,
    computing: Cell<bool>,
    deps: RefCell<Vec<DepSlot>>,
    observers: RefCell<Vec<Weak<dyn Observer>>>,
    me: Weak<MemoNode<T>>,
}

pub fn memo<T: Clone + PartialEq + 'static>(compute: impl Fn() -> T + 'static) -> Memo<T> {
    Memo::new(compute)
}

impl<T: Clone + PartialEq + 'static> Memo<T> {
    pub fn new(compute: impl Fn() -> T + 'static) -> Self {
        Self {
            inner: Rc::new_cyclic(|weak| MemoNode {
                compute: RefCell::new(Some(Box::new(compute))),
                value: RefCell::new(None),
                version: Cell::new(0),
                dirty: Cell::new(true),
                retry: Cell::new(false),
                computing: Cell::new(false),
                deps: RefCell::new(Vec::new()),
                observers: RefCell::new(Vec::new()),
                me: weak.clone(),
            }),
        }
    }

    pub fn get(&self) -> T {
        self.try_get().unwrap_or_else(|| {
            panic!(
                "sequoia-reactive: memo has no value \
                 (compute panicked earlier, or a cycle on first computation?)"
            )
        })
    }

    pub fn try_get(&self) -> Option<T> {
        self.inner.ensure_fresh();
        track_read(&self.inner);
        self.inner.value.borrow().as_ref().cloned()
    }

    pub fn get_untracked(&self) -> T {
        untracked(|| self.get())
    }
}

impl<T: Clone + PartialEq + 'static> MemoNode<T> {
    fn ensure_fresh(&self) {
        if !self.dirty.get() {
            return;
        }
        if self.computing.get() {
            log::error!(
                "sequoia-reactive: reactive cycle detected \
                 (memo read while it is being computed) — keeping stale value"
            );
            return;
        }

        let has_value = self.value.borrow().is_some();

        let mut changed = self.retry.get() || !has_value;
        if !changed {
            let deps = self.deps.borrow();
            for (_, dep, version) in deps.iter() {
                match dep.upgrade() {
                    Some(d) => {
                        d.validate();
                        if d.version() != *version {
                            changed = true;
                            break;
                        }
                    }

                    None => {
                        changed = true;
                        break;
                    }
                }
            }
        }
        if !changed {
            self.dirty.set(false);
            return;
        }

        let Some(this) = self.me.upgrade() else {
            return;
        };
        self.computing.set(true);
        let _cycle_guard = FlagGuard::new(&self.computing);

        unregister_all(&self.deps, Observer::id(self));

        let mut slots: Vec<DepSlot> = Vec::new();
        let taken;
        let outcome;
        {
            let observer: Rc<dyn Observer> = this.clone();
            let _frame = FrameGuard::new(Some(Rc::downgrade(&observer)), &mut slots);

            taken = self.compute.borrow_mut().take();
            let Some(compute) = taken.as_ref() else {
                *self.compute.borrow_mut() = taken;
                return;
            };
            outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| (*compute)()));
        }

        let new_value = match outcome {
            Ok(v) => v,
            Err(payload) => {
                if self.compute.borrow().is_none() {
                    *self.compute.borrow_mut() = taken;
                }
                *self.deps.borrow_mut() = slots;
                self.retry.set(true);
                std::panic::resume_unwind(payload);
            }
        };

        if self.compute.borrow().is_none() {
            *self.compute.borrow_mut() = taken;
        }
        *self.deps.borrow_mut() = slots;
        self.dirty.set(false);
        self.retry.set(false);

        let mut value = self.value.borrow_mut();
        let output_changed = match value.as_ref() {
            Some(old) => old != &new_value,
            None => true,
        };
        if output_changed {
            *value = Some(new_value);
            drop(value);
            self.version.set(self.version.get() + 1);
            propagate(self);
        }
    }
}

impl<T: Clone + PartialEq + 'static> Observer for MemoNode<T> {
    fn id(&self) -> usize {
        self as *const _ as usize
    }

    fn mark_dirty(&self) -> bool {
        if self.computing.get() {
            log::error!(
                "sequoia-reactive: reactive cycle detected \
                 (memo dirtied while computing — compute writes to its own dependency)"
            );
            return false;
        }
        let was_dirty = self.dirty.replace(true);
        !was_dirty
    }

    fn dependents(&self) -> Vec<Weak<dyn Observer>> {
        observers_in(&self.observers)
    }
}

impl<T: Clone + PartialEq + 'static> Dependency for MemoNode<T> {
    fn id(&self) -> usize {
        self as *const _ as usize
    }

    fn version(&self) -> u64 {
        self.version.get()
    }

    fn validate(&self) {
        self.ensure_fresh();
    }

    fn observers(&self) -> Vec<Weak<dyn Observer>> {
        observers_in(&self.observers)
    }

    fn register_observer(&self, observer: Weak<dyn Observer>) {
        register_observer_in(&self.observers, observer);
    }

    fn unregister_observer(&self, observer_id: usize) {
        unregister_observer_in(&self.observers, observer_id);
    }
}
