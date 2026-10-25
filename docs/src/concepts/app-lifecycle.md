# Application lifecycle

A Sequoia application is created with an `App` builder, mounted, and then run
on the main thread. The builder configures the primary window and shared
services; `run` takes over the thread and drives the event loop.

## Building an application

```rust,no_run
use sequoia_ui::prelude::*;

fn main() {
    App::new()
        .title("Lifecycle")
        .size(900.0, 600.0)
        .min_size(480.0, 320.0)
        .resizable(true)
        .theme(Theme::dark())
        .root(Text::new("Hello"))
        .run()
        .expect("failed to start the application");
}
```

The builder methods used here are:

- `title` sets the primary window title.
- `size` sets its logical size in pixels.
- `min_size` and `max_size` constrain resizing.
- `resizable`, `decorations`, `transparent`, `visible`, `position`,
  `maximized`, `fullscreen`, and `always_on_top` control window behavior.
- `theme` selects the initial `Theme` provided to views.
- `root` mounts the top-level view.
- `run` starts the event loop.

Most window settings have the same names on `WindowSpec`, so the same
configuration can be used for additional windows.

## Multiple windows

`App` treats the first window as primary; the chainable methods such as
`title` and `size` apply to it. Add more windows with `window`:

```rust
# use sequoia_ui::prelude::*;
let app = App::new()
    .title("Main")
    .root(Text::new("Main window"))
    .window(
        WindowSpec::new("Inspector", Text::new("Inspector"))
            .size(360.0, 480.0)
            .resizable(false),
    );

assert_eq!(app.window_count(), 2);
```

Each `WindowSpec` carries its own root view, size, settings, and close
handler. The runtime owns the windows; use the `Windows` service at runtime
to open or close them from application code.

## What happens on startup

When `run` is called, the platform layer:

1. Creates the native event loop.
2. Registers the initial windows and allocates their IDs.
3. Creates the worker executor used by the async bridge.
4. Builds the shared `Environment` and provides the theme, asset registry,
   clipboard, window manager, and executor.
5. Creates the native windows and GPU surfaces.
6. Mounts each root view, performs the first layout, and paints the first
   frame.
7. Routes input events to views and schedules redraws when reactive state
   changes.
8. Exits when the last window is closed.

`run` consumes the builder and blocks the calling thread. It is not possible
to re-enter the event loop from a callback; use the async bridge or `Windows`
for work that happens after startup.

## Redraw modes

The default mode is `RedrawMode::OnDemand`. The runtime paints when input
arrives, layout changes, or reactive state is marked dirty. This is the right
choice for most applications.

Use `RedrawMode::Continuous` for content that changes every frame, such as
particles, video, or shader-driven animation:

```rust,no_run
use sequoia_ui::prelude::*;

fn main() {
    App::new()
        .root(Text::new("Animation"))
        .redraw_mode(RedrawMode::Continuous)
        .fps_limit(60)
        .run()
        .expect("event loop failed");
}
```

`fps_limit` applies to continuous mode and caps the frame rate. `show_fps`
displays frame statistics in the window title. Do not enable continuous mode
for a mostly static UI; it keeps the GPU busy and drains battery.

## Closing a window

Clicking the close button calls the window's close handler, if one is
configured. The handler returns `true` to allow the close and `false` to keep
the window open:

```rust
# use sequoia_ui::prelude::*;
# use std::cell::Cell;
# use std::rc::Rc;
let dirty = Rc::new(Cell::new(true));
let guard = dirty.clone();

let app = App::new()
    .root(Text::new("Editor"))
    .on_close_request(move || !guard.get());

assert_eq!(app.window_count(), 1);
```

Closing a window removes it from the application. When the last window is
closed, the event loop stops and `run` returns. Save work in the close handler
or flush it through an async task before allowing the close.

## Errors and shutdown

`run` returns `Result<(), sequoia_ui::platform::AppError>`. Typical failures
are an event-loop initialization error, a window creation error, or GPU
initialization failure after the event loop starts. Surface these errors to
the user instead of discarding them:

```rust,no_run
use sequoia_ui::prelude::*;

fn main() -> Result<(), sequoia_ui::platform::AppError> {
    App::new()
        .title("Strict startup")
        .root(Unit)
        .run()
}
```

`Unit` is a zero-size view that renders nothing. It is useful as a
placeholder root or as a neutral child.

When `run` returns, the event loop has stopped, window resources have been
released, and no further UI callbacks will fire. Dropping remaining signals
and effects is safe at that point.

## Shared services

The platform creates one `Environment` for the application and provides
services by type:

- `Theme` for semantic colors and text defaults.
- `AssetRegistry` for icons and fonts.
- Clipboard services used by text widgets.
- `Windows` for opening, closing, and updating windows.
- `Executor` for async work and timers.

Built-in views read these services from `PaintCtx` and `EventCtx`. Custom
views can do the same. Application-specific values can be passed directly to
views or stored in signals; the environment is a service locator, not a
global state container.

## Next steps

- Understand the tree that `root` mounts: View tree.
- Add frame-driven effects: Animation.
- Run background work without blocking the UI: Async and tasks.
