# Sequoia

A reactive GUI framework for Rust with a declarative `.seq` language.

`sequoia-ui` is the facade crate for the whole framework. Add it as the single
dependency and reach every subsystem through the re-exported modules and the
prelude.

## Installation

```toml
[dependencies]
sequoia-ui = "0.1"
```

Requires Rust 1.88 or newer.

## Quick start

```rust
use sequoia_ui::prelude::*;

fn main() {
    App::new()
        .title("Hello, Sequoia")
        .size(800.0, 600.0)
        .root(
            Column::new()
                .fill_size()
                .child(Text::new("Hello, Sequoia"))
                .child(Button::new("Click me").on_click(|| println!("clicked!"))),
        )
        .run()
        .expect("failed to run the app");
}
```

## Declarative `.seq` files

The framework ships with a declarative language (`sequoia-ui::seq`) that is
parsed and lowered into regular views:

```rust
use sequoia_ui::prelude::*;

fn main() {
    let loaded = SeqLoader::load_file("Main.seq").expect("invalid .seq file");

    App::new()
        .title(loaded.meta.title)
        .size(loaded.meta.size.0, loaded.meta.size.1)
        .theme(loaded.meta.theme)
        .root(loaded.view)
        .run()
        .expect("failed to run the app");
}
```

`SeqLoader` supports loading from disk, inline strings, and in-memory import
maps. Use `HostBindings` to expose Rust signals, actions, and row data to the
`.seq` program.

## Modules

The facade re-exports the framework crates as modules, including:

`animation`, `assets`, `async_bridge`, `core`, `draw`, `elements`, `export`,
`input`, `layout`, `platform`, `reactive`, `render`, `seq`, `style`, `text`,
and `view`.

## License

Licensed under the [MIT License](../../LICENSE-MIT).
