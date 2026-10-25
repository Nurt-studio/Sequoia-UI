# Your first app

This chapter builds a small counter. It introduces the application builder,
the reactive `Signal`, a few common views, and event handlers.

## The complete program

Replace the contents of `src/main.rs` with:

```rust,no_run
use sequoia_ui::prelude::*;

fn main() {
    let count = Signal::new(0i32);
    let shown = count.clone();

    App::new()
        .title("Counter")
        .size(420.0, 280.0)
        .root(
            Column::new()
                .center()
                .gap(16.0)
                .fill_size()
                .padding(24.0)
                .child(
                    Text::with(move || format!("Count: {}", shown.get()))
                        .size(32.0)
                        .bold(),
                )
                .child(
                    Button::new("Increment")
                        .on_click(move || count.update(|n| *n += 1)),
                ),
        )
        .run()
        .expect("event loop failed");
}
```

Run it with:

```text
cargo run
```

Click the button and the number updates. The text label is not manually
refreshed: it re-renders because it read the signal.

## How it fits together

### The application builder

`App::new()` creates the application. Builder calls configure the native
window and the root of the view tree:

- `title` sets the window title.
- `size` sets the logical window size in pixels.
- `root` mounts the top-level view.
- `run` starts the event loop and blocks until the application exits.

`run` returns a `Result`, so the example uses `expect` to surface startup
errors such as a missing GPU adapter.

### Reactive state

`Signal::new(0i32)` creates a reactive cell. Its value is read with `get`,
replaced with `set`, and modified in place with `update`:

```rust
# use sequoia_ui::prelude::*;
let count = Signal::new(0i32);
count.set(10);
count.update(|n| *n += 1);
assert_eq!(count.get(), 11);
```

Clones share the same underlying value, so a signal can be moved into several
callbacks. In the example, `count` is moved into the button handler and
`shown` into the label builder.

`Text::with` takes a closure rather than a fixed string. The closure runs
inside a reactive scope, so reading `shown.get()` subscribes the label to the
signal. When the value changes, only subscribers are scheduled for another
frame.

### Views and layout

`Column` stacks its children vertically. The chain in the example does the
following:

- `center` centers children on both axes.
- `gap(16.0)` inserts 16 logical pixels between children.
- `fill_size` stretches the column to the available space.
- `padding(24.0)` insets the content from the window edges.

`Button` is a built-in widget. `on_click` registers a callback that runs on the
UI thread.

## What runs when

1. `run` creates the window, GPU surface, and renderer.
2. The root view is mounted and laid out for the current window size.
3. Input events are routed to views and widgets.
4. Changing a signal marks its subscribers dirty and requests a redraw.
5. The event loop exits when the last window closes.

The next chapters explain reactivity, layout, and event handling in depth.

## Try it yourself

- Add a `Decrement` button that subtracts one.
- Add a `Reset` button that calls `count.set(0)`.
- Replace the label with `Text::with` and format the count as a sentence.

Continue with [Project structure](project-structure.md) to organize a larger
application.
