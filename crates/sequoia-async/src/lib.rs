#![forbid(unsafe_code)]

mod emitter;
mod resource;
mod runtime;
mod signal_ext;
mod stream;
mod task;
mod testing;
mod time;
mod view_ext;
mod wake;

pub use emitter::{Emitter, StreamError};
#[doc(hidden)]
pub use emitter::{StreamKind, StreamMsgAny};
pub use resource::{AsyncState, Resource};
pub use runtime::AsyncRuntime;
pub use signal_ext::{SignalAsyncExt, pipe_result_to_signal, pipe_to_signal};
pub use stream::{backoff_delays, collect_stream, retry_with_backoff};
pub use task::{JoinError, TaskHandle, TaskId};
pub use testing::{TestDrain, drain_until, run_until};
pub use time::{
    Debouncer, Elapsed, Interval, Throttler, Ticks, TimeoutError, sleep, ticks, timeout,
    with_timeout,
};
pub use view_ext::ScopedTask;
pub use wake::{WakeFn, fn_wake, noop_wake};

pub mod prelude {
    pub use crate::{
        AsyncState, Debouncer, Emitter, Resource, ScopedTask, SignalAsyncExt, TaskHandle, TaskId,
        Throttler, sleep, timeout,
    };
}
