use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use crate::runtime::{
    DepSlot, FrameGuard, Observer, batch_depth, enqueue_effect, flush_effects, unregister_all,
};

pub fn effect(run: impl Fn() + 'static) -> Effect {
    let handle = Effect::new(run);
    if batch_depth() == 0 {
        flush_effects();
    }
    handle
}

#[must_use = "effect lives while the handle lives; use `.forget()` for an app-lifetime effect"]
pub struct Effect {
    pub(crate) inner: Rc<EffectNode>,
}

pub(crate) struct EffectNode {
    run: RefCell<Option<Box<dyn Fn()>>>,
    deps: RefCell<Vec<DepSlot>>,
    dirty: Cell<bool>,
    queued: Cell<bool>,
    disposed: Cell<bool>,
    me: Weak<EffectNode>,
}

impl Effect {
    pub fn new(run: impl Fn() + 'static) -> Self {
        let inner = Rc::new_cyclic(|weak: &Weak<EffectNode>| EffectNode {
            run: RefCell::new(Some(Box::new(run))),
            deps: RefCell::new(Vec::new()),
            dirty: Cell::new(false),
            queued: Cell::new(false),
            disposed: Cell::new(false),
            me: weak.clone(),
        });
        inner.queued.set(true);
        enqueue_effect(&Rc::downgrade(&inner));
        Self { inner }
    }

    pub fn run_now(&self) {
        flush_effects();
    }

    pub fn dispose(self) {
        self.inner.dispose();
    }

    pub fn forget(self) {
        std::mem::forget(self);
    }
}

impl Drop for Effect {
    fn drop(&mut self) {
        self.inner.dispose();
    }
}

impl EffectNode {
    fn dispose(&self) {
        if self.disposed.replace(true) {
            return;
        }
        unregister_all(&self.deps, self.id());
        self.run.borrow_mut().take();
    }

    pub(crate) fn clear_queued(&self) {
        self.queued.set(false);
    }

    pub(crate) fn is_disposed(&self) -> bool {
        self.disposed.get()
    }

    pub(crate) fn run(self: &Rc<Self>) {
        self.dirty.set(false);
        unregister_all(&self.deps, self.id());

        let taken = self.run.borrow_mut().take();
        let Some(f) = taken.as_ref() else {
            *self.run.borrow_mut() = taken;
            return;
        };
        let mut slots: Vec<DepSlot> = Vec::new();
        let outcome = {
            let observer: Rc<dyn Observer> = self.clone();
            let _frame = FrameGuard::new(Some(Rc::downgrade(&observer)), &mut slots);
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(f))
        };
        if let Err(payload) = outcome {
            if self.disposed.get() {
                return;
            }
            if self.run.borrow().is_none() {
                *self.run.borrow_mut() = taken;
            }
            *self.deps.borrow_mut() = slots;
            std::panic::resume_unwind(payload);
        }
        if self.disposed.get() {
            return;
        }

        if self.run.borrow().is_none() {
            *self.run.borrow_mut() = taken;
        }
        *self.deps.borrow_mut() = slots;
    }
}

impl Observer for EffectNode {
    fn id(&self) -> usize {
        self as *const _ as usize
    }

    fn mark_dirty(&self) -> bool {
        if self.disposed.get() {
            return false;
        }
        self.dirty.set(true);
        if !self.queued.replace(true) {
            enqueue_effect(&self.me);
        }
        false
    }

    fn dependents(&self) -> Vec<Weak<dyn Observer>> {
        Vec::new()
    }
}

#[must_use = "subscription expires on drop — bind the handle to a variable"]
pub struct Subscription {
    _effect: Effect,
}

impl Subscription {
    pub(crate) fn new(effect: Effect) -> Self {
        Self { _effect: effect }
    }
}

#[must_use = "effect lives while the handle lives; keep it or call `.leak()` for app-lifetime"]
pub struct EffectHandle {
    inner: Option<Effect>,
}

impl EffectHandle {
    pub fn new(effect: Effect) -> Self {
        Self {
            inner: Some(effect),
        }
    }

    pub fn run_now(&self) {
        flush_effects();
    }

    pub fn is_alive(&self) -> bool {
        self.inner.is_some()
    }

    pub fn take(&mut self) -> Option<Effect> {
        self.inner.take()
    }

    pub fn dispose(mut self) {
        if let Some(e) = self.inner.take() {
            e.dispose();
        }
    }

    pub fn leak(self) {
        std::mem::forget(self);
    }

    pub fn forget(self) {
        self.leak();
    }
}

impl Drop for EffectHandle {
    fn drop(&mut self) {
        drop(self.inner.take());
    }
}

impl From<Effect> for EffectHandle {
    fn from(effect: Effect) -> Self {
        Self::new(effect)
    }
}

#[must_use = "scope disposes effects on drop; keep it or call `.leak()`"]
#[derive(Default)]
pub struct EffectScope {
    effects: Vec<Effect>,
}

impl EffectScope {
    pub fn new() -> Self {
        Self {
            effects: Vec::new(),
        }
    }

    pub fn push(&mut self, effect: Effect) {
        self.effects.push(effect);
    }

    pub fn extend(&mut self, effects: impl IntoIterator<Item = Effect>) {
        self.effects.extend(effects);
    }

    pub fn len(&self) -> usize {
        self.effects.len()
    }

    pub fn is_empty(&self) -> bool {
        self.effects.is_empty()
    }

    pub fn dispose_all(&mut self) {
        while let Some(e) = self.effects.pop() {
            e.dispose();
        }
    }

    pub fn leak(self) {
        std::mem::forget(self);
    }

    pub fn forget(self) {
        self.leak();
    }
}

impl Drop for EffectScope {
    fn drop(&mut self) {}
}
