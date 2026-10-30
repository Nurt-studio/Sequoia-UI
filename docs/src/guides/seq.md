# Seq

Seq is Sequoia's declarative UI language. A `.seq` file describes a window,
widgets, styles, and reusable components without writing Rust. Static files can
be loaded and previewed directly; files that need application state or native
callbacks are connected to Rust through `HostBindings`.

The language is deliberately small. Values are literals or simple references,
not general expressions. Arithmetic, branches, and loops belong in `fn`
declarations and run only from callbacks.

## Loading a file

`SeqLoader` parses, checks, and lowers a file into an `AnyView`:

```rust,no_run
# use sequoia_ui::prelude::*;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
let loaded = SeqLoader::load_file("Main.seq")?;
let (width, height) = loaded.meta.size;

let app = App::new()
    .title(loaded.meta.title)
    .size(width, height)
    .theme(loaded.meta.theme)
    .root(loaded.view);
# let _ = app;
# Ok(())
# }
```

`LoadedApp` contains:

- `meta`: `title`, `size`, and the resolved `theme`.
- `view`: the root view.
- `files`: the main file and every imported file that was read. A preview
  watcher uses this list to know when to reload.

The quickest way to try a static file is the preview binary:

```text
cargo run -p sequoia-preview -- Main.seq --watch
```

Without `--watch`, the file is loaded once. With it, the preview restarts when
the main file or an imported file changes.

`SeqLoader` also has in-memory entry points for tests and tools:
`load_str`, `load_str_with`, `load_str_map`, and `load_str_with_map`. The
`*_map` variants take `&[(&str, &str)]` and resolve imports without touching the
filesystem.

## App and window

A file contains one `app` block. `title`, `size`, and at least one `window` are
required:

```text
app Hello {
  title: "Hello"
  size: (320, 160)
  theme: dark
  window Main {
    Column(padding: 16, gap: 8) {
      Text("Hello from Seq", size: 20, bold: true)
      Text("Static files need no Rust bindings.")
    }
  }
}
```

`theme` accepts `dark`, `light`, or a quoted path to a JSON theme-token file.
The path is resolved relative to the `.seq` file. The loader lowers the first
window into the returned view; additional `window` blocks are parsed but are not
mapped to separate native windows by `SeqLoader`.

The static example above can be loaded and painted without host bindings:

```rust
# use sequoia_ui::prelude::*;
# use sequoia_test::Harness;
const APP: &str = r#"
app Hello {
  title: "Hello"
  size: (320, 160)
  window Main {
    Column(padding: 16, gap: 8) {
      Text("Hello from Seq", size: 20, bold: true)
      Text("Static files need no Rust bindings.")
    }
  }
}
"#;

let loaded = SeqLoader::load_str("Hello.seq", APP).unwrap();
assert_eq!(loaded.meta.title, "Hello");
assert_eq!(loaded.meta.size, (320.0, 160.0));

let h = Harness::new(loaded.view);
h.layout(320.0, 160.0);
assert!(!h.paint().is_empty());
```

## Nodes and attributes

A node is a component name, an optional positional label, named attributes, and
an optional child block:

```text
Column(padding: 12, gap: 8) {
  Text("Title", size: 18, bold: true)
  Button("Continue", on_click: { continue_app() })
}
```

Positional labels are accepted only by `Text`, `Button`, and `Image`. All other
components use named attributes. Every attribute has the form `name: value`.
There are no positional component arguments and no arbitrary expressions in an
attribute.

Common value forms are:

- Strings, numbers, booleans, and colors such as `#RRGGBB` or `#RRGGBBAA`.
- `px(12)`, `fill(1)`, `percent(50)`, and `fit` for lengths.
- Tuples `(x, y)` and arrays `[width, color]`.
- `theme.bg`, `theme.surface`, `theme.border`, `theme.ink`, `theme.text`,
  `theme.muted`, `theme.text_secondary`, `theme.accent`, and `theme.disabled`.
- `env.name` for a host environment value.
- `dynamic(name)` for a host property.
- `item.field` inside a `List` or `VirtualList` template.

The UI catalog includes `Column`, `Row`, `Container`, `Panel`, `ScrollView`,
`Text`, `Button`, `Spacer`, `Icon`, `Image`, `List`, `VirtualList`, `Canvas`,
`Stack`, `Grid`, `Wrap`, `Masonry`, `SplitPanel`, the input widgets, `If`,
`Opacity`, `Scale`, `Rotate`, `Translate`, `Pulse`, `Shake`, `Async`, and
`Effect`. See the Widgets, Inputs, Collections, and Canvas guides for their
Rust equivalents. Seq shape nodes such as `Rect`, `Circle`, `Line`, and `Path`
are valid only inside a `Canvas`.

`Panel` is a background/frame leaf. Use `Container` when the node needs
children. A `Canvas` accepts shapes and `List`, not ordinary UI views or
components.

## State and functions

State is declared inside the `app` block with a type and a literal initial
value. Functions can read and write that state:

```text
app Counter {
  title: "Counter"
  size: (240, 160)
  prop count: Int
  state clicks: Int = 0
  fn bump() { clicks = clicks + 1; count = clicks; }
  window Main {
    Column(padding: 12, gap: 8) {
      Text("Seq counter")
      Button("Bump", on_click: { bump() })
      Text(dynamic: count)
    }
  }
}
```

Seq state is internal to the mounted runtime. It is not a widget property and
does not automatically trigger a rebuild when it changes. To show state, write
it to a host property as the example does with `count = clicks`. The host
property signal then updates the `dynamic` text.

State survives callback calls and view rebuilds while the app is mounted. It is
not persisted to disk and is reset when the app or preview restarts.

Functions support parameters, local `let`/`mut` bindings, assignment, `if`,
`for`, `match`, and `return`. They can be called from callbacks and `on_run`;
they cannot be called from ordinary attribute values. The evaluator has a
100,000-step limit and a call-depth limit of 64.

## Host properties and callbacks

A host-bound app declares the properties and callbacks it needs:

```text
app Actions {
  title: "Actions"
  size: (240, 120)
  on save()
  window Main {
    Button("Save", on_click: { save() })
  }
}
```

Connect them with `HostBindings` and load with `load_str_with`:

```rust
# use std::cell::Cell;
# use std::rc::Rc;
# use sequoia_ui::prelude::*;
# use sequoia_test::Harness;
const APP: &str = r#"
app Actions {
  title: "Actions"
  size: (240, 120)
  on save()
  window Main {
    Button("Save", on_click: { save() })
  }
}
"#;

let called = Rc::new(Cell::new(false));
let flag = called.clone();
let bindings = HostBindings::new().on("save", move || flag.set(true));
let loaded = SeqLoader::load_str_with("Actions.seq", APP, &bindings).unwrap();

let h = Harness::new(loaded.view);
h.layout(240.0, 120.0);
h.paint();
h.tick();
let root = h.root().unwrap();
let button = h.children(root)[0];
let rect = h.rect(button).unwrap();
h.click(
    rect.left() + rect.size.width * 0.5,
    rect.top() + rect.size.height * 0.5,
);
h.tick();
assert!(called.get());
```

`prop` declarations accept `String`, `Bool`, `Int`, and `Float`. Bind them with
`prop_str`, `prop_bool`, `prop_int`, and `prop_float`. `memo_*` and `env_*`
variants expose derived and environment values; `dynamic(name)` can read them.

`on` declarations accept these signatures:

| Seq signature | Host binding |
| --- | --- |
| `on action()` | `on("action", ...)` |
| `on action(value: Int)` | `on_int("action", ...)` |
| `on action(value: Float)` | `on_float("action", ...)` |
| `on action(value: String)` | `on_str("action", ...)` |
| `on action(value: Bool)` | `on_bool("action", ...)` |
| `on action(x: Float, y: Float)` | `on_point("action", ...)` |
| `on action(x: Float, y: Float, name: String)` | `on_point_named("action", ...)` |
| `on action(x: Float, y: Float, d: Float)` | `on_wheel` or `on_pinch` |

An `on` name that matches a Seq `fn` does not need a host binding. A callback
declared only for the host must have a matching binding, or loading fails.

`Text(dynamic: name)` reads a host property. `Text(text: dynamic(name))` is the
equivalent named-attribute form. Neither form accepts an arbitrary expression:

```text
Text(dynamic: count)
Text(text: dynamic(count))
Text(t: "save")
```

`Text(t: "key")` uses an `I18n` instance installed with `HostBindings::i18n`.

## Components

A component is a reusable node block. Component names start with an uppercase
letter and cannot collide with built-in components. Parameters use only
`String`, `Bool`, `Int`, `Float`, or `Color`:

```text
component Card(title: String) {
  Container(padding: 12, radius: 8, background: theme.surface) {
    Column(gap: 4) {
      Text(text: title, bold: true)
      children
    }
  }
}
```

Call components with named parameters. A component may contain one bare
`children` tag; the caller's children are spliced at that point:

```rust
# use sequoia_ui::prelude::*;
# use sequoia_test::Harness;
const APP: &str = r#"
component Card(title: String) {
  Container(padding: 12, radius: 8, background: theme.surface) {
    Column(gap: 4) {
      Text(text: title, bold: true)
      children
    }
  }
}
app Components {
  title: "Components"
  size: (260, 160)
  window Main {
    Card(title: "Build") {
      Text("Finished in 2.4s")
    }
  }
}
"#;

let loaded = SeqLoader::load_str("Components.seq", APP).unwrap();
let h = Harness::new(loaded.view);
h.layout(260.0, 160.0);
assert!(!h.paint().is_empty());
```

A component without a `children` placeholder rejects children. `children` is
only valid inside a component body and cannot appear inside a `Canvas`.
Components cannot be used inside a `Canvas`; canvas content is limited to shapes
and `List`.

Components in another file must be `pub component`:

```text
import "widgets.seq"
app Imports {
  title: "Imports"
  size: (220, 120)
  window Main {
    Badge(text: "New")
  }
}
```

Imports are resolved relative to the importing file. Imported files cannot
contain an `app` block. Cycles are rejected, imports are limited to 32 levels,
and each imported file is read once. `load_str_map` supplies the imported files
directly:

```rust
# use sequoia_ui::prelude::*;
# use sequoia_test::Harness;
const MAIN: &str = r#"
import "widgets.seq"
app Imports {
  title: "Imports"
  size: (220, 120)
  window Main {
    Badge(text: "New")
  }
}
"#;
const WIDGETS: &str = r#"
pub component Badge(text: String) {
  Container(padding: 6, radius: 999, background: theme.accent) {
    Text(text: text, color: theme.bg)
  }
}
"#;

let loaded = SeqLoader::load_str_map(
    "Main.seq",
    MAIN,
    &[("widgets.seq", WIDGETS)],
)
.unwrap();
let h = Harness::new(loaded.view);
h.layout(220.0, 120.0);
assert!(!h.paint().is_empty());
```

## Styles

A `style` block is declared inside the `app` block. Its target is `Box`
(`Container` or `Panel`), `Text`, or `Button`. Styles can extend another style of
the same target:

```text
style Surface: Box { background: theme.surface, radius: 8 }
style Card: Box extends Surface { border: [1, theme.border] }
```

Apply it with the `style` attribute:

```text
Container(style: Card, padding: 12) {
  Text("Styled card")
}
```

Box styles support `background`, `radius`, `border`, per-side border fields,
`shadow`, and `inset_shadow`. Text styles support `size`, `color`, one weight
flag, and `line_height`. Button styles support background/hover/pressed/disabled
colors, text colors, font size/weight, radius, padding, and border fields.
Buttons also accept the presets `primary`, `secondary`, `danger`, `ghost`,
`dark_filled`, and `outline_rose`.

Styles are resolved when the file is loaded. They do not switch with a runtime
theme; changing the theme means reloading with a different `theme` value or
building the UI in Rust.

## Lists and rows

`List` and `VirtualList` take the name of a host rows property, not inline data.
The host supplies rows with `HostBindings::rows`:

```rust
# use sequoia_ui::prelude::*;
# use sequoia_test::Harness;
#[derive(Clone, PartialEq)]
struct Row {
    id: i64,
    name: String,
}

impl SeqRow for Row {
    fn id(&self) -> i64 {
        self.id
    }

    fn fields(&self) -> Vec<(String, FieldSignal)> {
        vec![(
            String::from("name"),
            FieldSignal::Str(Signal::new(self.name.clone())),
        )]
    }
}

const APP: &str = r#"
app Rows {
  title: "Rows"
  size: (240, 180)
  on open(id: Int)
  window Main {
    List(items: rows, key: id) {
      Text(dynamic: item.name)
      Button("Open", on_click: { open(item.id) })
    }
  }
}
"#;

let rows = Signal::new(vec![
    Row { id: 1, name: String::from("Ada") },
    Row { id: 2, name: String::from("Linus") },
]);
let bindings = HostBindings::new()
    .rows("rows", rows)
    .on_int("open", |_id| {});
let loaded = SeqLoader::load_str_with("Rows.seq", APP, &bindings).unwrap();
let h = Harness::new(loaded.view);
h.layout(240.0, 180.0);
assert!(!h.paint().is_empty());
```

A row must implement `SeqRow`, which supplies an `i64` ID and named fields.
`FieldSignal` can be `Str`, `Bool`, `Int`, or `Float`. Read a field with
`item.field`; the row ID is available as `item.id`. `key` currently accepts only
`id`. `VirtualList` adds `item_height` and `overscan` attributes for large data
sets.

Rows are snapshotted when the list rebuilds. Put durable data in the row signal
or in application state, not in the temporary field signals.

## Canvas

A static canvas contains shapes with local coordinates:

```rust
# use sequoia_ui::prelude::*;
# use sequoia_test::Harness;
const APP: &str = r#"
app Board {
  title: "Board"
  size: (320, 220)
  window Main {
    Canvas(background: theme.bg) {
      Rect(x: 24, y: 24, w: 120, h: 80, fill: theme.accent, radius: 8)
      Circle(c: (220, 80), r: 24, fill: theme.surface)
    }
  }
}
"#;

let loaded = SeqLoader::load_str("Board.seq", APP).unwrap();
let h = Harness::new(loaded.view);
let stats = h.paint_stats(320.0, 220.0);
assert!(stats.quads >= 2, "{stats:?}");
```

Only shapes and `List` belong inside a `Canvas`; shapes outside a `Canvas` are
rejected. Components and ordinary UI views cannot be placed inside it.

Pointer callbacks are declared at app level and connected through host actions:

```text
app Board {
  title: "Board"
  size: (320, 220)
  on click_at(x: Float, y: Float)
  on drag_to(x: Float, y: Float)
  window Main {
    Canvas(on_click_at: { click_at(x, y) }, on_drag: { drag_to(x, y) }) {
      Rect(x: 24, y: 24, w: 120, h: 80, fill: theme.accent)
    }
  }
}
```

Bind `click_at` and `drag_to` with `on_point`. A three-argument
`on_click_at`/`on_press`/`on_drag` callback can receive the shape name or a drag
delta; `on_wheel` and `on_pinch` receive `(x, y, delta)`. Canvas coordinates are
local to the canvas. See the Canvas guide for camera, grid, and rendering
details.

## Async and effects

`Async` is a host-backed state switch. `source` names a
`HostBindings::resource` value, and the body contains state blocks:

```text
Async(source: users) {
  Loading { Spinner }
  Data { Text(dynamic: data) }
  Error { Text(dynamic: error) }
  Idle { Text("Not loaded") }
  Cancelled { Text("Cancelled") }
}
```

`Loading`, `Data`, `Error`, `Idle`, and `Cancelled` are only valid directly
inside `Async`. The `Data` block receives a `data` string property and the
`Error` block receives an `error` string property. Resources in this API are
`Resource<String>` values.

`Effect` runs a callback when a tracked host value changes:

```text
Effect(track: dynamic(flag), on_run: { refresh() })
```

`track` must name a host property, and `on_run` takes no arguments. The effect
does not fire on initial mount; it fires after the tracked value changes.
`Async` and `Effect` require host bindings and cannot be used in a static file.

## Testing

Use `SeqLoader` with `Harness` to test a complete `.seq` app. Load the file,
mount `loaded.view`, lay it out, and interact with the same tree a real window
uses. The examples in this guide use that pattern for a counter, a host action,
rows, and canvas shapes.

For a host app, keep the `Signal`s and `HostBindings` in the test fixture so
assertions can inspect both the UI and the application state. A dynamic text
node can be checked with `Harness::text`; a canvas can be checked with
`paint_stats`; a button can be clicked through its `Harness::rect`.

## Limits and errors

- One `app` block per root file; `title`, `size`, and at least one `window` are
  required.
- Component names start uppercase, must be unique, cannot shadow built-ins, and
  may contain at most one `children` placeholder.
- Imports are relative, cycle-checked, read once, and limited to 32 levels.
- Component nesting is limited to 32 levels and expansion to 100,000 nodes.
- Parser nesting is limited to 256 levels.
- Function evaluation is limited to 100,000 steps and 64 nested calls.
- Static loading rejects host properties, callbacks, `dynamic`, `env`, `List`,
  `Async`, and `Effect`.
- `SeqError` exposes `file`, `line`, `col`, and `message`; its display form is
  `file:line:col: message`. Some checker messages are still in Polish.

## Related guides

- [Widgets](widgets.md) for the UI catalog.
- [Collections](collections.md) for `List` and `VirtualList`.
- [Canvas](canvas.md) for shapes and pointer callbacks.
- [Assets](assets.md) for icons, fonts, and images referenced by a Seq app.
- [Custom views](custom-views.md) when the UI needs Rust behavior beyond the
  Seq language.
