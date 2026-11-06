# Embedded GPU surfaces

`WgpuSurface` is the bridge between the Sequoia layout and your own `wgpu`
drawing. It is a regular leaf view: the layout gives it a rectangle, and the
window renderer calls your `WgpuRenderer` implementation with the device, queue,
and command encoder for that rectangle.

Use a surface when the content cannot be expressed as quads, text, images, or
render-list shader passes: a 3D viewport, a game scene, a video or camera feed,
a tile map, or a custom multi-pass GPU pipeline. For 2D drawing that uses the
Sequoia painter, use [`Canvas`](canvas.md). For full-screen fragment effects
over UI content, use [custom shaders](shaders.md).

The view type is `WgpuSurface<R>`; the trait you implement is `WgpuRenderer`.
Do not confuse the trait with `sequoia_ui::render::GpuRenderer`, which is the
window backend that drives the whole frame.

A complete runnable example that places an animated surface next to ordinary
widgets lives in
[`crates/sequoia-ui/examples/wgpu_surface.rs`](https://github.com/Nurt-studio/Sequoia-UI/blob/main/crates/sequoia-ui/examples/wgpu_surface.rs).
Run it with `cargo run -p sequoia-ui --example wgpu_surface`.

## Minimal example

The renderer creates its pipelines lazily from `frame.device`, because the GPU
device does not exist until the application window is created. Draw calls go
into the supplied encoder; the host submits it at the end of the frame.

```rust,no_run
use sequoia_ui::platform::{WgpuFrame, WgpuRenderer, WgpuSurface};
use sequoia_ui::prelude::*;
use sequoia_ui::wgpu;

const SHADER: &str = r#"
struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOut {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0),
    );
    var uvs = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(2.0, 1.0),
        vec2<f32>(0.0, -1.0),
    );
    var out: VertexOut;
    out.clip = vec4<f32>(positions[index], 0.0, 1.0);
    out.uv = uvs[index];
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let glow = 1.0 - length(in.uv - vec2<f32>(0.5));
    return vec4<f32>(0.1 + 0.6 * glow, 0.35, 0.9 - 0.4 * glow, 1.0);
}
"#;

struct Gradient {
    pipeline: Option<wgpu::RenderPipeline>,
}

impl Gradient {
    fn pipeline(&mut self, frame: &WgpuFrame<'_>) -> &wgpu::RenderPipeline {
        self.pipeline.get_or_insert_with(|| {
            let module = frame.device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("surface-gradient"),
                source: wgpu::ShaderSource::Wgsl(SHADER.into()),
            });
            frame.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("surface-gradient"),
                layout: None,
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some("fs_main"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: frame.format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            })
        })
    }
}

impl WgpuRenderer for Gradient {
    fn render(&mut self, frame: &mut WgpuFrame) {
        let region = frame.region;
        let pipeline = self.pipeline(frame);

        let mut pass = frame.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("surface-gradient"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: frame.target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_viewport(
            region.x as f32,
            region.y as f32,
            region.width as f32,
            region.height as f32,
            0.0,
            1.0,
        );
        pass.set_scissor_rect(region.x, region.y, region.width, region.height);
        pass.set_pipeline(pipeline);
        pass.draw(0..3, 0..1);
    }
}

fn main() {
    let viewport = WgpuSurface::new(Gradient { pipeline: None })
        .background(Color::hex(0x101014))
        .border(1.0, Color::hex(0x30363d))
        .radius(10.0)
        .expand();

    let view = Column::new()
        .padding(16.0)
        .gap(12.0)
        .child(Text::new("Preview").bold())
        .child(viewport);

    App::new()
        .title("Embedded GPU surface")
        .size(720.0, 480.0)
        .root(view)
        .run()
        .expect("event loop failed");
}
```

Two rules in that callback matter more than the shader itself:

- Use `frame.region` as the viewport and scissor rectangle. In direct mode the
  target is the whole window, not a texture that belongs to the surface.
- Use `LoadOp::Load` in direct mode. The host has already painted the UI
  underneath; clearing the target would erase it.

## Layout integration

A surface is a leaf. It has no children and no internal layout, so size it like
any other leaf through `width`, `height`, `min_width`, `max_width`,
`min_height`, `max_height`, `flex`, or `expand`. `WgpuSurface::new` starts with
`Length::Fill(1.0)` on both axes, which fills its parent.

In a `Row` or `Column`, combine `expand` or `flex` with fixed-size siblings.
The same options as `Canvas` apply: it can sit in `Row`, `Column`, `Grid`,
`Stack`, `SplitPanel`, or `DockArea`, and an ancestor `ScrollView` clips it.

```rust,no_run
# use sequoia_ui::platform::{WgpuFrame, WgpuRenderer, WgpuSurface};
# use sequoia_ui::prelude::*;
# struct Scene;
# impl WgpuRenderer for Scene {
#     fn render(&mut self, _frame: &mut WgpuFrame) {}
# }
let viewport = WgpuSurface::new(Scene)
    .min_height(200.0)
    .expand();

let view = Row::new()
    .child(
        Column::new()
            .width(220.0)
            .padding(12.0)
            .child(Text::new("Layers")),
    )
    .child(Column::new().padding(12.0).child(viewport));

# let _ = view;
```

When the surface is fully scrolled out of view, the runtime culls it and the
window renderer destroys its offscreen texture. It is created again when the
surface becomes visible. Use `Stack` when real widgets must float above a
surface:

```rust,no_run
# use sequoia_ui::platform::{WgpuFrame, WgpuRenderer, WgpuSurface};
# use sequoia_ui::prelude::*;
# struct Scene;
# impl WgpuRenderer for Scene {
#     fn render(&mut self, _frame: &mut WgpuFrame) {}
# }
let view = Stack::new()
    .child(WgpuSurface::new(Scene))
    .child_at(Button::new("Reset camera"), StackPos::top_right());

# let _ = view;
```

The button receives input because it is a separate view above the surface. A
`WgpuSurface` cannot host focusable children itself.

## The per-frame contract

`WgpuFrame` describes one draw call:

| Field | Meaning |
| --- | --- |
| `device`, `queue` | The window device and queue. Create resources here. |
| `encoder` | Record commands into this encoder. Do not submit it yourself. |
| `target` | The texture view to draw into. |
| `depth` | Optional depth attachment supplied by the host. |
| `format` | The actual format of `target`; create pipelines for it. |
| `size` | Target size in physical pixels. In direct mode this is the window framebuffer. |
| `scale` | Logical-to-physical pixel ratio. |
| `region` | The physical-pixel rectangle that belongs to this surface. |
| `adapter`, `backend` | The selected adapter and backend. |
| `index`, `delta` | Monotonic frame index and time since the previous frame. |

The `region` rectangle is the important one:

- In **direct** mode it is the visible part of the surface, already clipped by
  ancestors. The `x` and `y` offsets are relative to the window target.
- In **cached** mode it is always `(0, 0, width, height)` of the offscreen
  texture. The host blits only the visible portion into the window.

`frame.size` is the target size, not the surface size. Derive the surface
aspect ratio from `region.width` and `region.height`:

```rust,ignore
let aspect = frame.region.width as f32 / frame.region.height as f32;
```

For a direct surface, set both viewport and scissor to `region`. If you need a
depth buffer, use `frame.depth` when it is present; otherwise create and manage
your own attachment in the callback.

## Direct and cached rendering

By default a surface draws directly into the window scene during the normal
paint pass. Each frame that paints the surface invokes `WgpuRenderer::render`.

With `.cached(true)`, the host allocates an offscreen texture for the surface,
clears it, calls `render` once, and then composites the texture. While the
surface is clean, later frames skip the callback and reuse the texture.

Caching pays off for expensive, mostly static content such as a 3D viewport
that only changes on camera movement. It also changes the callback contract:

- The host clears the offscreen target before `render`, so use `LoadOp::Load`.
- The renderer decides when to redraw through `is_dirty`. The default is
  `true`, which means "redraw every frame" and makes `.cached(true)` pointless
  until you implement a dirty flag.
- Call `mark_clean` state changes after a successful draw. The host invokes
  `WgpuRenderer::mark_clean` automatically after it renders the cached surface.
- A size, format, or sample-count change forces a redraw regardless of
  `is_dirty`. A cache eviction or leaving the tree also forces a redraw when
  the surface returns.

```rust,no_run
# use sequoia_ui::platform::{WgpuFrame, WgpuRenderer, WgpuSurface};
# use sequoia_ui::prelude::*;
struct Scene {
    dirty: bool,
}

impl WgpuRenderer for Scene {
    fn render(&mut self, _frame: &mut WgpuFrame) {
        // Record draws for the current scene.
    }

    fn is_dirty(&self) -> bool {
        self.dirty
    }

    fn mark_clean(&mut self) {
        self.dirty = false;
    }
}

let surface = WgpuSurface::new(Scene { dirty: true })
    .cached(true)
    .height(240.0);
# let _ = surface;
```

Both modes suppress partial damage repainting: a window that contains any
embedded surface repaints the full frame. Keep the number of surfaces small and
avoid full-window surfaces at high device pixel ratios when a render-list
effect can do the job.

## Formats, color, and multisampling

Direct surfaces draw with the window's render format. Cached surfaces default
to `Rgba8UnormSrgb` offscreen. Use `.format(...)` to request a different
offscreen format:

```rust,no_run
# use sequoia_ui::platform::{WgpuFrame, WgpuRenderer, WgpuSurface};
# use sequoia_ui::prelude::*;
# use sequoia_ui::wgpu;
# struct Scene;
# impl WgpuRenderer for Scene {
#     fn render(&mut self, _frame: &mut WgpuFrame) {}
# }
let surface = WgpuSurface::new(Scene)
    .cached(true)
    .format(wgpu::TextureFormat::Rgba16Float)
    .samples(4)
    .height(240.0);
# let _ = surface;
```

- `.format(...)` forces the offscreen path even when `.cached(true)` is not
  set, because content in another format must be composited through a texture.
- The format must be a renderable, filterable color format. Unsupported formats
  log a warning and fall back to the default offscreen format; always read the
  actual value from `WgpuFrame::format` instead of assuming the request
  succeeded.
- sRGB targets expect linear color output and encode it in hardware. Non-sRGB
  targets store shader output as-is and are treated as linear when composited.
  `frame.color_space()` returns the implied color space.
- Cached content uses straight alpha. Do not premultiply RGB by alpha in the
  fragment output.
- `.samples(...)` applies to offscreen surfaces. Values map to `1`, `2`, or `4`
  samples and are lowered to `1` when the format does not support the request.
  The pipeline's multisample count must match the actual texture; for direct
  drawing, match the window target instead.

## Pointer input, focus, and keys

Input callbacks receive window coordinates, not surface-local coordinates:

```rust,no_run
# use sequoia_ui::platform::{WgpuFrame, WgpuRenderer, WgpuSurface};
# use sequoia_ui::prelude::*;
# struct Scene;
# impl WgpuRenderer for Scene {
#     fn render(&mut self, _frame: &mut WgpuFrame) {}
# }
let last_click = Signal::new(Point::ZERO);
let clicks = last_click.clone();

let surface = WgpuSurface::new(Scene)
    .on_pointer(move |event, area| {
        if let PointerEvent::Down { pos, .. } = event {
            let local = Point::new(pos.x - area.left(), pos.y - area.top());
            clicks.set(local);
        }
    })
    .focusable(true)
    .on_key(|event| {
        if event.key == Key::ESCAPE {
            // Cancel a tool or close an overlay.
        }
    });
# let _ = surface;
```

`area` is the full layout rectangle of the surface, in logical pixels. Subtract
its origin to get local coordinates, then apply your own camera or zoom. The
callback also fires for `Move`, `Up`, and wheel events while the pointer is over
the surface or captured by it.

A surface captures the pointer by default, which keeps drags alive when the
pointer leaves its bounds. `.click_through(true)` disables that capture so a
parent `ScrollView`, split panel, or drag handler can own the gesture.
`click_through` does not make the surface transparent to hit testing; it still
receives events.

`.focusable(true)` adds the surface to the Tab order and lets a left click focus
it. `on_key` receives key events only while the surface is focused. It has no
accessible role or label; if the content needs assistive technology, expose the
same actions through real widgets around the surface.

Input callbacks are `Fn`, not `FnMut`. Keep mutable state in a `Signal`, `Cell`,
or inside the renderer and update it through `surface.renderer()`.

## Styling the surface

`background`, `border`, `radius`, and `padding` are host-drawn decorations:

- `background` fills the surface rectangle behind the callback. For cached
  surfaces it also becomes the clear color of the offscreen texture.
- `border` draws an outer border and insets the content by the border width.
- `padding` insets the content in direct mode, so `frame.region` shrinks by the
  padding and border. In cached mode the offscreen target still covers the full
  surface rectangle; apply the same inset inside the callback if the content
  must line up with the border.
- `radius` rounds the background and border only. It does not clip the callback
  content. Clip or mask inside your pipeline when content must respect rounded
  corners.

```rust,no_run
# use sequoia_ui::platform::{WgpuFrame, WgpuRenderer, WgpuSurface};
# use sequoia_ui::prelude::*;
# struct Scene;
# impl WgpuRenderer for Scene {
#     fn render(&mut self, _frame: &mut WgpuFrame) {}
# }
let surface = WgpuSurface::new(Scene)
    .background(Color::hex(0x0f1117))
    .border(1.0, Color::hex(0x30363d))
    .radius(12.0)
    .padding(8.0)
    .height(280.0);
# let _ = surface;
```

## Sharing renderer state

The builder takes ownership of the renderer, but the surface keeps it in a
shared cell. `surface.renderer()` returns an `Rc<RefCell<R>>` for updating
camera, scene, or animation state from event handlers, buttons, or effects:

```rust,no_run
# use sequoia_ui::platform::{WgpuFrame, WgpuRenderer, WgpuSurface};
# use sequoia_ui::prelude::*;
struct Scene {
    yaw: f32,
}

impl WgpuRenderer for Scene {
    fn render(&mut self, _frame: &mut WgpuFrame) {
        let _yaw = self.yaw;
    }
}

let surface = WgpuSurface::new(Scene { yaw: 0.0 });
let scene = surface.renderer();

Button::new("Rotate").on_click(move || {
    scene.borrow_mut().yaw += 0.1;
});
# let _ = surface;
```

Cloning a `WgpuSurface` shares the same `Rc` and therefore the same renderer,
state, and cache key. Cloning is cheap, but each painted copy invokes `render`
for the same instance. Do not hold a `RefCell` borrow across a frame; mutate
state before or after rendering, or use `try_borrow_mut` when unsure.

Create GPU resources inside `render` from `frame.device`, not before the
window exists, and never use resources created on a different device. When a
surface is cloned into two windows, each window renderer has its own device
unless the application explicitly shares one.

## Animation and redraw

Sequoia paints on demand. A direct or cached surface that must animate has to
ask for the next frame from inside `render`:

```rust,no_run
# use sequoia_ui::platform::{WgpuFrame, WgpuRenderer};
struct Spinner {
    angle: f32,
    spinning: bool,
}

impl WgpuRenderer for Spinner {
    fn render(&mut self, frame: &mut WgpuFrame) {
        if self.spinning {
            self.angle += frame.delta.as_secs_f32() * 2.0;
            frame.request_frame();
        }
        // Draw at self.angle.
    }

    fn is_dirty(&self) -> bool {
        self.spinning
    }
}
# let _ = Spinner { angle: 0.0, spinning: false };
```

`request_frame` sets a one-shot redraw flag that the platform consumes after
the frame is painted and forwards into the runtime as a frame request, so
`paint_if_dirty` paints the next frame instead of skipping it. Call it every
frame for continuous animation, not once.
For cached surfaces, `is_dirty` must also stay `true` while the content is
animating; otherwise the host skips `render` and never sees the request.

The window-level `RedrawMode::Continuous` option is a simpler alternative when
the whole window is animated, but it paints even when nothing changed. Prefer
`request_frame` for a few animated surfaces, and use `fps_limit` when you need
to cap a continuous loop.

Two small helpers reduce bookkeeping:

- `PendingFrames::request(n)` and `request_if(changed, n)` schedule a bounded
  number of follow-up frames; `tick` consumes one per frame.
- `Revision::changed(hash)` returns `true` only when a state hash differs from
  the previous call, which is useful for implementing `is_dirty`.

## Performance and lifecycle

- Resources created through `frame.device` live until the renderer is dropped.
  Recreate pipelines only when `frame.format` or device features change.
- Cached textures are keyed by renderer identity, so keeping one stable
  `WgpuSurface` (or a clone) preserves the cache across rebuilds.
- Under memory pressure the renderer may drop cached surfaces and lower its
  quality level. A dropped surface is rendered again on its next visible frame,
  so the callback must not assume it keeps its texture between draws.
- Surfaces prevent partial-damage repainting for the entire window. A small
  surface still forces a full repaint each time the window paints.
- `RenderStats::dropped_surfaces` counts surfaces skipped because of the blit
  or memory budget. Watch it if a viewport stays blank on low-end devices.
- Do not call `queue.submit` inside `render`; record commands into the provided
  encoder and let the host submit them. `queue.write_buffer` is fine for small
  uniform updates.
- Do not keep the surface's `RefCell` borrowed while the frame is being
  rendered, and do not mutate signals from `render` unless the mutation is
  idempotent; a `Signal` write there can schedule another paint in the middle
  of the current one.

## Testing without a GPU

A surface participates in layout even when no GPU is available. Mount it in a
`Runtime`, paint, and inspect the rectangles the renderer would receive with
`take_surface_draws`:

```rust
use sequoia_ui::platform::{WgpuFrame, WgpuRenderer, WgpuSurface};
use sequoia_ui::prelude::*;
use sequoia_ui::view::Runtime;
use sequoia_ui::text::TextSystem;
use sequoia_ui::core::RenderList;

struct Stub;

impl WgpuRenderer for Stub {
    fn render(&mut self, _frame: &mut WgpuFrame) {}
}

let runtime = Runtime::new();
runtime.mount(
    Column::new()
        .child(WgpuSurface::new(Stub).height(100.0))
        .child(Unit),
);

let mut text = TextSystem::new();
runtime.layout(Rect::from_xywh(0.0, 0.0, 400.0, 400.0), &mut text);

let mut list = RenderList::new();
runtime.paint(&mut list, &mut text);
assert!(list.is_empty(), "a surface emits no render-list items");

let draws = runtime.take_surface_draws();
assert_eq!(draws.len(), 1);
assert_eq!(draws[0].rect, Rect::from_xywh(0.0, 0.0, 400.0, 100.0));
```

`draws[0].rect` is the visible (clipped) rectangle and `draws[0].full` is the
full layout rectangle. Use them to verify sizing, scrolling, and culling before
opening a window.

## Pitfalls

- Clearing the target with `LoadOp::Clear` in direct mode erases the rest of
  the UI. Use `Load` and rely on `region` for bounds.
- Ignoring `frame.region` draws at the wrong offset or over the whole window.
- Using a pipeline whose format, sample count, or blend state does not match
  the target produces a GPU validation error.
- Treating pointer positions as local coordinates. Always subtract `area`.
- Forgetting `frame.request_frame()` produces a still image after the first
  frame; calling it once only produces one extra frame.
- Enabling `.cached(true)` without implementing `is_dirty` redraws every frame
  and wastes the cache.
- Expecting padding or radius to clip cached content. Apply the inset and any
  rounded mask inside the renderer.
- Holding a `RefCell` borrow across the next paint panics inside `render`.
- Building pipelines at application start, before `frame.device` exists, or on
  another window's device.

## See also

- [Rendering and frames](../concepts/rendering.md)
- [Canvas](canvas.md)
- [Custom shaders](shaders.md)
- [Layout](../concepts/layout.md)
- [Events](../concepts/events.md)
- [Custom views](custom-views.md)
