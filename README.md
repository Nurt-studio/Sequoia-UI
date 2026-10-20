# Sequoia

**Native GUI framework for Rust** — persistent view tree, fine-grained reactivity, rendering via `wgpu`. No Virtual DOM, no WebView, no intermediate layers.

```rust
use sequoia_platform::App;
use sequoia_reactive::Signal;
use sequoia_view::{Button, Column, Text};

fn main() {
    let count = Signal::new(0i32);
    let c = count.clone();

    App::new()
        .title("Counter")
        .size(480.0, 320.0)
        .root(
            Column::new()
                .gap(12.0)
                .padding(24.0)
                .child(Text::with(move || format!("Count: {}", c.get())))
                .child(
                    Button::new("Increment")
                        .on_click(move || count.update(|n| *n += 1)),
                ),
        )
        .run()
        .expect("runtime");
}
```

---

## What kind of framework is this?

Sequoia is a **retained-mode framework with fine-grained reactivity**. Three keywords:

1. **Retained tree** — the view tree is persistent. Nodes have stable `ElementId`s, state (input, scroll, hover, animations) is preserved between frames, and layout is not rebuilt from scratch.
2. **Fine-grained reactivity** — changing a `Signal` **does not** cause the entire tree to re-render. A signal knows its dependencies; a change only triggers associated `Effect`s/`Memo`s and invalidates only those view nodes that read them. No tree diffing.
3. **Immediate painting** — every frame renders the entire tree to a `RenderList` from scratch. Layout, style, state, and node identity are retained; the drawing commands themselves are generated per-frame.

### This is NOT

| | |
|---|---|
| ❌ Virtual DOM | no diffing; React / Yew / Dioxus |
| ❌ Immediate mode | tree is persistent; egui / imgui |
| ❌ Pure retained | nodes rebuild locally; GTK / Qt Widgets |

### This is closest to

- **Solid / Leptos / Sycamore** — reactivity model (signals, memos, effects, no VDOM).
- **Flutter** — retained widget tree + immediate paint to a command list.
- **Taffy / Yoga** — layout engine (flexbox, grid, stack).

In practice: `Signal` → `Effect`/`View` → invalidate specific subtree → rebuild only that subtree → layout → paint. All on a single UI thread.

---

## Reactivity Model

Source of truth: `sequoia-reactive`. Zero macros, pure Rust API.

```rust
use sequoia_reactive::{Signal, memo, effect, batch, untracked};

let a = Signal::new(2);
let b = Signal::new(3);

// Memo — lazy, cached; recalculates only when dependencies change.
let sum = memo({
    let (a, b) = (a.clone(), b.clone());
    move || a.get() + b.get()
});

// Effect — runs on change and **re-tracks** dependencies after each run.
let _e = effect({
    let sum = sum.clone();
    move || println!("sum = {}", sum.get())
});

// Batch — multiple writes = single redraw.
batch(|| {
    a.set(10);
    b.set(20);
});

// Read without subscription (e.g., in handlers).
let snapshot = untracked(|| a.get());
```

Key features:

- **`get()` subscribes, `get_untracked()` does not.** Every read inside `effect()` / `memo()` / view constructor becomes a dependency.
- **`set()` compares values** (`PartialEq`) — writing the same value is a no-op (no propagation).
- **`Memo` returns a new value only when the result has changed** — downstream doesn't recalculate if `src % 2` remains the same.
- **`Effect` re-tracks** — if you read `if flag { a.get() } else { b.get() }` inside an effect, when `flag` changes, dependency `a` disappears and `b` appears.

### Store and history

```rust
use sequoia_reactive::Store;

let doc = Store::new(vec![1, 2, 3]);
doc.transact("Add two", || {
    doc.update(|v| v.push(4));
    doc.update(|v| v.push(5));
});

assert!(doc.undo());  // undoes the **entire** transaction
assert_eq!(doc.get_untracked(), vec![1, 2, 3]);
```

`UndoSignal<T>` is a lighter version with coalescing (fast typing = single undo).

---

## View Tree

Every widget implements the `View` trait:

```rust
pub trait View {
    fn body(&self) -> impl View + 'static;              // children (composition)
    fn layout_style(&self) -> LayoutStyle;              // sizes, margins, flex
    fn paint_self(&self, ctx: &mut PaintCtx);           // self-drawing
    fn on_event(&self, event: &Event, ctx: &mut EventCtx);
    fn reuse_state(&self, old: &Self);                  // preserve state after rebuild
    fn keyed_children(&self) -> Option<Vec<(u64, AnyView)>>; // reconciliation by keys
    fn focusable(&self) -> bool;
    fn captures_pointer(&self) -> bool;
    // ... accessibility, cursor, surface_target, drag&drop, on_unmount
}
```

### Node lifecycle

```
mount(body()) → layout → paint ─── (signal change) ──→ rebuild(body())
                    ↑                                       │
                    │                                       ↓
                    └────────── paint ←── reuse_state(old) ←┘
```

- `body()` is called **once on mount** and again when the node's reactive dependencies change.
- `reuse_state(old)` transfers internal state (e.g., `TextField::caret`, `SplitPanel::fraction`) from the old instance to the new one.
- `keyed_children()` forces a rebuild when keys change (like `key` in React, but without a VDOM).
- `ElementId` remains stable for the entire lifetime of the node.

### Reactive view wrappers

This is where the tree becomes "reactive" in the view sense.

```rust
use sequoia_reactive::Signal;
use sequoia_view::{when, Live, LiveOpacity, Scaled, Rotated, LiveOffset, AnimatedSize};
use sequoia_core::Vec2;

// Conditional rendering — child node is inserted/removed when the boolean changes.
let visible = Signal::new(true);
let conditional = when(visible.clone(), Text::new("Visible"));

// Reactive opacity with animation.
let opacity = Signal::new(1.0f32);
let faded = LiveOpacity::new(
    Text::new("Fading"),
    Live::from(opacity.clone()),
).animate(sequoia_view::AnimSpec::SMOOTH);

// Reactive rotation.
let angle = Signal::new(0.0f32);
let spinning = Rotated::new(
    Text::new("↻"),
    Live::from(angle.clone()),
);

// Reactive offset (Vec2).
let dx = Signal::new(0.0f32);
let dy = Signal::new(0.0f32);
let live_off = Live::<Vec2>::derived(move || Vec2::new(dx.get(), dy.get()));
let moving = LiveOffset::new(Text::new("Moving"), live_off);

// Animated size (width / height).
let w = Signal::new(120.0f32);
let resizing = AnimatedSize::width(Text::new("Auto"), Live::from(w.clone()));
```

Each of these wrappers is a regular `View` that only invalidates its own subtree when the signal changes — siblings are left untouched.

### Reactive text

```rust
Text::with(move || format!("{}", count.get()))
```

`Text::with` creates a `Text` node that reads the signal in `body()`, thereby subscribing. Value change → only this `Text` is rebuilt.

---

## Widgets

Basic ones in `sequoia-view`: `Column`, `Row`, `Container` (from `sequoia-elements`), `Panel`, `Stack`, `Grid`, `Wrap`, `ScrollView`, `Text`, `Icon`, `Image`, `Spacer`, `Canvas`, `List`, `VirtualList`, `SplitPanel`, `Overlay`, `Presence`, `Motion`, `when`, `Live*`.

Higher-level elements in `sequoia-elements`: `Button`, `Checkbox`, `Switch`, `Chip`, `Avatar`, `Badge`, `Divider`, `Kbd`, `ProgressBar`, `Spinner`, `Skeleton`, `Tooltip`, `Modal`, `Dialog`, `Sheet`, `Drawer`, `DropdownButton`, `FloatingMenu`, `ContextMenu`, `TabStrip`, `TextField`, `TextArea`, `Slider`, `Slider1D`, `SvPad`, `DragValue`, `ResizeHandle`, `Field`, `NumberField`, `DateField`, `MultiSelect`, `ToastStack`, `ColorPicker`, `CommandPalette`, `MenuBar`, `Form`.

All of them share the same fluent builder:

```rust
use sequoia_elements::{Button, Container};
use sequoia_view::{Row, Text};
use sequoia_core::Color;

Container::new()
    .background(Color::hex(0x1F1F1F))
    .radius(8.0)
    .border(1.0, Color::hex(0x2E2E2E))
    .padding(16.0)
    .child(
        Row::new()
            .gap(8.0)
            .child(Text::new("Ready?"))
            .child(
                Button::new("Save")
                    .style(sequoia_style::ButtonStyle::PRIMARY)
                    .on_click(|| println!("saved")),
            ),
    )
```

---

## Animations

`sequoia-animation` provides four mechanisms at different levels.

```rust
use sequoia_animation::{
    Animation, Easing, Keyframe, LoopMode, Track, Value,
    animate, animation, spring,
    StateMachine, MachineState, Transition, FromState, ToState, Condition,
};
use std::time::Duration;

// 1. One-off tween
let t = animation(0.0, 1.0)
    .duration(Duration::from_millis(200))
    .ease(Easing::EaseOutCubic);

// 2. Keyframe clip with looping
let clip = Animation::new("pulse", 0.8)
    .with_loop(LoopMode::PingPong)
    .with_track("scale", Track::try_new(vec![
        Keyframe::new(0.0, Value::f32(1.0)),
        Keyframe::new(0.4, Value::f32(1.05)).with_easing(Easing::EaseOutBack),
        Keyframe::new(0.8, Value::f32(1.0)),
    ]).unwrap());

// 3. Spring (physical, with damping)
let s = spring(1.0).from(0.0).stiffness(180.0).damping(20.0);

// 4. State machine with crossfade (Rive-style)
let sm = StateMachine::try_new(
    vec![
        MachineState::entry(),
        MachineState::animation("idle", "idle"),
        MachineState::animation("wave", "wave"),
    ],
    vec![
        Transition::new(FromState::Entry, ToState::State("idle".into()), vec![]),
        Transition::new(
            FromState::State("idle".into()),
            ToState::State("wave".into()),
            vec![Condition::Trigger("go".into())],
        ).with_mix(0.3),
    ],
    animations,
).unwrap();

// 5. Animator — ties clips + spring + state machine, you call `advance_frame()`
let mut player = sequoia_animation::Animator::new().with_clip(clip);
// player.advance_frame();
```

For views: `AnimSpec::{FAST, SMOOTH, SNAPPY, GENTLE, SLOW}` — ready-made curves for `Live*`, `Scaled`, `Rotated`, `AnimatedSize` wrappers.

---

## Overlays, drag & drop, canvas

### Overlays

Everything that needs to be above the tree (modal, menu, tooltip) goes through the same primitives:

```rust
use sequoia_view::{
    Portal, OverlayEntry, OverlayLayer, OverlayId,
    Anchor, Placement, Dismiss, DismissableLayer, FocusScope,
};

// Classic modal with focus trap:
let content = FocusScope::new()
    .trap(true)
    .child(my_dialog);

let layer = DismissableLayer::wrap(content)
    .bind(open.clone())
    .on_escape(move || open.set(false));

let modal = sequoia_elements::Modal::new(open)
    .width(520.0)
    .dismiss(Dismiss::OutsideClick)
    .child(layer);
```

- `Portal` — teleports a node to the global overlay runtime (like in React).
- `OverlayEntry` — manual entry with `Anchor`, `Placement`, `Layer`.
- `DismissableLayer` — click outside / Escape.
- `FocusScope` — focus trap (`trap(true)`) and restoration to previous location.

### Drag & drop

```rust
use sequoia_elements::{Draggable, Droppable};

let card = Draggable::new(my_card, MyPayload { id: 42 })
    .preview(my_drag_preview)                 // image following the cursor
    .on_drag_start(|| println!("start"));

let target = Droppable::new(my_zone, move |payload: MyPayload, pos| {
    println!("dropped {:?} on {:?}", payload, pos);
})
.highlight(hover_signal);
```

Payload is typed (`DragPayload` is `Rc<dyn Any>`). `Droppable<T>` filters by `TypeId` — no need to manually check the type.

### Canvas and `Painter`

Immediate drawing inside a retained tree:

```rust
use sequoia_view::{Canvas, Painter};
use sequoia_core::{Color, Fill, Point, Rect};

Canvas::new()
    .width(sequoia_core::Length::Fill(1.0))
    .height(sequoia_core::Length::Px(300.0))
    .on_draw(|g: &mut Painter| {
        g.fill(
            Rect::from_xywh(20.0, 20.0, 200.0, 100.0),
            Fill::solid(Color::hex(0x35, 0x6F, 0xD1)),
        );
        g.circle(Point::new(100.0, 100.0), 40.0, Fill::solid(Color::RED));
        g.line(
            Point::new(0.0, 0.0),
            Point::new(300.0, 300.0),
            sequoia_core::BrushStroke::new(2.0),
            Color::WHITE,
        );
    })
    .on_pointer(|pos, event, ctx| {
        // full pointer events with local coordinates
    })
```

`Canvas` also supports `tick:`, camera, clip, and groups with their own transformations (`Group(rotate:, translate:, scale:, pivot:)`).

### Custom `wgpu` inside the UI

```rust
use sequoia_render::{WgpuSurface, WgpuRenderer, WgpuFrame};

struct MyScene { /* GPU resource state */ }

impl WgpuRenderer for MyScene {
    fn render(&mut self, frame: &mut WgpuFrame) {
        // you have `frame.device`, `frame.queue`, `frame.encoder`, `frame.target`
        // you paint to a texture allocated by Sequoia
    }
}

let surface = WgpuSurface::new(MyScene::new())
    .cached(true)              // don't re-render every frame if not needed
    .samples(4);               // MSAA x4
```

`WgpuSurface` integrates with layout, clipping, focus, and input — your GPU artwork behaves like a regular widget.

---

## Async

Worker thread pool configured in `App`, bridged to UI via channels and `Signal`.

```rust
use sequoia_platform::{App, Executor};
use sequoia_async::Resource;
use sequoia_reactive::Signal;

let rt = AsyncRuntime::new(4);   // or: App::new().worker_threads(4)

// Async result → UI (safely, via `apply`)
rt.spawn_future(
    async { heavy_work().await },
    move |result| my_signal.set(result),
);

// To Signal:
rt.spawn_to_signal(&status, async { check_status().await });

// Resource — Idle / Loading / Ready / Failed / Cancelled state with generation
let user: Resource<User> = Resource::new();
rt.spawn_resource(&user, fetch_user());

// Stream (e.g., progress):
rt.spawn_stream(
    |em| async move {
        for i in 0..=100 {
            em.emit(i).unwrap();
        }
    },
    move |pct| progress.set(pct),
);
```

`App` creates an `Executor` with `worker_threads = cores - 1` (leaving one core for the UI). Results and wake-ups go through `EventLoopProxy<JobWake>` — no polling.

---

## Full example

```rust
use sequoia_platform::{App, AsyncRuntime, WindowSpec};
use sequoia_reactive::Signal;
use sequoia_view::{
    Button, Column, Live, LiveOpacity, Row, ScrollView, Text, when, AnimSpec,
};
use sequoia_elements::TextField;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let text = Signal::new(String::new());
    let show = Signal::new(true);
    let opacity = Signal::new(1.0f32);

    let rt = AsyncRuntime::new(4);
    let status = Signal::new("—".to_string());
    rt.spawn_to_signal(&status, async { "done".to_string() });

    let app = App::new()
        .title("Demo")
        .size(640.0, 480.0)
        .theme(sequoia_style::Theme::dark())
        .worker_threads(4)
        .accelerator(sequoia_input::Key::from_char('q'), sequoia_input::Modifiers::CONTROL, || {
            std::process::exit(0);
        })
        .root(
            Column::new()
                .padding(24.0)
                .gap(12.0)
                .child(TextField::new(text.clone()).placeholder("Type something"))
                .child(Text::with(move || format!("You typed: {}", text.get())))
                .child(Text::with(move || format!("Status: {}", status.get())))
                .child(
                    Row::new()
                        .gap(8.0)
                        .child(Button::new("Hide").on_click({
                            let show = show.clone();
                            move || show.update(|v| *v = !*v)
                        }))
                        .child(Button::new("Fade").on_click({
                            let op = opacity.clone();
                            move || op.set(if op.get() > 0.5 { 0.2 } else { 1.0 })
                        })),
                )
                .child(
                    ScrollView::new(when(
                        show.clone(),
                        LiveOpacity::new(
                            Text::new("Content that can be hidden"),
                            Live::from(opacity.clone()),
                        ).animate(AnimSpec::SMOOTH),
                    )),
                ),
        );

    // Second window
    let app = app.window(
        WindowSpec::new("Panel", Text::new("Second window"))
            .size(320.0, 200.0),
    );

    app.run()
}
```

---

## Architecture

The code is split into **23 crates**. Each has a single responsibility; you can use just a subset (e.g., `sequoia-draw` without UI).

| Crate | Role |
|---|---|
| `sequoia-core` | Geometry, colors, `RenderList`, `Pixels` |
| `sequoia-reactive` | `Signal`, `Memo`, `Effect`, `Store`, `UndoSignal` |
| `sequoia-view` | View tree (`View`), `Runtime`, `ElementId`, viewports |
| `sequoia-layout` | Flexbox, Grid, Stack, Wrap, Dock |
| `sequoia-style` | `BoxStyle`, `Theme`, tokens, presets |
| `sequoia-text` | Shaping, glyph atlas, IME |
| `sequoia-draw` | Vector paths, stroke, AA, waveforms |
| `sequoia-render` | `wgpu` renderer, pipelines, backdrop/filter, `WgpuSurface` |
| `sequoia-elements` | Ready-made widgets |
| `sequoia-animation` | Tween, keyframes, springs, mixer, state machine |
| `sequoia-assets` | SVG icons, fonts, cache |
| `sequoia-buffer` | Text engine (piece table, undo, cursor) |
| `sequoia-input` | Events: pointer, keyboard, IME, touch, pen |
| `sequoia-async` | Worker threads, async ↔ UI bridges |
| `sequoia-dock` | Headless IDE-style model (persistence) |
| `sequoia-export` | Headless PNG (1×–8× HiDPI) |
| `sequoia-hotkey` | Global OS shortcuts (Windows) |
| `sequoia-platform` | `App`, windows, exec, clipboard, dialogs |
| `sequoia-test` | `Harness` for headless tests |
| `sequoia-ui` | Umbrella (re-exports public API) |
| `sequoia-seq` | (WIP) `.seq` language parser/checker |
| `sequoia-seq-lsp` | (WIP) LSP server for `.seq` |
| `sequoia-preview` | (WIP) `.seq` CLI with auto-reload |

Layers:

```
 ┌────────────────────────────────────────────┐
 │  Your Views — builders, when, Live*, …     │
 ├────────────────────────────────────────────┤
 │  elements · style · layout · animation     │
 ├────────────────────────────────────────────┤
 │  render (wgpu)  ·  draw  ·  text  ·  assets│
 ├────────────────────────────────────────────┤
 │  core  ·  reactive  ·  input  ·  async     │
 ├────────────────────────────────────────────┤
 │  platform (winit · wgpu)                   │
 └────────────────────────────────────────────┘
```

---

## Testing

`sequoia-test` provides a `Harness` — a headless environment with layout, paint, input simulation, and tree inspection.

```rust
use sequoia_test::Harness;

let h = Harness::new(my_view);
h.layout(400.0, 300.0);
h.paint();

h.click(150.0, 24.0);
h.touch_down(1, 100.0, 50.0, None);
h.wheel(200.0, 100.0, -400.0);
h.key(sequoia_input::Key::from_char('a'));

// Tree access:
let root = h.root().unwrap();
for child in h.children(root) {
    let rect = h.rect(child).unwrap();
    let text = h.text(child);
}
```

Everything runs on a single thread, without a window and without GPU — perfect for CI.

---

## License

Licensed under the [MIT License](LICENSE-MIT).