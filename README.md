# Sequoia

Native, reactive GUI framework for Rust built on `wgpu` — a retained view tree with fine-grained signals. No Virtual DOM, no WebView.

## Install

```toml
[dependencies]
sequoia-ui = "0.1"
```

`sequoia-ui` is the single facade crate and re-exports the whole framework:
platform, views, widgets, layout, styling, rendering, animation, and async.
You do not need to clone this repository or add the individual crates by hand.

Requires Rust 1.88 or newer.

## Quick start

```rust
use sequoia_ui::prelude::*;

fn main() {
    let count = Signal::new(0i32);
    let display = count.clone();

    App::new()
        .title("Counter")
        .size(480.0, 320.0)
        .root(
            Column::new()
                .gap(12.0)
                .padding(24.0)
                .child(Text::with(move || format!("Count: {}", display.get())))
                .child(
                    Button::new("Increment")
                        .on_click(move || count.update(|n| *n += 1)),
                ),
        )
        .run()
        .expect("failed to run the app");
}
```

## Documentation

- API docs: <https://docs.rs/sequoia-ui>
- Crate: <https://crates.io/crates/sequoia-ui>
- Modules: `animation`, `assets`, `async_bridge`, `core`, `draw`, `elements`,
  `export`, `input`, `layout`, `platform`, `reactive`, `render`, `seq`, `style`,
  `text`, `view`.

## License

Licensed under the [MIT License](LICENSE-MIT).
