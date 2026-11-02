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

## Shaders

Two GPU effect demos built with the `shader!` macro. `progressive_blur` shows a
separable blur with a UV-space ramp; `shader_gallery` renders a blurred card
over a background.

```text
cargo run -p sequoia-ui --example progressive_blur
cargo run -p sequoia-ui --example shader_gallery
```

See [Custom shaders](../guides/shaders.md) for the macro, registration, and
offscreen passes they use.

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
