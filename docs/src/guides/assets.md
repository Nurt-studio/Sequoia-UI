# Assets

`AssetRegistry` holds an application's icons and bundled fonts. It is also the
entry point for decoding photos safely. A normal `App` starts with the bundled
icons already registered; use the registry when you need your own SVGs, font
files, or image dimensions.

## Registering assets

The concise path is `App::icon`, `App::font`, and `App::default_font`:

```rust,no_run
# use sequoia_ui::prelude::*;
const LOGO: &str = r#"<svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="9"/></svg>"#;

let app = App::new()
    .icon("brand:logo", LOGO)
    .font("Brand", std::fs::read("assets/Brand-Regular.ttf").expect("font file"))
    .default_font("Brand");
# let _ = app;
```

`App::icon` logs and skips invalid SVG. Use `App::try_icon` when you want the
`SvgError` back, or build an `AssetRegistry` directly:

```rust,no_run
# use sequoia_ui::prelude::*;
const LOGO: &str = r#"<svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="9"/></svg>"#;

let assets = AssetRegistry::with_builtin_icons();
assets.register_icon("brand:logo", LOGO).expect("valid SVG");
assets.register_font(
    "Brand",
    std::fs::read("assets/Brand-Regular.ttf").expect("font file"),
);
assets.set_default_font("Brand");

let app = App::new().assets(assets);
# let _ = app;
```

`App::new()` already contains the built-in icon set. `App::assets` replaces the
whole registry, so `AssetRegistry::new()` drops the built-ins; use
`with_builtin_icons()` when you want both. Register fonts before calling
`App::run`; the text system installs them during startup. `AssetRegistry` is
cloneable, but it is reference-counted and single-threaded: do not move it to a
worker thread.

## Icons

`Icon` is a leaf that requests a rasterized sprite from the registry:

```rust
# use sequoia_ui::prelude::*;
let icon = Icon::new("search")
    .size(18.0)
    .color(Color::hex(0x8b93a1));
# let _ = icon;
```

The default size is 16 logical pixels and the default color is the theme text
color. `Icon` keeps its aspect ratio, centers the glyph in its layout box, and
supports the usual `width`, `height`, `padding`, and `margin` builders. The
sprite is rasterized at `size * scale_factor`, so a 16 px icon on a 2x display
requests 32 physical pixels.

Built-in names include `chevron-down`, `chevron-up`, `chevron-left`,
`chevron-right`, `check`, `close`, `plus`, `minus`, `search`, `play`, `pause`,
`folder`, `file`, `sliders`, `trash`, `star`, `move`, `eye`, `image`, and
`lock`. Enumerate the exact set with `icon_names()`:

```rust
# use sequoia_ui::prelude::*;
let assets = AssetRegistry::with_builtin_icons();
let names = assets.icon_names();
assert!(names.iter().any(|name| name == "search"));
```

Icon names may be namespaced with a colon. Lookup first tries the exact name,
then falls back to the part after the colon, so registering `logo` also makes
`brand:logo` resolve. `icons_in_namespace("brand")` lists names that were
registered with that prefix. Names are just map keys; the fallback is useful
for themes that add their own prefix to common icon names.

The SVG's own colors are not preserved. Icons are rasterized as alpha masks
and tinted with `Icon::color` (or the theme text color). Using `currentColor`
in the SVG is fine, but the painted color still comes from the widget.

An unknown icon paints nothing and does not panic. Check at registration or
test time when an icon is required:

```rust
# use sequoia_ui::prelude::*;
const DOT: &str = r#"<svg viewBox="0 0 10 10"><circle cx="5" cy="5" r="4"/></svg>"#;

let assets = AssetRegistry::with_builtin_icons();
assets.register_icon("dot", DOT).unwrap();

assert!(assets.icon("dot").is_some());
assert!(assets.icon("app:dot").is_some());
assert!(assets.icon("missing").is_none());

let sprite = assets.rasterize_icon("dot", 16).unwrap();
assert_eq!((sprite.width, sprite.height), (16, 16));
```

`rasterize_icon` accepts at most `MAX_RASTER_PX` (512) physical pixels and
returns `AssetError::Unknown` for a missing name. The registry caches sprites by
name and size, so request a small number of sizes and let `Icon::size` handle
the common case. The cache is bounded and may evict old entries.

Icons have no accessible label. Treat them as decorative and put the meaning
in adjacent text, a labeled button, or another accessible control.

## SVG subset

Sequoia's icon parser is intentionally small. It supports:

- `<svg>`, `<g>`, `<path>`, `<rect>`, `<circle>`, `<ellipse>`, `<line>`,
  `<polyline>`, and `<polygon>`.
- A `viewBox`, or `width` and `height` as a fallback; one of these is required.
- Absolute and relative path commands, including curves, arcs, and smooth
  variants.
- `translate`, `scale`, `rotate`, `skewX`, `skewY`, and `matrix` transforms.
- Fill rules, `fill`, `stroke`, stroke width/caps/joins/miter limit, dash
  arrays and offsets, and `vector-effect="non-scaling-stroke"`.
- Transform, fill, and stroke properties from the element's `style` attribute.

Not supported: `<defs>`, gradients, `<use>`, `<text>`, embedded `<image>`,
clips, masks, filters, and external CSS. Unknown elements are skipped. A
gradient fill is treated as a solid tint, and all SVG paint colors collapse to
the widget's tint color. Keep icons to stroked or filled paths:

```rust
# use sequoia_ui::prelude::*;
const CHECK: &str = r#"
<svg viewBox="0 0 24 24" fill="none" stroke="currentColor"
     stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
  <path d="M5 12 L10 17 L19 7"/>
</svg>
"#;

let assets = AssetRegistry::new();
assets.register_icon("check-custom", CHECK).unwrap();
assert!(assets.icon("check-custom").is_some());
```

Validate icons during startup rather than discovering a blank icon at runtime.
`AssetRegistry::register_icon` returns `SvgError`; `parse_svg` is public if you
want to inspect an `SvgIcon` before registering it.

## Fonts

Register each font file with an alias, then make that alias the default or use
it explicitly on text:

```rust,no_run
# use sequoia_ui::prelude::*;
let regular = std::fs::read("assets/Brand-Regular.ttf").expect("regular");
let bold = std::fs::read("assets/Brand-Bold.ttf").expect("bold");

let assets = AssetRegistry::new();
assets.register_font("Brand", regular);
assets.register_font("Brand", bold);
assets.set_default_font("Brand");

let app = App::new().assets(assets);
# let _ = app;
```

An alias is resolved to the family reported by the font file. Register the
faces of one family under the same alias; `.weight(FontWeight::BOLD)` then
selects the matching face when the family provides it. Use the alias in
`Text::font("Brand")`; weights and styles work through the normal text builders.

A default font may also name an installed system family, even when no bytes
were registered. Invalid font data is logged and skipped rather than panicking.
The built-in default is Inter. If no custom default is set, text uses that
fallback.

## Images

`Image::file` displays a file path and fills its available space by default.
The painted image is contain-fitted, preserving aspect ratio; unused space
remains empty:

```rust
# use sequoia_ui::prelude::*;
let image = Image::file("assets/hero.png")
    .width(320.0)
    .height(180.0);
# let _ = image;
```

`Image` does not crop to cover, and it has no rounded-corner builder. Wrap it in
a clipping container when you need a crop or rounded frame. `Image` is a leaf,
so it cannot contain a caption; use a `Column` for an image and label.

For an in-memory RGBA image, use `Image::rgba(id, width, height, data)`. The ID
must be stable and unique per texture, and `data` must contain exactly
`width * height * 4` bytes in RGBA order:

```rust
# use sequoia_ui::prelude::*;
let red = vec![255, 0, 0, 255].repeat(4);
let image = Image::rgba(1, 2, 2, red)
    .width(64.0)
    .height(64.0);
# let _ = image;
```

When the pixels change, use `Image::dynamic(id, width, height, data, version)`
and increment `version`; reusing an ID with new data without a new version may
reuse the old texture. `Pixels::to_source()` creates an `ImageSource` for the
lower-level painter API, but `Image::rgba`/`Image::dynamic` are the convenient
widget constructors.

`Image::file` obtains its intrinsic dimensions through the `AssetRegistry`, so
the file must be reachable when the image is laid out. Relative paths are
resolved by the process working directory. If decoding fails, the image paints
nothing; check the path during development.

## Photos and thumbnails

Use the photo helpers instead of reading arbitrary image bytes yourself:

```rust,no_run
# use sequoia_ui::prelude::*;
# fn example() -> Result<(), PhotoError> {
let thumb = load_pixels_thumbnail("photos/cover.jpg", 320)?;
let image = Image::dynamic(
    100,
    thumb.width(),
    thumb.height(),
    thumb.into_data(),
    1,
);
# let _ = image;
# Ok(())
# }
```

- `load_pixels(path)` decodes a full image from disk.
- `load_pixels_thumbnail(path, max_dim)` decodes and downscales so the longest
  side is at most `max_dim`.
- `decode_pixels(bytes)` and `decode_pixels_thumbnail(bytes, max_dim)` do the
  same for bytes already in memory.
- `Pixels::thumbnail(max_dim)` and `Pixels::scaled_to_fit(max_dim)` resize an
  image you already hold.

`MAX_PHOTO_DIM` is 8192 pixels per side and `MAX_PHOTO_BYTES` is 64 MiB of
decoded RGBA. The loaders reject images past either limit, and decode from
memory checks dimensions before allocating. `PhotoError` distinguishes I/O,
oversized, and decode failures. `AssetRegistry::photo_size(path)` probes
dimensions without decoding the pixels.

Decoding a large image is synchronous file and CPU work. Do it off the UI
thread, keep only the resulting `Pixels`, and build the widget on the UI thread.
For lists and grids, decode a thumbnail at the display size instead of keeping
full-resolution photos alive.

## Limits and conventions

- Register icons and fonts once during startup; avoid adding hundreds of unused
  SVGs or font faces.
- Use a small number of icon sizes. Every distinct physical size is a separate
  raster cache entry.
- Icons are monochrome alpha masks; use `Icon::color`, not SVG colors.
- Unknown icons are silent. Validate required names in tests.
- Photos have hard dimension and memory limits; prefer thumbnails for browsing.
- `AssetRegistry` is single-threaded. Do not capture it in async work that runs
  on another thread.
