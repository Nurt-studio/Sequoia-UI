#![forbid(unsafe_code)]

use std::sync::Arc;

pub type WakeFn = Arc<dyn Fn() + Send + Sync>;

pub fn noop_wake() -> WakeFn {
    Arc::new(|| {})
}

pub fn fn_wake(f: impl Fn() + Send + Sync + 'static) -> WakeFn {
    Arc::new(f)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn noop_wake_does_nothing() {
        noop_wake()();
    }

    #[test]
    fn fn_wake_calls_closure() {
        let n = Arc::new(AtomicUsize::new(0));
        let probe = Arc::clone(&n);
        let w = fn_wake(move || {
            probe.fetch_add(1, Ordering::SeqCst);
        });
        w();
        w();
        assert_eq!(n.load(Ordering::SeqCst), 2);
    }
}
