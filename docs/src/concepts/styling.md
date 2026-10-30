# Styling

Sequoia separates structure from appearance. A view describes *what* is on
screen; a theme and per-widget style values describe *how* it looks. Styles are
plain Rust values, so they can be computed, stored in signals, and shared
between views.

## The theme

`Theme` is a small, copyable set of semantic colors and metrics. Every
application starts with `Theme::dark()` unless another theme is passed to
`App::theme`.

```rust
# use sequoia_ui::prelude::*;
let theme = Theme::dark();

assert_eq!(theme.background, Color::rgb8(0x19, 0x19, 0x19));
assert_eq!(theme.accent, Color::rgb8(0x35, 0xC2, 0xF1));
assert!(theme.radius > 0.0);
```

`Theme::light()` provides the same slots for light interfaces. Because
`Theme` is a normal struct with public fields, you can start from a preset and
override individual values:

```rust
# use sequoia_ui::prelude::*;
let mut theme = Theme::light();
theme.accent = Color::hex(0x5B8DEF);
theme.accent_hover = Color::hex(0x76A3F2);
theme.accent_pressed = Color::hex(0x4477C9);
theme.radius = 6.0;

let app = App::new().title("Styled").theme(theme);
```

Keep theme construction near application startup. Widgets receive the theme
through the runtime; changing `App::theme` later changes the initial palette,
not a live value. For a runtime switch, drive your own `Signal<Theme>` and pass
it to views that need it (see the settings and theme cookbook entries).

## Per-widget styles

Each built-in widget has a dedicated style struct. Every field is optional, so
a style can be a partial override. `ButtonStyle` is representative:

```rust
# use sequoia_ui::prelude::*;
let primary = Button::new("Save").style(ButtonStyle::PRIMARY);
let secondary = Button::new("Cancel").style(ButtonStyle::SECONDARY);
let ghost = Button::new("Learn more").style(ButtonStyle::GHOST);
let outline = Button::new("Details").style(ButtonStyle::OUTLINE);
let danger = Button::new("Delete").style(ButtonStyle::DANGER);

let _ = (primary, secondary, ghost, outline, danger);
```

Available presets are `PRIMARY`, `SECONDARY`, `DANGER`, `GHOST`, `OUTLINE`,
`DARK_FILLED`, and `OUTLINE_ROSE`. Build a custom style from an empty one:

```rust
# use sequoia_ui::prelude::*;
let style = ButtonStyle::new()
    .background(Color::hex(0x2D6CDF))
    .hover(Color::hex(0x3B7DF0))
    .pressed(Color::hex(0x2458B8))
    .text(Color::WHITE)
    .font_size(13.0)
    .font_weight(600)
    .radius(8.0)
    .padding_xy(14.0, 7.0);

let button = Button::new("Publish").style(style);
```

`Button::style_with` derives a style from the current one when you need a
small tweak:

```rust
# use sequoia_ui::prelude::*;
let button = Button::new("Save")
    .style(ButtonStyle::PRIMARY)
    .style_with(|style| style.radius(10.0));
```

## Boxes, borders, and shadows

`BoxStyle` describes the rectangle behind a view. It is used by `Container`,
`Stack`, `Grid`, `DockArea`, and other layout containers.

```rust
# use sequoia_ui::prelude::*;
let panel = BoxStyle::new()
    .background(Color::hex(0x1B1B1F))
    .border(1.0, Color::hex(0x2E2E36))
    .radius(10.0)
    .shadow(ShadowSpec::new(Color::hexa(0x00000066), 18.0).offset(0.0, 6.0))
    .inset_shadow(ShadowSpec::new(Color::hexa(0xFFFFFF10), 1.0));

let card = Container::new()
    .style(panel)
    .padding(16.0)
    .child(Text::new("Card"));
```

Borders can be set per edge. A common example is a panel with a stronger
accent stripe on the left:

```rust
# use sequoia_ui::prelude::*;
let panel = Container::new()
    .background(Color::hex(0x202024))
    .border_left(4.0, Color::hex(0x4DA3FF))
    .border_y(1.0, Color::hex(0x2E2E36))
    .radius(6.0);
```

`ShadowSpec::new(color, blur)` accepts an optional spread and offset. A shadow
is only painted when the color is not transparent and the blur is finite and
positive.

## Text styles

`TextStyle` collects the typographic properties shared by text views:

```rust
# use sequoia_ui::prelude::*;
let heading = TextStyle::new()
    .size(22.0)
    .color(Color::WHITE)
    .weight(700)
    .line_height(1.2);

let label = Text::new("Dashboard")
    .size(heading.size.unwrap_or(14.0))
    .color(heading.color.unwrap_or(Color::WHITE))
    .bold()
    .font("Inter");
```

For rich content, `RichText` accepts a list of `TextSpan` values. Each span
can carry its own color, weight, italics, underline, size delta, and link:

```rust
# use sequoia_ui::prelude::*;
let text = RichText::new(vec![
    TextSpan::new("Sequoia").bold().color(Color::WHITE),
    TextSpan::new(" is a ").color(Color::hex(0xAAAAAA)),
    TextSpan::new("fine-grained").italic(),
    TextSpan::new(" Rust GUI framework.").color(Color::hex(0xAAAAAA)),
]);
```

Tabular data often needs alignment rather than decoration. Use `Text` with a
fixed width and `.align(...)` or wrap the label in a container; layout is
covered in [Layout](layout.md).

## Visual state

Interactive widgets resolve colors from a `VisualState`: normal, hovered,
pressed, focused, selected, and disabled. Style presets already encode sensible
transitions. When you build a custom control, resolve the accent color
consistently:

```rust
# use sequoia_ui::prelude::*;
let theme = Theme::dark();
let mut state = VisualState::default();
state.hovered = true;

assert_eq!(resolve_accent(&theme, state), theme.accent_hover);
```

Primitive views also expose simple interaction feedback without a full style:

```rust
# use sequoia_ui::prelude::*;
let card = Container::new()
    .width(180.0)
    .height(96.0)
    .background(Color::hex(0x26262C))
    .hover(Color::hex(0x303038))
    .radius(8.0);
```

Hover/press feedback is stateful; the runtime records the interaction and
repaints the affected region.

## Backdrop filters

Backdrops blur or dim what has already been painted underneath. They are
independent of the widget style and are useful for sheets, menus, and toasts.

```rust
# use sequoia_ui::prelude::*;
let glass = BackdropBlur::new(
    Container::new()
        .width(240.0)
        .padding(16.0)
        .radius(12.0)
        .background(Color::hexa(0xFFFFFF14))
        .child(Text::new("Frosted panel")),
    Backdrop::MD,
);
```

`Backdrop::NONE`, `XS`, `SM`, and `MD` are ready-made blur levels. Use
`.blur()`, `.dim()`, `.noise()`, and `.radius()` for finer control, or the
`Filter`/`Effect` API on the `Filtered` and `EffectNode` wrappers for
saturation and color operations.

## Where to apply a style

Prefer the widget's own style methods (`Button::style`, `TextField::style`,
`Table::style`) over wrapping it in a `Container`. The widget knows which
parts of its anatomy each field applies to. Use `BoxStyle` when you need a
panel, card, or background, and `Container` when you need padding, layout, or
clipping around children.

Styles compose in this order:

1. The application theme supplies defaults.
2. The widget's style value overrides matching fields.
3. Inline builder calls such as `.radius(4.0)` on a `Container` override the
   style value.

Keep style definitions in named constants or small functions. That makes
them reusable across views and easy to change in one place:

```rust
# use sequoia_ui::prelude::*;
fn sidebar_style() -> BoxStyle {
    BoxStyle::new()
        .background(Color::hex(0x131318))
        .border_right(1.0, Color::hex(0x2A2A32))
}

let sidebar = Container::new().style(sidebar_style()).fill_height();
```

## See also

- [Rendering](rendering.md) explains how styled quads become pixels.
- [Animation](animation.md) covers animating style values.
- [Custom views](../guides/custom-views.md) shows how to paint your own
  widgets.
