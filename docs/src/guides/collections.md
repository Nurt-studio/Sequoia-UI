# Collections

Sequoia groups collection views by how much work they do per frame. `List`
builds every row; `VirtualList`, `Table`, and `TreeView` mount only the visible
window; `Masonry` keeps every card but packs them into columns. `ScrollView`
adds scrolling to arbitrary content.

None of these views owns a selection model. A collection renders rows from a
closure; selection, hover, and activation stay in your own `Signal`s. That keeps
the data flow explicit and lets one collection drive several views.

## Choosing a collection

| View | Virtualized | Row height | Use it for |
| --- | --- | --- | --- |
| `List` | No | Natural | Short and medium vertical lists with mixed row content |
| `VirtualList` | Yes | Fixed | Long uniform lists, logs, search results |
| `Table` | Yes | Fixed | Columns, sorting, per-column filters |
| `TreeView` | Yes | Fixed | Expandable hierarchies |
| `Masonry` | No | Natural | Card walls where rows do not share a height |
| `ScrollView` | No | Natural | Arbitrary content larger than its viewport |

Virtualization means the view builds only the visible items plus a small
overscan. It keeps element counts flat as the data grows, but it requires a
fixed item height and a stable key.

## Keys and identity

Every collection takes a key closure that returns a `u64`:

```rust
# use sequoia_ui::prelude::*;
#[derive(Clone, PartialEq)]
struct Item {
    id: u64,
    title: String,
}

let items = Signal::new(vec![Item { id: 7, title: "Seven".into() }]);
let list = List::new({
    let items = items.clone();
    move || items.get()
})
.key(|item: &Item| item.id)
.item(|item: &Item| Text::new(item.title.clone()));
```

The key decides which `ElementId` is reused when the data changes. With a stable
key, reordering a list moves existing rows instead of remounting them, so focus,
scroll position, and per-row view state survive.

- `List` falls back to the index when no key is given.
- `VirtualList` and `Table` default to a constant key, so always call `.key(...)`
  when rows can move.
- Keys must be unique among mounted rows. Derive them from a database id or hash
  a stable string; do not hash mutable display text.
- The item closure runs for each mounted row. Return owned data from the `Vec`;
  the collection does not keep your original slice.

## List

`List` is the simple vertical collection. It builds every child at its natural
height, fills the available width, and sizes itself to its content:

```rust
# use sequoia_ui::prelude::*;
let todos = Signal::new(vec![
    ("Write docs".to_string(), 1u64),
    ("Ship the release".to_string(), 2),
]);

let list = List::new({
    let todos = todos.clone();
    move || todos.get()
})
.key(|(_, id): &(String, u64)| *id)
.item(|(title, _): &(String, u64)| {
    Text::new(title.clone()).padding_y(6.0)
});

let scrollable = ScrollView::new(list).vertical();
```

`List` has no scrollbar of its own; wrap it in `ScrollView` when the content can
exceed the viewport. Because it measures every row, use `List` for dozens or a
few hundred rows, not for a 100,000-line log.

## VirtualList

`VirtualList` is the building block behind `Table` and `TreeView`. It mounts a
window of fixed-height rows around the scroll offset:

```rust
# use sequoia_ui::prelude::*;
let items = Signal::new((0..10_000u64).collect::<Vec<_>>());

let list = VirtualList::new({
    let items = items.clone();
    move || items.get()
})
.key(|id: &u64| *id)
.item(|id: &u64| Text::new(format!("Row {id}")))
.item_height(24.0)
.overscan(6)
.scrollbar_visibility(ScrollbarVisibility::Auto);
#
# let _ = list;
```

- `item_height` is fixed (default 28). The item's own height is ignored; content
  taller than the row is clipped. Use `List` if rows must size to content.
- `overscan` controls how many extra rows stay mounted beyond the viewport.
  Larger values reduce blank rows during fast scrolling at the cost of more
  elements.
- `item_indexed(|index, item| ...)` receives the position in the filtered data,
  which is useful for alternating backgrounds and row numbers.
- Wheel events scroll the list. When focused, Up/Down move by `scroll_step`,
  Page Up/Page Down move by one viewport, and Home/End jump to the ends.

The scroll position is available without a runtime handle:

```rust
# use sequoia_ui::prelude::*;
let list = VirtualList::new(move || vec![1u64, 2, 3])
    .key(|id: &u64| *id)
    .item(|id: &u64| Text::new(format!("{id}")))
    .item_height(20.0)
    .fill_size();

list.scroll_to_bottom();
assert!(list.max_offset() > 0.0);
list.scroll_to_top();
list.scroll_by(40.0);
list.page_down();
```

`scroll_to` clamps to `0..=max_offset()`, so it is safe before the first layout.
`on_scroll` observes every offset change; use it to persist a position or load
more data. `VirtualList::offset(&list)` reads the current offset. A plain
`list.offset()` call resolves to `ViewModifiers::offset(dx, dy)` from the
prelude, so use UFCS or the `on_scroll` callback.

## Table

`Table` combines a header row with a `VirtualList` of cells. Rows are your data
type; each column decides how to render one field:

```rust
# use sequoia_ui::prelude::*;
#[derive(Clone, PartialEq)]
struct FileRow {
    id: u64,
    name: String,
    size: u64,
}

let rows = Signal::new(vec![
    FileRow { id: 1, name: "notes.txt".into(), size: 2_048 },
    FileRow { id: 2, name: "photo.png".into(), size: 512_000 },
]);

let table = Table::new({
    let rows = rows.clone();
    move || rows.get()
})
.key(|row: &FileRow| row.id)
.label("Files")
.column(
    TableColumn::new("Name")
        .width(Length::Fill(2.0))
        .sort_key(|a: &FileRow, b: &FileRow| a.name.cmp(&b.name))
        .matches(|row: &FileRow, query: &str| {
            row.name.to_lowercase().contains(&query.to_lowercase())
        })
        .text_cell(|row: &FileRow| row.name.clone()),
)
.column(
    TableColumn::new("Size")
        .width(Length::Px(100.0))
        .sort_key(|a: &FileRow, b: &FileRow| a.size.cmp(&b.size))
        .text_cell(|row: &FileRow| format!("{} KB", row.size / 1024)),
)
.row_height(32.0);

assert_eq!(table.filtered_sorted().len(), 2);
```

- `TableColumn::new(header)` starts as a `Length::Fill(1.0)` column; give fixed
  columns a `Length::Px(...)` and let the flexible ones share the rest.
- `sort_key` implies `sortable()`. Clicking the header cycles
  `SortDir::None -> Asc -> Desc -> None`. Read or drive the state through
  `sort_signal()`, which exposes `Signal<Option<(usize, SortDir)>>`.
- `matches(predicate)` implies `filterable()` and defines what a query matches.
  `filterable()` alone reserves a filter but keeps every row, so pair it with
  `matches` unless you only want the filter UI.
- `cell(|row| view)` renders any view in a cell; `text_cell` is the shorthand
  for text.
- The table styles the header and optional stripes through `TableStyle`:
  `header_size`, `header_color`, and `row_stripe` (applied to odd rows in the
  filtered result).
- `.label(...)` supplies the `AccessibleRole::Table` label. The table exposes no
  row or cell semantics; interactive cells need their own labels.

Filtering and sorting happen when `filtered_sorted()` is called (or when the view
renders). Point a search field at the column's filter signal:

```rust
# use sequoia_ui::prelude::*;
# #[derive(Clone, PartialEq)]
# struct FileRow { id: u64, name: String }
# let rows = Signal::new(vec![FileRow { id: 1, name: "notes.txt".into() }]);
# let table = Table::new({ let rows = rows.clone(); move || rows.get() })
#     .key(|row: &FileRow| row.id)
#     .column(TableColumn::new("Name")
#         .matches(|row: &FileRow, q: &str| row.name.contains(q))
#         .text_cell(|row: &FileRow| row.name.clone()));
let search = TextField::new(table.column_filter(0).unwrap())
    .placeholder("Filter by name");

let active = sequoia_ui::view::table::table_filters(&table);
# let _ = (search, active);
```

`table_filters` renders a compact, read-only row of the active queries. It does
not edit them; bind your own `TextField` (or another input) to
`column_filter(index)` for that. The filter signal is read during rendering, so
setting it repaints the table.

The table has no built-in row selection or double-click callback. Put a `Button`
in a cell, or wrap the cell's content in an interactive `Container`. This keeps
keyboard activation and accessibility labels explicit instead of hiding them
inside a row click. Give the table a height: it fills its parent, so place it in
a `Column` with `flex(1.0)` or give it `fill_size()`.

## TreeView

`TreeView` flattens an expandable tree into a `VirtualList`. Each node carries an
`id`, a value, child nodes, and an `expanded` signal:

```rust
# use sequoia_ui::prelude::*;
let docs = TreeNode::branch("docs", "Docs".to_string(), vec![
    TreeNode::leaf("guide", "Guide".to_string()),
    TreeNode::leaf("reference", "Reference".to_string()),
]);
let handle = docs.clone();

let tree = TreeView::new(move || vec![docs.clone()])
    .row(|label: &String, depth| {
        Text::new(format!("{}{}", "  ".repeat(depth), label))
    })
    .row_height(26.0)
    .indent(14.0)
    .label("Documentation");

assert_eq!(tree.flatten().len(), 3);

handle.expanded.set(false);
assert_eq!(tree.flatten().len(), 1);
```

- `TreeNode::branch` starts expanded; `TreeNode::leaf` has no children.
  `.expanded(false)` collapses a branch initially. `id` should be stable and
  unique; the view remembers expansion per id and hashes it into the
  virtual-list key, so toggles survive scrolling and rebuilt nodes.
- The roots closure may rebuild nodes each frame: the first `expanded` signal
  seen for an id is kept for that tree. Store nodes in a `Signal` if you also
  need to drive their state programmatically.
- `row` receives the node value and its depth (root is 0). The built-in arrow is
  a separate click target that toggles `expanded`; the row body is not clickable
  by default.
- `indent`, `row_height`, and `overscan` behave like `VirtualList`. `TreeStyle`
  controls the arrow color and arrow font size, not the row content.
- `.label(...)` supplies the `AccessibleRole::Tree` label. The tree does not
  publish `TreeItem` children, so put meaningful labels on interactive row
  content.
- `flatten()` returns `FlatRow<T>` values (`id`, `depth`, `has_children`,
  `expanded`, `value`). It is useful for tests and for rendering a flat
  selection index.

`TreeView` has no selection model either. Keep `Signal<Option<String>>` with the
selected node id and render that state in the `row` closure.

## Masonry

`Masonry` places each child in the shortest column, so cards keep their natural
height and rows do not align:

```rust
# use sequoia_ui::prelude::*;
let cards = Masonry::new(3)
    .gap(12.0)
    .align_items(Align::Stretch)
    .children([
        Panel::new().height(80.0),
        Panel::new().height(140.0),
        Panel::new().height(60.0),
        Panel::new().height(100.0),
    ]);
```

- `columns` is clamped to at least one. Use `gap` for both axes, or
  `column_gap` and `row_gap` separately.
- `align_items` accepts `Align::Start`, `Center`, `End`, and `Stretch`; it
  controls how a narrow card sits inside its column.
- The default width fills the parent and the height fits the tallest column.
  `.fit()` shrink-wraps the container to the widest card in each column.
- Absolute-positioned children skip the column flow.
- Every child is measured and mounted. Use masonry for a few dozen cards; for
  hundreds, paginate or build rows with `VirtualList`.
- Wrap a tall masonry in `ScrollView`; like `List`, it has no scrollbar itself.

## Selection and activation

Because selection belongs to your model, the pattern is the same for every
collection: a signal stores the selected key, and the item closure sets it from
an interactive child.

```rust
# use sequoia_ui::prelude::*;
let selected = Signal::new(None::<u64>);

let list = List::new(move || vec![10u64, 20, 30])
    .key(|id: &u64| *id)
    .item({
        let selected = selected.clone();
        move |id: &u64| {
            let id = *id;
            let selected = selected.clone();
            Container::new()
                .fill_width()
                .padding(6.0)
                .on_click(move || selected.set(Some(id)))
                .child(Text::new(format!("Item {id}")))
        }
    });

assert_eq!(selected.get(), None);
```

Read the same `selected` signal inside the item closure to paint the selected
row. Keep the key stable so the highlighted row does not lose its element
identity when the data is reordered.

## Accessibility and keyboard

- `Table` reports `AccessibleRole::Table` and `TreeView` reports
  `AccessibleRole::Tree`, both with the `.label(...)` text. Their rows are not
  separate accessible nodes; interactive cells and custom rows must label
  themselves.
- `List`, `VirtualList`, and `Masonry` do not add container roles. The cells'
  own widgets provide the semantics.
- `VirtualList` is focusable: when it has focus, arrows, Page Up/Page Down, and
  Home/End scroll it. `List` and `Masonry` are not focusable containers, so
  focus their interactive children instead.
- A table header is clickable but not keyboard-focusable. If sorting is
  essential, add a small toolbar with buttons that set `sort_signal()`.
- Do not put a focusable control inside a row that also handles clicks; the two
  activation paths will compete. Choose a row button or per-cell controls.

## Performance notes

- Virtualized views are cheap because they skip unmounted rows, not because
  sorting is free. `Table` filters and sorts the whole data set on each render;
  keep the row closure cheap and cache derived data if the set is large.
- Give every virtualized row a stable key. Index keys make every row below an
  insertion look new.
- Fixed heights are a feature: they make offset calculations O(1). For variable
  heights, use `List` inside `ScrollView` or split the data into fixed-height
  sections.
- Avoid allocating large strings in `item`/`cell` closures when a borrowed or
  cheaper representation is available. Rows rebuild on scroll.
- `Masonry` measures all children on layout. It is a layout tool, not a
  virtualization strategy.

See [Scrolling](scrolling.md) for the `ScrollView` API, scroll offsets, and
scroll-into-view helpers.
