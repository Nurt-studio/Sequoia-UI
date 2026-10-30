# Async work

Sequoia keeps reactive state on the UI thread. Background work runs on a worker
pool, and its results are applied to signals only when the runtime drains the
completion queue. That rule makes async code safe by construction: views and
signals are never touched from a worker thread.

The async layer has three parts:

- `Resource<T>` and `AsyncView<T>` model loading, ready, failed, and cancelled
  states for a piece of data.
- `Executor` (or `AsyncRuntime`) runs closures, futures, and streams and
  delivers their results to the UI thread.
- `Debouncer`, `Throttler`, `sleep`, `timeout`, and `ticks` handle time.

Add `AsyncRuntime` to dependencies only if you embed the view layer without the
platform crate; applications usually use `Executor`, which is provided by the
environment.

## Resources and AsyncView

A `Resource<T>` is a small state machine around a signal. `begin_loading`
returns a generation number; results carrying an older generation are ignored,
so a slow request cannot overwrite a newer one:

```rust
# use sequoia_ui::prelude::*;
let users = Resource::<Vec<String>>::new();
assert!(matches!(users.state().get_untracked(), AsyncState::Idle));

let generation = users.begin_loading();
assert!(users.is_loading());

users.complete(generation, vec!["Ada".to_string(), "Grace".to_string()]);
assert_eq!(users.data_untracked().unwrap().len(), 2);
assert_eq!(users.state().get_untracked().error(), None);
```

`Resource::with_initial` starts in the ready state, which is useful when cached
data should be visible immediately while a refresh runs. `fail` stores an error
message, `cancel` moves to `Cancelled`, and `reset` returns to `Idle`. All of
them bump the generation, so in-flight work becomes stale automatically.

`AsyncView` maps each state to a view. It subscribes to the resource signal, so
a later `complete` or `fail` repaints only that subtree:

```rust
# use sequoia_ui::prelude::*;
let users = Resource::<Vec<String>>::with_initial(vec!["Ada".to_string()]);

let view = AsyncView::new(users.clone())
    .loading(|| Text::new("Loading users..."))
    .data(|rows| {
        Column::new().children(
            rows.iter()
                .map(|name| Text::new(name.clone()))
                .collect::<Vec<_>>(),
        )
    })
    .error(|message| Text::new(format!("Could not load: {message}")))
    .retry("Try again", move || {
        users.begin_loading();
    });

assert!(matches!(
    view.resource().state().get_untracked(),
    AsyncState::Ready(_)
));
```

Use `idle` and `cancelled` when those states should look different from
loading. `progress` accepts a `Signal<f32>` between zero and one and draws a
progress bar under the loading content. The retry callback is where you start
the request again; the resource itself does not know how to fetch anything.

## Executor and AsyncRuntime

`Executor` is installed in the `Environment` before the first frame. Inside an
event handler or a paint method it is available through
`ctx.environment.get::<Executor>()`:

```rust
# use sequoia_ui::prelude::*;

fn load_users(ctx: &EventCtx, users: Resource<Vec<String>>) {
    let Some(exec) = ctx.environment.get::<Executor>() else {
        return;
    };
    exec.spawn_resource(&users, async {
        vec!["Ada".to_string(), "Grace".to_string()]
    });
}
```

`spawn_resource` calls `begin_loading` for you and completes the resource when
the future resolves. `spawn_resource_result` takes `Future<Output = Result<T, E>>`
and routes the error into `Resource::fail`. Both return a `JobId` that can be
passed to `Executor::cancel`.

The methods that matter most:

- `spawn(work, apply)` runs a blocking closure on a worker and calls `apply` on
  the UI thread with its result.
- `spawn_future(future, apply)` and `spawn_task(future)` are the future-based
  equivalents.
- `spawn_to_signal(&signal, future)` writes the future output into a signal.
- `spawn_stream(make, on_item)` runs a stream; `make` receives an `Emitter<T>`
  and `on_item` receives each value on the UI thread.
- `pending()` is a signal with the number of outstanding jobs, and `is_busy()`
  reports whether that number is non-zero.

`AsyncRuntime` exposes the same core API when you are not using `App`. Waking
is the only difference: the platform executor wakes the event loop after
draining, while a standalone runtime leaves that to you.

```rust
# use sequoia_ui::prelude::*;
# use std::time::Duration;

let runtime = AsyncRuntime::with_default_threads();
let quote = Signal::new(String::new());

runtime.spawn_to_signal(&quote, async {
    sleep(Duration::from_millis(20)).await;
    "forty-two".to_string()
});

runtime.drain_until(Duration::from_secs(2));
assert_eq!(quote.get_untracked(), "forty-two");
```

`drain` applies completed jobs that are already available and returns how many
it handled. `drain_until` waits up to a timeout, which is what tests use. In an
application, the platform layer drains automatically at the start of each
frame.

## Streams

A stream is a future that emits zero or more values. It is useful for progress
reports, log tails, and incremental search results:

```rust,no_run
# use sequoia_ui::prelude::*;
# use std::time::Duration;
let runtime = AsyncRuntime::with_default_threads();
let seen = Signal::new(0usize);
let target = seen.clone();

runtime.spawn_stream(
    move |tx| async move {
        for value in 0..3usize {
            let _ = tx.emit(value);
            sleep(Duration::from_millis(10)).await;
        }
    },
    move |_value| target.update(|n| *n += 1),
);
```

`Emitter::emit` returns a `Result`; a closed stream means the consumer went
away, so stopping is the right response. `spawn_stream_scoped` returns a
`ScopedTask` that cancels the stream when it is dropped, and
`TaskHandle::cancel` cancels a single job by id.

## Debouncing and throttling

`Debouncer` tracks generations so only the newest request may complete, while
`Throttler` spaces work by wall-clock time. Both are cheap and allocation-free
after construction:

```rust
# use sequoia_ui::prelude::*;
use std::time::Duration;

let debounce = Debouncer::new();
let first = debounce.request();
let second = debounce.request();
assert!(!debounce.is_latest(first));
assert!(debounce.is_latest(second));

let throttle = Throttler::new(Duration::from_millis(100));
assert!(throttle.should_run());
assert!(!throttle.should_run());
```

`Debouncer::invalidate` cancels every outstanding request. The typical search
field pattern is: bump the debouncer on every keystroke, capture the generation
in the future, and ignore the result when `is_latest` is false. The cookbook's
[debounced search](../cookbook/debounced-search.md) builds the complete example.

## Time

`sleep(duration)` and `timeout(duration, future)` are executor-friendly: they
yield instead of blocking a worker thread. `ticks(period)` and `Interval`
produce a repeating stream of instants, and `with_timeout` wraps a future
without changing its output type. Avoid `std::thread::sleep` in an async task;
it occupies a worker and delays every other job in the pool.

## Errors and cancellation

There are two independent cancellation mechanisms:

- **Data-level staleness.** `Resource` generations make an old response
  harmless even if the task still runs to completion. This is the simplest and
  safest option for queries.
- **Task-level cancellation.** `TaskHandle::cancel` and `Executor::cancel` stop
  pending future polls; blocking closures cannot be interrupted once they have
  started.

When a view that owns a task is unmounted, store the task in a `ScopedTask` so
its `Drop` cancels it. Long-running background work that must survive the view
should be explicit; detach the handle and keep it somewhere with application
lifetime.

## Guidelines

- Keep `Resource<T>` small and `Clone`; store ids and records, not textures.
- Let `AsyncView` own loading, error, and retry presentation instead of
  duplicating those states in every screen.
- Use `spawn_resource_result` rather than hand-rolling generation checks.
- Debounce text input; throttle expensive rendering; never debounce explicit
  user commands such as Save.
- Prefer several small tasks over one long task, so cancellation and progress
  stay responsive.

For the reactive side of asynchronous data, see [Reactivity](reactivity.md).
For rendering updates that follow state changes, see
[Rendering](rendering.md).
