use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};

/// How many notification rounds one `provide` may take when listeners keep
/// providing further values. A plain recursive call would grow the stack.
const MAX_REDISPATCH_ROUNDS: usize = 64;

#[derive(Clone, Default)]
pub struct Environment {
    values: Rc<RefCell<HashMap<TypeId, Rc<dyn Any>>>>,
    version: Rc<Cell<u64>>,
    listeners: Rc<RefCell<Vec<(u64, Rc<dyn Fn()>)>>>,
    next_listener: Rc<Cell<u64>>,
    /// Set while listeners are being called. A listener that provides another
    /// value queues a re-notification instead of recursing.
    dispatching: Rc<Cell<bool>>,
    pending: Rc<Cell<bool>>,
}

#[must_use = "subscription expires on drop — bind the handle to a variable"]
pub struct Listener {
    listeners: Weak<RefCell<Vec<(u64, Rc<dyn Fn()>)>>>,
    id: u64,
}

impl Drop for Listener {
    fn drop(&mut self) {
        if let Some(listeners) = self.listeners.upgrade() {
            listeners.borrow_mut().retain(|(id, _)| *id != self.id);
        }
    }
}

impl Environment {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn provide<T: 'static>(&self, value: T) {
        self.values
            .borrow_mut()
            .insert(TypeId::of::<T>(), Rc::new(value));
        self.version.set(self.version.get() + 1);

        if self.dispatching.replace(true) {
            // Called from a listener: let the outer `provide` notify again
            // once the current round finishes. Recursing here would stack one
            // frame per listener and overflow on mutual providers.
            self.pending.set(true);
            return;
        }
        struct ResetGuard<'a>(&'a Cell<bool>);
        impl Drop for ResetGuard<'_> {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }
        let _guard = ResetGuard(&self.dispatching);

        for _ in 0..MAX_REDISPATCH_ROUNDS {
            self.pending.set(false);
            let listeners: Vec<Rc<dyn Fn()>> = self
                .listeners
                .borrow()
                .iter()
                .map(|(_, f)| Rc::clone(f))
                .collect();
            for l in listeners {
                // One broken listener must not stop the others from seeing the
                // new value.
                if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| l())).is_err() {
                    log::error!(
                        "sequoia-reactive: environment listener panicked — continuing with the rest"
                    );
                }
            }
            if !self.pending.get() {
                return;
            }
        }
        log::error!(
            "sequoia-reactive: environment listeners keep providing values — stopping after \
             {MAX_REDISPATCH_ROUNDS} rounds"
        );
    }

    pub fn on_change(&self, f: impl Fn() + 'static) -> Listener {
        let id = self.next_listener.get();
        self.next_listener.set(id + 1);
        self.listeners.borrow_mut().push((id, Rc::new(f)));
        Listener {
            listeners: Rc::downgrade(&self.listeners),
            id,
        }
    }

    pub fn listener_count(&self) -> usize {
        self.listeners.borrow().len()
    }

    pub fn get<T: Clone + 'static>(&self) -> Option<T> {
        self.values
            .borrow()
            .get(&TypeId::of::<T>())
            .and_then(|v| v.downcast_ref::<T>().cloned())
    }

    pub fn get_rc<T: 'static>(&self) -> Option<Rc<T>> {
        self.values
            .borrow()
            .get(&TypeId::of::<T>())?
            .clone()
            .downcast::<T>()
            .ok()
    }

    pub fn version(&self) -> u64 {
        self.version.get()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct Theme {
        accent: &'static str,
    }

    #[test]
    fn provide_get_roundtrip() {
        let env = Environment::new();
        env.provide(Theme { accent: "red" });
        assert_eq!(env.get::<Theme>().map(|t| t.accent), Some("red"));
        env.provide(42i32);
        assert_eq!(env.get::<i32>(), Some(42));
        assert!(env.get::<u32>().is_none());
    }

    #[test]
    fn listener_is_removed_when_handle_drops() {
        use std::cell::Cell;

        let env = Environment::new();
        let hits = Rc::new(Cell::new(0u32));
        let listener = env.on_change({
            let hits = Rc::clone(&hits);
            move || hits.set(hits.get() + 1)
        });
        assert_eq!(env.listener_count(), 1);
        env.provide(1u8);
        assert_eq!(hits.get(), 1);

        drop(listener);
        assert_eq!(env.listener_count(), 0, "handle unregisters the listener");
        env.provide(2u8);
        assert_eq!(hits.get(), 1, "detached listener is not called");
    }

    #[test]
    fn panicking_listener_does_not_block_the_others() {
        let env = Environment::new();
        let hits = Rc::new(Cell::new(0u32));
        let _bad = env.on_change(|| panic!("boom listener"));
        let _good = env.on_change({
            let hits = Rc::clone(&hits);
            move || hits.set(hits.get() + 1)
        });

        let hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        env.provide(1u8);
        std::panic::set_hook(hook);

        assert_eq!(hits.get(), 1, "a panicking listener is isolated");
    }

    #[test]
    fn provide_from_a_listener_is_queued_not_stacked() {
        let env = Environment::new();
        let rounds = Rc::new(Cell::new(0u32));
        let _l = env.on_change({
            let (env, rounds) = (env.clone(), Rc::clone(&rounds));
            move || {
                rounds.set(rounds.get() + 1);
                if rounds.get() < 4 {
                    env.provide(rounds.get());
                }
            }
        });
        env.provide(0u8);
        assert_eq!(rounds.get(), 4, "re-provided values are still delivered");
    }

    #[test]
    fn rc_access_and_versioning() {
        let env = Environment::new();
        env.provide(vec![1, 2, 3]);
        let rc = env.get_rc::<Vec<i32>>().unwrap();
        assert_eq!(*rc, vec![1, 2, 3]);
        let v0 = env.version();
        env.provide(1u8);
        assert!(env.version() > v0);
    }
}
