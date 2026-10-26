# Events

Sequoia delivers input as events. Pointer and keyboard events are routed through
the view tree, so a parent can observe or intercept input before a child
receives it. Most applications use the event handlers built into widgets;
custom leaves and `Canvas` implement the same protocol directly.

## Delivery

Pointer input starts with a hit test. The runtime finds the topmost leaf under
the pointer and builds the path from the root to that leaf. It then dispatches
the event in three phases:

1. **Capture** runs from the root down to the target's parent.
2. **Target** runs on the element that was hit.
3. **Bubble** runs from the target's parent back to the root.

Keyboard events use the focused element as the target and follow the same three
phases. `EventCtx::phase` reports the current phase:

```rust
# use sequoia_ui::prelude::*;
fn handle(event: &Event, ctx: &mut EventCtx) {
    if ctx.phase != EventPhase::Target {
        return;
    }
    if let Event::Key(ke) = event
        && ke.key == Key::ENTER
    {
        ctx.handled();
    }
}
```

`ctx.handled()` marks the event as consumed and stops delivery.
`ctx.stop_propagation()` stops delivery without marking the event handled. Use
capture listeners sparingly, for behavior such as dismissing an overlay or
implementing a shortcut that must win before a focused child sees the key.

## Pointer input

`PointerEvent` represents mouse, pen, and touch input. The common mouse variants
carry a position (in window coordinates), modifiers, and, for presses, a
`PointerButton`:

- `Move { pos, modifiers }`
- `Down { button, pos, modifiers }`
- `Up { button, pos, modifiers }`
- `Cancel`
- `Wheel { delta, pos, modifiers }`

Pen and touch input add pressure, tilt, or contact IDs. Match the variants you
need and let the rest fall through:

```rust
# use sequoia_ui::prelude::*;
fn on_pointer(ev: &PointerEvent, ctx: &mut EventCtx) {
    match ev {
        PointerEvent::Down {
            button: PointerButton::Left,
            ..
        } => {
            let local = ctx.local_pos();
            let _ = local;
            ctx.request_focus();
            ctx.handled();
        }
        PointerEvent::Wheel { delta, .. } => {
            let _ = delta;
        }
        _ => {}
    }
}
```

`ctx.local_pos()` is relative to the target's top-left corner. `EventCtx::pos`
and the positions carried by `PointerEvent` are window coordinates.

## Capture

A view can keep receiving a pointer gesture after the pointer leaves its bounds.
Call `ctx.capture_pointer()` after a press; the runtime sends subsequent moves
and the release to the captured element. Call `ctx.release_pointer()` when the
gesture is finished, or let the runtime release it on `Up`.

Some views request capture automatically by reporting `captures_pointer()`.
`Canvas` does this by default, so drags that start inside a canvas keep
delivering to the canvas even outside its rectangle. Use `.capture(false)` to
disable that behavior. Custom leaves can implement the same method when they
need gesture retention.

## Keyboard input

`KeyEvent` contains a `key` and `modifiers`. Compare against `Key` constants and
test modifiers with `contains`:

```rust
# use sequoia_ui::prelude::*;
let save = Key::from_char('s');
let is_save = |ke: &KeyEvent| {
    ke.key == save
        && ke.modifiers.contains(Modifiers::CONTROL)
        && !ke.modifiers.contains(Modifiers::ALT)
};
let _ = is_save;
```

`Modifiers` is a bitflags type with `SHIFT`, `CONTROL`, `ALT`, and `SUPER`.
`Key` includes `TAB`, `ENTER`, `ESCAPE`, `SPACE`, `BACKSPACE`, `DELETE`, arrow
keys, function keys, and character keys created with `Key::from_char`.

Keyboard events go to the focused element. A non-focusable view does not
receive them. Return `true` from `Canvas::on_key` to mark the event handled, or
call `ctx.handled()` in a custom leaf. `KeyEvent` describes a key press; it does
not contain committed text. Text widgets and `Event::Ime` handle text entry.

Tab moves focus through the tree. The runtime finds the next focusable element
after the current one, wrapping at the end. Shift-Tab moves backward.

## Focus

Only focusable elements can receive keyboard input or IME events. Built-in
buttons, text fields, and `Canvas` are focusable; plain layout containers are
not. A custom leaf declares `focusable() -> true` and requests focus when it is
activated:

```rust
# use sequoia_ui::prelude::*;
let field = Canvas::new()
    .on_pointer(|_, ev, ctx| {
        if matches!(
            ev,
            PointerEvent::Down {
                button: PointerButton::Left,
                ..
            }
        ) {
            ctx.request_focus();
            ctx.handled();
        }
    });

let _ = field;
```

Focus follows element identity. Rebuilding a view does not blur a focused node
as long as reconciliation keeps the same element. Removing the node, replacing
it with a different type, or changing its key drops focus.

## Visual state and cursors

The runtime tracks hover, press, focus, disabled, and selected state in
`VisualState`. It is available as `ctx.state` during events and as
`ctx.state`/`paint.ctx.state` during painting.

A custom leaf that looks different when hovered should return `true` from
`has_visual_state()`. That opts into repainting when the pointer enters or
leaves the element. `cursor()` returns the `CursorIcon` for the current state,
such as `Pointer`, `Text`, or `Grab`.

## Canvas example

`Canvas` is the shortest path to a custom interactive surface. Its callback
receives a point that is already local to the canvas:

```rust
# use sequoia_ui::prelude::*;
let clicks = Signal::new(0_u32);

let canvas = Canvas::new()
    .background(Color::hex(0x0d1117))
    .track({
        let clicks = clicks.clone();
        move || {
            let _ = clicks.get();
        }
    })
    .on_draw({
        let clicks = clicks.clone();
        move |p| {
            let label = format!("Clicks: {}", clicks.get_untracked());
            p.text_tl(&label, Point::new(16.0, 12.0), 14.0, Color::WHITE);
        }
    })
    .on_pointer({
        let clicks = clicks.clone();
        move |_pos, ev, ctx| {
            if let PointerEvent::Down {
                button: PointerButton::Left,
                ..
            } = ev
            {
                clicks.update(|n| *n += 1);
                ctx.request_focus();
                ctx.handled();
            }
        }
    })
    .on_key({
        let clicks = clicks.clone();
        move |ke| {
            if ke.key == Key::SPACE || ke.key == Key::ENTER {
                clicks.update(|n| *n += 1);
                true
            } else {
                false
            }
        }
    });

let _ = canvas;
```

`track` subscribes the canvas to the click signal. `on_draw` reads it without
tracking because painting must not create reactive dependencies. The pointer
handler requests focus, and the key handler accepts Space or Enter while the
canvas is focused.

## Drag and drop

Forground drags, call `ctx.begin_drag(payload, DragOptions::default())` with a
`'static` payload. The runtime starts the drag once the pointer moves past the
drag threshold; `ctx.drag_session()` reports the active payload. Drop targets
can inspect `ctx.drop_position(modifiers)` to choose an edge or an insertion
point. Pressing Escape cancels an active drag.

For external files, use `Event::FilesHovered`, `Event::FilesDropped`, and
`Event::FilesCancelled`. Each carries paths in window coordinates; the runtime
routes them to the element under the pointer, so a custom leaf can accept drops
in `on_event` without special registration.

## Pitfalls

- Do not handle pointer events in every phase. Guard target-only behavior with
  `ctx.phase == EventPhase::Target`.
- Call `ctx.handled()` for events your widget consumes; otherwise a parent
  shortcut may also act on them.
- Request focus on activation. A custom leaf that never calls `request_focus`
  will not receive keyboard input.
- Use `ctx.local_pos()` for hit regions; pointer event positions are global.
- Avoid reading signals in paint. Use `get_untracked()` or read in `track` and
  `body` instead.

## Next steps

- Build a leaf that paints and handles input:
  [Custom views](../guides/custom-views.md).
- Position the canvas and other children: [Layout](layout.md).
- Understand element identity and focus survival: [View tree](view-tree.md).
