# Layout

Sequoia layout has two passes. The measure pass asks each view how large it
wants to be for the available constraints. The arrange pass assigns each view
its final rectangle. The runtime repeats both passes after a size change or a
reactive update that changes layout.

Most interfaces are built from flex containers, grids, and wrapping rows.
Reach for a custom leaf only when the arrangement needs behavior that these
primitives do not express.

## The box model

Every view has a `LayoutStyle`. It controls the requested width and height,
padding, margin, minimum and maximum sizes, flex behavior, positioning, and
overflow. The built-in containers expose setters for the common fields:

```rust
# use sequoia_ui::prelude::*;
let inspector = Column::new()
    .width(Length::px(320.0))
    .fill_height()
    .padding(Edges::xy(16.0, 12.0))
    .background(Color::hex(0x161b22))
    .border(1.0, Color::hex(0x30363d))
    .radius(8.0)
    .child(Text::new("Inspector"))
    .child(Spacer::vertical(1.0))
    .child(Text::new("Ready"));

let _ = inspector;
```

- Padding is space inside the view, between its border and its children.
- Margin is space outside the view, between it and its siblings.
- Border and radius are painted by the container, and a border adds inset for
  its content.
- `min_width`, `max_width`, `min_height`, and `max_height` clamp the resolved
  size.
- `overflow` controls whether content draws outside the view. Use `Hidden` or
  `Scroll` when a child must be clipped.

## Lengths

`LayoutStyle` widths and heights use `Length`:

| Value | Meaning |
| --- | --- |
| `Length::Px(v)` | Fixed logical pixels. |
| `Length::Percent(p)` | Percentage of the parent's available space. |
| `Length::Fill(weight)` | Share of the remaining space, proportional to `weight`. |
| `Length::Fit` | The view's intrinsic size. |

All built-in containers accept `impl Into<Length>`, and an `f32` converts to
pixels, so `.width(320.0)` is equivalent to `.width(Length::px(320.0))`.

`Fill` behaves differently on the two axes. On the main axis it claims free
space together with its siblings. On the cross axis it stretches to the
parent's available cross size. `Fit` on the main axis uses the measured content
size and does not grow unless a flex weight is set.

When the available space is unbounded, `Percent` and `Fill` fall back to the
intrinsic size. A view can only fill space that its parent actually bounds.

## Rows and columns

`Row` places children left to right; `Column` places them top to bottom. Both
support `gap`, `padding`, `align_items`, `justify_content`, and the decoration
methods.

`child_flex` makes one child grow in proportion to a weight:

```rust
# use sequoia_ui::prelude::*;
let split = Row::new()
    .gap(8.0)
    .child(Text::new("Explorer"))
    .child_flex(Text::new("Editor"), 2.0)
    .child_flex(Text::new("Inspector"), 1.0);

let _ = split;
```

The two flexible children first take their intrinsic sizes, then share the
remaining width at a 2:1 ratio. A `Spacer` is a convenient anonymous flexible
child:

```rust
# use sequoia_ui::prelude::*;
let toolbar = Row::new()
    .gap(8.0)
    .padding(8.0)
    .align_items(Align::Center)
    .child(Button::new("Back"))
    .child(Spacer::flex(1.0))
    .child(Button::new("Share"));

let _ = toolbar;
```

Use `Spacer::flex` in a row and `Spacer::vertical` in a column. `Spacer::fixed`
reserves a fixed amount of space.

`child_flex`, the `flex` style, and `Length::Fill` all express growth. Use one
of them for a given child; combining them does not add the weights together.
The default `flex_shrink` is `1.0`. Set `.flex_shrink(0.0)` on a sidebar that
must keep its width when the window gets narrow.

## Alignment

The two axes have separate alignment controls:

- `Align` controls the cross axis: `Start`, `Center`, `End`, or `Stretch`.
- `Justify` controls the main axis: `Start`, `Center`, `End`, `SpaceBetween`,
  `SpaceAround`, or `SpaceEvenly`.

```rust
# use sequoia_ui::prelude::*;
let footer = Row::new()
    .justify_content(Justify::SpaceBetween)
    .align_items(Align::Center)
    .child(Text::new("Ready"))
    .child(Text::new("UTF-8"));

let _ = footer;
```

`.center()` is shorthand for centering on both axes. `.align_self_value` on a
child overrides the parent's cross-axis alignment for that child. Alignment
only distributes space that is left over after padding, margins, and gaps.

`Container` exposes `justify_content`, `align_items`, `axis`, and `center` for
placing its single child inside the available space.

## Grids

`Grid` places children in rows and columns. Define tracks with `Track` and
place children explicitly or let the grid auto-place them:

```rust
# use sequoia_ui::prelude::*;
let dashboard = Grid::new([
        Track::minmax(160.0, Track::Fill(1.0)),
        Track::minmax(160.0, Track::Fill(1.0)),
    ])
    .gap(12.0)
    .fill_width()
    .child_span(Text::new("Metrics"), 0, 0, 2, 1)
    .child(Text::new("CPU"))
    .child(Text::new("Memory"));

let _ = dashboard;
```

- `Track::Px` and `Track::Percent` are fixed tracks.
- `Track::Fit` sizes a track to its content.
- `Track::Fill` shares free space.
- `Track::minmax` sets a floor and a maximum track definition.

`child_span(child, column, row, column_span, row_span)` places a child at a
specific cell. `child_at` takes a `GridPlacement` when the placement is
dynamic. `rows`, `column_gap`, `row_gap`, `align_items`, and `justify_items`
control the rest of the grid.

## Wrapping

`Wrap` lays children out in lines and moves to a new line when the available
main-axis space runs out. It is useful for tags, toolbars, and galleries:

```rust
# use sequoia_ui::prelude::*;
let tags = Wrap::new()
    .gap(6.0)
    .child(Text::new("Rust"))
    .child(Text::new("GPU"))
    .child(Text::new("Reactive"))
    .child(Text::new("Cross-platform"));

let _ = tags;
```

`Wrap::vertical()` wraps into columns instead of rows. `column_gap` and
`row_gap` set the two gaps independently.

`.columns(n)` and `.rows(n)` replace width-based wrapping with an explicit
count. On a horizontal `Wrap`, `columns` is the number of items per line and
`rows` is the number of lines. On a vertical `Wrap`, `rows` is the number of
items per column and `columns` is the number of columns. The count that matches
the main axis wins when both are set.

## Overlays and docking

Use `Stack` when children overlap. The first child is at the back and later
children draw on top. `child_at` takes a `StackPos` with top, right, bottom,
left, offset, and z-order values:

```rust
# use sequoia_ui::prelude::*;
let badge = Stack::new()
    .fill_size()
    .child(Text::new("Content"))
    .child_at(Text::new("New"), StackPos::top());

let _ = badge;
```

`DockArea` reserves edges for toolbars and sidebars, then gives the rest to
one fill child:

```rust
# use sequoia_ui::prelude::*;
let shell = DockArea::new()
    .top(Text::new("Toolbar"))
    .left(Text::new("Explorer"))
    .fill(Text::new("Editor"))
    .fill_size();

let _ = shell;
```

Add the fill child last. The fill region receives the space left after the
docked edges and gaps.

## Scrolling and splits

`ScrollView` gives a child a bounded viewport and scrolls its content:

```rust
# use sequoia_ui::prelude::*;
let editor = ScrollView::new(
    Column::new()
        .gap(8.0)
        .padding(16.0)
        .child(Text::new("First section"))
        .child(Text::new("Second section"))
        .child(Text::new("Third section")),
)
.vertical()
.scrollbar_width(8.0);

let _ = editor;
```

Use `.horizontal()` or `.both()` for other axes. The scroll offset belongs to
the `ScrollView` element, so it survives parent rebuilds as long as the element
keeps its identity.

`SplitPanel` divides space between two children and lets the user drag the
divider:

```rust
# use sequoia_ui::prelude::*;
let fraction = Signal::new(0.35_f32);
let panes = SplitPanel::new(fraction)
    .min_pane(160.0)
    .first(Text::new("Explorer"))
    .second(Text::new("Editor"));

let _ = panes;
```

The fraction is a `Signal<f32>` in the range `0.0..=1.0`. Store it if the
layout should be saved or controlled from elsewhere.

## Responsive layouts

`Responsive` rebuilds its subtree when a window-size signal changes:

```rust
# use sequoia_ui::prelude::*;
let viewport = Signal::new(Size::new(900.0, 600.0));
let page = Responsive::new(viewport, |size| {
    if Breakpoint::of_size(size).is_compact() {
        AnyView::new(Column::new().child(Text::new("Compact")))
    } else {
        AnyView::new(Row::new().child(Text::new("Wide")))
    }
})
.fill_size();

let _ = page;
```

`Breakpoint` classifies widths using Material-style compact and medium
thresholds. `WithSize` is the lower-level variant: it reports the measured size
of its own subtree, which is useful for container queries rather than window
breakpoints.

## Pitfalls

- A `Fill` child inside a `Fit` parent has no free space to claim. Make the
  ancestor fill the available area, or give it a bounded size.
- Percentages and fills fall back to intrinsic size when the parent does not
  provide a bound.
- Flex weights distribute only within one container. Nesting a `Row` inside a
  flex child does not move space between grandchildren.
- Gaps and padding are logical pixels and scale with the display; do not
  convert them manually.
- A clipped `ScrollView` needs a bounded height or it cannot decide when to
  scroll.

## Next steps

- Build custom leaves with layout and painting:
  [Custom views](../guides/custom-views.md).
- Route input through the tree: [Events](events.md).
- Reconcile dynamic children: [View tree](view-tree.md).
