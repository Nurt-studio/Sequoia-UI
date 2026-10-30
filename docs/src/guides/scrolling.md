# Scrolling

Sequoia has two scrolling stories:

- `ScrollView` clips and offsets arbitrary content. Use it for forms, articles,
  settings pages, and mixed layouts.
- `VirtualList`, `Table`, and `TreeView` scroll themselves and mount only visible
  rows. Use them for large data sets. Do not wrap them in a `ScrollView`.

`TextArea` also owns its scrolling. A text field or text area inside a
`ScrollView` keeps its own scroll position.

## A basic scroll view

`ScrollView::new(child)` fills the available space, clips the child to the
viewport, and measures the child without a height limit on the scrolling axis:

```rust
# use sequoia_ui::prelude::*;
# use sequoia_test::Harness;
let mut content = Column::new();
for i in 0..20 {
    content = content.child(Text::new(format!("Row {i}")).height(30.0));
}

let scroll = ScrollView::new(content).vertical();
let h = Harness::new(scroll.clone());
h.sync(300.0, 200.0);

let rt = h.runtime();
let el = h.root().unwrap();
assert_eq!(scroll.content_height(&rt, el), 600.0);
assert_eq!(scroll.viewport_height(&rt, el), 200.0);
```

The scroll view itself uses `Length::Fill(1.0)` for both axes. Give it a bounded
parent: inside a `Column`, let it take the remaining height (for example with
`child_flex`), or place it directly as a window root.

## Axes

`ScrollAxis` is `Vertical` by default. Choose an axis with a builder:

```rust
# use sequoia_ui::prelude::*;
let vertical = ScrollView::new(Text::new("long page")).vertical();
let horizontal = ScrollView::new(Text::new("wide row")).horizontal();
let both = ScrollView::new(Text::new("map")).both();
# let _ = (vertical, horizontal, both);
```

- On the scrolling axis, the child is measured with an infinite maximum, so a
  `Column` can grow tall or a `Row` can grow wide.
- On the fixed axis, the child is constrained to the viewport. A vertical
  scroller therefore wraps text to its width.
- With `Both`, both axes are unbounded. Content that uses `Fill` on both axes
  has nothing to fill against on the scroll axes; give it explicit or intrinsic
  sizes.

## Scrollbars

`ScrollbarVisibility` controls when bars appear:

| Variant | Behavior |
| --- | --- |
| `Auto` (default) | Shows a bar only while the content overflows |
| `Always` | Always paints the track; the thumb fills it when content fits |
| `Never` | Hides bars; wheel, touch, and keyboard still scroll |
| `OnHover` | Hides the bar until the pointer is near the edge or dragging; reserves a gutter |
| `Overlay` | Same reveal behavior as `OnHover`, but draws over the content with no gutter |

Only `OnHover` reserves layout space. `Auto`, `Always`, and `Overlay` paint on
top of the content, so keep interactive controls away from the right edge when a
bar can appear.

Style the bar per scroll view:

```rust
# use sequoia_ui::prelude::*;
let scroll = ScrollView::new(Text::new("content"))
    .vertical()
    .scrollbar_visibility(ScrollbarVisibility::Overlay)
    .scrollbar_width(10.0)
    .scrollbar_track(Color::rgb8(0x20, 0x20, 0x20))
    .scrollbar_thumb(Color::rgb8(0x70, 0x70, 0x70));
```

Defaults: bar width 8, track `theme.border`, thumb `theme.text_secondary`, key
scroll step 40. `show_scrollbar(bool)` is a shortcut for `Always`/`Never`.

## Input

- The mouse wheel scrolls the configured axes. A positive `delta.y` moves the
  content down (offset decreases).
- Shift+wheel maps vertical wheel delta to horizontal scrolling when the view
  scrolls horizontally.
- Dragging the thumb scrolls; clicking the track jumps the thumb to that
  position and starts a drag.
- Touch drag pans the content after an 8 px threshold, and the scroll view
  captures the pointer until release.
- When a key event reaches the scroll view, Arrow keys move by `scroll_step`,
  Page Up/Page Down move by one viewport, Home/End jump to the start or end, and
  Ctrl+Home/Ctrl+End jump to the absolute start or end of both axes.

The scroll view is not focusable by itself. Keyboard scrolling happens when a
focused child (or another view in the key event path) lets the event bubble.

## Observing the scroll position

`on_scroll` receives the new `Vec2` offset and fires only when the offset
actually changes:

```rust
# use sequoia_ui::prelude::*;
use std::cell::Cell;
use std::rc::Rc;

let last = Rc::new(Cell::new(Vec2::ZERO));
let last_for_cb = last.clone();
let scroll = ScrollView::new(Text::new("content"))
    .vertical()
    .on_scroll(move |offset| last_for_cb.set(offset));

# let _ = (scroll, last);
```

Use it to persist a position, lazily load a page, or drive a "back to top"
button. It does not fire during layout; it fires from user input or programmatic
scroll calls.

## Programmatic scrolling

The position lives in the runtime, keyed by the scroll view's `ElementId`. The
query and control methods therefore take `&Runtime` and `ElementId`:

```rust
# use sequoia_ui::prelude::*;
# use sequoia_test::Harness;
let mut content = Column::new();
for i in 0..40 {
    content = content.child(Text::new(format!("Row {i}")).height(30.0));
}

let scroll = ScrollView::new(content).vertical();
let h = Harness::new(scroll.clone());
h.sync(300.0, 200.0);

let rt = h.runtime();
let el = h.root().unwrap();

scroll.scroll_to_y(&rt, el, 120.0);
assert_eq!(scroll.offset_y(&rt, el), 120.0);

scroll.page_down(&rt, el);
assert_eq!(scroll.offset_y(&rt, el), 320.0);

scroll.scroll_to_bottom(&rt, el);
assert_eq!(scroll.offset_y(&rt, el), scroll.max_offset(&rt, el));
```

`Runtime` and `ElementId` are not in the prelude. Inside a custom view you can
pass `ctx.runtime()` and `ctx.element()`, but those refer to the view handling
the event, not necessarily to the `ScrollView` element. For application code,
keep the `ScrollView` handle and the target `ElementId` together, or drive
scrolling through an observed offset signal.

Queries:

- `offset`, `offset_x`, `offset_y`, `offset_vec` return the current offset.
- `max_offset`, `max_offset_x`, `max_offset_y`, `max_offset_vec` return the
  scroll range; values are `0.0` when the content fits.
- `can_scroll_x`, `can_scroll_y` combine the configured axis with the range.
- `scroll_fraction_x`, `scroll_fraction_y`, `scroll_fraction` return `0.0..=1.0`.
- `content_width/height/size` and `viewport_width/height/size` describe the
  measured geometry.

Commands:

- `scroll_to(offset)`, `scroll_to_xy(x, y)`, `scroll_to_x(x)`, `scroll_to_y(y)`
  clamp to the axis range. Axes that the view does not scroll are left alone.
- `scroll_by(delta)` adds a `Vec2`; `scroll_to_top/bottom` and
  `scroll_to_left/right` jump to an edge.
- `page_up/down/left/right` move by one viewport.
- `scroll_into_view(rect)` takes a rectangle in content coordinates and moves
  the minimum distance needed to reveal it.
- `ensure_visible(rect)` also takes content coordinates but ensures the whole
  rectangle is visible, aligning an oversized rectangle to the start. It returns
  `true` when the offset changed.
- `scroll_element_into_view(child)` reads the child's laid-out rectangle and
  calls `scroll_into_view` for you.

`scroll_to` and `ensure_visible` clamp at the edges, so they are safe to call
before the first layout or after the content shrinks. For a newly focused child,
call `scroll_element_into_view` yourself; focusing a view does not scroll it into
view automatically.

## Testing scroll behavior

`Harness` exposes wheel, pointer, and key input plus runtime access:

```rust
# use sequoia_ui::prelude::*;
# use sequoia_test::Harness;
let scroll = ScrollView::new(Text::new("content").height(800.0)).vertical();
let h = Harness::new(scroll.clone());
h.sync(300.0, 200.0);

h.wheel(150.0, 100.0, -40.0);
h.layout(300.0, 200.0);

let rt = h.runtime();
let el = h.root().unwrap();
assert_eq!(scroll.offset_y(&rt, el), 40.0);
```

Wheel deltas are inverted into offsets, so a negative `dy` increases the offset.
Call `layout` again after input that changes scroll state before reading child
rectangles.

## Layout advice

- Do not put a `VirtualList`, `Table`, `TreeView`, or `TextArea` inside a
  `ScrollView`. They scroll internally and expect a bounded viewport; nesting
  produces two scrollbars and wasted layout work.
- Avoid same-axis nested scroll views. Wheel events bubble when the inner view
  cannot move, which is useful for scroll chaining, but two visible bars confuse
  users. Prefer one scroll container per axis.
- Keep the scrolled child's width intrinsic when scrolling horizontally, and
  avoid percentage widths that resolve against an infinite constraint.
- The scroll offset is part of layout state. Changing content sizes can clamp an
  existing offset on the next layout.
- `ScrollView` clips its content and does not add an accessibility role. Label
  the content itself and keep focusable controls within reach; see
  [Accessibility](../concepts/accessibility.md).

See [Collections](collections.md) for virtualized lists and tables.
