# Custom shaders

A custom shader is a GPU fragment program attached to a rectangle. Sequoia
compiles the shader, validates its WGSL, and draws it as an instanced quad
through the normal render list. Use shaders for effects that cannot be
expressed with fills, gradients, or backdrop filters: procedural backgrounds,
image processing, masks, transitions, and multi-pass blurs.

The typed path uses the `shader!` macro. It turns a WGSL fragment body into a
Rust struct with typed uniform fields, so call sites look like ordinary painter
operations:

```rust,no_run
use sequoia_ui::prelude::*;
use sequoia_ui::shader::Color as ShaderColor;
use sequoia_ui::shader::shader;

shader! {
    pub fn procedural_grid(
        uv: FragUv,
        cell: f32,
        line: Color,
        background: Color,
    ) -> Color {
        let position = uv * cell;
        let edge = abs(fract(position) - vec2<f32>(0.5, 0.5));
        let on_line = step(0.45, max(edge.x, edge.y));
        return mix(background, line, on_line);
    }
}

fn main() {
    let view = Canvas::new().on_draw(|g| {
        g.shader(
            ProceduralGrid {
                cell: 16.0,
                line: ShaderColor::rgba(0.16, 0.18, 0.22, 1.0),
                background: ShaderColor::rgba(0.10, 0.11, 0.13, 1.0),
            },
            g.rect(),
            &[],
        );
    });

    App::new()
        .title("Procedural grid")
        .shader_type::<ProceduralGrid>()
        .root(view)
        .run()
        .expect("event loop failed");
}
```

The macro parses the signature itself and consumes the parameter type tokens;
only the body is copied verbatim into WGSL. That is why `FragUv` and `Color` do
not need to be imported from `sequoia_ui::shader` when they appear only in the
signature. Import the shader `Color`, `Vec2`, `Vec3`, or `Vec4` when you need to
construct uniform values, as the example does with `ShaderColor`.

## Managing shader sources

`ShaderRegistry` collects shaders before they reach the app. Names are unique:
registering a name twice replaces the earlier source, so overlapping feature
modules or an optional override do not create duplicate pipelines.

```rust,no_run
use sequoia_ui::prelude::*;
use sequoia_ui::shader::ShaderRegistry;

let shaders = ShaderRegistry::new()
    .with::<ProceduralGrid>()
    .with::<Invert>()
    .with_raw("effects/scanlines", include_str!("../shaders/scanlines.wgsl"));

App::new()
    .shader_registry(shaders)
    .root(Canvas::new().with_shader_pass(ShaderPass::new(ProceduralGrid {
        cell: 16.0,
        line: ShaderColor::WHITE,
        background: ShaderColor::BLACK,
    })))
    .run()
    .expect("event loop failed");
```

`with_file` and `with_dir` load WGSL from disk; directory shaders are named by
relative path (`effects/invert`). `get`, `contains`, and `names` make the
registry usable outside startup, for example to assert that an optional effect
is available. `App::shader_names` and `App::has_shader` expose the final set,
and `Runtime::shader_names` reports what the active runtime registered.

## Passes and chains

`ShaderPass` configures one draw: rectangle, UV window, blend mode, and named
texture inputs. `primary_input` binds the first `Sampler2D`, which keeps the
common single-input effect free of string matching:

```rust,no_run
# use sequoia_ui::prelude::*;
# use sequoia_ui::core::BlendMode;
# shader! { pub fn invert(content: Sampler2D, uv: FragUv) -> Color {
#     let c = textureSample(content, content_sampler, uv);
#     return vec4<f32>(1.0 - c.rgb, c.a);
# } }
let view = Canvas::new().with_effect_pass(
    ShaderPass::new(Invert)
        .blend(BlendMode::Screen)
        .uv([0.0, 0.0, 1.0, 0.5]),
    |g| {
        g.fill(g.rect(), Color::hex(0x2d7ff9).into());
    },
);
# let _ = view;
```

`Canvas::with_shader_pass` draws a procedural shader without capturing content.
`Canvas::with_layer_effect` captures two layers and binds them in declaration
order. `ShaderChain` sequences passes with an offscreen target between them:

```rust,no_run
let view = Canvas::new().with_effect_chain(
    |g| {
        g.fill(g.rect(), Color::hex(0x2d7ff9).into());
    },
    ShaderChain::new()
        .effect(Invert)
        .effect(Blur { radius: 6.0 }),
);
# let _ = view;
```

A chain pass that is not registered is skipped and its input is forwarded; if
the last pass is missing, the captured layer is blitted back unchanged, so an
optional effect degrades instead of blanking the layer. Low-level callers
can use `Painter::try_shader` and `try_shader_with_blend` to get a `bool` for
the same diagnostics; `Painter::shader` keeps its infallible signature and logs
a warning.

## When to use a shader

Use a shader when the effect is per-pixel and not covered by the normal paint
vocabulary. Typical cases:

- procedural patterns, noise, gradients with custom falloff, and vignettes;
- color adjustments such as saturation, contrast, hue shifts, or channel
  operations;
- masks, rounded-rect effects, and transitions between two captured layers;
- separable blurs and other effects that need an offscreen pass.

Prefer built-in shapes, `Fill` gradients, `BackdropFiltered`, and
`BackdropBlur` for everyday UI. A shader runs for every pixel of its rectangle;
it is rarely the cheapest way to draw a border or a card.

Typed shaders are fragment-only. They receive the engine vertex stage and cannot
change geometry. A complete custom WGSL module can provide its own `vs_main` and
`fs_main`, but the typed macro is the supported path for drawing effects through
`Painter`.

## Parameters and uniforms

Every parameter becomes one of three things:

| Parameter type | Meaning | In the WGSL body |
| --- | --- | --- |
| `FragPos`, `FragLocal` | Fragment position in draw-rect pixels | `frag.local` |
| `FragUv` | Texture coordinates in `0.0..=1.0` | `frag.uv` |
| `FragWorld` | Window coordinates in pixels | `frag.world` |
| `Sampler2D` | `texture_2d` plus filtering sampler | `name` and `name_sampler` |
| `Sampler3D` | `texture_3d` plus filtering sampler | `name` and `name_sampler` |
| `f32`, `i32`, `u32`, `bool` | Four-byte uniform | `name` |
| `Vec2`, `Vec3`, `Vec4` | Vector uniform | `name` |
| `Color` | Straight-alpha RGBA uniform | `name` as `vec4<f32>` |
| `[f32; N]` | Packed float array | `name` as `array<f32, N>` |
| `[Vec4; N]` | Vector array | `name` as `array<vec4<f32>, N>` |

The generated body can also read `frag.local`, `frag.world`, `frag.uv`,
`frag.rect_size`, and `screen.size` directly. `screen.size` is the physical
framebuffer size; use it to convert pixel distances into UV offsets:

```text
let offset = vec2<f32>(radius, 0.0) / screen.size.xy;
let sample_uv = uv + offset;
```

Uniform fields are packed in declaration order with WGSL alignment rules. The
macro enforces a 256-byte total limit, so a shader can carry at most sixteen
`vec4` values. Reorder fields if the generated struct would waste space, or move
constant values into the shader body.

`bool` is encoded as `u32` and converted back in the generated WGSL. `Vec3`
has the same 16-byte stride as WGSL `vec3<f32>`.

## Textures and offscreen passes

A `Sampler2D` parameter declares one texture input. Up to six are allowed.
Each input is bound to its own group starting at group 1, so a shader can sample
several captured layers:

```rust,no_run
use sequoia_ui::prelude::*;
use sequoia_ui::core::{BlendMode, TexInput};
use sequoia_ui::shader::shader;

shader! {
    pub fn invert(content: Sampler2D, uv: FragUv) -> Color {
        let sampled = textureSample(content, content_sampler, uv);
        return vec4<f32>(1.0 - sampled.rgb, sampled.a);
    }
}

let view = Canvas::new().with_effect(Invert, |g| {
    g.fill(g.rect(), Color::hex(0x2d7ff9).into());
    g.circle(Point::new(120.0, 80.0), 32.0, Color::WHITE.into());
});
# let _ = view;
```

`Canvas::with_effect` renders the closure into an offscreen target, then draws
the shader over the whole canvas with the target bound as `content`. The shader
must be registered with `App::shader_type::<Invert>()`.

For manual passes, render content into a target with `Painter::group`, then draw
the shader with the target as an input:

```rust,no_run
# use sequoia_ui::prelude::*;
use sequoia_ui::core::{BlendMode, TexInput};
use sequoia_ui::shader::shader;

shader! {
    pub fn invert(content: Sampler2D, uv: FragUv) -> Color {
        let sampled = textureSample(content, content_sampler, uv);
        return vec4<f32>(1.0 - sampled.rgb, sampled.a);
    }
}

let view = Canvas::new().on_draw(|g| {
    let rect = g.rect();
    let (target, ()) = g.group(rect.size, BlendMode::Normal, |inner| {
        inner.fill(rect, Color::hex(0x2d7ff9).into());
        inner.circle(Point::new(120.0, 80.0), 32.0, Color::WHITE.into());
    });

    g.shader(Invert, rect, &[("content", TexInput::Target(target))]);
});
# let _ = view;
```

Names in the input slice must match the `Sampler2D`/`Sampler3D` parameters
exactly. A missing input samples an opaque white texture; an input name the
shader does not declare is ignored with a warning. `Painter::shader` uses the current blend mode,
and `shader_with_blend` takes an explicit `BlendMode` when a pass needs one.
`shader_at` lets you choose the source UV rectangle, which is useful for
inspecting or repeating part of a captured layer.

Targets are ordered: a target must be finished before a later draw samples it.
`group` records the target bounds in the render list and returns its
`RenderTargetId`; keep the id only for draws that follow it.

## Arrays and 3D LUTs

Two uniform shapes are meant for tables: `[f32; N]` and `[Vec4; N]`. WGSL
requires uniform array strides to be multiples of 16 bytes, so `[f32; N]` is
stored packed four values per `vec4` and unpacked into a local `array<f32, N>`
before the body runs — `weights[i]` reads a single `f32`, while `[Vec4; N]`
keeps one `vec4<f32>` per element.

A `Sampler3D` parameter binds a `texture_3d<f32>` with a linear, clamp-to-edge
sampler. Together with `ImageSource::cube` it applies an Adobe/IRIDAS `.cube`
colour LUT: the sampled colour becomes the 3D lookup coordinate, and the
sampler interpolates the table trilinearly.

```rust,no_run
use sequoia_ui::prelude::*;
use sequoia_ui::core::{BlendMode, ImageSource, TexInput};
use sequoia_ui::shader::shader;

shader! {
    pub fn apply_lut(content: Sampler2D, lut: Sampler3D, uv: FragUv) -> Color {
        let sampled = textureSample(content, content_sampler, uv);
        let mapped = textureSample(lut, lut_sampler, sampled.rgb);
        return vec4<f32>(mapped.rgb, sampled.a);
    }
}

let view = Canvas::new().on_draw(|g| {
    let rect = g.rect();
    let (photo, ()) = g.group(rect.size, BlendMode::Normal, |inner| {
        inner.image(rect, [0.0, 0.0, 1.0, 1.0], ImageSource::File("photo.jpg".into()));
    });
    g.shader(
        ApplyLut,
        rect,
        &[
            ("content", TexInput::Target(photo)),
            ("lut", TexInput::Image(ImageSource::cube("lut.cube"))),
        ],
    );
});
# let _ = view;
```

The renderer parses the `.cube` file, uploads it as a `Rgba16Float`
`texture_3d` on first use and caches it by path. Only 3D tables with
`LUT_3D_SIZE` in `2..=128` and the default `DOMAIN_MIN 0 0 0` /
`DOMAIN_MAX 1 1 1` are accepted; 1D or 2D LUTs and custom domains are rejected
with a warning. Because the sampler interpolates between table entries, a 33³
LUT reproduces what a photo editor applying the same table produces. 3D inputs
count toward the six-texture limit, and a missing one samples an opaque white
1×1×1 volume.

## Canvas shortcuts

Two helpers cover the common cases:

- `Canvas::with_effect(shader, content)` captures `content` and runs `shader`
  over it. This is the shortest path for a single-pass filter.
- `Canvas::with_mask(mask, content)` captures both closures and draws the
  built-in `apply_mask` shader. The mask's alpha multiplies the content's
  alpha; the content keeps its color.

`App` registers `apply_mask` automatically, so `with_mask` needs no
`App::shader_type` call. It is also available as
`sequoia_ui::shader::ApplyMask` if you want to draw it manually with
`Painter::shader`.

## Registration

Shaders are registered when the window renderer is created:

```rust,no_run
# use sequoia_ui::prelude::*;
# use sequoia_ui::shader::shader;
# shader! { pub fn invert(content: Sampler2D, uv: FragUv) -> Color {
#     let sampled = textureSample(content, content_sampler, uv);
#     return vec4<f32>(1.0 - sampled.rgb, sampled.a);
# } }
let app = App::new()
    .title("Effects")
    .shader_type::<Invert>();
# let _ = app;
```

- `shader_type::<S>()` registers a typed shader under `S::NAME`.
- `shader(name, wgsl)` registers a complete WGSL module by name.
- `shader_file(name, path)` reads a `.wgsl` file. A read error is logged and the
  shader is skipped.
- `shader_dir(dir)` recursively registers every `.wgsl` file. The shader name is
  the path relative to `dir` without the extension, using `/` separators:
  `effects/invert.wgsl` becomes `effects/invert`.
- `shaders()` returns the pending `(name, wgsl)` pairs for diagnostics.

Registration failures from driver validation are logged and the shader is
skipped. `Painter::shader` also logs a warning and draws nothing when the name is
not registered, so a missing `shader_type` shows up as a blank rectangle rather
than an error.

### Hot reload

For development, `watch_shaders()` loads `assets/shaders` and rescans it while
the app runs; `watch_shader_dir(dir)` does the same for another directory. An
edited file is re-read, registered on every open window under the same name, and
the window repaints. A shader that fails to compile is logged and the previous
version keeps rendering.

The `shader!` macro validates the generated module with `naga` at compile time,
including entry points and type checking. Registration validates again at
runtime, which catches hand-written modules and driver-specific restrictions.

## Raw WGSL modules

For a custom vertex stage or a shader generated outside the macro, register a
complete module whose `vs_main` and `fs_main` match the engine ABI. A body that
only needs a fragment function can be wrapped:

```rust,no_run
use sequoia_ui::render::compose_user_shader;

let wgsl = compose_user_shader(
    r#"
fn fs_user(frag: ShaderVarying) -> vec4<f32> {
    return vec4<f32>(frag.uv.x, frag.uv.y, 0.5, 1.0);
}
"#,
);

assert!(wgsl.contains("fn vs_main"));
assert!(wgsl.contains("fn fs_main"));
```

Raw shaders read uniforms from the `uniforms.values` array instead of typed
fields. In practice, prefer a typed `SequoiShader` wrapper: implement
`SequoiShader` for your type, return the composed WGSL from `WGSL`, and encode
the uniform bytes yourself. That makes the shader usable with `Painter::shader`
and `App::shader_type` without a separate drawing path.

## Reusable WGSL helpers

A shader body automatically includes the `sq_*` helper library. The most useful
groups are:

| Area | Helpers |
| --- | --- |
| Color space | `sq_luminance`, `sq_srgb_to_linear`, `sq_linear_to_srgb` |
| Color spaces | `sq_rgb_to_hsv`, `sq_hsv_to_rgb`, `sq_hsl_to_rgb`, `sq_rgb_to_hsl` |
| Interpolation | `sq_mix3`, `sq_mix4`, `sq_lerp`, `sq_smoothstep`, `sq_clamp01` |
| Noise | `sq_hash21`, `sq_noise21`, `sq_fbm21` |
| Transforms | `sq_rotate`, `sq_remap`, `sq_checker_uv` |
| Adjustments | `sq_contrast`, `sq_saturate`, `sq_vignette` |
| Blends | `sq_overlay`, `sq_soft_light`, `sq_hard_light`, `sq_screen`, `sq_multiply`, `sq_add_blend` |

The same operations exist on the CPU in `sequoia_ui::shader::stdlib`, which is
useful for tests and for animating uniforms without a GPU. The CPU functions
take the shader crate's `Vec2`, `Vec3`, and `Vec4` types.

## Blending and compositing

The fragment output is straight-alpha RGBA. The engine wraps every shader with a
premultiplied-alpha fragment stage, applies clip and opacity, and discards
fragments outside the clip region. Do not premultiply RGB by alpha yourself;
return the color as you want it composited.

To choose a blend mode, either set it for a scope or pass it explicitly:

```rust,no_run
# use sequoia_ui::prelude::*;
# use sequoia_ui::core::BlendMode;
# use sequoia_ui::shader::shader;
# shader! { pub fn add(uv: FragUv) -> Color {
#     return vec4<f32>(uv.x, uv.y, 0.5, 0.5);
# } }
# let view = Canvas::new().on_draw(|g| {
let rect = g.rect();
g.with_blend(BlendMode::Screen, |g| {
    g.shader(Add, rect, &[]);
});
g.shader_with_blend(Add, rect, &[], BlendMode::Multiply);
# });
# let _ = view;
```

`BlendMode` applies to the shader draw, not to the shader body. A shader that
returns varied alpha composites correctly with the standard source-over mode;
use additive or screen modes for glows and light effects.

## Performance and pitfalls

- Prefer one `with_effect_chain` over manually created targets: each link in the
  chain is a single offscreen pass, and an empty chain draws content directly.
- `ShaderPass` and `ShaderChain` clone cheaply (a small vector of bindings or
  pass closures), so they can live in a view struct instead of a draw closure.

- Every shader draw is a quad with per-pixel work. Full-window effects at high
  device pixel ratios can be expensive; keep passes few and reuse targets.
- A two-pass separable blur is much cheaper than a single large-kernel blur.
  See `cargo run -p sequoia-ui --example progressive_blur`.
- Shader draws are render items, so they follow the same damage tracking as
  other draws. Targets and filters used by a pass can still force a full
  repaint; see [Rendering](../concepts/rendering.md).
- Textures are bound by name. Renaming a `Sampler2D`/`Sampler3D` parameter without updating
  the input slice silently samples the fallback texture; check for warnings.
- The macro rejects unsupported parameter types, duplicate parameter names, more
  than three textures, and uniform payloads over 256 bytes at compile time.
- `VertexIndex` is accepted by the macro parser but the fragment stage currently
  receives `0`; do not use it for per-vertex variation.
- WGSL entry points cannot be defined in the body; the macro supplies
  `vs_main` and `fs_main`. The body is inside `fs_user`, so use `return` rather
  than declaring another entry point.
- The renderer validates shader modules on the UI thread when windows are
  created. Many large shaders can add startup time; prefer registering only the
  shaders a window uses.

## Complete examples

- `cargo run -p sequoia-ui --example progressive_blur` builds a separable,
  progressive blur with two shader passes and a UV-space ramp. It shows `group`,
  `shader`, `screen.size`, and the `sq_hash21` dithering helper.
- `cargo run -p sequoia-ui --example shader_gallery` renders a blurred card over
  a background and shows how to keep the blur ramp aligned with a UI rectangle.

See [Canvas](canvas.md) for the painter lifecycle and offscreen groups, and
[Rendering](../concepts/rendering.md) for how shader draws fit into the render
list. For raw `wgpu` pipelines that draw their own geometry, use
[Embedded GPU surfaces](gpu-surfaces.md) instead of a shader pass.
