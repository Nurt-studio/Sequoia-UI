# Examples

The repository contains runnable examples that exercise the framework without
extra setup. Run them from the workspace root.

## Counter

A minimal reactive application with three buttons and a live label.

```text
cargo run -p sequoia-ui --example counter
```

The example is in
[`crates/sequoia-ui/examples/counter.rs`](https://github.com/Nurt-studio/Sequoia-UI/blob/main/crates/sequoia-ui/examples/counter.rs).

## Layout

A single window that demonstrates `Column`, `Row`, flex weights, `Grid`,
`Stack`, `Wrap`, and `DockArea`.

```text
cargo run -p sequoia-ui --example layout
```

The example is in
[`crates/sequoia-ui/examples/layout.rs`](https://github.com/Nurt-studio/Sequoia-UI/blob/main/crates/sequoia-ui/examples/layout.rs).

## Showcase

A broad tour of the widget library: forms, overlays, navigation, feedback, and
theming in one window.

```text
cargo run -p sequoia-ui --example showcase
```

## Split panel

A resizable two-level layout built on `SplitPanel`, with an outer horizontal
split and an inner vertical split.

```text
cargo run -p sequoia-ui --example split-panel
```

## Form

A small account form with text and number fields, validation rules, a radio
group, a slider, and a submit button. It shows how `Form` coordinates the
fields it registers.

```text
cargo run -p sequoia-ui --example form
```

The example is in
[`crates/sequoia-ui/examples/form.rs`](https://github.com/Nurt-studio/Sequoia-UI/blob/main/crates/sequoia-ui/examples/form.rs).

## Todo

A keyed list with stable IDs, toggling, removal, and derived counts. It shows
how a `List` rebuilds from a `Signal<Vec<T>>` while keeping element state.

```text
cargo run -p sequoia-ui --example todo
```

The example is in
[`crates/sequoia-ui/examples/todo.rs`](https://github.com/Nurt-studio/Sequoia-UI/blob/main/crates/sequoia-ui/examples/todo.rs).

## Tabs

A tabbed window built with `TabStrip` and a custom view that rebuilds its body
when the active tab changes.

```text
cargo run -p sequoia-ui --example tabs
```

The example is in
[`crates/sequoia-ui/examples/tabs.rs`](https://github.com/Nurt-studio/Sequoia-UI/blob/main/crates/sequoia-ui/examples/tabs.rs).

## Seq hello

A window described entirely in `.seq` source and loaded with
`SeqLoader::load_str_with`. The button calls a Rust action through
`HostBindings`. The `.seq` format is experimental; the Rust API is the
supported path.

```text
cargo run -p sequoia-ui --example seq_hello
```

The example is in
[`crates/sequoia-ui/examples/seq_hello.rs`](https://github.com/Nurt-studio/Sequoia-UI/blob/main/crates/sequoia-ui/examples/seq_hello.rs).

## Wgpu surface

A custom `wgpu` pipeline embedded in a normal `Column`/`Row` layout. The side
panel pauses or resets the animation through `WgpuSurface::renderer()`, showing
how application widgets and raw GPU drawing share state.

```text
cargo run -p sequoia-ui --example wgpu_surface
```

The example is in
[`crates/sequoia-ui/examples/wgpu_surface.rs`](https://github.com/Nurt-studio/Sequoia-UI/blob/main/crates/sequoia-ui/examples/wgpu_surface.rs).
See [Embedded GPU surfaces](../guides/gpu-surfaces.md) for the render contract,
cached mode, formats, and input handling.

## Shaders

`shader_gallery` renders a blurred card over a background and shows a custom
effect chain built with the `shader!` macro.

```text
cargo run -p sequoia-ui --example shader_gallery
```

See [Custom shaders](../guides/shaders.md) for the macro, registration, and
offscreen passes it uses.

## Paint

A larger desktop application in `example/paint` with a dock layout, toolbars,
file handling, and canvas tools.

```text
cargo run -p paint
```

## Vision design

A design-tool example in `example/vision-design` with a canvas editor,
component tree, property inspector, and `.seq`/Svelte code generation.

```text
cargo run -p vision-design
```

## Read examples like documentation

Examples are ordinary binaries, not snippets. When adapting one:

1. Copy only the part that demonstrates the API you need.
2. Replace hard-coded colors and labels with your own values.
3. Move state into signals or stores instead of keeping it in locals if it
   must survive rebuilds.
4. Run the example again after the change; layout and reactivity bugs are
   easiest to spot in a running window.

## Add your own example

Cargo discovers files under `examples/` automatically:

```text
my-app/
└── examples/
    └── gallery.rs
```

Run it with `cargo run --example gallery`. Examples are a good place for
reproductions and visual checks, because they do not affect the application
binary.
