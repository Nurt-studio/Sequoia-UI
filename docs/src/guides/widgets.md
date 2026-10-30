# Widgets

`sequoia_ui` ships a widget library in `sequoia-elements` and re-exports it
through the prelude. Widgets are ordinary views: they implement `View`, compose
with layouts, participate in focus and accessibility, and can be styled with the
same `Style` types described in [Styling](../concepts/styling.md).

This chapter is a map of the library. The companion guides go deeper:

- [Inputs](inputs.md) covers text, numbers, dates, selection, and forms.
- [Overlays](overlays.md) covers dialogs, drawers, menus, tooltips, and toasts.
- [Collections](collections.md) covers lists, tables, and trees.
- [Scrolling](scrolling.md) covers `ScrollView` and virtualization.

## Buttons and actions

`Button` is the workhorse action widget. It has three constructors for the
common shapes, and every visual aspect can be overridden:

```rust
# use sequoia_ui::prelude::*;
let save = Button::new("Save")
    .style(ButtonStyle::PRIMARY)
    .on_click(|| { /* persist the document */ });

let toolbar = Row::new()
    .gap(6.0)
    .align_items(Align::Center)
    .child(Button::icon("folder").style(ButtonStyle::GHOST))
    .child(Button::ribbon("save", "Save").style(ButtonStyle::PRIMARY))
    .child(save)
    .child(Button::new("Delete").style(ButtonStyle::DANGER).disabled(true))
    .child(Divider::vertical())
    .child(Kbd::new("Ctrl+S"));
```

- `Button::new` is a text button.
- `Button::icon` is an icon-only button; give it a `Tooltip` when the icon is
  not self-explanatory.
- `Button::ribbon` places an icon next to a label.
- `on_click` runs after a full press and release. `on_press` fires on press
  instead, which is useful for toolbar buttons that repeat.
- `style` selects a preset (`PRIMARY`, `SECONDARY`, `DANGER`, `GHOST`,
  `DARK_FILLED`, `OUTLINE_ROSE`), and `style_with` adjusts one.
- `hover_anim`, `press_anim`, and `no_animation` control the state transition.
- `WindowControls::new()` provides the minimize, maximize, and close cluster for
  a custom title bar.

Buttons are focusable and activate with Enter or Space when focused. Wrap them
in a [focus ring](../concepts/accessibility.md) when focus visibility matters.

## Text, icons, and identity

```rust
# use sequoia_ui::prelude::*;
let user = Row::new()
    .gap(8.0)
    .align_items(Align::Center)
    .child(Avatar::named("Ada Lovelace").size(28.0))
    .child(
        Column::new()
            .child(Text::new("Ada Lovelace").semibold())
            .child(Text::new("@ada").size(12.0).color(Color::hex(0x9A9A9A))),
    )
    .child(Badge::count_capped(12, 9));

assert_eq!(badge_label(12, 9), "9+");
```

- `Text` and `RichText` are covered in [Text](../concepts/text.md).
- `Icon` draws a named icon from the asset registry; `.size()` and `.color()`
  adjust it. `Image` draws a raster image or a texture.
- `Avatar::named` derives initials and a stable color from a name.
  `Avatar::icon` uses an icon instead, and `auto_color(false)` keeps the
  background under your control.
- `Badge::new`, `Badge::count`, `Badge::count_capped`, and `Badge::dot` cover
  labels, counters, and unread markers.
- `Kbd::new("Ctrl+S")` renders a keyboard hint.
- `Divider::horizontal()` and `Divider::vertical()` separate regions.

## Selection controls

```rust
# use sequoia_ui::prelude::*;
let notifications = Signal::new(true);
let plan = Signal::new(1usize);
let volume = Signal::new(40.0);

let settings = Column::new()
    .gap(12.0)
    .child(Checkbox::new(notifications.clone()).label("Notify on save"))
    .child(Switch::new(notifications).label("Desktop notifications"))
    .child(
        RadioGroup::new(plan, ["Free", "Pro", "Team"])
            .label("Plan")
            .horizontal(),
    )
    .child(Slider::new(volume).min(0.0).max(100.0).width(200.0));
```

`Checkbox` and `Switch` both bind a `Signal<bool>`. Use a checkbox for an option
that is confirmed with a form, and a switch for a setting that applies
immediately. Both accept `.disabled()` and `.on_toggle`.

`RadioGroup` owns the options and writes the selected index into its signal;
`RadioButton` is the lower-level single button when you need custom layout.
`Slider` writes an `f32` and supports `min`, `max`, `width`, `on_change_start`,
and `on_commit` for expensive work. `Slider1D` is a compact alternative for
single-value strips, and `DragValue` is a numeric drag control.

## Forms

A form is a validation registry, not a container. Build the layout yourself and
register each field so `Form` can report validity as a whole:

```rust
# use sequoia_ui::prelude::*;
let form = Form::new();

let email = Signal::new(String::new());
let seats = Signal::new(2.0);
let field = Field::new(email, "Email").placeholder("you@example.com");
let seat_field = NumberField::new(seats, "Seats").min(1.0).max(50.0);
form.register(&field);
form.register(&seat_field);

assert_eq!(form.len(), 2);
assert!(form.is_valid());

let layout = Column::new()
    .gap(12.0)
    .child(field)
    .child(seat_field)
    .child(Button::new("Create account").style(ButtonStyle::PRIMARY));
```

`Field` wraps a `TextField` with a label, validation error, and touched state.
`NumberField`, `DateField`, and `MultiSelect` follow the same pattern. `Form`
exposes `is_valid`, `touch_all`, and `summary`; `summary` renders the first
error in a form-level banner. See [Inputs](inputs.md) for the full control set.

## Progress and feedback

```rust
# use sequoia_ui::prelude::*;
let progress = Signal::new(0.42);

let status = Row::new()
    .gap(12.0)
    .align_items(Align::Center)
    .child(Spinner::new().size(18.0))
    .child(ProgressBar::new(progress).height(8.0).fill(Color::hex(0x3FB950)))
    .child(Skeleton::new().width(160.0).height(12.0))
    .child(Tooltip::new(Button::icon("check"), "Mark as done"));
```

- `ProgressBar::new` reads a `Signal<f32>` in `0.0..=1.0`.
- `Spinner` is indeterminate and animates on its own.
- `Skeleton` reserves space while content loads; combine it with a layout that
  matches the final content to avoid reflow.
- `Tooltip::new(child, tip)` wraps any view and shows the tip after the pointer
  rests on it.

Toasts are queued rather than constructed one by one:

```rust
# use sequoia_ui::prelude::*;
let queue = ToastQueue::new();
let stack = ToastStack::new(queue.clone());
let id = queue.success("Saved", "Your changes are on disk.");

assert_eq!(queue.len(), 1);
assert!(queue.dismiss(id));
assert!(queue.is_empty());
```

`ToastQueue` also exposes `info`, `warn`, `error`, `push` (with a custom TTL),
`clear`, and the `items()` signal. Place `ToastStack` near the root so toasts
overlay other content.

## Navigation and commands

```rust
# use sequoia_ui::prelude::*;
let tab = Signal::new(0usize);
let tabs = TabStrip::new(tab, ["Editor", "Preview", "Settings"]).height(34.0);

let commands = CommandRegistry::new();
let palette_open = Signal::new(false);
commands.register(
    "save",
    "Save file",
    "File",
    Some("Ctrl+S"),
    || { /* save */ },
);
let palette = CommandPalette::new(commands.clone(), palette_open);
let menu = MenuBar::new(commands);
```

`TabStrip` writes the active index into its signal and supports closable,
tearable tabs through `on_close` and `on_tear`. `MenuBar` builds menus from a
`CommandRegistry`, grouping commands by category. `CommandPalette` adds fuzzy
search, keyboard selection, and a modal surface. Command shortcuts are display
text only; bind the actual accelerator with `App::accelerator` (see
[Hotkeys](hotkeys.md)).

`Drawer`, `Sheet`, `Modal`, and `Dialog` control larger surfaces. They are
covered in [Overlays](overlays.md), along with `FloatingMenu`, `Popper`, and
`Tooltip`.

## Collections and scrolling

`ScrollView` wraps content larger than its viewport. `List` renders a keyed,
non-virtualized column; `VirtualList` and `Table` mount only the visible rows;
`TreeView` flattens an expandable hierarchy into a virtualized list; `Masonry`
places cards in the shortest column. None of them owns selection: keep the
selected key in a `Signal`. See [Collections](collections.md) and
[Scrolling](scrolling.md).

## The element inventory

| Area | Widgets |
| --- | --- |
| Actions | `Button`, `WindowControls` |
| Text and media | `Text`, `RichText`, `Kbd`, `Badge`, `Avatar`, `Icon`, `Image`, `Divider` |
| Inputs | `TextField`, `TextArea`, `Checkbox`, `Switch`, `RadioButton`, `RadioGroup`, `Slider`, `Slider1D`, `DragValue`, `NumberField`, `DateField`, `Field`, `MultiSelect` |
| Pickers | `DropdownButton`, `ColorPicker`, `PickerFloat`, `SvPad` |
| Feedback | `Spinner`, `ProgressBar`, `Skeleton`, `Tooltip`, `ToastQueue`, `ToastStack` |
| Navigation | `TabStrip`, `MenuBar`, `CommandRegistry`, `CommandPalette`, `FloatingMenu` |
| Surfaces | `Modal`, `Dialog`, `Drawer`, `Sheet`, `PopupState`, `DismissableLayer` |
| Collections | `List`, `Table`, `TreeView`, `VirtualList`, `ScrollView`, `Masonry` |
| Layout | `Container`, `Row`, `Column`, `Stack`, `Grid`, `Wrap`, `SplitPanel`, `Panel`, `ResizeHandle`, `Spacer`, `AspectRatio`, `Responsive` |
| Drawing | `Canvas` |
| Docking | `DockView`, `DockModel`, `DockArea`, `DockPanel` |

## Styling and state

Widgets follow one precedence rule: builder overrides win over an explicit
`style(...)`, which wins over the `Theme`. Almost every widget exposes both a
preset setter and a per-property setter, so a one-off accent should use
`.background(...)` rather than a new style type.

Two states are universal:

- `.disabled(true)` removes the widget from focus and pointer interaction.
  Disabled text and backgrounds fall back to the theme's `disabled` color.
- Focus is drawn by `FocusRing`, not by the widget. A widget may paint a focus
  tint when `ctx.state.focused` is set, but the ring is what the user sees.

Hover and press animations smooth color changes. They are short by design; use
`hover_anim` and `press_anim` only when a widget needs a different rhythm, and
respect reduced motion (handled automatically by the runtime).

## When to use a widget or a custom view

Use a built-in widget when its behavior matches what you need: focus handling,
keyboard activation, and accessibility labels are already implemented. Reach for
a custom view when the interaction is unusual, but keep the same contract:
implement `accessibility`, mark the view focusable if it responds to keys, and
handle events on the target phase. The [Custom views](custom-views.md) guide
walks through the full trait.
