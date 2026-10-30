# Canvas

`Canvas` is a leaf view with a custom paint callback. Reach for it when a
layout of standard widgets is not enough: charts, node graphs, drawing tools,
game boards, timelines, or any custom 2D visual with its own hit testing.

Use a custom `View` instead when you need child views, nested layout, or a
reusable widget. A `Canvas` is one leaf and one pointer target; it cannot host
focusable children. If buttons or text fields should float over a drawing, put
the `Canvas` and the widgets in a `Stack`.

## A first canvas

`Canvas::new()` fills both axes, clips to its bounds, accepts focus, and
captures the pointer. Give it a size in the surrounding layout and draw in its
local coordinate system:

```rust
# use sequoia_ui::prelude::*;
use sequoia_ui::view::{BrushStroke, Fill, GradientStops};

let canvas = Canvas::new()
    .background(Color::hex(0x17191d))
    .cursor(CursorIcon::Crosshair)
    .on_draw(|g| {
        let size = g.size();

        g.rounded(
            Rect::from_xywh(24.0, 24.0, 180.0, 96.0),
            10.0,
            Fill::horizontal(GradientStops::two(
                Color::hex(0x2d7ff9),
                Color::hex(0x8f5cff),
            )),
        );
        g.circle(
            Point::new(size.width * 0.75, size.height * 0.5),
            28.0,
            Fill::solid(Color::hex(0xf5c542)),
        );
        g.line(
            Point::new(24.0, size.height - 24.0),
            Point::new(size.width - 24.0, size.height - 24.0),
            BrushStroke::round(2.0),
            Color::WHITE.with_alpha(0.4),
        );
        g.text_tl("Canvas", Point::new(44.0, 48.0), 20.0, Color::WHITE);
    });
# let _ = canvas;
```

`background` accepts a `Color` or any `Fill`, including a gradient. Use
`no_background()` to clear one. The usual sizing builders are available:
`width`, `height`, `min_width`, `max_width`, `min_height`, `max_height`,
`flex`, and `expand`.

`on_draw` receives a `&mut Painter`. Coordinates are local to the canvas:
`(0.0, 0.0)` is its top-left and `g.rect()` is its full paint rect. The
callback runs during painting; do not mutate signals or perform expensive
setup there. Prepare data before building the canvas and only issue draw calls
inside the callback.

## Painter vocabulary

`Painter` exposes a focused set of primitives rather than the whole render
list. The most useful groups are:

| Need | Methods |
| --- | --- |
| Filled shapes | `fill`, `rounded`, `circle`, `ellipse`, `polygon`, `regular_polygon`, `star`, `knob` |
| Strokes | `line`, `polyline`, `curve`, `ring`, `rect_outline`, `ellipse_outline`, `capsule`, `arrow` |
| Markers and frames | `frame`, `cross`, `x_marker`, `marquee` |
| Grids | `grid`, `grid_offset`, `grid_split`, `dots`, `axes` |
| Text | `text` (baseline), `text_tl` (top-left), `text_center`, `text_center_x`, `text_styled`, `text_tl_rotated`, `text_width`, `text_width_weighted` |
| Images | `image`, `pixels`, `pixels_nearest`, `pixels_region` |
| Clipping | `clipped`, `clipped_rounded` |
| Transforms | `with_transform`, `unproject`, `to_global_point`, `to_local_point` |

Filled methods take a `Fill`; stroked methods take a `BrushStroke` and a
`Color`. That distinction matters:

```rust
# use sequoia_ui::prelude::*;
# use sequoia_ui::view::{BrushStroke, Fill};
# let canvas = Canvas::new().on_draw(|g| {
let box_rect = Rect::from_xywh(16.0, 16.0, 120.0, 80.0);
g.rounded(box_rect, 8.0, Fill::solid(Color::hex(0x242830)));
g.rect_outline(
    box_rect,
    8.0,
    BrushStroke::new(1.5),
    Color::hex(0x4a5160),
);
g.text_center(box_rect, "Node", 14.0, Color::WHITE);
# });
# let _ = canvas;
```

`BrushStroke::new(width)` and `BrushStroke::round(width)` are the common
constructors. `Fill::solid`, `Fill::horizontal`, `Fill::vertical`,
`Fill::linear`, and `Fill::radial` build fills; `GradientStops::two` and
`GradientStops::new` describe gradient stops. A solid `Color` converts into a
`Fill` with `.into()`.

`text` uses a baseline point. `text_tl` uses a top-left point and is usually
what UI labels want. Text drawing is immediate; use the normal `Text` view when
you need wrapping, styling, or selection.

## Reactive redraw

A paint callback is not a reactive scope. Reading a `Signal` only inside
`on_draw` does not subscribe the canvas to it. Use `track` to read the signals
your drawing depends on, then use `get_untracked` while painting:

```rust
# use sequoia_ui::prelude::*;
use sequoia_ui::view::Fill;

let progress: Signal<f32> = Signal::new(0.35);
let draw_progress = progress.clone();
let track_progress = progress.clone();

let canvas = Canvas::new()
    .track(move || {
        track_progress.get();
    })
    .on_draw(move |g| {
        let value = draw_progress.get_untracked();
        let width = g.size().width;
        g.fill(
            Rect::from_xywh(0.0, 0.0, width * value, 6.0),
            Fill::solid(Color::hex(0x2d7ff9)),
        );
    });
# let _ = canvas;
```

Calling `progress.set(...)` now marks the canvas dirty and repaints it. A
`track` closure may read several signals. Keep it small and read only what the
drawing uses; it runs during the build phase, not during painting.

## Pointer input

`on_pointer` receives the pointer position in canvas-local coordinates, the
event, and an `EventCtx`:

```rust
# use std::cell::Cell;
# use std::rc::Rc;
# use sequoia_ui::prelude::*;
# use sequoia_ui::view::BrushStroke;

let points: Signal<Vec<Point>> = Signal::new(Vec::new());
let pressed = Rc::new(Cell::new(false));
let on_pointer = points.clone();
let track_points = points.clone();
let draw_points = points.clone();
let pressed_on_pointer = pressed.clone();

let canvas = Canvas::new()
    .cursor(CursorIcon::Crosshair)
    .on_pointer(move |pos, event, ctx| match event {
        PointerEvent::Down {
            button: PointerButton::Left,
            ..
        } => {
            pressed_on_pointer.set(true);
            on_pointer.update(|points| points.push(pos));
            ctx.handled();
        }
        PointerEvent::Move { .. } if pressed_on_pointer.get() => {
            on_pointer.update(|points| points.push(pos));
            ctx.handled();
        }
        PointerEvent::Up { .. } => {
            pressed_on_pointer.set(false);
            ctx.handled();
        }
        _ => {}
    })
    .track(move || {
        track_points.get();
    })
    .on_draw(move |g| {
        for segment in draw_points.get_untracked().windows(2) {
            g.line(
                segment[0],
                segment[1],
                BrushStroke::round(2.0),
                Color::WHITE,
            );
        }
    });
# let _ = canvas;
```

Canvas captures the pointer by default, so `Move` and `Up` continue to arrive
when the pointer leaves its bounds. Use `.capture(false)` when an ancestor
(for example a `ScrollView`) should handle the gesture instead. Call
`ctx.handled()` only for events the canvas actually consumed so other handlers
can respond.

Positions are already local. With a camera, convert them explicitly:

```rust
# use sequoia_ui::prelude::*;
let camera_pos = Vec2::new(40.0, 20.0);
let zoom = 1.5;
let local = Point::new(120.0, 80.0);
let world = Canvas::local_to_world(local, camera_pos, zoom);
let roundtrip = Canvas::world_to_local(world, camera_pos, zoom);
assert!((roundtrip.x - local.x).abs() < 1e-4);
assert!((roundtrip.y - local.y).abs() < 1e-4);
```

`snap_every(step)` snaps local positions before they reach `on_pointer`. It
does not snap the draw callback, and the step is in local pixels even when a
camera is set; use `Canvas::snap` or `Canvas::snap_point` yourself when you
need world-space snapping.

## Focus and keys

A canvas is focusable by default and requests focus on a left-button press.
Exact key bindings are matched before the fallback handler:

```rust
# use sequoia_ui::prelude::*;

let canvas = Canvas::new()
    .key(Key::DELETE, Modifiers::NONE, || {})
    .on_key(|event| {
        if event.key == Key::ESCAPE {
            true
        } else {
            false
        }
    });
# let _ = canvas;
```

A binding fires only when both key and modifiers match exactly. The fallback
`on_key` runs when no binding matches; return `true` to consume the event and
`false` to let it bubble. Keyboard handling requires focus, so keep
`focusable(true)` and provide a visual focus affordance. Use
`.focusable(false)` for purely decorative canvases so they do not take a Tab
stop.

`Canvas` has no accessible role or label. It is a pointer and key target, not
a semantic control. For anything keyboard- or screen-reader-operable, expose
the same actions through real widgets next to or above the canvas, and draw a
visible focus indicator yourself. Canvas-drawn buttons are not buttons to
assistive technology.

## Camera, grid, and level of detail

A camera is a position and zoom pair used by the canvas grid, axes, and
coordinate helpers:

```rust
# use sequoia_ui::prelude::*;

let canvas = Canvas::new()
    .grid_colored(20.0, Color::hex(0x2a2d33))
    .axes_colored(Color::hex(0x50545c))
    .camera(Vec2::new(40.0, 20.0), 1.5)
    .snap_every(10.0)
    .on_draw(|g| {
        let visible = Canvas::view_bounds(
            g.size(),
            Vec2::new(40.0, 20.0),
            1.5,
        );
        let _ = visible;
    });
# let _ = canvas;
```

The camera does **not** transform the painter for you. `.grid()` and `.axes()`
account for it, but drawing inside `on_draw` still uses canvas-local pixels.
Apply the same transform to your content when you want world-space drawing:

```rust
# use sequoia_ui::prelude::*;
# use sequoia_ui::view::Fill;

let camera_pos = Vec2::new(40.0, 20.0);
let zoom = 1.5;
let world_transform = Transform::translate(-camera_pos.x, -camera_pos.y)
    .then(Transform::uniform(zoom));

let canvas = Canvas::new().on_draw(move |g| {
    g.with_transform(world_transform, |g| {
        g.fill(
            Rect::from_xywh(0.0, 0.0, 200.0, 120.0),
            Fill::solid(Color::hex(0x2d7ff9)),
        );
    });
});
# let _ = canvas;
```

`Canvas::fit_view(content, size, padding)` returns `(camera_pos, zoom)` that
centers `content` in `size`. `Canvas::view_bounds(size, camera_pos, zoom)`
returns the visible world rectangle and is the right culling bound for
world-space drawings. The camera is configured with `.camera(pos, zoom)`; the
canvas stores the pair and uses it for its grid and axes.

For dense scenes, scale detail with the helpers:

```rust
# use sequoia_ui::prelude::*;
let breaks = [1.0, 0.55, 0.3];
let band = Canvas::lod_banded(0.7, &breaks);
assert_eq!(band, 1);

let show_labels = Canvas::show_detail(0.7, 2.0, 1.0);
assert!(show_labels);

let alpha = Canvas::detail_alpha(0.7, 2.0, 1.0, 0.5);
assert!((alpha - 0.8).abs() < 1e-6);
```

`lod_banded` takes breaks from full detail to coarse and returns `0` for the
full-detail band. `show_detail` compares a world-space size multiplied by zoom
against a minimum pixel size; `detail_alpha` ramps in detail over a fade
window instead of popping. Cull off-screen work with `g.rect()` or
`view_bounds`, and avoid rebuilding large vectors every frame.

## Clipping and overflow

A canvas clips to its paint rect by default. `allow_overflow(px)` inflates the
clip for shadows, handles, or selection outlines that extend past the layout
bounds. `clip(false)` disables canvas clipping entirely. Overflowing pixels
are still outside the canvas hit area; do not rely on overflow for
interaction.

```rust
# use sequoia_ui::prelude::*;
let canvas = Canvas::new().allow_overflow(12.0);
# let _ = canvas;
```

Painter also has local clipping scopes. They nest and restore automatically:

```rust
# use sequoia_ui::prelude::*;
# use sequoia_ui::view::Fill;
# let canvas = Canvas::new().on_draw(|g| {
let viewport = Rect::from_xywh(20.0, 20.0, 160.0, 90.0);
g.clipped(viewport, |g| {
    g.fill(g.rect(), Fill::solid(Color::hex(0x242830)));
});
# });
# let _ = canvas;
```

`clipped` and `clipped_rounded` affect painting only. Clip pointer coordinates
against your own model when the drawn content does not fill the canvas.

## Images and pixels

Use `pixels` for an RGBA bitmap and `image` for a prepared image source:

```rust
# use sequoia_ui::prelude::*;

let mut bitmap = Pixels::filled(64, 64, Color::hex(0x2d7ff9));
bitmap.set(32, 32, Color::WHITE);

let canvas = Canvas::new().on_draw(move |g| {
    g.pixels(g.rect(), &bitmap);
});
# let _ = canvas;
```

`pixels` samples linearly; `pixels_nearest` keeps hard pixel edges, and
`pixels_region` draws a source rectangle. `Pixels` also supports `set`,
`blend`, `fill_rect`, `crop`, resizing, and `to_source()` for `image`.
Prepare or update bitmaps outside the paint callback; `on_draw` is `Fn`, not
`FnMut`, so it cannot mutate captured data in place. See the Assets guide for
registering image assets and icons.

## Testing

Use `Harness` to lay out the canvas and inspect what it painted:

```rust
# use sequoia_ui::prelude::*;
# use sequoia_ui::view::Fill;
# use sequoia_test::Harness;

let h = Harness::new(Canvas::new().on_draw(|g| {
    g.fill(
        Rect::from_xywh(0.0, 0.0, 10.0, 10.0),
        Fill::solid(Color::RED),
    );
}));

let stats = h.paint_stats(120.0, 80.0);
assert_eq!(stats.quads, 1);
```

Pointer tests use the same local coordinates as the callback:

```rust
# use std::cell::Cell;
# use std::rc::Rc;
# use sequoia_ui::prelude::*;
# use sequoia_test::Harness;

let seen = Rc::new(Cell::new(None));

let canvas = {
    let seen = seen.clone();
    Canvas::new().on_pointer(move |pos, event, _ctx| {
        if let PointerEvent::Down { .. } = event {
            seen.set(Some(pos));
        }
    })
};

let h = Harness::new(canvas);
h.layout(200.0, 200.0);
h.pointer_down(30.0, 40.0);

assert_eq!(seen.get(), Some(Point::new(30.0, 40.0)));
```

`Harness::paint_stats` reports quads, text, glyphs, images, and other render
items. Use it for primitive counts and culling assertions. For signaling,
update a tracked `Signal` and paint again; the canvas should be dirty.

## Limits

- `Canvas` is a leaf. It has no child layout and no focusable children.
- A canvas has no built-in accessible role or label. Supply semantic controls
  separately; canvas-drawn content is invisible to assistive technology.
- There is no built-in zoom, pan, or selection model. Camera helpers are
  math utilities; implement gestures and undo yourself.
- Paint callbacks should not allocate large temporary structures per frame.
  Cache paths, text widths, and bitmaps.
- The auto grid is capped at roughly 4096 vertical and 8192 horizontal lines;
  `dots` stops at 16384 dots. For denser visuals, draw your own visible subset.
