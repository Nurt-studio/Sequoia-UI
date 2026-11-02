# Debounced search

This recipe builds a search box that keeps the newest request's results and
drops late responses from older ones. It combines four pieces:

- `TextField` writes the query into a `Signal<String>`.
- `Debouncer` gives each request a generation number. An older response that
  arrives late is ignored.
- `Resource<Vec<String>>` owns the loading, ready, and failed states and
  supplies the `AsyncView` that renders them.
- `Executor` is read from the environment so the future runs on a worker and
  its result is applied on the UI thread.

```rust,no_run
use std::rc::Rc;
use std::time::Duration;

use sequoia_ui::prelude::*;

struct SearchController {
    query: Signal<String>,
    results: Resource<Vec<String>>,
    debounce: Debouncer,
}

impl SearchController {
    fn new() -> Rc<Self> {
        Rc::new(Self {
            query: Signal::new(String::new()),
            results: Resource::new(),
            debounce: Debouncer::new(),
        })
    }

    fn search(&self, env: &Environment) {
        let Some(exec) = env.get::<Executor>() else {
            return;
        };

        let query = self.query.get_untracked().trim().to_string();
        if query.is_empty() {
            self.results.reset();
            return;
        }

        let request = self.debounce.request();
        let debounce = self.debounce.clone();
        let results = self.results.clone();
        let generation = results.begin_loading();

        exec.spawn_future(
            async move {
                sleep(Duration::from_millis(200)).await;
                search_remote(&query).await
            },
            move |hits| {
                if debounce.is_latest(request) {
                    results.complete(generation, hits);
                }
            },
        );
    }
}

async fn search_remote(query: &str) -> Vec<String> {
    sleep(Duration::from_millis(150)).await;
    let people = [
        "Ada Lovelace",
        "Grace Hopper",
        "Alan Turing",
        "Barbara Liskov",
        "Donald Knuth",
    ];
    people
        .iter()
        .filter(|name| name.to_lowercase().contains(&query.to_lowercase()))
        .map(|name| (*name).to_string())
        .collect()
}

fn main() {
    let search = SearchController::new();

    let field = TextField::new(search.query.clone()).placeholder("Search people...");

    let button = {
        let search = Rc::clone(&search);
        Button::new("Search")
            .style(ButtonStyle::PRIMARY)
            .on_click_env(move |env| search.search(env))
    };

    let results = AsyncView::new(search.results.clone())
        .loading(|| Text::new("Searching...").color(Color::hex(0x9A9A9A)))
        .data(|hits| {
            Column::new().children(
                hits.iter()
                    .map(|name| Text::new(name.clone()))
                    .collect::<Vec<_>>(),
            )
        })
        .error(|message| Text::new(format!("Search failed: {message}")));

    let view = Column::new()
        .padding(24.0)
        .gap(12.0)
        .child(Row::new().gap(8.0).child(field).child(button))
        .child(results);

    App::new()
        .title("Debounced search")
        .size(520.0, 380.0)
        .root(view)
        .run()
        .expect("event loop failed");
}
```

## How the pieces fit

The generation check and the resource generation are belt and braces. The
debouncer check happens first and prevents an older closure from touching the
resource at all. The resource generation check inside `complete` is still
useful because it protects every other write path, including cancellation and
retry.

`search` reads the query with `get_untracked` because it runs in a button
callback, not during reactive tracking. Reading it in a view body would mark
the view as dependent on the field, which is not needed here because the field
already repaints itself.

The 200 ms delay in the future is deliberate: it simulates work that cannot be
cancelled. If the user starts another search while this one sleeps, the newer
request gets a newer generation and the older result is dropped when it
finishes. For truly expensive work, also keep the returned `JobId` and call
`Executor::cancel`; the generation check keeps the result correct either way.

## Triggering from typing

`Button::on_click_env` is used here because it provides the `Environment`
needed to reach the `Executor`. The same `search` method can be called from any
callback that has an environment, including a custom view's `on_event` through
`ctx.environment`. If you wrap the field in your own view, call `search` for
every text change; it bumps the debouncer internally, so the generation check
handles the coalescing.

Do not call `begin_loading` from a worker thread and do not write the resource
from the future. `spawn_future`'s apply closure runs on the UI thread, which is
where `complete`, `fail`, and signal writes are safe.

## Testing the logic

The debouncer and resource can be tested without a window:

```rust
# use sequoia_ui::prelude::*;

let debounce = Debouncer::new();
let first = debounce.request();
let second = debounce.request();
assert!(!debounce.is_latest(first));
assert!(debounce.is_latest(second));

let results = Resource::<Vec<String>>::new();
let old = results.begin_loading();
let new = results.begin_loading();
results.complete(old, vec!["stale".to_string()]);
assert!(results.is_loading());
results.complete(new, vec!["fresh".to_string()]);
assert_eq!(results.data_untracked().unwrap(), vec!["fresh".to_string()]);
```

For a headless app-level test, mount the view in a `Harness`, set the query
signal, call `search` with the harness environment, and drain the executor. See
[Testing](../guides/testing.md) for the harness setup.
