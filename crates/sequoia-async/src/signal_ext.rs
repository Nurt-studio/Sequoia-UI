#![forbid(unsafe_code)]

use futures_core::Future;

use sequoia_reactive::Signal;

pub trait SignalAsyncExt<T: Clone + PartialEq + Send + 'static> {
    fn async_setter(&self) -> impl Fn(T) + Clone + 'static
    where
        Self: Clone;
}

impl<T: Clone + PartialEq + Send + 'static> SignalAsyncExt<T> for Signal<T> {
    fn async_setter(&self) -> impl Fn(T) + Clone + 'static
    where
        Self: Clone,
    {
        let signal = self.clone();
        move |value: T| signal.set(value)
    }
}

pub fn pipe_to_signal<T>(signal: &Signal<T>) -> impl FnOnce(T) + 'static
where
    T: Clone + PartialEq + Send + 'static,
{
    let signal = signal.clone();
    move |value: T| signal.set(value)
}

pub fn pipe_result_to_signal<T, E>(
    signal: &Signal<T>,
    on_error: impl FnOnce(E) + 'static,
) -> impl FnOnce(Result<T, E>) + 'static
where
    T: Clone + PartialEq + Send + 'static,
    E: Send + 'static,
{
    let signal = signal.clone();
    move |result: Result<T, E>| match result {
        Ok(v) => signal.set(v),
        Err(e) => on_error(e),
    }
}

#[allow(dead_code)]
pub(crate) fn assert_ui_future<T: Send, F: Future<Output = T>>(_: &F) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pipe_writes_to_signal() {
        let s = Signal::new(0u32);
        let apply = pipe_to_signal(&s);
        apply(5);
        assert_eq!(s.get_untracked(), 5);
    }

    #[test]
    fn pipe_result_routes_ok_and_err() {
        let s = Signal::new(0u32);
        let apply = pipe_result_to_signal(&s, |e: String| assert_eq!(e, "boom"));
        apply(Ok(3));
        assert_eq!(s.get_untracked(), 3);
        let s2 = Signal::new(0u32);
        let apply2 = pipe_result_to_signal(&s2, |e: String| assert_eq!(e, "boom"));
        apply2(Err("boom".to_string()));
        assert_eq!(s2.get_untracked(), 0);
    }
}
