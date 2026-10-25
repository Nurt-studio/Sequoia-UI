# Reactivity

Sequoia uses fine-grained reactivity. Views read signals inside reactive
scopes, and the runtime tracks those reads. When a signal changes, only the
views and effects that read it are scheduled for another frame.

There is no virtual DOM and no diffing pass over the whole tree.

## Signals

A `Signal<T>` is the basic reactive cell:

```rust
# use sequoia_ui::prelude::*;
let count = Signal::new(0i32);

assert_eq!(count.get(), 0);
count.set(1);
count.update(|n| *n += 1);
assert_eq!(count.get(), 2);
```

- `get` clones the current value and subscribes the current reactive scope.
- `set` replaces the value and notifies subscribers.
- `update` clones the value, mutates the clone, and calls `set`.
- Setting an equal value is a no-op for `T: PartialEq`; no notification is
  sent.

Clones share the same cell:

```rust
# use sequoia_ui::prelude::*;
let count = Signal::new(0i32);
let alias = count.clone();

alias.set(7);
assert_eq!(count.get(), 7);
```

Signals use `Rc` internally, so they are single-threaded. Do not move a signal
to a worker thread. Compute on the worker, then deliver the result to the UI
thread with the async bridge.

### Reading without cloning

`with` borrows the value and returns a derived result:

```rust
# use sequoia_ui::prelude::*;
let name = Signal::new(String::from("Sequoia"));

let len = name.with(|s| s.len());
assert_eq!(len, 7);
```

Both `get` and `with` track a read. Use the `_untracked` variants when a read
must not subscribe the caller:

```rust
# use sequoia_ui::prelude::*;
let count = Signal::new(0i32);
count.set(3);

let snapshot = count.get_untracked();
assert_eq!(snapshot, 3);
```

`untracked(|| ...)` wraps a larger piece of code and suspends tracking for its
duration.

## Reactive scopes

A reactive scope is code whose signal reads are recorded. The runtime uses
these scopes:

- View builders that read signals, such as the closure passed to `Text::with`.
- Effects created with `effect`.
- Memos created with `memo`.
- Subscriptions created with `Signal::subscribe`.

Reads performed outside a scope are not tracked.

### Effects

An effect runs once immediately and then again whenever one of the signals it
read changes:

```rust
# use sequoia_ui::prelude::*;
# use std::cell::Cell;
# use std::rc::Rc;
let count = Signal::new(0i32);
let seen = Rc::new(Cell::new(0));
let handle = {
    let count = count.clone();
    let seen = seen.clone();
    effect(move || seen.set(count.get()))
};

assert_eq!(seen.get(), 0);
count.set(5);
assert_eq!(seen.get(), 5);
drop(handle);
```

Keep the returned `Effect` alive. Dropping it disposes the effect and stops
future runs. Use `Effect::forget` only for effects that must live for the
whole application.

`Signal::subscribe` is a convenience wrapper that returns a `Subscription`:

```rust
# use sequoia_ui::prelude::*;
# use std::cell::Cell;
# use std::rc::Rc;
let count = Signal::new(0i32);
let runs = Rc::new(Cell::new(0));
let sub = count.subscribe({
    let runs = runs.clone();
    move || runs.set(runs.get() + 1)
});

assert_eq!(runs.get(), 1, "the listener runs once immediately");
count.set(1);
assert_eq!(runs.get(), 2);

drop(sub);
count.set(2);
assert_eq!(runs.get(), 2, "disposed subscriptions stop running");
```

### Memos

A `Memo<T>` caches a derived value and recomputes it only when one of its
dependencies changes:

```rust
# use sequoia_ui::prelude::*;
let width = Signal::new(2i32);
let height = Signal::new(3i32);
let area = memo({
    let width = width.clone();
    let height = height.clone();
    move || width.get() * height.get()
});

assert_eq!(area.get(), 6);
height.set(4);
assert_eq!(area.get(), 8);
```

Use memos for expensive derivations that are read from several places. Plain
closures are enough for cheap formatting.

## Batching

By default, every `set` flushes pending effects before returning. Batching
defers the flush until the outermost batch finishes:

```rust
# use sequoia_ui::prelude::*;
# use std::cell::Cell;
# use std::rc::Rc;
let left = Signal::new(0i32);
let right = Signal::new(0i32);
let runs = Rc::new(Cell::new(0));
let handle = {
    let left = left.clone();
    let right = right.clone();
    let runs = runs.clone();
    effect(move || {
        left.get();
        right.get();
        runs.set(runs.get() + 1);
    })
};

assert_eq!(runs.get(), 1);
batch(|| {
    left.set(10);
    right.set(20);
});
assert_eq!(runs.get(), 2, "one flush for both changes");
drop(handle);
```

Use `batch` when several related signals represent one logical update. UI
views still see intermediate values if they read directly inside the batch,
but effects run after it completes.

`transaction(name, || ...)` is a named batch. The name is available through
`current_transaction` for diagnostics. `Store::transact` combines a
transaction with its own snapshot so a compound edit becomes one undo step.

## Stores and undo

`Signal` has no history. Use `Store<T>` when a value needs undo and redo:

```rust
# use sequoia_ui::prelude::*;
let name = Store::new(String::from("draft"));

name.set(String::from("saved"));
assert!(name.can_undo());
assert!(name.undo());
assert_eq!(name.get(), "draft");
assert!(name.redo());
assert_eq!(name.get(), "saved");
```

`Store` records one history entry per `set` and keeps up to 100 entries. Wrap
a compound edit in `transact` so it becomes a single undo step:

```rust
# use sequoia_ui::prelude::*;
let lines = Store::new(Vec::<String>::new());

lines.transact("paste", || {
    lines.update(|v| v.push(String::from("first")));
    lines.update(|v| v.push(String::from("second")));
});
assert_eq!(lines.get().len(), 2);

assert!(lines.undo());
assert!(lines.get().is_empty());
```

For text-like editing, `UndoSignal<T>` stores its own history and can
coalesce rapid changes into one step:

```rust
# use sequoia_ui::prelude::*;
# use std::time::Duration;
let text = UndoSignal::new(String::new(), 100)
    .with_coalesce_window(Duration::from_secs(2));

text.transact_coalesced("typing", |s| s.push('a'));
text.transact_coalesced("typing", |s| s.push('b'));
assert_eq!(text.depths(), (1, 0));

assert!(text.undo());
assert_eq!(text.get(), "");
```

Choose `Store` for discrete edits and `UndoSignal` for continuous input such
as typing or dragging a slider.

## Localization

`I18n` is a signal-backed key/value store. Reading a translation inside a view
builder subscribes that view to locale changes:

```rust
# use sequoia_ui::prelude::*;
let i18n = I18n::new("en");
i18n.add("en", "save", "Save");
i18n.add("pl", "save", "Zapisz");

i18n.set_locale("pl");
assert_eq!(i18n.t("save"), "Zapisz");

i18n.set_locale("de");
assert_eq!(i18n.t("save"), "Save", "falls back to the base locale");
```

To react to a locale switch in a view, call `t` inside `Text::with` or another
reactive closure. Use `t_untracked` from non-reactive code.

## Next steps

- Learn how views consume signals: View tree.
- Bridge background work safely: Async and tasks.
