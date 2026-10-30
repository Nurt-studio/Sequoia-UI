# Inputs

Input widgets own a `Signal` and write user edits back into it. There is no
separate form state to synchronize: the signal is the value, and any other view
that reads it updates automatically.

This guide covers single-line and multiline text, numeric controls, pickers,
and validation. For selection controls such as checkboxes and radio groups, see
[Widgets](widgets.md).

## Text fields

`TextField` binds a `Signal<String>`:

```rust
# use sequoia_ui::prelude::*;
let query = Signal::new(String::new());
let submitted = std::rc::Rc::new(std::cell::Cell::new(0));

let search = TextField::new(query.clone())
    .placeholder("Search files")
    .max_length(120)
    .width(240.0)
    .on_submit({
        let submitted = submitted.clone();
        move |_text| submitted.set(submitted.get() + 1)
    });

assert_eq!(search.caret(), 0);
assert!(search.selection().is_none());
```

- `placeholder` is shown while the value is empty.
- `max_length` truncates input at the limit.
- `on_edit_start` fires when the field gains focus, `on_edit_end` when it loses
  focus and receives the final string, and `on_submit` runs on Enter.
- `font_size`, `weight`, `text_color`, `background`, `border`, `radius`,
  `padding`, and `width` are the common styling hooks. `appearance` applies a
  `TextFieldStyle` instead.
- The caret and selection are also controllable: `caret`, `selection`,
  `selected_text`, `select_range`, `clear_selection`, and `select_all`.

The field handles editing keys, clipboard shortcuts, and IME composition. When
an input method is active, preedit text is displayed but is not written to the
signal until the composition commits.

Treat the signal as the value. Internal editing state (caret, selection, and
preedit text) is synchronized with it, but application code should read and
write the signal.

## Multiline text

`TextArea` is the editor for notes, code, and logs:

```rust
# use sequoia_ui::prelude::*;
let notes = Signal::new(String::new());

let editor = TextArea::new(notes)
    .placeholder("Write a release note")
    .min_lines(3)
    .max_lines(12)
    .line_height_factor(1.4)
    .auto_indent(true)
    .accept_tab(false)
    .highlight_current_line(true)
    .width(Length::Fill(1.0));
```

- `min_lines` and `max_lines` control the measured height. The editor scrolls
  internally once the content exceeds `max_lines`.
- `auto_indent` repeats the previous line's indentation on Enter.
- `accept_tab(true)` inserts a tab character; the default `false` moves focus,
  which is the right choice for prose and most forms.
- `highlight_current_line` and `current_line` style the caret's line.
- `on_edit_start`, `on_edit_end`, and `on_scroll` mirror the single-line field.
- `indent` sets the string inserted for one indentation level.

The editor exposes `caret`, `selection`, `selections` (multiple carets),
`set_caret`, `select_range`, `selected_text`, `scroll_to`, `line_count`, and
`caret_line_col`. These are useful for toolbar actions such as bold or
highlighting.

## Numeric and drag controls

`NumberField` is a labelled numeric input. `DragValue` is the compact control
behind it and can be used directly for inline values:

```rust
# use sequoia_ui::prelude::*;
let seats = Signal::new(2.0);
let opacity = Signal::new(0.8);

let seats_field = NumberField::new(seats.clone(), "Seats")
    .min(1.0)
    .max(20.0)
    .step(1.0)
    .precision(0);

let opacity_drag = DragValue::new(opacity)
    .min(0.0)
    .max(1.0)
    .precision(2)
    .sensitivity(0.01)
    .trim_zeros(true);

let row = Row::new()
    .gap(12.0)
    .align_items(Align::Center)
    .child(seats_field)
    .child(opacity_drag);
```

- `step` is the increment used by arrow keys and the number field's buttons.
- `precision` controls displayed decimals.
- `sensitivity` is the number of units changed per pixel dragged.
- `on_change_start` and `on_commit` split live updates from the final value,
  which is useful when a drag triggers expensive recomputation.
- `Slider` and `Slider1D` cover bounded values with a track. They write an
  `f32` and accept `min`, `max`, `width`, and `on_commit`.

For a slider that drives a live preview, read the signal in the preview view and
use `on_commit` only for persistence. That keeps the preview smooth without
writing to disk on every frame.

## Selecting from options

`DropdownButton` writes the selected index:

```rust
# use sequoia_ui::prelude::*;
let size = Signal::new(1usize);

let picker = DropdownButton::new(size.clone(), vec!["Small", "Medium", "Large"])
    .placeholder("Choose a size")
    .on_select(|index| {
        let _ = index;
    });

assert_eq!(size.get_untracked(), 1);
```

`DropdownButton` opens a floating menu anchored below the button. The menu
closes on selection, Escape, or an outside click. Use `on_select` for side
effects and read the signal for the current value.

`MultiSelect` keeps a set of indices and validates a minimum selection:

```rust
# use sequoia_ui::prelude::*;
let topics = MultiSelect::new(
    vec!["Rust".to_string(), "Design".to_string(), "Docs".to_string()],
    "Topics",
)
.min_selected(1);

topics.toggle(0);
assert_eq!(topics.selected_values(), vec!["Rust".to_string()]);
assert!(topics.is_valid());
assert!(topics.error().is_none());
```

`MultiSelect` exposes `selected_indices`, `selected_values`, `summary_text`,
`touch`, and `selected_signal`. It renders a dropdown of checkable rows and
shows the current summary on the button.

## Color pickers

`ColorPicker` is a full picker with HSV channels, history, and an eyedropper.
`SvPad` is the reusable saturation/value pad, and `PickerFloat` is the picker in
floating-panel form:

```rust
# use sequoia_ui::prelude::*;
let picker = ColorPicker::new();
picker.set_hex("#3584E4");

assert!(picker.is_open());
assert!(picker.cur_rgb().2 > 0);

let panel = picker.panel();
let floating = picker.floating();
```

- `set_hex` accepts `#RGB`, `#RRGGBB`, `0xRRGGBB`, and bare hex.
- `hsv_signals` exposes the hue, saturation, and value as signals.
- `cur_rgb`, `cur_hex`, and `cur_color` read the current value.
- `open`, `is_open`, and `set_on_close` control the popup; closing is handled by
  the floating panel's dismiss layer.
- `toasts` returns the queue used for invalid-input feedback, so errors appear
  in the same place as the rest of the application's toasts.
- `panel()` returns the inline picker surface; `floating()` wraps it in a
  `PickerFloat` for popup use.

`SvPad::new(sat, val, hue)` builds a custom pad when the full picker is too much
UI.

## Validation and forms

Every form field carries its own rules. `Field`, `NumberField`, and
`MultiSelect` implement `ValidField`; `Form` is a registry that checks them
together.

```rust
# use sequoia_ui::prelude::*;
let form = Form::new();

let email = Signal::new(String::new());
let field = Field::new(email.clone(), "Email")
    .placeholder("you@example.com")
    .rule("Email is required", |value| !value.trim().is_empty())
    .rule("Enter a valid email", |value| value.contains('@'));

form.register(&field);
assert!(!form.is_valid());

email.set("ada@example.com".to_string());
assert!(field.is_valid());
assert!(form.is_valid());
```

`rule(message, check)` adds a predicate and the message to show when it fails.
Rules run in order and the first failure wins, so put the cheapest or most
specific check first. Errors are only displayed after a field is touched;
`Form::touch_all` marks every field touched, which is what a submit button
should call before showing `summary()`.

`NumberField::rule` takes `Fn(f32) -> bool`, so numeric constraints read
naturally:

```rust
# use sequoia_ui::prelude::*;
let seats = Signal::new(0.0);
let seats_field = NumberField::new(seats.clone(), "Seats")
    .min(1.0)
    .max(20.0)
    .rule("Choose at least one seat", |value| value >= 1.0);

assert!(!seats_field.is_valid());
seats.set(2.0);
assert!(seats_field.is_valid());
```

`DateField::new(value, label)` validates an ISO date (`YYYY-MM-DD`) and exposes
`value_signal` and `editing_signal`; use `parse_iso_date` when you need the
parsed components. `Form::register` accepts any `ValidField + Clone`, so custom
fields can participate in the same summary.

## Focus and keyboard

Text inputs are focusable and manage their own caret when focused. Tab moves
focus to the next control; in a `TextArea`, set `accept_tab(true)` if Tab should
insert indentation instead. `TextField` submits on Enter, while `TextArea` uses
Enter for a newline.

Place each input inside a `FocusRing` when the surrounding layout hides the
caret or when the input's own focus affordance is subtle:

```rust
# use sequoia_ui::prelude::*;
let name = Signal::new(String::new());
let field = FocusRing::wrap(
    TextField::new(name).placeholder("Display name"),
)
.reserve_space(true);
```

`FocusRing` draws around the input and keeps its space reserved, so focusing
does not shift the form.

## Patterns

**Debounce remote search.** Keep the field's signal as the source of truth and
debounce only the request. The [Async work](../concepts/async.md) guide shows
the `Debouncer` and `Executor` combination.

**Separate live and committed values.** A slider or drag value can write to a
preview signal on every change and to a model on `on_commit`. This keeps
expensive work off the drag path.

**Reset a field.** Set its signal to the empty string; the widget follows. Do
not keep a separate "displayed value" that can drift from the signal.

**Validate on submit, not per keystroke.** Rules are cheap to run, but error
messages while a user is still typing are noisy. Use `touch_all()` on submit,
then show errors for touched fields.

## Accessibility

`TextField` reports `AccessibleRole::TextField`, using the placeholder as the
label when no other label is available. `Field` renders a visible label above
its input; keep that label concise and do not duplicate it in the placeholder.
`MultiSelect` and `DropdownButton` label their menus from the selected summary.
If you build a custom input, implement `accessibility` and `focusable` as
described in [Accessibility](../concepts/accessibility.md).

