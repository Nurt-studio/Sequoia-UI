# Text

Text is the most common thing a UI draws. Sequoia shapes text through
cosmic-text, caches shaped runs, and draws glyphs from an atlas. The `Text`
view exposes the common controls without hiding the underlying metrics.

## Static text

```rust
# use sequoia_ui::prelude::*;
let title = Text::new("Sequoia UI")
    .size(28.0)
    .bold()
    .color(Color::WHITE);

let caption = Text::new("Fine-grained Rust views")
    .size(12.0)
    .color(Color::hex(0x9A9A9A))
    .letter_spacing(0.0);
```

Available weight helpers are `light`, `normal`, `medium`, `semibold`, `bold`,
and `black`, plus `weight(FontWeight)` for an explicit value. `italic`,
`oblique`, and `upright` set the font style. `font("Inter")` selects a family
registered with the application.

## Reactive text

`Text::with` builds the string inside a reactive scope. The view subscribes to
every signal read in the closure and rebuilds when one changes:

```rust
# use sequoia_ui::prelude::*;
let count = Signal::new(0i32);
let shown = count.clone();

let label = Text::with(move || format!("{} items", shown.get()))
    .size(14.0);

count.set(3);
```

This is more efficient than recomputing the whole tree: only the text view is
marked dirty. Use `Text::with` for labels that depend on signals, and
`Text::new` for content fixed at construction time.

## Wrapping and overflow

By default text lays out on one line. `wrap_width` enables wrapping at a fixed
logical width, while `.wrap()` uses the width available from layout:

```rust
# use sequoia_ui::prelude::*;
let paragraph = Text::new(
    "A long description that should flow onto multiple lines instead of \
     stretching the containing panel beyond its allotted width.",
)
.size(13.0)
.wrap()
.line_height(1.45);
```

For narrow columns, switch from word wrapping to character wrapping:

```rust
# use sequoia_ui::prelude::*;
let narrow = Text::new("verylongunbrokenidentifier")
    .wrap_width(90.0)
    .wrap_mode(WrapMode::Character);
```

Limit the number of lines and choose where the ellipsis appears:

```rust
# use sequoia_ui::prelude::*;
let summary = Text::new("A preview that is capped at two lines...")
    .wrap_width(240.0)
    .max_lines(2)
    .elide(ElideMode::Right);
```

`ElideMode::Left` and `ElideMode::Middle` are useful for paths and URLs where
the beginning or the end is more informative. `truncate(max_width)` hard-clips
a single line, and `.truncate_fill()` truncates to the available layout width.

## Alignment and vertical placement

Horizontal alignment is a property of the surrounding layout. Use a
`Column`/`Row` with `align_items`/`justify_content`, or a fixed-width
container, and let the text fill it:

```rust
# use sequoia_ui::prelude::*;
let header = Container::new()
    .fill_width()
    .child(Text::new("Centered title").size(16.0));
```

Vertical alignment inside a text box is controlled with `align_y`:

```rust
# use sequoia_ui::prelude::*;
# use sequoia_ui::text::TextVAlign;
let cell = Text::new("Middle")
    .height(40.0)
    .align_y(TextVAlign::Center);
```

Line height can be a multiple of the font size (`line_height`) or an absolute
pixel value (`line_height_px`). Absolute values are handy when aligning text
with icons or table rows.

## Rich text

`RichText` draws a sequence of spans. Spans can change color, background,
weight, style, family, size, and decoration, and can carry links or mentions:

```rust
# use sequoia_ui::prelude::*;
let message = RichText::new(vec![
    TextSpan::new("Build ").color(Color::hex(0xAAAAAA)),
    TextSpan::new("#42").bold().color(Color::hex(0x4DA3FF)),
    TextSpan::new(" succeeded in ").color(Color::hex(0xAAAAAA)),
    TextSpan::new("1.8s").code(true),
    TextSpan::new(".").color(Color::hex(0xAAAAAA)),
])
.size(14.0)
.wrap_width(420.0);
```

Rich text supports `.max_lines`, `.line_height`, `.align(RichAlign::Center)`,
`.centered()`, `.right_aligned()`, and `.on_span_click(|index| ...)`. Span
indices refer to the order passed to `RichText::new`, which makes links and
mentions straightforward to handle.

## Fonts

Register custom fonts before mounting the interface. The bytes are owned by the
registry and can be shared by multiple families:

```rust,no_run
# use sequoia_ui::prelude::*;
fn main() {
    let inter = std::fs::read("assets/Inter-Regular.ttf").expect("font file");
    let view = Text::new("Custom typeface").font("Inter").size(18.0);

    App::new()
        .title("Fonts")
        .font("Inter", inter)
        .default_font("Inter")
        .root(view)
        .run()
        .expect("event loop failed");
}
```

When a requested family is missing, the text system falls back to the default
font. Keep a known-good default and treat custom fonts as enhancements.

## Measurement

Inside painting code you can measure a string before placing it. `PaintCtx` and
`Painter` expose text measurement:

```rust
# use sequoia_ui::prelude::*;
let canvas = Canvas::new()
    .width(320.0)
    .height(80.0)
    .on_draw(|p| {
        let width = p.text_width("Measured", 16.0);
        let x = (p.size().width - width) * 0.5;
        p.text_tl("Measured", Point::new(x, 24.0), 16.0, Color::WHITE);
    });
```

Measurement uses the same shaping cache as rendering, so repeated calls are
cheap. For text that changes every frame, prefer normal `Text` views; they
participate in damage tracking and avoid forcing unrelated repaints.

## Internationalization

`I18n` stores translations and exposes the active locale as a signal. Reading
`t` inside `Text::with` makes the label update when the locale changes:

```rust
# use sequoia_ui::prelude::*;
let i18n = I18n::new("en");
i18n.add("en", "save", "Save");
i18n.add("pl", "save", "Zapisz");

let save_label = {
    let i18n = i18n.clone();
    Text::with(move || i18n.t("save"))
};

i18n.set_locale("pl");
```

Missing keys fall back to the configured fallback locale, then to the key
itself. `has` and `missing_in` help validate translation tables. The
[Localization guide](../guides/localization.md) shows a small file loader and
manual placeholder formatting; `I18n` does not choose a file format or provide
pluralization.

## Performance notes

- Shaped runs are cached by text, size, weight, family, style, spacing, and
  wrapping inputs.
- Changing an unrelated signal does not reshape text.
- Very long documents should use `RichText` only for visible content, or a
  virtualized list; shaping is linear in the number of laid-out lines.
- Prefer `.truncate_fill()` over measuring manually when a single line must
  fit.
- Glyph rasterization is cached in an atlas; many labels with the same style
  reuse the same glyphs.

## See also

- [Styling](styling.md) for `TextStyle` and theme colors.
- [Rendering](rendering.md) for the glyph pipeline.
- [Localization](../guides/localization.md) for translation workflows.
