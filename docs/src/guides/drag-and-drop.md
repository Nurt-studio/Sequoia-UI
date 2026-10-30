# Drag and drop

Sequoia splits drag and drop into two small views:

- `Draggable` starts a drag after the pointer moves past a threshold.
- `Droppable` accepts payloads of one concrete Rust type and runs your drop
  callback.

The runtime handles the pointer session, hit testing, preview overlays, and
cancellation. Your application owns the data model and decides what a drop
means. There is no built-in list reordering, insertion animation, or external
file transfer.

## A minimal source and target

Pass the payload to `Draggable` and name the payload type in the `Droppable`
callback:

```rust
# use sequoia_ui::prelude::*;
# use sequoia_test::Harness;
let dropped = Signal::new(String::new());
let sink = dropped.clone();

let board = Row::new()
    .child(
        Draggable::new(Text::new("Card"), "card-1".to_string())
            .width(80.0)
            .height(40.0),
    )
    .child(
        Droppable::new(
            Text::new("Drop here"),
            move |id: String, _pos: DropPosition| sink.set(id),
        )
        .width(100.0)
        .height(40.0),
    );

let h = Harness::new(board);
h.layout(300.0, 100.0);
h.drag((10.0, 10.0), (100.0, 20.0));
h.tick();
assert_eq!(dropped.get_untracked(), "card-1");
```

A press alone does not start a drag. The drag begins only after the pointer
moves at least 4 px from the press position. A click that never crosses the
threshold still reaches the child, so buttons and text inputs keep working
inside a `Draggable`.

## Drag sources

`Draggable::new(child, payload)` requires `T: Clone + 'static`. Payloads are
cloned when the press is registered, so the drag sees a snapshot even if the
source data changes mid-drag.

```rust
# use sequoia_ui::prelude::*;
let card = Draggable::new(Text::new("Task 7"), 7u64)
    .threshold(6.0)
    .button(PointerButton::Left)
    .preview(Text::new("Task 7"))
    .on_drag_start(|| {})
    .on_drag_end(|end| {
        let _ = (end.dropped, end.cancelled);
    })
    .width(160.0)
    .height(48.0);
# let _ = card;
```

- `threshold(px)` controls how far the pointer must move. The default is
  `DRAG_THRESHOLD` (4 px).
- `button` selects the initiating mouse button. It defaults to
  `PointerButton::Left`.
- `preview(view)` shows an arbitrary view in a `DragPreview` overlay while the
  drag is active. Without a preview, the source stays where it is and only the
  cursor changes.
- `on_drag_start` runs when the threshold is crossed, not on press.
- `on_drag_end` receives `DragEnd { dropped, cancelled }`. `dropped` is true
  only when a target accepts and handles the drop. `cancelled` is true after
  Escape, a non-left press, or a pointer cancel. Releasing outside every target
  leaves both flags false.
- Width, height, min/max size, flex, fill, and margin builders match the other
  elements. Styling such as a background or border belongs on the child; wrap it
  in a `Container` when the drag card needs a visual surface.

Draggable sources use `CursorIcon::Grab`, switching to `Grabbing` while
pressed. The preview overlay is positioned at `current - grab_offset`, so the
part of the source that was grabbed stays under the cursor.

## Drop targets

`Droppable::new(child, on_drop)` accepts exactly the payload type named by the
callback:

```rust
# use sequoia_ui::prelude::*;
let over = Signal::new(false);

let zone = Droppable::new(
    Container::new()
        .child(Text::new("Drop a task"))
        .width(180.0)
        .height(80.0),
    |id: u64, pos: DropPosition| {
        let _ = (id, pos.local, pos.fraction);
    },
)
.highlight(over.clone())
.on_over(|pos| {
    let _ = pos.insertion_index(3, Axis::Vertical);
})
.width(180.0)
.height(80.0);
# let _ = zone;
```

- A `Droppable<_, String>` ignores a numeric payload. Type matching uses
  `TypeId`, so it is exact: `u32` and `u64` are different payload types.
- `on_over` runs while this view is the current drop target, including as the
  pointer moves within it. There is no separate leave callback.
- `highlight(signal)` mirrors whether this view, or a descendant, is the current
  target. Read the signal in a component body to restyle a border or background.
- The callback runs on release, after which the target calls `ctx.handled()`.
  A custom target must call `ctx.handled()` itself for `DragEnd::dropped` to be
  true.
- Droppable views do not paint anything by default. Style the child.

Drop targets are resolved from the deepest hit element upward. The nearest
ancestor whose `accepts_drop` returns true wins, so a `Droppable` around a list
accepts drops over any row in that list, while a nested `Droppable` around one
row takes precedence over the outer target.

## Payloads and type erasure

The runtime stores a cloned, type-erased `DragPayload`. The type is fixed at the
press:

```rust
# use sequoia_ui::prelude::*;
let payload = DragPayload::new(("task".to_string(), 3usize));

assert!(payload.is::<(String, usize)>());
assert_eq!(
    payload.get::<(String, usize)>().map(|value| value.1),
    Some(3)
);
```

Prefer a small ID or `Rc<T>` over large owned data. A payload must be `'static`;
borrowed data cannot outlive the drag session. `DragPayload::{type_id,
type_name}` are useful for diagnostics and for custom views that accept several
payload kinds.

## Drop geometry

`DropPosition` describes the release point relative to the target:

| Field | Meaning |
| --- | --- |
| `global` | Window-space release point |
| `target_rect` | Target bounds in window space |
| `local` | Release point relative to the target's top-left corner |
| `fraction` | `local / target size`, clamped to `0..=1` per axis |
| `modifiers` | Modifier keys at release |

Two helpers cover the common cases:

- `pos.before_midpoint(axis)` splits a target into a leading and trailing half.
  Use it on per-row drop targets in a reorderable list.
- `pos.insertion_index(count, axis)` divides a target strip into `count` slots
  and returns a slot in `0..=count`. It assumes evenly spaced items across the
  whole target rect. To make the final slot reachable, use a dedicated tail drop
  zone or per-row midpoint targets.

Modifiers are informational. Internal drag and drop does not automatically
apply copy, move, or link semantics; read `pos.modifiers` or
`effect_for_modifiers(pos.modifiers)` if the gesture should mean different
things with Ctrl or Alt held.

## Reordering a list

Wrap each row in a `Droppable` and the row content in a `Draggable`. Use stable
IDs as payloads and as list keys:

```rust
# use sequoia_ui::prelude::*;
# use sequoia_ui::view::move_item;
# use sequoia_test::Harness;
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct TaskId(u64);

let order = Signal::new(vec![TaskId(1), TaskId(2), TaskId(3)]);

let list = List::new({
    let order = order.clone();
    move || order.get()
})
.key(|id: &TaskId| id.0)
.item({
    let order = order.clone();
    move |id: &TaskId| {
        let id = *id;
        let sink = order.clone();
        let row = Draggable::new(Text::new(format!("Task {}", id.0)), id)
            .width(180.0)
            .height(40.0);

        Droppable::new(row, move |from: TaskId, pos: DropPosition| {
            let mut items = sink.get();
            let Some(from_index) = items.iter().position(|item| *item == from) else {
                return;
            };
            let Some(to_index) = items.iter().position(|item| *item == id) else {
                return;
            };
            let slot = if pos.before_midpoint(Axis::Vertical) {
                to_index
            } else {
                to_index + 1
            };
            move_item(&mut items, from_index, slot);
            sink.set(items);
        })
        .width(180.0)
        .height(40.0)
    }
});

let h = Harness::new(list);
h.layout(200.0, 200.0);
h.drag((20.0, 10.0), (20.0, 110.0));
h.tick();
assert_eq!(order.get_untracked(), vec![TaskId(2), TaskId(3), TaskId(1)]);
```

`move_item(&mut Vec<T>, from, slot)` removes the item first and adjusts the
destination when the item moves forward, so the `slot` can be the index in the
original list. Both `move_item` and the free `insertion_index` helper live in
`sequoia_ui::view`; they are not in the facade prelude.

Use stable keys. When the key is the row index, reordering reuses elements for
different items and loses focus, hover, and child state. If rows have different
heights, use `before_midpoint` on each row instead of mapping a fraction over
the whole list.

## Custom drag views

`Draggable` and `Droppable` cover most cases. A custom view can start a drag from
a specific handle and override any of the drag hooks:

```rust
# use sequoia_ui::prelude::*;
struct DragHandle {
    task: u64,
}

impl View for DragHandle {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn is_leaf(&self) -> bool {
        true
    }

    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Px(24.0),
            height: Length::Px(24.0),
            ..LayoutStyle::default()
        }
    }

    fn on_event(&self, event: &Event, ctx: &mut EventCtx) {
        if let Event::Pointer(PointerEvent::Down {
            button: PointerButton::Left,
            pos,
            ..
        }) = event
            && ctx.rect.contains(*pos)
        {
            ctx.begin_drag(self.task, DragOptions::new().threshold(4.0));
        }
    }

    fn accepts_drop(&self, payload: &DragPayload) -> bool {
        payload.is::<u64>()
    }

    fn on_drop(&self, pos: DropPosition, ctx: &mut EventCtx) {
        let _ = (pos, ctx.drag_session().map(|session| session.current));
        ctx.handled();
    }
}
# let _ = DragHandle { task: 1 };
```

`DragOptions` supports the same threshold and preview configuration as
`Draggable`. While a session is active, `ctx.drag_session()` returns a snapshot
with the payload, source and target `ElementId`s, current point, grab offset,
modifiers, and preview overlay. `DropPosition::in_rect` and
`session.position_in(rect)` build positions for custom hit testing.

The other `View` hooks are `on_drag_start(ctx)`, `on_drag_over(pos, ctx)`, and
`on_drag_end(end, ctx)`. Only `on_drop` needs `ctx.handled()` for the source to
report a successful drop; an accepting target that does not handle the drop
leaves `DragEnd::dropped` false.

## Lifecycle and cancellation

- The press creates a pending drag. Crossing the threshold creates a `DragId`,
  shows the preview overlay, resolves the initial target, and calls
  `on_drag_start`.
- Every pointer move recomputes the nearest accepting target. The old target is
  not notified when the pointer leaves it; use `highlight` for visual state.
- Releasing the left button finishes the session exactly once and calls the
  source's `on_drag_end`.
- Escape, a non-left button press, or a pointer cancel cancels the session and
  hides the preview. The source receives `cancelled: true`.
- If the source unmounts during a drag, the session continues with no source, so
  no `on_drag_end` is delivered. If a target unmounts, it stops being a target.
- Rebuilding the source does not kill the session. Element identity is what
  matters, so keep the source view in the same position in the tree.

There is no automatic scrolling while dragging. To drag into a long
`ScrollView`, watch the pointer position in `on_over` or `on_drag_end` and call
the scroll view's programmatic scroll methods on a timer or pointer event.

## Files from the operating system

OS file drops do not create a `DragPayload` or a `DragSession`. They arrive as
events on the hit-tested view and bubble through its ancestors:

```rust
# use sequoia_ui::prelude::*;
struct FileTarget;

impl View for FileTarget {
    fn body(&self) -> impl View + 'static {
        Text::new("Drop files")
    }

    fn on_event(&self, event: &Event, ctx: &mut EventCtx) {
        match event {
            Event::FilesHovered { paths, .. } => {
                let _ = paths;
            }
            Event::FilesDropped { paths, .. } => {
                let _ = paths;
                ctx.handled();
            }
            Event::FilesCancelled => {}
            _ => {}
        }
    }
}
# let _ = FileTarget;
```

`FilesHovered` is delivered repeatedly while files are over the window;
`FilesCancelled` arrives when the OS cancels the operation. Validate paths before
using them, and remember that file drags cannot share payloads with internal
drags.

## Accessibility and testing

Drag and drop is pointer-only. `Draggable` and `Droppable` add no accessible
role, label, or keyboard command. Provide an equivalent action for keyboard
users, such as a context menu with Move Up / Move Down commands, and label the
underlying content normally. A drag preview is a passthrough overlay and should
not contain focusable controls.

`Harness` exposes the session for deterministic tests:

```rust
# use sequoia_ui::prelude::*;
# use sequoia_test::Harness;
let card = Draggable::new(Text::new("Card"), 1u64)
    .width(80.0)
    .height(40.0);
let h = Harness::new(card);
h.layout(200.0, 100.0);

h.pointer_down(10.0, 10.0);
assert!(h.has_pending_drag());
assert!(h.drag_session().is_none());

h.pointer_move(60.0, 10.0);
let session = h.drag_session().expect("threshold crossed");
assert_eq!(session.payload.get::<u64>(), Some(&1));

h.drag_cancel();
h.tick();
assert!(h.drag_session().is_none());
```

`Harness::drag(from, to)` performs a down, five moves, and an up;
`drag_steps` controls the number of moves. `overlay_count` and `overlay_rect`
check previews, and `drop_target_at(x, y, &payload)` inspects target resolution
without starting a drag.

## Design notes

- Keep payloads small and copyable when possible. The session holds a clone for
  its whole lifetime.
- Let `Droppable` match one type per intent. Use separate wrapper types such as
  `MoveTask(TaskId)` and `CopyTask(TaskId)` when the same data means different
  operations.
- Prefer IDs over indices for payloads and keys. Indices change during a drop.
- The source remains visible during a drag. If the gesture should look like a
  move, hide or dim the original row from `on_drag_start` and restore it from
  `on_drag_end`.
- Reordering is a data operation: update the `Signal<Vec<T>>`, then let the
  collection rebuild. Do not mutate row views directly.
