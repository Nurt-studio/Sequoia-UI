# Headless export

`sequoia-export` renders a view tree to an RGBA image or a PNG file without
opening a window. It runs the normal layout and paint pipeline, then rasterizes
the resulting `RenderList` on the CPU. Use it for documentation screenshots,
thumbnails, previews, and deterministic image checks in CI.

Export is not a screenshot of a live window. Build the same view tree you want
to capture, give it the data it needs, and render that tree. The exporter does
not drive the application event loop, so load async data and settle animations
before calling it.

The facade crate re-exports the exporter, so an application that already
depends on `sequoia-ui` does not need another dependency:

```rust
use sequoia_ui::export::{ExportOptions, SCALE_2X, render_to_image};
```

## Render an image

`render_to_image` lays out the root in a logical rectangle and returns an
`image::RgbaImage`:

```rust
# use sequoia_ui::prelude::*;
use sequoia_ui::export::{ExportOptions, SCALE_2X, render_to_image};

let root = Panel::new().background(Color::RED).fill_size();
let opts = ExportOptions::new(320.0, 180.0).scale(SCALE_2X);

let image = render_to_image(root, &opts).expect("render the view");
assert_eq!((image.width(), image.height()), (640, 360));
```

The root is laid out at `width` by `height` logical pixels. Use a root that
fills that rectangle, such as `Panel::fill_size`, or the rest of the image is
painted with the background color. The root must be `'static`; clone owned
data, strings, and signals into it instead of borrowing from a shorter-lived
value.

The returned image is a plain RGBA buffer. You can inspect pixels directly,
encode it with the `image` crate, or pass it to another consumer.

## Logical size and scale

`ExportOptions::width` and `height` are logical pixels, matching the units used
by layout and view builders. `scale` converts them to physical pixels:

```text
output width  = round(width  * scale)
output height = round(height * scale)
```

The default scale is `SCALE_2X`, not 1x. Use `SCALE_1X` when you need an image
whose pixel dimensions match the layout exactly:

```rust
# use sequoia_ui::prelude::*;
use sequoia_ui::export::{ExportOptions, SCALE_1X, SCALE_3X};

let exact = ExportOptions::new(320.0, 180.0).scale(SCALE_1X);
let retina = ExportOptions::new(320.0, 180.0).scale(SCALE_3X);

assert_eq!(exact.scale, 1.0);
assert_eq!(retina.scale, 3.0);
```

`SCALE_1X`, `SCALE_2X`, `SCALE_3X`, `SCALE_4X`, and `SCALE_8X` are convenience
constants. Any finite value between `1.0` and `8.0` is accepted. Values outside
that range are clamped, and a non-finite scale falls back to 1x. The physical
size is rounded to whole pixels.

Large scales are expensive. A 1600x900 layout at 8x is 12800x7200 pixels and
requires hundreds of megabytes for the image and compositing layers. Prefer 2x
for normal documentation and reserve 3x or 4x for images that need extra
detail.

## Background and theme

The default theme is `Theme::dark()`. If no background is set, the exporter
uses the theme's `background` color, so the image is opaque:

```rust
# use sequoia_ui::prelude::*;
use sequoia_ui::export::{ExportOptions, SCALE_2X};

let opts = ExportOptions::new(320.0, 180.0)
    .scale(SCALE_2X)
    .theme(Theme::light());
```

`ExportOptions::background` overrides the theme background without changing
widget colors. `transparent()` explicitly requests a transparent background,
which is useful for icons, logos, and images that will be composited elsewhere:

```rust
# use sequoia_ui::prelude::*;
use sequoia_ui::export::ExportOptions;

let opts = ExportOptions::new(64.0, 64.0).transparent();
assert_eq!(opts.background, Some(Color::TRANSPARENT));
```

The theme is also installed in the temporary environment used during export, so
widgets that have no explicit colors use the selected theme. If a view sets its
own colors, those colors win.

## Save a PNG

`export_png` renders the view and writes it to disk:

```rust,no_run
# use sequoia_ui::prelude::*;
use sequoia_ui::export::{ExportOptions, SCALE_2X, export_png};

# let path = std::env::temp_dir().join("sequoia-preview.png");
let saved = export_png(
    Panel::new().background(Color::BLUE).fill_size(),
    &ExportOptions::new(320.0, 180.0).scale(SCALE_2X),
    &path,
)
.expect("write the PNG");

assert_eq!(saved, path);
```

`export_png` creates missing parent directories, overwrites an existing file,
and returns the path it wrote. The image format is chosen from the file
extension, so use `.png` for a PNG. The function is synchronous and blocks while
rasterizing, so plan for the pause when exporting a large image.

## Errors and limits

The exporter rejects sizes that cannot produce a safe image. The limits are:

- `MAX_EXPORT_DIMENSION`: 16384 pixels in either dimension.
- `MAX_EXPORT_PIXELS`: 134,217,728 pixels total, across both dimensions.
- `MAX_EXPORT_SCALE`: 8x.

Invalid input is reported as `ExportError`:

```rust
# use sequoia_ui::prelude::*;
use sequoia_ui::export::{ExportError, ExportOptions, render_to_image};

let result = render_to_image(
    Panel::new().fill_size(),
    &ExportOptions::new(0.0, 0.0),
);

match result {
    Ok(_) => panic!("zero-sized exports must fail"),
    Err(ExportError::InvalidSize { width, height }) => {
        assert_eq!((width, height), (0.0, 0.0));
    }
    Err(other) => panic!("unexpected export error: {other}"),
}
```

`ExportError` is `#[non_exhaustive]`, so match it with a wildcard arm. The
variants are:

- `InvalidSize { width, height }` for non-finite, zero, or negative sizes.
- `TooLarge { w, h }` when the physical output exceeds a dimension or pixel
  limit.
- `Io(String)` when the destination directory cannot be created or the image
  cannot be saved.

The size check happens before any rendering. A view that paints an enormous
canvas is still limited by the output rectangle, not by its internal geometry.

## What is rendered

The exporter mounts a fresh runtime, lays out the view, and paints it with a
`TextSystem` scaled to the requested output scale. It rasterizes:

- quads, borders, rounded corners, and clipping;
- text runs, including glyph color and text transforms;
- images from files or RGBA data;
- outer and inset shadows;
- waveform bars;
- linear and radial gradients;
- opacity groups, composited once per group.

It does not create a native window or a GPU surface, so window chrome, menus,
platform dialogs, and GPU-only effects are not part of the image. Backdrop
blur/filter effects are recorded outside the normal render items and are not
reproduced by the CPU rasterizer. The rasterizer also uses bilinear image
sampling and applies transforms to text; other item types are exported in their
untransformed geometry.

The temporary environment contains `AssetRegistry::with_builtin_icons()` and
the default text fonts. It does not inherit an application's custom asset
registry, so a view that depends on a custom icon or font should provide a
fallback or use a rendering variant that does not require it.

## Practical workflow

For repeatable images, keep the export recipe explicit:

1. Build the root from the same view types and current data used by the app.
2. Choose a fixed logical size and scale.
3. Select a theme and background instead of relying on ambient state.
4. Render once and assert dimensions or key pixels in a test.
5. Save with a stable file name and extension.

A small CI check can catch layout or theme regressions without a display
server:

```rust
# use sequoia_ui::prelude::*;
use sequoia_ui::export::{ExportOptions, SCALE_1X, render_to_image};

let image = render_to_image(
    Panel::new().background(Color::RED).fill_size(),
    &ExportOptions::new(64.0, 32.0).scale(SCALE_1X),
)
.expect("render");

assert_eq!((image.width(), image.height()), (64, 32));
let center = image.get_pixel(32, 16).0;
assert!(center[0] > 200 && center[1] < 60 && center[2] < 60);
```

## See also

- [Rendering and frames](../concepts/rendering.md)
- [Assets](assets.md)
- [Testing views](testing.md)
- [Custom views](custom-views.md)
