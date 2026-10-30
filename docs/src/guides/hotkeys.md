# Hotkeys

Sequoia has two kinds of keyboard shortcuts:

- **Accelerators** are handled by the application while one of its windows has
  keyboard focus.
- **Global hotkeys** are registered with the operating system and fire even when
  the application is in the background.

Both use `Key` and `Modifiers`. `Modifiers` is a bitflag set, so combine keys
with `|`:

```rust
# use sequoia_ui::prelude::*;
let key = Key::from_char('s');
let mods = Modifiers::CONTROL | Modifiers::SHIFT;

assert_eq!(key, Key::from_char('s'));
assert_eq!(mods, Modifiers::CONTROL | Modifiers::SHIFT);
assert_ne!(mods, Modifiers::CONTROL);
```

`Key::from_char('s')` creates a character key. Named keys such as `Key::ENTER`,
`Key::ESCAPE`, `Key::DELETE`, `Key::F5`, and `Key::ARROW_UP` are constants.
`Modifiers::NONE` means no modifier keys.

## Accelerators

Register an accelerator on `App`:

```rust,no_run
# use sequoia_ui::prelude::*;
let app = App::new()
    .title("Editor")
    .accelerator(Key::from_char('s'), Modifiers::CONTROL, || {
        // save the current document
    })
    .accelerator(Key::from_char('o'), Modifiers::CONTROL, || {
        // open a document
    });

# let _ = app;
```

Accelerators are checked before the key is delivered to the view tree. When a
shortcut matches, its action runs, the key event is consumed, and the window is
asked to redraw. If no accelerator matches, the key follows the normal focused
widget path, so typing into a text field still works.

The match is exact: an accelerator registered with `Modifiers::CONTROL` does
not fire when Shift is also held. Register the shifted combination separately if
it should do something different. The first matching accelerator in registration
order wins, so avoid overlapping bindings.

### Context and guards

Use `accelerator_ctx` when the action needs window information:

```rust,no_run
# use sequoia_ui::prelude::*;
let app = App::new().accelerator_ctx(
    Key::from_char('n'),
    Modifiers::CONTROL,
    |ctx| {
        let _window = ctx.window;
        let _size = ctx.size;
        let _scale = ctx.scale;
    },
);

# let _ = app;
```

`AcceleratorCtx` contains:

- `window`: the `WindowId` that received the key.
- `size`: the logical size of that window.
- `scale`: its display scale factor.
- `environment`: the shared environment, including services such as `Windows`.

Use `accelerator_when` or `accelerator_ctx_when` to make a shortcut conditional.
The guard is evaluated when the key is pressed:

```rust,no_run
# use sequoia_ui::prelude::*;
let editing = Signal::new(false);

let app = App::new()
    .accelerator_when(
        Key::DELETE,
        Modifiers::NONE,
        not_editing(editing.clone()),
        || {
            // delete the selected item
        },
    )
    .accelerator_when(
        Key::from_char('z'),
        Modifiers::CONTROL,
        not_editing(editing.clone()),
        || {
            // undo
        },
    );

# let _ = app;
```

`not_editing` returns a guard that is true when the supplied `Signal<bool>` is
false. It does not inspect the view tree; keep that signal up to date from your
own focus or edit state. Guards are useful for bare keys such as Delete, which
would otherwise steal input from a focused text field.

You can also construct an accelerator directly:

```rust
# use sequoia_ui::prelude::*;
let editing = Signal::new(false);
let acc = Accelerator::simple(Key::from_char('s'), Modifiers::CONTROL, || {})
    .with_when(not_editing(editing.clone()));

assert!(acc.should_fire());
editing.set(true);
assert!(!acc.should_fire());
```

`Accelerator::new` takes a closure receiving `&AcceleratorCtx`; `simple` takes a
closure with no arguments. `should_fire` evaluates the guard, and `fire` invokes
the action with a context. The `App` builders use these same types.

### Repeat behavior

An accelerator fires on the initial key press. Held character keys, Space,
Backspace, Delete, arrow keys, Home, End, Page Up, Page Down, and Insert can also
repeat according to `Key::allows_repeat`. Named keys such as Enter, Escape, Tab,
and the function keys do not repeat. If a shortcut must run once per press even
for a repeating key, add your own debounce or state guard.

## Global hotkeys

Global hotkeys are registered with the operating system, so they work while the
application is unfocused:

```rust,no_run
# use sequoia_ui::prelude::*;
let app = App::new()
    .title("Quick Capture")
    .global_hotkey(
        Key::from_char('m'),
        Modifiers::CONTROL | Modifiers::ALT,
        || {
            // show the capture window
        },
    );

# let _ = app;
```

Global hotkeys currently use the Windows backend. On other platforms,
`GlobalHotkeys::new()` returns `HotkeyError::Unsupported`, and `App` logs a
warning and continues without global shortcuts. Registration can also fail
because another application owns the combination or because the key cannot be
mapped to a system shortcut; `App::global_hotkey` logs those failures and skips
the binding. Use a unique combination and test on the target platform.

The direct API reports errors instead of logging them:

```rust
# use sequoia_ui::prelude::*;
let mut hotkeys = GlobalHotkeys::stub();
let id = hotkeys
    .register(Key::from_char('m'), Modifiers::CONTROL | Modifiers::ALT)
    .expect("register in the stub backend");

assert!(hotkeys.registered().contains_key(&id));
assert!(hotkeys.drain().is_empty());
assert!(hotkeys.unregister(id));
assert!(!hotkeys.unregister(id));

match hotkeys.register(Key::ENTER, Modifiers::NONE) {
    Err(HotkeyError::UnmappableKey) => {}
    other => panic!("expected an unmappable key, got {other:?}"),
}
```

`GlobalHotkeys::stub()` is a backend-free implementation for tests and tools.
It validates keys and tracks registrations, but never receives OS events, so
`drain()` stays empty. `GlobalHotkeys::new()` starts the real backend. `register`
returns a `HotkeyId`, `unregister` returns whether the id was present, and
`drain` returns the ids that fired since the last call. Dropping the manager
unregisters the backend and shuts down its thread.

The error variants are:

- `Unsupported`: global shortcuts are not available on this platform.
- `UnmappableKey`: the key cannot be represented as a system shortcut.
- `OsRejected`: the operating system rejected the combination, usually because
  another application already owns it.
- `BackendGone`: the shortcut backend thread is no longer responding.

`App` registers global hotkeys during startup. If you need to add or remove
bindings while the application is running, use `GlobalHotkeys` directly and
poll `drain` on your own schedule.

## Testing shortcuts

Accelerators can be tested without a window by checking the guard and firing the
action with a constructed context:

```rust
# use sequoia_ui::prelude::*;
use std::cell::Cell;
use std::rc::Rc;

let fired = Rc::new(Cell::new(0));
let counter = fired.clone();
let acc = Accelerator::simple(Key::from_char('s'), Modifiers::CONTROL, move || {
    counter.set(counter.get() + 1);
});
let ctx = AcceleratorCtx {
    window: WindowId::PRIMARY,
    size: Size::new(800.0, 600.0),
    scale: 1.0,
    environment: Environment::new(),
};

assert!(acc.should_fire());
acc.fire(&ctx);
assert_eq!(fired.get(), 1);
```

`Harness` drives the view tree, but it does not install `App` accelerators or
the platform hotkey backend. Test guards and actions directly, and test the
global registration bookkeeping with `GlobalHotkeys::stub`.

## Command menus

`CommandRegistry` shortcut strings are labels for menus and the command palette.
They do not create key bindings. Register the real behavior with `App`:

```rust,no_run
# use sequoia_ui::prelude::*;
let commands = CommandRegistry::new();
commands.register(
    "save",
    "Save",
    "File",
    Some("Ctrl+S"),
    || {
        // save the current document
    },
);

let app = App::new()
    .accelerator(Key::from_char('s'), Modifiers::CONTROL, || {
        // the same save action
    });

# let _ = (commands, app);
```

Keep the display label and the accelerator in sync when the shortcut changes.

## See also

- [Inputs](inputs.md)
- [Widgets](widgets.md)
- [Desktop integration](desktop-integration.md)
- [Testing views](testing.md)
