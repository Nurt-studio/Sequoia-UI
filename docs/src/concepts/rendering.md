# Rendering and frames

Sequoia does not diff a virtual DOM. A view tree is mounted once, reactive
reads are tracked, and the runtime rebuilds only the parts that depend on
changed signals. Painting produces a flat `RenderList` that the GPU backend
consumes.

This page describes that pipeline end to end.

## The frame loop

A frame has four stages:

1. **Events.** The platform backend feeds pointer, keyboard, IME, and window
   events into the runtime.
2. **Reactive update.** Signal writes mark dependent effects and view
   builders as dirty. Dirty subtrees are rebuilt and laid out.
3. **Paint.** The runtime walks the tree and emits quads, text runs, images,
   shadows, waveforms, and backdrop requests into a `RenderList`.
4. **Render.** The backend uploads the list and draws it. On wgpu this is
   handled by `WgpuRenderer`.

`App::run` owns this loop. For most applications you never call these stages
yourself.

## Redraw modes

By default the window is driven by demand: it paints after input, a signal
change, or an animation request, and otherwise sleeps.

```rust,no_run
# use sequoia_ui::prelude::*;
fn main() {
    let view = Text::new("Idle windows cost almost nothing");
    App::new()
        .title("On demand")
        .size(480.0, 200.0)
        .redraw_mode(RedrawMode::OnDemand)
        .root(view)
        .run()
        .expect("event loop failed");
}
```

Continuous mode keeps scheduling frames. That is appropriate for a
continuously animated surface, but it burns CPU and GPU time when nothing
changes. If you need it, pace it with a frame limit:

```rust,no_run
# use sequoia_ui::prelude::*;
fn main() {
    let view = Text::new("60 fps");
    App::new()
        .title("Continuous")
        .size(320.0, 160.0)
        .redraw_mode(RedrawMode::Continuous)
        .fps_limit(60)
        .root(view)
        .run()
        .expect("event loop failed");
}
```

Animations and animated widgets request frames themselves, so a static
application can stay in `OnDemand` mode and still animate smoothly.

## Damage tracking

The runtime records *damage*: which regions need repainting and why. Bookkeeping
changes such as hit-test updates do not require pixels; hover, press, layout,
text, and animation changes do.

Damage is kept as up to eight merged rectangles. Each rectangle gets a small
margin, is clipped to the framebuffer, and becomes a GPU scissor region; if more
than eight disjoint regions accumulate, they collapse to their bounding box.
Compositing paths such as backdrop filters, filters, opacity groups, and
embedded surfaces still force a full-frame GPU repaint, while `Cached` subtrees
can replay filter, backdrop, and opacity spans from the CPU-side layer cache.

At the runtime level, `paint_if_dirty` compares the accumulated damage with the
previous frame:

- `FrameOutcome::Skipped` means no render list was built.
- `FrameOutcome::Painted` means the list was rebuilt and is ready to draw.

This is what makes idle windows cheap. You can observe the same behavior in
tests with `Harness::paint_if_dirty` and `Harness::damage_stats`, which return
the number of painted and skipped frames.

Damage tracking works together with the reactive graph:

- Setting a signal to an equal value is a no-op for `T: PartialEq`.
- Reads inside a view builder subscribe that builder.
- Batching several writes before the next frame produces one repaint.
- An animation or `Pulse` keeps requesting frames until it settles.

## Culling

Painting a long scroll view should not shape every off-screen label. The
runtime supports culling: subtrees whose rectangles fall completely outside the
visible window are skipped during paint. The output is identical because the
render list already clips off-screen quads; culling saves the work of producing
them.

Culling is enabled for normal app frames. It can be disabled for diagnostics
and tests, and `Harness::paint_culled` exposes the culled path directly.

A view can opt out of the culling fast path when it needs to paint outside its
layout rectangle, for example a glow or a shadow that extends beyond its
bounds. Such a view should report extra paint overflow; built-in views such as
`Container` and `Canvas` already do this through their overflow settings.

## The render list

`RenderList` is an ordered list of drawing commands. The main item types are:

- quads (solid rectangles, optionally rounded)
- text runs with positioned glyphs
- images
- shadows
- waveforms
- backdrops and filters

Items are painted in order, so later items appear on top. Clip regions can be
pushed around groups of items.

You normally interact with the list through `PaintCtx` inside `View::paint_self`
or through `Painter` inside `Canvas::on_draw`.

## Custom painting

Implement `View::paint_self` when a widget needs custom drawing. Use
`PaintCtx::theme()` for colors and `ctx.rect` for the layout rectangle:

```rust
# use sequoia_ui::prelude::*;
struct Badge {
    color: Color,
}

impl View for Badge {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn is_leaf(&self) -> bool {
        true
    }

    fn paint_self(&self, ctx: &mut PaintCtx) {
        ctx.quad(ctx.rect, self.color, 4.0);
    }
}

let badge = Badge { color: Color::hex(0xE06C9F) };
```

For immediate-mode geometry, `Canvas` hands you a `Painter` with a local
coordinate system and higher-level primitives:

```rust
# use sequoia_ui::prelude::*;
# use sequoia_ui::view::BrushStroke;
let plot = Canvas::new()
    .width(240.0)
    .height(120.0)
    .background(Color::hex(0x101014))
    .on_draw(|p| {
        let size = p.size();
        p.line(
            Point::new(8.0, size.height - 8.0),
            Point::new(size.width - 8.0, 8.0),
            BrushStroke::new(2.0),
            Color::hex(0x4DA3FF),
        );
    });
```

The canvas chapter of the guide covers transforms, cameras, hit testing, and
pointer handling in detail.

## Backends and presentation

The default backend is `GpuBackend::Auto`, which lets wgpu choose the best
available API. You can request a specific one:

```rust
# use sequoia_ui::prelude::*;
use sequoia_ui::platform::GpuBackend;

let app = App::new().backend(GpuBackend::Vulkan);
```

`Auto` is the right choice for production. Forcing a backend is useful when
diagnosing driver issues or when shipping a controlled environment.

Presentation is handled by the platform layer. It chooses a surface format,
handles resize and scale factor changes, and skips frames when the window is
occluded or when the compositor times out. The renderer itself exposes
`PresentMode` and `PowerPreference` settings through `WindowSpec`.

## Text and glyphs

Text is shaped once per unique run and cached. A text item in the render list
references positioned glyphs, not a string; the GPU receives glyph instances
from the atlas. This makes repeated labels inexpensive and keeps scrolling
smooth.

When text or its layout inputs change, the affected run is reshaped. Reactive
text built with `Text::with` subscribes to its signals and participates in
normal damage tracking. See [Text](text.md) for shaping, wrapping, and
ellipsis controls.

## Measuring work

`RenderList` offers accessors for diagnostics:

- `len()` and `is_empty()`
- `items()`
- `quads()`, `images()`, `shadows()`
- `backdrops()`

The test harness summarizes them in `PaintStats`: item count, quads, text
runs, glyphs, images, shadows, waveform bars, backdrops, and
`gpu_instances()`. Use these numbers to catch accidental over-painting and to
verify that culling or virtualization actually reduces work.

## Export

The same paint pipeline can render to an image without opening a window. That
is how thumbnails and previews are produced. See [Headless export](../guides/export.md).

## Guidelines

- Keep `redraw_mode` on demand unless the whole surface is animated.
- Let signals, not timers, decide when to repaint.
- Prefer cullable, rectangular views; report paint overflow when needed.
- Use `Canvas` for bespoke geometry and normal views for everything else.
- Measure with the test harness before optimizing.

## See also

- [View tree](view-tree.md)
- [Styling](styling.md)
- [Animation](animation.md)
- [Custom views](../guides/custom-views.md)
