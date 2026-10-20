#![forbid(unsafe_code)]

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use futures_core::Stream;
use pin_project_lite::pin_project;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimeoutError;

impl std::fmt::Display for TimeoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "operation timed out")
    }
}

impl std::error::Error for TimeoutError {}

pub type Elapsed = TimeoutError;

pub async fn sleep(duration: Duration) {
    if duration.is_zero() {
        return;
    }
    std::thread::sleep(duration);
}

pin_project! {

    pub struct Timeout<F> {
        #[pin]
        future: F,
        deadline: Instant,
    }
}

impl<F> Timeout<F> {
    fn new(future: F, duration: Duration) -> Self {
        Self {
            future,
            deadline: Instant::now() + duration,
        }
    }
}

impl<F: Future> Future for Timeout<F> {
    type Output = Result<F::Output, TimeoutError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        if let Poll::Ready(v) = this.future.poll(cx) {
            return Poll::Ready(Ok(v));
        }
        if Instant::now() >= *this.deadline {
            return Poll::Ready(Err(TimeoutError));
        }

        let waker = cx.waker().clone();
        let wait = (*this.deadline).saturating_duration_since(Instant::now());
        let wait = wait.min(Duration::from_millis(10));
        std::thread::spawn(move || {
            std::thread::sleep(wait);
            waker.wake();
        });
        Poll::Pending
    }
}

pub fn timeout<F>(duration: Duration, future: F) -> Timeout<F>
where
    F: Future,
{
    Timeout::new(future, duration)
}

pub fn with_timeout<F>(duration: Duration, future: F) -> Timeout<F>
where
    F: Future,
{
    timeout(duration, future)
}

#[derive(Clone, Copy, Debug)]
pub struct Interval {
    period: Duration,
    next: Instant,
}

impl Interval {
    pub fn new(period: Duration) -> Self {
        Self {
            period: period.max(Duration::from_millis(1)),
            next: Instant::now() + period.max(Duration::from_millis(1)),
        }
    }

    pub async fn tick(&mut self) {
        let now = Instant::now();
        if self.next > now {
            std::thread::sleep(self.next - now);
        }
        self.next = Instant::now() + self.period;
    }

    pub fn period(&self) -> Duration {
        self.period
    }
}

pin_project! {

    pub struct Ticks {
        interval: Interval,
    }
}

impl Ticks {
    pub fn new(period: Duration) -> Self {
        Self {
            interval: Interval::new(period),
        }
    }
}

impl Stream for Ticks {
    type Item = Instant;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        let now = Instant::now();
        if now >= this.interval.next {
            this.interval.next = now + this.interval.period;
            return Poll::Ready(Some(now));
        }
        let waker = cx.waker().clone();
        let wait = this.interval.next.saturating_duration_since(now);
        std::thread::spawn(move || {
            std::thread::sleep(wait.min(Duration::from_millis(50)));
            waker.wake();
        });
        Poll::Pending
    }
}

pub fn ticks(period: Duration) -> Ticks {
    Ticks::new(period)
}

#[derive(Clone, Debug, Default)]
pub struct Debouncer {
    generation: std::rc::Rc<std::cell::Cell<u64>>,
}

impl Debouncer {
    pub fn new() -> Self {
        Self {
            generation: std::rc::Rc::new(std::cell::Cell::new(0)),
        }
    }

    pub fn request(&self) -> u64 {
        let next = self.generation.get() + 1;
        self.generation.set(next);
        next
    }

    pub fn is_latest(&self, generation: u64) -> bool {
        generation == self.generation.get()
    }

    pub fn invalidate(&self) {
        self.generation.set(self.generation.get() + 1);
    }
}

#[derive(Clone, Debug)]
pub struct Throttler {
    min_interval: Duration,
    last: std::rc::Rc<std::cell::Cell<Option<Instant>>>,
}

impl Throttler {
    pub fn new(min_interval: Duration) -> Self {
        Self {
            min_interval,
            last: std::rc::Rc::new(std::cell::Cell::new(None)),
        }
    }

    pub fn should_run(&self) -> bool {
        let now = Instant::now();
        let last = self.last.get();
        match last {
            None => {
                self.last.set(Some(now));
                true
            }
            Some(t) if now.duration_since(t) >= self.min_interval => {
                self.last.set(Some(now));
                true
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sleep_zero_returns_fast() {
        let t = Instant::now();
        pollster::block_on(sleep(Duration::ZERO));
        assert!(t.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn timeout_ok_when_fast() {
        let out = pollster::block_on(timeout(Duration::from_secs(2), async { 7 }));
        assert_eq!(out, Ok(7));
    }

    #[test]
    fn timeout_errors_when_slow() {
        let out = pollster::block_on(timeout(
            Duration::from_millis(20),
            std::future::pending::<u32>(),
        ));
        assert_eq!(out, Err(TimeoutError));
    }

    #[test]
    fn interval_ticks_advance() {
        let mut iv = Interval::new(Duration::from_millis(5));
        assert_eq!(iv.period(), Duration::from_millis(5));
        pollster::block_on(iv.tick());
        pollster::block_on(iv.tick());
    }
}
