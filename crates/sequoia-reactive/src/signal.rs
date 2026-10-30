use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use crate::effect::{Effect, Subscription, effect};
use crate::runtime::{
    Dependency, Observer, batch_depth, flush_effects, observers_in, propagate,
    register_observer_in, track_read, unregister_observer_in,
};

pub struct Signal<T> {
    inner: Rc<SignalNode<T>>,
}

impl<T> Clone for Signal<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Rc::clone(&self.inner),
        }
    }
}

struct SignalNode<T> {
    value: RefCell<T>,
    version: Cell<u64>,
    observers: RefCell<Vec<Weak<dyn Observer>>>,
}

impl<T: 'static> Signal<T> {
    pub fn new(value: T) -> Self {
        Self {
            inner: Rc::new(SignalNode {
                value: RefCell::new(value),
                version: Cell::new(0),
                observers: RefCell::new(Vec::new()),
            }),
        }
    }

    pub fn version(&self) -> u64 {
        self.inner.version.get()
    }

    pub fn id(&self) -> usize {
        Rc::as_ptr(&self.inner) as usize
    }

    pub fn track(&self) {
        track_read(&self.inner);
    }
}

impl<T: Clone + PartialEq + 'static> Signal<T> {
    pub fn get(&self) -> T {
        track_read(&self.inner);
        self.inner.value.borrow().clone()
    }

    pub fn get_untracked(&self) -> T {
        self.inner.value.borrow().clone()
    }

    /// Reads the value by reference and tracks the read.
    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        track_read(&self.inner);
        let borrowed = self.inner.value.borrow();
        f(&borrowed)
    }

    /// Reads the value by reference without cloning or tracking.
    ///
    /// Use this in paint and event loops when the value is large (for example
    /// `Vec<Layer>`); `get_untracked()` clones the whole value on every call.
    pub fn with_untracked<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        let borrowed = self.inner.value.borrow();
        f(&borrowed)
    }

    pub fn set(&self, value: T) {
        {
            let mut current = self.inner.value.borrow_mut();
            if *current == value {
                return;
            }
            *current = value;
        }
        self.inner.version.set(self.inner.version.get() + 1);
        propagate(self.inner.as_ref());
        if batch_depth() == 0 {
            flush_effects();
        }
    }

    pub fn update(&self, f: impl FnOnce(&mut T)) {
        let next = {
            let current = self.inner.value.borrow();
            let mut next = current.clone();
            f(&mut next);
            next
        };
        self.set(next);
    }

    #[must_use = "subscription expires on drop — bind the handle to a variable"]
    pub fn subscribe(&self, listener: impl Fn() + 'static) -> Subscription {
        let inner = Rc::clone(&self.inner);
        Subscription::new(effect(move || {
            track_read(&inner);
            listener();
        }))
    }

    /// Keeps this signal and `other` synchronized in both directions.
    ///
    /// The returned [`Binding`] owns the two effects; dropping it stops the
    /// synchronization and disposes them, so there is no need for
    /// `.forget()`. `Signal::set` ignores equal values, so the pair settles
    /// instead of looping.
    #[must_use = "binding stops syncing when dropped — keep it with the signals"]
    pub fn bind(&self, other: &Signal<T>) -> Binding {
        let a = self.clone();
        let b = other.clone();
        let forward = effect(move || b.set(a.get()));

        let a = self.clone();
        let b = other.clone();
        let backward = effect(move || a.set(b.get()));

        Binding {
            _forward: forward,
            _backward: backward,
        }
    }
}

/// A live two-way link between two [`Signal`]s.
///
/// Dropping the binding disposes both synchronization effects.
#[must_use = "binding stops syncing when dropped — keep it with the signals"]
pub struct Binding {
    _forward: Effect,
    _backward: Effect,
}

impl Binding {
    /// Stops syncing immediately instead of waiting for the drop.
    pub fn dispose(self) {
        drop(self);
    }
}

impl<T: 'static> Dependency for SignalNode<T> {
    fn id(&self) -> usize {
        self as *const _ as usize
    }

    fn version(&self) -> u64 {
        self.version.get()
    }

    fn validate(&self) {}

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
