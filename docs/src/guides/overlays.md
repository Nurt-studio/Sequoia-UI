# Overlays

Overlays are views that paint outside their parent's bounds. Sequoia has two
levels of API:

1. **Widgets** such as `Dialog`, `Modal`, `Drawer`, `Sheet`, `FloatingMenu`, and
   `Tooltip` handle placement, dismissal, focus, and animation for common cases.
2. **Overlay primitives** such as `Popper`, `Portal`, `OverlayEntry`, and
   `DismissableLayer` let a custom view place arbitrary content in an overlay
   layer.

All overlays flow through the same runtime overlay store, which owns layering,
input mode, and dismissal. That is why a dialog can trap focus while a tooltip
cannot.

## Dialogs

`Dialog` is the standard modal for a decision. It provides a title, a content
area, action buttons, a focus trap, and Escape-to-close:

```rust
# use sequoia_ui::prelude::*;
let open = Signal::new(false);
let dialog = Dialog::new(open, "Delete project?")
    .content(Text::new("This action cannot be undone."))
    .cancel("Cancel")
    .danger("Delete", || { /* delete the project */ });

dialog.open_signal().set(true);
assert!(dialog.is_open());

dialog.close();
assert!(!dialog.is_open());
```

- `new(open, title)` binds a `Signal<bool>`; opening and closing is just setting
  the signal.
- `content` accepts any view. Long content scrolls; do not build a second
  scroll container inside.
- `action(label, callback)` adds a normal button, `danger` adds a destructive
  one, and `cancel(label)` adds a button that closes the dialog without running
  an action.
- `on_close` runs whenever the dialog closes, including Escape and the cancel
  button.
- `width` bounds the card, and `style` applies a `DialogStyle`.
- `scrim()` returns the `ModalScrim` so you can render the backdrop separately; it blurs the background by default.

Use a dialog for blocking decisions. For non-blocking content, use a `Popper` or
a `FloatingMenu`; using a modal for every popup makes routine interactions slow.

## Modals, drawers, and sheets

`Modal` is the generic centered surface. `Drawer` slides from a side and keeps
the previous screen visible; `Sheet` is the same idea for bottom sheets and
side panels:

```rust
# use sequoia_ui::prelude::*;
let open = Signal::new(false);

let modal = Modal::new(open.clone())
    .width(420.0)
    .padding(24.0)
    .dismiss(Dismiss::Escape)
    .child(Text::new("Custom modal body"));

let drawer = Drawer::new(open.clone())
    .right()
    .width(320.0)
    .child(Text::new("Navigation"));

let sheet = Sheet::new(open)
    .bottom()
    .height(240.0)
    .child(Text::new("Details"));

modal.open();
assert!(modal.is_open());
modal.close();
```

All three share the same builder surface:

- `child`, `child_if`, and `child_opt` add content.
- `width`, `height`, and `size` take `Length`, so `Length::Fill(1.0)` works.
- `padding`, `background`, `border`, `radius`, and `style` control appearance.
- `backdrop` and `scrim_color` style the dimming layer.
- `dismiss` chooses `Dismiss::Escape`, `OutsideClick`, `FocusLost`, or
  `Manual`.
- `open`, `close`, `toggle`, and `is_open` control the signal from code.

`Drawer` and `Sheet` add `side`, `left`, `right`, `top`, and `bottom` (with
`DrawerSide` and `SheetSide` as the enum forms). A drawer is usually persistent
navigation; a sheet is usually a task that needs more space than a dialog but
less commitment than a full screen.

`ModalScrim::new(open)` paints only the scrim. Use it when the content is
positioned by your own layout but should still dim the background.

## Popovers with Popper

`Popper` anchors an overlay to the view that paints it:

```rust
# use sequoia_ui::prelude::*;
let open = Signal::new(false);

let popover = Popper::new(open.clone())
    .anchor_self()
    .placement(Placement::BottomStart)
    .offset(0.0, 6.0)
    .dismiss(Dismiss::OutsideClick)
    .child(
        Container::new()
            .padding(12.0)
            .radius(8.0)
            .background(Color::hex(0x262626))
            .child(Text::new("Details")),
    );

popover.open();
assert!(open.get_untracked());
open.set(false);
```

Place the `Popper` in the tree next to the control that should anchor it, or use
`.anchor_view(element_id)` when the anchor is elsewhere.

> **Prefer `anchor_view` for menus and tooltips.** `anchor_self` anchors to the
> popper's own layout box, which is usually zero-sized and sits wherever the
> `Popper` happens to be in the tree—not under the button the user clicked. For
> a menu under a button, store the button's `ElementId` (for example
> `ctx.element()` from its paint pass) and use `.anchor_view(button_id)`. That
> anchor follows the button when it moves or the layout changes. Keep
> `anchor_self` only when the `Popper` itself is placed exactly where the
> overlay should attach.

Key methods:

- `anchor_self` anchors to the popper's own element; `anchor_rect` and
  `anchor_view` anchor to explicit geometry.
- `at_cursor` anchors to the pointer and uses `Placement::Cursor`.
- `placement` chooses `Top`, `Bottom`, `Left`, `Right`, `Center`, the `Start`
  and `End` variants, or `Cursor`.
- `offset(dx, dy)` nudges the overlay after placement.
- `overflow` controls what happens near a window edge:
  `OverlayOverflow::FlipAndClamp` (default), `Clamp`, or `Visible`.
- `layer` selects the overlay layer; `input_mode` selects passthrough,
  interactive, or modal input.
- `dismiss` and `on_dismiss` control automatic closing.

`Portal` uses the same options but defaults to `Placement::Center` and
`Dismiss::Manual`. Use it to lift arbitrary content (a floating toolbar, a
drag preview, a command surface) out of the normal layout; use `Popper` when the
content should follow an anchor.

For raw control, build an `OverlayEntry` and show it from a custom view:

```rust
# use sequoia_ui::prelude::*;
let entry = OverlayEntry::new(Text::new("Floating label"))
    .anchor(Anchor::Cursor)
    .placement(Placement::Cursor)
    .layer(OverlayLayer::Tooltip)
    .input_mode(InputMode::Passthrough)
    .dismiss(Dismiss::Manual);

let _entry = entry;
```

`PaintCtx::show_overlay(entry)` returns an `OverlayId`; keep it and call
`hide_overlay(id)` when the overlay should disappear. This is the mechanism
widgets use internally, and it is the escape hatch for overlays that do not fit
the builders.

## Menus and context menus

`FloatingMenu` builds a menu around a trigger:

```rust
# use sequoia_ui::prelude::*;
let menu = FloatingMenu::wrap(Button::new("Actions"))
    .below_invoker()
    .open_on_click()
    .popup_width(220.0)
    .text_row("Duplicate", || { /* duplicate */ })
    .separator()
    .text_row("Delete", || { /* delete */ });
```

- `wrap(child)` attaches the menu to a trigger; `new(child)` builds a menu from
  a content view.
- `below_invoker` and `open_on_click` produce the usual toolbar behavior.
- `text_row`, `row`, and `shortcut_row` add items; `separator` divides groups;
  `custom_row` and `custom_item` accept arbitrary views.
- `item(label)` returns a `MenuItem` for icon, shortcut, danger, and disabled
  flags.
- `open_signal`, `is_open`, and `popup_width` control the overlay.
- `dismiss` is handled by the menu; selecting an item closes it.

`ContextMenu` opens on right-click:

```rust
# use sequoia_ui::prelude::*;
let area = ContextMenu::new(
    Container::new()
        .fill_size()
        .child(Text::new("Right-click here")),
)
.item("Copy", || { /* copy */ })
.item("Paste", || { /* paste */ });
```

The child is the hit area; menu items are appended in order. Use
`ContextMenu` for secondary actions and `FloatingMenu` for a visible trigger.

`CommandPalette` is a specialized menu-driven overlay with search and keyboard
navigation. See [Widgets](widgets.md) and [Hotkeys](hotkeys.md).

## Tooltips

`Tooltip` wraps a view and shows a short label after the pointer rests:

```rust
# use sequoia_ui::prelude::*;
let tooltip = Tooltip::new(
    Button::icon("trash").style(ButtonStyle::DANGER),
    "Move to trash",
)
.background(Color::hex(0x111111))
.text_color(Color::WHITE);
```

Tooltips are for names and short hints. Do not put interactive content or long
instructions in a tooltip; use a popover instead. The tooltip layer is above
normal content but below menus and modals, so it never blocks a menu.

## Toasts

Toasts are queued notifications rendered by `ToastStack`. The stack owns the
overlay and animates entries; the queue owns the data:

```rust
# use sequoia_ui::prelude::*;
let queue = ToastQueue::new();
let stack = ToastStack::new(queue.clone())
    .max_visible(3)
    .width(320.0);

let id = queue.error("Upload failed", "Check your connection and retry.");
assert!(queue.dismiss(id));
```

`ToastQueue` provides `info`, `success`, `warn`, `error`, and `push` with a
custom TTL. Toasts expire automatically; `clear` removes all of them. Place one
`ToastStack` near the root and share the queue through the environment or a
clone; do not create a stack per screen, or toasts will overlap.

## Layering, input, and dismissal

Overlay layers are ordered from bottom to top:

```text
Tooltip < Popup < Menu < DragPreview < Modal < System
```

Use the lowest layer that works. A tooltip on the `Modal` layer would paint
above dialogs and steal focus from them.

`InputMode` decides how pointer events reach the rest of the UI:

- `Passthrough` lets events reach content below the overlay. Use it for purely
  visual overlays.
- `Interactive` (default) captures events inside the overlay but leaves the rest
  of the window active. Use it for popovers and menus.
- `Modal` blocks input to everything below. Use it for dialogs and modal
  sheets.

`Dismiss` controls automatic closing:

| Value | Behavior |
| --- | --- |
| `Manual` | Only code closes the overlay. |
| `Escape` | Escape closes it. |
| `OutsideClick` | A click outside closes it. |
| `FocusLost` | Losing focus closes it. |

A dialog usually wants Escape plus its focus trap. A popover usually wants
outside-click. A context menu usually wants Escape and outside-click. If you
need both, wrap the content in `DismissableLayer` and pick the combination:

```rust
# use sequoia_ui::prelude::*;
let open = Signal::new(true);
let layer = DismissableLayer::wrap(
    Container::new().padding(16.0).child(Text::new("Popover")),
)
.dismiss_on_escape(true)
.bind(open.clone());

assert!(open.get_untracked());
```

`DismissableLayer` also exposes `on_dismiss` and `on_escape` for custom side
effects. `bind` closes the signal on dismissal.

## Focus and accessibility

Dialogs and modal surfaces should contain focus. `Dialog` does this for you. For
custom modal content, wrap it in `FocusScope::new().trap(true)` and set the
overlay's input mode to `InputMode::Modal`.

An overlay that contains interactive controls must be reachable by keyboard:
use `Popper`/`Portal` with `InputMode::Interactive` and a `FocusScope`, and give
every custom control an `Accessible` role and label. A tooltip should never be
the only place a control is named. See
[Accessibility](../concepts/accessibility.md) for the full contract.

## Motion

Overlays appear and disappear abruptly by default. For custom content, wrap the
child in `when(open, child).enter(...).exit(...)` or animate opacity and offset
with `Live`; the overlay store already handles removal, so the exit motion must
finish before the view leaves the tree. `Dialog`, `Drawer`, and `Sheet` provide
their own transitions and respect reduced motion.

Keep overlay motion short. A dialog that takes longer than about 200 ms to
appear feels sluggish, and an exit that blocks the next action is worse than no
animation at all.
