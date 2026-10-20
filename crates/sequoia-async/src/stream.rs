#![forbid(unsafe_code)]

use std::pin::Pin;
use std::time::Duration;

use futures_core::Stream;
use futures_task::{Context, Poll};

pub fn backoff_delays(base: Duration, attempts: usize) -> Vec<Duration> {
    (0..attempts)
        .map(|i| {
            let shift = i.min(10) as u32;
            base.saturating_mul(1 << shift)
        })
        .collect()
}

pub async fn retry_with_backoff<T, E, F, Fut>(
    attempts: usize,
    base_delay: Duration,
    mut factory: F,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
{
    assert!(attempts >= 1, "retry needs at least 1 attempt");
    for delay in backoff_delays(base_delay, attempts.saturating_sub(1)) {
        match factory().await {
            Ok(v) => return Ok(v),
            Err(_) => crate::time::sleep(delay).await,
        }
    }
    factory().await
}

pub async fn collect_stream<S>(mut stream: S) -> Vec<S::Item>
where
    S: Stream + Unpin,
{
    let mut out = Vec::new();
    let waker = futures_task::noop_waker();
    let mut cx = Context::from_waker(&waker);
    loop {
        match Pin::new(&mut stream).poll_next(&mut cx) {
            Poll::Ready(Some(item)) => out.push(item),
            Poll::Ready(None) => break,
            Poll::Pending => {
                std::thread::sleep(Duration::from_millis(1));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn backoff_grows_exponentially() {
        let d = backoff_delays(Duration::from_millis(10), 4);
        assert_eq!(
            d,
            vec![
                Duration::from_millis(10),
                Duration::from_millis(20),
                Duration::from_millis(40),
                Duration::from_millis(80),
            ]
        );
        assert!(backoff_delays(Duration::from_millis(10), 0).is_empty());
    }

    #[test]
    fn retry_succeeds_after_failures() {
        let n = Arc::new(AtomicUsize::new(0));
        let probe = Arc::clone(&n);
        let out = pollster::block_on(retry_with_backoff(3, Duration::from_millis(1), move || {
            let probe = Arc::clone(&probe);
            async move {
                let i = probe.fetch_add(1, Ordering::SeqCst);
                if i < 2 {
                    Err::<u32, _>("nope")
                } else {
                    Ok(7u32)
                }
            }
        }));
        assert_eq!(out, Ok(7));
        assert_eq!(n.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn retry_returns_last_error() {
        let out = pollster::block_on(retry_with_backoff(2, Duration::from_millis(1), || async {
            Err::<u32, _>("bad")
        }));
        assert_eq!(out, Err("bad"));
    }
}
