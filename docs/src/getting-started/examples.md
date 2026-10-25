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

## Paint

A larger desktop application in `example/paint` with a dock layout, toolbars,
file handling, and canvas tools.

```text
cargo run -p paint
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
