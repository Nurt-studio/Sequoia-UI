# Project structure

Sequoia does not impose a project layout. A single `main.rs` is enough for a
small tool; larger applications usually split views, state, and assets into
modules.

## A typical application

```text
my-app/
├── Cargo.toml
├── assets/
│   ├── icons/
│   └── images/
└── src/
    ├── main.rs
    ├── app.rs
    ├── state.rs
    └── views/
        ├── mod.rs
        └── editor.rs
```

- `main.rs` owns startup and calls `App::run`.
- `app.rs` builds the root view and wires state to views.
- `state.rs` keeps signals, stores, and actions.
- `views/` contains reusable view functions or custom view types.
- `assets/` contains fonts, icons, and images loaded at runtime.

## Keep startup small

A useful pattern is to keep `main.rs` focused on configuration:

```rust,ignore
use sequoia_ui::prelude::*;

mod app;
mod state;

fn main() {
    App::new()
        .title("My App")
        .size(1100.0, 720.0)
        .min_size(720.0, 480.0)
        .root(app::root())
        .run()
        .expect("event loop failed");
}
```

The `app` and `state` modules are defined in the project tree above, so this
fragment is not compiled on its own.

`app::root` returns any type that implements `View`. The concrete type can be a
container, a custom view, or `impl View`.

## Split construction from rendering

View builders run whenever a view is created. Reactive closures run whenever
their dependencies change. Keeping the two separate makes it easier to reason
about state:

```rust
# use sequoia_ui::prelude::*;
pub struct State {
    pub query: Signal<String>,
    pub results: Signal<Vec<String>>,
}
```

Pass clones of the signals into views and handlers rather than passing the
whole state through many layers.

## When to use `.seq`

Sequoia can also load a UI from a `.seq` file at runtime. A mixed project
usually looks like this:

```text
my-app/
├── Main.seq
└── src/
    └── main.rs
```

The `.seq` language is a good fit for static layout that designers or
tooling can edit. Rust remains the place for application logic, host bindings,
and custom views. See the `.seq` guide for the loader and host bindings.

## Use the workspace for multiple binaries

If an application ships a main binary, a preview binary, and shared views, use
a Cargo workspace:

```toml
[workspace]
members = ["app", "shared"]
resolver = "3"
```

Keep framework dependencies on the facade crate in each binary and put shared
view functions in a library crate.

## Next steps

- Run the bundled examples: [Examples](examples.md).
- Learn how state flows through the tree: Reactivity.
