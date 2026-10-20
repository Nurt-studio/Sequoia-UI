#![forbid(unsafe_code)]

use std::time::{Duration, Instant};

pub fn drain_until<R>(rt: &R, timeout: Duration) -> usize
where
    R: TestDrain,
{
    let deadline = Instant::now() + timeout;
    loop {
        let n = rt.test_drain();
        if n > 0 || Instant::now() >= deadline {
            return n;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}

pub fn run_until<R>(rt: &R, timeout: Duration, mut predicate: impl FnMut() -> bool) -> bool
where
    R: TestDrain,
{
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        rt.test_drain();
        if predicate() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    rt.test_drain();
    predicate()
}

pub trait TestDrain {
    fn test_drain(&self) -> usize;
}

impl TestDrain for crate::runtime::AsyncRuntime {
    fn test_drain(&self) -> usize {
        self.drain()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Counter {
        n: std::cell::Cell<usize>,
        fire_at: usize,
    }

    impl TestDrain for Counter {
        fn test_drain(&self) -> usize {
            let n = self.n.get() + 1;
            self.n.set(n);
            if n >= self.fire_at { 1 } else { 0 }
        }
    }

    #[test]
    fn drain_until_returns_on_fire() {
        let c = Counter {
            n: std::cell::Cell::new(0),
            fire_at: 3,
        };
        assert_eq!(drain_until(&c, Duration::from_secs(2)), 1);
    }

    #[test]
    fn run_until_respects_predicate() {
        let c = Counter {
            n: std::cell::Cell::new(0),
            fire_at: 100,
        };
        assert!(!run_until(&c, Duration::from_millis(10), || false));
    }
}
