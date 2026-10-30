# Docking

Sequoia has a small static edge layout and a larger model-driven docking
system. Choose the right level before reaching for APIs:

| Need | Use |
| --- | --- |
| A fixed toolbar, sidebar, and main area | `DockArea` |
| Tabs, splitters, movable panels, floating cards | `DockModel` + `DockView` |
| Panel chrome for a custom dock shell | `Dockable` / `DockPanel` |
| Panels in separate OS windows | `DockWindowHost` + `DockWindowRoot` |

`DockArea` is covered first because it is the simpler layout primitive.
`DockView` and its model are the IDE-style docking system.

## Static edge layout

`DockArea` reserves edges and gives the remaining space to a fill child. It
does not create tabs, drag panels, or persist anything:

```rust
# use sequoia_ui::prelude::*;
let shell = DockArea::new()
    .top(Text::new("Toolbar").height(40.0))
    .left(Text::new("Explorer").width(240.0))
    .bottom(Text::new("Status").height(24.0))
    .fill(Text::new("Editor"));

let shell = shell
    .gap(2.0)
    .padding(4.0);
# let _ = shell;
```

`DockArea::left/right/top/bottom/fill/center` add children. By default it
uses border-mode order: top and bottom docks are laid out first, then left and
right, then the fill child. `.ordered(true)` lays children out in declaration
order instead; `.last_child_fill(true)` only has an effect in that mode. In
border mode, use an explicit `.fill` for the flexible child and keep it last.
Use `dock_at(index, child, Dock::Left)`, `remove`, `move_child`, and `set_dock`
for dynamic rearrangement, or `*_if`/`*_opt` for conditional slots. For a
scrollable sidebar inside the filled area, compose `DockArea` with
`ScrollView` or `SplitPanel`.

## The docking model

`DockModel` is a plain value tree. It is cheap to snapshot, compare, serialize,
and put in a `Signal`:

```rust
# use sequoia_ui::prelude::*;
let mut model = DockModel::single(PanelId::new(1));

model.apply(DockOp::Insert {
    panel: PanelId::new(2),
    target: PanelId::new(1),
    drop: DockDrop::Right,
    frac: 0.3,
    moving: false,
});

model.apply(DockOp::Insert {
    panel: PanelId::new(3),
    target: PanelId::new(2),
    drop: DockDrop::Tab,
    frac: 0.5,
    moving: false,
});

assert_eq!(
    model.panels(),
    vec![PanelId::new(1), PanelId::new(2), PanelId::new(3)]
);
assert_eq!(model.active_tab_of(PanelId::new(3)), Some(1));
```

A model contains:

- `root: Option<DockNode>` for docked panels.
- `floating: Vec<FloatingPanel>` for overlay or detached panels.

`DockNode` has three shapes:

```rust
# use sequoia_ui::prelude::*;
let tree = DockNode::Split {
    axis: SplitAxis::Horizontal,
    frac: 0.35,
    first: Box::new(DockNode::Leaf(PanelId::new(1))),
    second: Box::new(DockNode::Tabs {
        tabs: vec![PanelId::new(2), PanelId::new(3)],
        active: 0,
    }),
};

assert_eq!(tree.path_to(PanelId::new(3)), Some(vec![true]));
assert!(tree.contains(PanelId::new(2)));
```

- `Leaf(panel)` is one panel filling its slot.
- `Tabs { tabs, active }` stacks panels in one slot.
- `Split { axis, frac, first, second }` divides a slot. `Horizontal` puts
  `first` left and `second` right; `Vertical` puts them top and bottom. `frac`
  is the share of the first child, clamped to `0..=1`.

`PanelId` is a `u64` newtype. Use stable IDs: they are the identity used by the
view, drag payloads, and saved layouts. `PanelId::new(7)`, `id.get()`, and
`id.0` are equivalent. `Display` prints `panel#7`.

## Building a DockView

`DockView` renders a `Signal<DockModel>` and calls your render closure for the
active panel in each tab group:

```rust
# use sequoia_ui::prelude::*;
let model = Signal::new(DockModel::single(PanelId::new(1)));

let dock = DockView::new(model.clone(), |panel| {
    AnyView::new(Text::new(format!("Content for {}", panel.get())))
})
.title_provider(|panel| format!("Panel {}", panel.get()))
.tabstrip_height(30.0)
.handle_width(8.0)
.edge(0.25)
.empty_label("Drop a panel here");
# let _ = dock;
```

- `new(model, render)` requires a renderer that returns `AnyView`. In
  `DockView`, it runs for the active tab of each tab group. Pass the same
  renderer to `DockFloatLayer` if floating panels should show the same content.
- `title_provider` supplies tab titles. The default is `panel#<id>`.
- `tabstrip_height` defaults to 28 px.
- `handle_width` defaults to 7 px and is clamped to at least 3 px.
- `edge` is the fraction of a slot treated as a split edge when dropping.
  It defaults to `EDGE_FRACTION` (0.22) and is clamped to `0.05..=0.45`.
- `empty_label` is shown when the root is empty.
- `width`, `height`, and `fill_size` size the dock. It fills its parent by
  default.

The renderer should be pure and keyed by `PanelId`; keep all mutable state in
signals. The same renderer can be shared by `DockView`, `DockFloatLayer`, and a
detached-window root.

`DockView` has no style builder and no background. Its tab strip and split
handles use fixed colors, so put it in a `Container` if the surrounding shell
needs a themed surface.

## Operations

Mutate through `Signal::update` or a local `DockModel`:

```rust
# use sequoia_ui::prelude::*;
let mut model = DockModel::single(PanelId::new(1));

model.apply(DockOp::Insert {
    panel: PanelId::new(2),
    target: PanelId::new(1),
    drop: DockDrop::Right,
    frac: 0.5,
    moving: false,
});

model.apply(DockOp::Insert {
    panel: PanelId::new(3),
    target: PanelId::new(2),
    drop: DockDrop::Tab,
    frac: 0.5,
    moving: false,
});

model.apply(DockOp::Activate { panel: PanelId::new(2) });
model.apply(DockOp::MoveTab { panel: PanelId::new(3), to: 0 });
assert_eq!(model.active_tab_of(PanelId::new(3)), Some(0));

model.apply(DockOp::SetFrac {
    panel: PanelId::new(1),
    frac: 0.6,
});
assert!((model.frac_at(&[]).unwrap() - 0.6).abs() < 1e-6);

model.apply(DockOp::Float {
    panel: PanelId::new(2),
    x: 80.0,
    y: 60.0,
    w: 320.0,
    h: 240.0,
});
assert!(model.is_floating(PanelId::new(2)));

model.apply(DockOp::DockFloating {
    panel: PanelId::new(2),
    target: PanelId::new(1),
    drop: DockDrop::Tab,
    frac: 0.5,
});
assert!(!model.is_floating(PanelId::new(2)));

model.apply(DockOp::Close { panel: PanelId::new(3) });
assert!(!model.contains(PanelId::new(3)));
```

| Operation | Effect |
| --- | --- |
| `Insert { panel, target, drop, frac, moving }` | Adds or moves `panel` relative to `target`: `Tab`, a side, or `Float` |
| `Float { panel, x, y, w, h }` | Removes a docked panel from the tree and places it in `floating` |
| `DockFloating { panel, target, drop, frac }` | Moves a floating panel back into the tree |
| `Close { panel }` | Removes from the tree and from `floating`, collapsing empty splits |
| `SetFrac { panel, frac }` | Sets the split fraction of the deepest split containing `panel` |
| `Activate { panel }` | Selects a tab |
| `MoveTab { panel, to }` | Reorders a panel within its tab group and activates it |

`apply` returns `false` for invalid operations, such as moving a panel onto
itself, inserting a panel that already exists with `moving: false`, or using
`DockDrop::Float` with `Insert`. `DockOp` is `#[non_exhaustive]`, and `apply`
does not panic on malformed operations. The table above is not the full set of
model helpers; `set_frac_at(path, frac)`, `frac_at(path)`, `dock_first_tab`,
`attach_window`, and `path_to` are also public.

Fractions are sanitized with `DockNode::sanitized_frac`: finite values are
clamped to `0..=1`, and non-finite values become `0.5`. A tab group's `active`
index is clamped to its last tab.

## Tabs, splits, and built-in gestures

`DockView` implements the common IDE gestures:

- Click a tab to activate it.
- Press a tab and move past the drag threshold to start dragging its
  `PanelPayload`.
- Drop a dragged tab in the center of another slot to add it as a tab.
- Drop near an edge to split that slot. The edge zone is controlled by
  `DockView::edge`.
- Hold Alt while dropping to float the panel instead.
- Drag the divider between split panes to change the fraction.
- Float cards can be dragged by their headers.

Drop previews are painted in blue and show the exact tab or split region that
will be created. Alt-drop uses `DockDrop::Float`; otherwise edge detection uses
`drop_from_fraction(fraction, edge)`, which returns `Left`, `Right`, `Top`,
`Bottom`, or `Tab` (`EDGE_FRACTION` is the default 0.22).

The built-in UI intentionally stays small:

- There is no tab overflow menu or scrollable tab strip.
- There is no close button on tabs. Add a command or context menu that applies
  `DockOp::Close`.
- Dragging a tab within its own strip is not a reorder gesture. Use
  `DockOp::MoveTab` from a menu, keyboard command, or tab strip customization.
- There is no keyboard docking or split-resizing interface, and the dock views
  add no accessible roles beyond the text in panel content.
- Dropping a tab onto its own content is ignored.

For custom docking gestures, use the lower-level pieces:
`drop_from_fraction_or_float(fraction, edge, force_float)` chooses a
`DockDrop`; `dock_preview_rect(host, drop, frac)` computes a preview rectangle;
`DockNode::path_to(panel)` locates a tab; and `set_frac_at(path, frac)` updates
a specific split. `PanelPayload(PanelId)` is the payload type used by dock
headers and tab buttons.

## Floating panels

A floating panel lives in `DockModel::floating` and is rendered by
`DockFloatLayer`, not by the dock tree:

```rust
# use sequoia_ui::prelude::*;
# use sequoia_test::Harness;
fn render(panel: PanelId) -> AnyView {
    AnyView::new(Text::new(format!("Panel {}", panel.get())))
}

let model = Signal::new(DockModel::single(PanelId::new(1)));
model.update(|m| {
    m.apply(DockOp::Float {
        panel: PanelId::new(1),
        x: 40.0,
        y: 30.0,
        w: 320.0,
        h: 240.0,
    });
});

let shell = Stack::new()
    .child(DockView::new(model.clone(), render).fill_size())
    .child(DockFloatLayer::new(model.clone(), render).fill_size());

let h = Harness::new(shell);
h.layout(800.0, 600.0);
let layers = h.children(h.root().expect("root"));
assert_eq!(layers.len(), 2);
let cards = h.children(layers[1]);
assert_eq!(cards.len(), 1);
assert_eq!(h.rect(cards[0]).expect("card").left(), 40.0);
```

`DockFloatLayer` is an absolute-position overlay. It uses the floating card's
`x`, `y`, `w`, and `h` and only renders panels whose `window` field is `None`.
Keep it above `DockView` in a `Stack` so cards are not clipped by dock content.
The layer itself is not hit-testable; its cards are.

`DockFloatLayer` options:

- `title_provider` matches the dock's titles.
- `card_background` and `header_background` set the card colors.
- `width`, `height`, and `fill_size` size the overlay.

Floating cards can be dragged by their title bar. The header also has buttons
to dock the panel back as the first tab (`dock_first_tab`) and to close it. Card
content does not start a card drag, so controls inside a floating panel keep
working.

`FloatingPanel` is public if you need to inspect or construct entries:
`FloatingPanel::overlay(panel, x, y, w, h)` creates an in-window card,
`is_overlay()` tests `window.is_none()`, and `window_id()` converts a `window`
field to `WindowId`.

## Detached windows

A floating panel with `window: Some(id)` is not drawn by `DockFloatLayer`.
Instead, the application creates a native window for it. `DockWindowHost`
watches the model and reports when windows need to open or close:

```rust
# use sequoia_ui::prelude::*;
let model = Signal::new(DockModel::single(PanelId::new(1)));
let render = |panel: PanelId| AnyView::new(Text::new(format!("Panel {}", panel.get())));

let host = DockWindowHost::new(model.clone(), render)
    .title_provider(|panel| format!("Panel {}", panel.get()))
    .on_open(|window| {
        let _ = (window.panel, window.title, window.w, window.h);
    })
    .on_close(|window_id| {
        let _ = window_id;
    });
# let _ = host;
```

`DetachedWindow { panel, title, w, h }` is passed to `on_open`; the app is
responsible for allocating the actual window and returning its id. Use
`DockWindowHost::detach(&model, panel, w, h, alloc)` to float a panel and attach
the window id in one step, and `DockModel::attach_window(panel, Some(id))` /
`attach_window(panel, None)` to change the association.

Inside the detached window, use `DockWindowRoot::new(model, panel, render)` as
the root view. It renders that panel while it exists in the model and shows a
fallback when the panel has been docked elsewhere. A `DockWindowHost` must stay
in the main view tree so it can reconcile model changes; it has no visual
content.

## Persistence and app state

The model serializes to a tagged text format:

```rust
# use sequoia_ui::prelude::*;
let model = DockModel::single(PanelId::new(1));
let text = dock_encode(&model);
let restored = dock_decode(&text).expect("valid dock layout");
assert_eq!(restored, model);
```

`dock_decode` validates the `dock-v1` tag, parses the tree and floating list,
and sanitizes fractions and active indexes. Invalid input returns
`DecodeError`; it never panics. Treat saved layouts as untrusted input and fall
back to a fresh model on error.

For filesystem persistence, use `DockLayoutStore`:

```rust,no_run
# use sequoia_ui::prelude::*;
let store = DockLayoutStore::new(Settings::with_dir("state"))
    .file_name("dock-layout.txt");
let model = DockModel::single(PanelId::new(1));
store.save(&model).expect("save dock layout");
let restored = store.load();
```

`save` writes the encoded model under `Settings::dir()`, and `load` returns
`None` for a missing or corrupt file. Save after the model changes (for example
with an effect or on window close), not on every drag frame.

## Custom panel chrome

`DockView` draws tabs and split handles itself. `Dockable` and `DockPanel` are
standalone pieces for custom dock shells or sidebars:

```rust
# use sequoia_ui::prelude::*;
let panel = DockPanel::new(PanelId::new(1), "Layers", Text::new("Layer list"))
    .header_height(30.0)
    .header_background(Color::hex(0x2C2C30))
    .background(Color::hex(0x1E1E22))
    .radius(6.0)
    .closable(true)
    .on_close(|| {});

let simple = Dockable::new(PanelId::new(2), "Inspector", Text::new("Properties"))
    .header_height(28.0);
# let _ = (panel, simple);
```

- `Dockable` stacks a `DockHeader` above its content without a close button.
- `DockPanel` adds an optional background, radius, and close button.
- A `DockHeader` starts a drag with `PanelPayload(panel)` when pressed. That
  payload is accepted by `DockView` slots, so these headers can drive a custom
  shell built around the docking model.
- `DockHeader::on_drag_start` runs when the press registers; `panel()` returns
  the header's `PanelId`.
- Both panel types fill their parent by default and arrange header and content
  vertically.

## Testing docking

Test the model first, then the view:

- Apply `DockOp`s to a plain `DockModel` and assert `panels`, `contains`,
  `active_tab_of`, `is_floating`, and `frac_at`.
- Use `Harness::new(DockView::new(model.clone(), render).fill_size())` and
  `layout` to inspect child counts and rectangles, as in the floating example.
- Drive tabs with `Harness::pointer_down`/`pointer_up`, or use `Harness::drag`
  to exercise tab drops. The harness exposes `drag_session`, `has_pending_drag`,
  and `drop_target_at` for precise assertions.
- Round-trip `dock_encode`/`dock_decode`, or `DockLayoutStore::save/load`, to
  catch model changes that break saved layouts.

Keep panel IDs stable across sessions. They are the only link between a saved
layout and the content your renderer supplies.
