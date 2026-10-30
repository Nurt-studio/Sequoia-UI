# Testing views

The `sequoia-test` crate provides `Harness`, a headless runtime for layout,
painting, input, overlays, and drag state. It uses the same view runtime as the
desktop application, but does not create a native window, GPU surface, or
platform dialog. Use it for deterministic tests of behavior and geometry.

Add the harness as a development dependency:

```toml
[dev-dependencies]
sequoia-test = { workspace = true }
```

The examples in this book also use `sequoia_ui` for the prelude:

```toml
[dev-dependencies]
sequoia-test = { workspace = true }
sequoia-ui = { workspace = true }
```

## A first test

A harness mounts one root view. Lay it out, drive input, and inspect the shared
signals or the view tree:

```rust
# use sequoia_ui::prelude::*;
# use sequoia_ui::view::ElementId;
# use sequoia_test::Harness;
#[derive(Clone)]
struct Counter {
    count: Signal<u32>,
}

impl View for Counter {
    fn body(&self) -> impl View + 'static {
        let count = self.count.clone();
        Column::new()
            .padding(12.0)
            .child(Text::new(format!("count: {}", self.count.get())))
            .child(
                Button::labeled("Increment")
                    .on_click(move || count.update(|n| *n += 1)),
            )
    }
}

fn find_button(h: &Harness) -> ElementId {
    let mut stack = vec![h.root().expect("root")];
    while let Some(id) = stack.pop() {
        if h.view::<Button>(id).is_some() {
            return id;
        }
        stack.extend(h.children(id));
    }
    panic!("button not found");
}

let count = Signal::new(0u32);
let h = Harness::new(Counter {
    count: count.clone(),
});
h.layout(240.0, 120.0);

let button = find_button(&h);
let rect = h.rect(button).unwrap();
h.click(
    rect.left() + rect.size.width * 0.5,
    rect.top() + rect.size.height * 0.5,
);
h.tick();

assert_eq!(count.get_untracked(), 1);
```

`Harness::new` mounts the root but does not lay it out. Call `layout` before
using `rect`, `hit_test`, or pointer coordinates. `tick` flushes effects and
pending rebuilds; call it after input when a callback changes a signal.

## Layout and the element tree

`Harness` exposes the same tree that layout sees. Element IDs remain stable
while a view keeps its position and type, and keyed collections preserve
identity across reordering:

```rust
# use sequoia_ui::prelude::*;
# use sequoia_test::Harness;
let h = Harness::new(
    Row::new()
        .padding(8.0)
        .child(Button::labeled("A"))
        .child(Button::labeled("B")),
);
h.layout(300.0, 80.0);

let root = h.root().unwrap();
let children = h.children(root);
assert_eq!(children.len(), 2);

let first = h.rect(children[0]).unwrap();
assert!(first.left() >= 8.0, "padding applied: {first:?}");
assert!(
    h.type_name_of(children[0])
        .as_deref()
        .is_some_and(|name| name.contains("Button"))
);
assert!(h.is_alive(children[0]));
```

Useful inspection methods:

- `root`, `children`, and `key_of` navigate the tree.
- `rect` returns the laid-out rectangle.
- `hit_test` returns the element that would receive a pointer event.
- `type_name_of` identifies the concrete view type.
- `view::<T>` downcasts an element to a concrete view.
- `text` reads the content of a `Text` view.
- `dump` formats the tree with types, rectangles, and element IDs for a failed
  assertion.
- `element_count`, `state_count`, and `scoped_count` report tree and state
  allocation sizes.

Prefer semantic lookup helpers over hard-coded child indexes when a layout is
likely to change. A small depth-first helper using `view::<T>` or
`type_name_of` is usually enough.

## Signals and rebuilds

`Harness::set` writes a signal and ticks in one step. A direct `signal.set`
follows the same rule as the runtime: call `tick` to flush effects, then
`layout` if geometry or content may have changed:

```rust
# use sequoia_ui::prelude::*;
# use sequoia_test::Harness;
#[derive(Clone)]
struct Label {
    text: Signal<String>,
}

impl View for Label {
    fn body(&self) -> impl View + 'static {
        Text::new(self.text.get())
    }
}

let text = Signal::new("ready".to_string());
let h = Harness::new(Label { text: text.clone() });
h.layout(200.0, 40.0);

let label = || h.children(h.root().unwrap())[0];
assert_eq!(h.text(label()).as_deref(), Some("ready"));

h.set(&text, "done".to_string());
h.layout(200.0, 40.0);
assert_eq!(h.text(label()).as_deref(), Some("done"));
```

`h.stats()` reports cumulative runtime work:

```rust
# use sequoia_ui::prelude::*;
# use sequoia_test::Harness;
let h = Harness::new(Text::new("hello"));
h.layout(200.0, 40.0);
let stats = h.stats();
assert!(stats.mounts > 0);
assert_eq!(stats.errors, 0);
```

The exact mount count depends on how many elements a composite view expands to,
so assert that work happened rather than pinning an absolute count. Use
`element_count` for the current tree size and `state_count` for the number of
allocated state slots.

## Pointer, keyboard, and text input

`Harness` routes input through the same runtime as the application. Coordinates
are logical window coordinates, with the origin at the top-left:

```rust
# use sequoia_ui::prelude::*;
# use sequoia_test::Harness;
let value = Signal::new(String::new());
let h = Harness::new(
    TextField::new(value.clone())
        .width(200.0)
        .height(40.0),
);
h.layout(200.0, 80.0);

h.click(20.0, 20.0);
h.key(Key::from_char('h'));
h.key(Key::from_char('i'));
h.key(Key::SPACE);
assert_eq!(value.get_untracked(), "hi ");

h.key(Key::BACKSPACE);
h.key(Key::BACKSPACE);
h.key(Key::BACKSPACE);
assert_eq!(value.get_untracked(), "");
```

Common input methods:

- `pointer_move`, `pointer_down`, `pointer_up`, and `click` use the left
  button. `pointer_down_at` and `pointer_up_at` select another button.
- `pointer_down_mods` and `pointer_up_mods` attach modifiers.
- `key`, `key_with_shift`, and `key_with_modifiers` send keyboard events.
  Use `Key::from_char` for printable characters.
- `wheel` and `wheel_xy` send scroll deltas.
- `ime_preedit` and `ime_commit` exercise text composition.
- `pen_down`, `pen_move`, `pen_up`, `touch_down`, `touch_move`, `touch_up`, and
  `pinch` cover pointer pressure and multi-touch paths.
- `drag`, `drag_steps`, `drag_cancel`, `drag_session`, `has_pending_drag`, and
  `drop_target_at` drive drag-and-drop sessions.

`click` is a down/up pair at one point. Use separate `pointer_down` and
`pointer_up` calls when the test needs to observe the pressed state or release
at a different location.

## Painting and render lists

`paint` builds a `RenderList`. It contains the same primitives the GPU backend
would receive, so tests can assert on colors, counts, clipping, and overlays
without a GPU:

```rust
# use sequoia_ui::prelude::*;
# use sequoia_test::Harness;
let h = Harness::new(
    Panel::new()
        .width(80.0)
        .height(40.0)
        .background(Color::RED),
);
h.layout(200.0, 100.0);

let list = h.paint();
assert_eq!(list.quads().count(), 1);
assert_eq!(list.quads().next().unwrap().color, Color::RED);

let stats = h.paint_stats(200.0, 100.0);
assert_eq!(stats.quads, 1);
assert_eq!(stats.gpu_instances(), 1);
```

`PaintStats` counts render items and GPU instances:

- `items`, `quads`, `texts`, `glyphs`, `images`, `shadows`, and
  `waveform_bars` describe the list.
- `backdrops` counts backdrop spans.
- `gpu_instances()` adds quads, glyphs, images, and waveform bars.

`paint_stats` lays out and paints the full tree. `paint_culled_stats` uses
culling for the current viewport, which is useful when comparing visible work
with the full render list. `paint_if_dirty` paints only when damage is pending;
`needs_frame` and `damage_stats` expose the frame-skip state. `sync` performs a
tick, layout, paint, and final tick in one call.

Text glyph counts depend on the fonts available to the test process. Guard
glyph assertions or assert on the presence of text items instead of a fixed
glyph count when tests must run in a minimal environment.

## Overlays

Overlays are mounted and measured by the same runtime:

```rust
# use sequoia_ui::prelude::*;
# use sequoia_test::Harness;
let h = Harness::new(
    Panel::new()
        .width(100.0)
        .height(100.0)
        .background(Color::RED),
);
h.layout(200.0, 200.0);

let id = h.overlay_show(OverlayEntry::new(
    Panel::new()
        .width(60.0)
        .height(30.0)
        .background(Color::BLUE),
));
h.layout(200.0, 200.0);

assert_eq!(h.overlay_count(), 1);
assert_eq!(h.overlay_ids(), vec![id]);
assert!(h.overlay_rect(id).is_some());
assert!(h.overlay_hide(id));
assert_eq!(h.overlay_count(), 0);
```

`overlay_show` returns an `OverlayId`. Use `overlay_rect`, `overlay_element`,
`overlay_ids`, and `overlay_hide` to inspect and dismiss it. Overlay hit testing
follows the normal runtime order, so pointer tests can verify that a modal or
menu blocks the view below.

## Environment and platform services

`Harness` starts with a default `Environment`. Replace it when a view reads a
theme, assets, executor, or other service:

```rust
# use sequoia_ui::prelude::*;
# use sequoia_test::Harness;
let env = Environment::new();
env.provide(Theme::dark());
let h = Harness::new(Text::new("themed"));
h.set_environment(env);
h.layout(200.0, 40.0);
```

`Harness` does not create a native window, so it does not provide the runtime
`Windows` service, platform file dialogs, or a real GPU surface. Test those
pieces separately:

- Construct a `Windows` service directly and assert on its registry.
- Point `Settings` at a temporary directory with `Settings::with_dir`.
- Keep native dialog calls behind a small trait or callback that the test can
  replace; the dialog itself is a blocking platform call.
- Use Sequoia test resources or explicit resource completion instead of
  waiting for a real async executor.

## Runtime errors and lifecycle

When a view panics during a build or paint pass, the runtime records the error
and keeps the rest of the tree alive. Inspect the captured errors after a
layout:

```rust
# use sequoia_ui::prelude::*;
# use sequoia_test::Harness;
let h = Harness::new(Text::new("ok"));
h.layout(200.0, 40.0);
assert!(h.runtime().errors().is_empty());
```

`stats().errors` is the cumulative error count; `errors_seq` is a signal that
changes when errors are recorded. `on_mount` and `on_unmount` run in the
harness just as they do in the application, so mount counters and effect
cleanup can be tested directly. `h.stats().mounts`, `unmounts`, and `rebuilds`
are useful for checking that keyed updates reuse elements instead of
remounting them.

## Practical limits

- The harness is headless: it produces a render list, not pixels. For visual
  regressions, use the export or rendering pipeline separately.
- Native windows, menus, dialogs, and GPU state are out of scope.
- Font availability varies by machine; guard text-shape assertions.
- `Harness` owns one root. Create separate harnesses for independent windows,
  but remember that cloned signals are shared between them.
- Tests run on the calling thread. Do not use a harness from multiple threads;
  the runtime and views are single-threaded.
- `tick` advances the frame clock and settles effects. Animation tests may need
  several ticks or a small sleep to cross a duration boundary.
