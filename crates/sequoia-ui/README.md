# Sequoia UI

A native, reactive GUI framework for Rust built on `wgpu`.

`sequoia-ui` is the facade crate for the whole framework. Add this one
dependency and reach the platform loop, retained view tree, widgets, layout,
styling, rendering, animation, async bridge, and export tools through
re-exported modules and the prelude.

Sequoia keeps a retained view tree and updates it with fine-grained signals.
There is no Virtual DOM, no WebView, and no JavaScript runtime. The UI is drawn
directly on the GPU with `wgpu`.

## Highlights

- **Native rendering.** `wgpu` on Metal, Vulkan, DX12, or OpenGL. No browser
  engine and no HTML layer.
- **Fine-grained reactivity.** `Signal`, `Memo`, `Store`, and effects update
  only the views that read them. No diff step.
- **Rust-first API.** Build the entire interface in Rust with views, widgets,
  and signals. No template language or code generator is required.
- **Layout system.** `Row`, `Column`, `Wrap`, `Grid`, `Stack`, `SplitPanel`,
  and `DockArea` cover linear flow, fixed tracks, layering, resizable panes, and
  IDE-style docking.
- **Widget library.** Buttons, text fields, text areas, checkboxes, switches,
  sliders, dropdowns, color pickers, progress bars, tabs, tables, tree views,
  virtual lists, tooltips, toasts, dialogs, sheets, drawers, and command
  palettes.
- **Text and input.** Shaping, wrapping, rich text, IME, keyboard and pointer
  events, drag and drop, and accessibility metadata.
- **Motion and effects.** Tweens, springs, keyframes, presence animations,
  backdrop blur, masks, typed shaders, offscreen passes, and effect chains.
- **Desktop integration.** Multi-window support, global hotkeys, clipboard,
  native file dialogs, recent files, and headless HiDPI PNG export.
- **Async bridge.** Futures and timers run through an executor that delivers
  results on the UI thread.

## Installation

```toml
[dependencies]
sequoia-ui = "0.1.4"
```

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

## State and updates

A `Signal<T>` is the state. Reading it inside a view or a `Text::with` closure
registers a dependency; writing it rebuilds only the affected part of the
tree. `update` mutates in place when `T: PartialEq`.

```rust
let name = Signal::new(String::from("Sequoia"));

let greeting = Text::with({
    let name = name.clone();
    move || format!("Hello, {}", name.get())
});

let rename = Button::new("Rename").on_click({
    let name = name.clone();
    move || name.set(String::from("Sequoia UI"))
});
```

Use `Memo` for derived values, `Store` for structured state, and effects or
the async bridge for side effects. State is scoped to the current UI thread by
default; `sequoia_ui::async_bridge` moves work off the thread and returns
results through the runtime.

## Layout and widgets

`Row` and `Column` are the primary layout containers. They support gaps,
padding, flex weights, alignment, and fill sizing. `Wrap` flows children onto
new lines, `Grid` places them in fixed tracks, and `Stack` layers them with
optional pinning.

```rust
let toolbar = Row::new()
    .gap(8.0)
    .padding(12.0)
    .child(Button::new("Open"))
    .child(Spacer::flex(1.0))
    .child(Text::new("Ready"));

let sidebar = Column::new()
    .width(220.0)
    .gap(8.0)
    .padding(16.0)
    .child(Text::new("Layers").bold())
    .child(Text::new("Background"))
    .child(Text::new("Card"));

let body = Row::new()
    .fill_size()
    .child(sidebar)
    .child(Container::new().fill_size().padding(16.0).child(Text::new("Canvas")));

let page = Column::new().fill_size().child(toolbar).child(body);
```

For desktop applications, `SplitPanel` provides resizable panes and `DockArea`
provides IDE-style panels with floating windows and drag-and-drop docking.

The widget modules live under `sequoia_ui::elements` and are re-exported from
the prelude. Use `Form` and `Field` for validation, `List` for keyed rows,
`TabStrip` for tabs, and `ToastQueue` or `Dialog` for feedback and overlays.

## GPU effects

Custom shaders are written with the `shader!` macro. The macro validates WGSL
at compile time and generates a typed Rust struct for the uniforms. Register
the generated type with `App::shader_type::<T>()`, then draw it with
`Canvas::with_shader_pass` or pipe captured content through
`Canvas::with_effect_pass` and `ShaderChain`.

The framework also ships backdrop blur, gradients, masks, and reusable effect
chains for effects that do not need a custom shader. See
[Custom shaders](https://nurt-studio.github.io/Sequoia-UI/guides/shaders.html)
for complete examples.

## Modules

The facade re-exports the framework crates as modules:

| Module | Responsibility |
| --- | --- |
| `animation` | Tweens, springs, keyframes, presence |
| `assets` | Icons, SVGs, photos, fonts, asset registry |
| `async_bridge` | Futures and timers on the UI thread |
| `core` | Geometry, colors, render lists, render targets |
| `draw` | Vector paths, fills, strokes, rasterization |
| `elements` | Buttons, inputs, forms, tables, overlays |
| `export` | Headless HiDPI PNG export |
| `input` | Keyboard, pointer, and IME event types |
| `layout` | Flex, grid, wrap, masonry, and track layout |
| `platform` | Window loop, windows, clipboard, native dialogs |
| `reactive` | Signals, memos, stores, effects, undo |
| `render` | `wgpu` renderer and surface management |
| `seq` | Experimental `.seq` parser and loader |
| `shader` | Typed shader runtime, WGSL stdlib, math types |
| `style` | Themes, visual states, shadows, gradients |
| `text` | Shaping, wrapping, glyphs, rich text |
| `view` | Retained view tree, runtime, layouts, docking |

## Examples

The repository contains runnable examples. Run them from the workspace root:

```text
cargo run -p sequoia-ui --example counter
cargo run -p sequoia-ui --example layout
cargo run -p sequoia-ui --example showcase
cargo run -p sequoia-ui --example form
cargo run -p sequoia-ui --example todo
cargo run -p sequoia-ui --example tabs
cargo run -p sequoia-ui --example split-panel
cargo run -p sequoia-ui --example shader_gallery
```

`showcase` is the broadest widget tour. `form`, `todo`, and `tabs` are smaller
focused applications. The repository root also contains `vision-design` and
`paint`, two complete desktop applications built with the same public API.

## Experimental `.seq` support

The repository includes an experimental declarative `.seq` format, loaded with
`SeqLoader`. It is useful for prototyping static layouts, but it is still
evolving and is not required for normal applications. The Rust API is the
supported path.

## Documentation

- Book: <https://nurt-studio.github.io/Sequoia-UI/>
- Book source: <https://github.com/Nurt-studio/Sequoia-UI/tree/main/docs/src>
- API reference: <https://docs.rs/sequoia-ui>
- Repository: <https://github.com/Nurt-studio/Sequoia-UI>
- Main README: <https://github.com/Nurt-studio/Sequoia-UI#readme>

## License

Licensed under the MIT License. See the repository's `LICENSE-MIT` file.
