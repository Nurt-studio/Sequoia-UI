# Examples

All examples live in `crates/sequoia-ui/examples` and run from the workspace
root. They need no extra setup beyond a working `wgpu` adapter.

```text
cargo run -p sequoia-ui --example counter
cargo run -p sequoia-ui --example layout
cargo run -p sequoia-ui --example showcase
cargo run -p sequoia-ui --example form
cargo run -p sequoia-ui --example photo_lab
cargo run -p sequoia-ui --example seq_dashboard
cargo run -p sequoia-ui --example split-panel
cargo run -p sequoia-ui --example wgpu_surface
cargo run -p sequoia-ui --example wgpu_surface_cached
```

## Counter

The smallest complete application: one `Signal<i32>`, a `Text` that reads it,
and two buttons that write it. Use it as the template for a new project.

## Layout

A tour of `Row`, `Column`, `Grid`, and flex sizing: fixed, fill, and content
lengths, gaps, padding, alignment, and nesting. Everything is static, so it is
the easiest example to edit while learning the box model.

## Showcase

The broadest widget tour in the repository. It groups buttons, inputs,
selection controls, progress indicators, navigation, collections, overlays,
and typography on a single scrollable canvas. Start here when you are looking
for the name of a widget or a style preset.

## Split panel

A resizable two-pane layout. It shows how to keep a split ratio in a signal,
clamp it while dragging, and rebuild only the panes that depend on it.

## Form

A focused form application: text fields with validation, a checkbox, a slider,
and a submit button that stays disabled until the input is valid. It shows how
to derive `Memo` values from several signals and wire them into widget state.

## Photo lab

`photo_lab` loads `assets/starry-night.jpg`, samples it with a `Sampler2D`
photo shader, and offers three 33³ `.cube` tables through a `Sampler3D`:
**Teal Orange**, **Bleach Bypass**, and **Amber Film**. Sliders control curve,
lift, saturation, and warmth; the preset picker restores the defaults.

```text
cargo run -p sequoia-ui --example photo_lab
```

The LUTs are ordinary Adobe/IRIDAS `.cube` files under `examples/assets/luts`.
Only 3D tables with the default `DOMAIN_MIN`/`DOMAIN_MAX` are accepted.

## Seq release board

`seq_dashboard` is a declarative example with **no animation**. The window is
described in `examples/seq_dashboard.seq` and loaded with `include_seq!`, so the
props and callbacks are type-checked when the example is built:

```text
cargo run -p sequoia-ui --example seq_dashboard
```

```text
component Meter(label: String, value: Float) {
  Column(gap: 8, flex: 1) {
    Text(text: label, size: 12, color: #8A93A6)
    Container(width: 48 + value * 240, height: 9, radius: 5) {}
  }
}

app ReleaseBoard {
  prop title: String
  prop coverage: Float
  on refresh()
  window Main {
    Column(padding: 24, gap: 16) {
      Text(text: title, size: 20 + coverage * 12)
      Meter(label: "Coverage", value: coverage)
      Slider(value: coverage, min: 0, max: 1)
    }
  }
}
```

The example is deliberately limited to the declarative feature set added in
0.1.6: a parameterised component, reactive attribute expressions such as
`size: 20 + coverage * 12`, two-way `TextField`/`Checkbox` bindings, `If`
subtrees, and typed host callbacks. There are no `enter`, `exit`, `behavior`,
spring, or tween declarations; every visual change comes from state.

## GPU surfaces

Two small examples embed a raw `wgpu` pipeline in an ordinary layout through
`sequoia_ui::platform::{WgpuSurface, WgpuFrame, WgpuRenderer}`:

- `wgpu_surface` draws a fullscreen-triangle gradient in direct mode.
- `wgpu_surface_cached` keeps an offscreen texture and redraws it only when
  `is_dirty` is true; three buttons change the tint through
  `surface.renderer()`.

```text
cargo run -p sequoia-ui --example wgpu_surface
cargo run -p sequoia-ui --example wgpu_surface_cached
```

Both examples use `frame.region` for the viewport and scissor rectangle and
load the existing frame with `wgpu::LoadOp::Load`. See
[Embedded GPU surfaces](../guides/gpu-surfaces.md) for the full contract.

## Complete applications

The workspace root also contains two larger applications:

```text
cargo run -p paint
cargo run -p vision-design
```

- `paint` is a desktop painting app with a dock layout, toolbars, and canvas
  tools.
- `vision-design` is the editor from the README screenshot: canvas, layers,
  property inspector, and code generation.

## Read examples like documentation

Each example is a single `main.rs` (plus a `.seq` file for `seq_dashboard`).
Read them in this order when learning the API:

1. `counter` for signals and events.
2. `layout` for the box model.
3. `form` for derived state and validation.
4. `showcase` for the widget inventory.
5. `seq_dashboard` for the declarative language.

## Add your own example

Drop a new file in `crates/sequoia-ui/examples/` and run it with
`cargo run -p sequoia-ui --example <name>`. Cargo picks up the file
automatically; no registration is needed. Keep examples self-contained and
load assets from `examples/assets` through `env!("CARGO_MANIFEST_DIR")`.
