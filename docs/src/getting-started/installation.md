# Installation

Sequoia is distributed as a set of crates. Applications depend on the facade
crate, `sequoia-ui`, which re-exports the whole framework.

## Requirements

- Rust 1.88 or newer.
- A desktop platform supported by `winit` and `wgpu` (Windows, macOS, or
  Linux).
- A working GPU driver with a supported backend: DirectX 12 or Vulkan on
  Windows, Metal on macOS, and Vulkan on Linux.

No web toolchain, JavaScript runtime, or system web view is required.

## Add the dependency

Create a binary crate and add Sequoia to `Cargo.toml`:

```text
cargo new my-app
cd my-app
cargo add sequoia-ui
```

The resulting dependency section looks like this:

```toml
[dependencies]
sequoia-ui = "0.1"
```

Cargo resolves the individual framework crates through the facade; do not add
`sequoia-view`, `sequoia-reactive`, or other internals by hand.

## Import the prelude

Most applications need a single import:

```rust
use sequoia_ui::prelude::*;
```

The prelude exposes `App`, `Signal`, common views such as `Column`, `Row`, and
`Text`, built-in widgets, layout types, colors, and animation helpers. Import
subsystem modules explicitly when a type is not in the prelude:

```rust
use sequoia_ui::prelude::*;
use sequoia_ui::render::GpuOptions;
```

Every framework subsystem is available as a module from the facade:

```text
sequoia_ui::{animation, assets, async_bridge, core, draw, elements, export,
             input, layout, platform, reactive, render, seq, style, text, view}
```

## Verify the toolchain

Check that the installed compiler meets the minimum version:

```text
rustc --version
```

Now continue with [Your first app](first-app.md).
