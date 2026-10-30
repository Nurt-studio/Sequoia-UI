# Custom views

A custom view is either a composite that returns other views or a leaf that
paints itself. Start with a composite when the widget can be expressed by
composing existing views; use a leaf only when the widget needs its own
painting or input handling.

## Composite views

Implement `View` and return the view's contents from `body`:

```rust
# use sequoia_ui::prelude::*;
struct Section {
    title: String,
    body: String,
}

impl View for Section {
    fn body(&self) -> impl View + 'static {
        Column::new()
            .gap(4.0)
            .padding(12.0)
            .child(Text::new(self.title.clone()).bold())
            .child(Text::new(self.body.clone()))
    }
}

let section = Section {
    title: String::from("Build"),
    body: String::from("Finished in 2.4s"),
};

let _ = section;
```

A composite is still one node in the tree. Its `body` is rebuilt when the
values it reads change, so the body should be a pure description of the current
state. Keep durable state in signals or a store.

Pass `&self` data into the body when the values are cheap to clone. When the
body reads a signal, clone the signal into a closure:

```rust
# use sequoia_ui::prelude::*;
struct Counter {
    count: Signal<i32>,
}

impl View for Counter {
    fn body(&self) -> impl View + 'static {
        let count = self.count.clone();
        Row::new()
            .gap(8.0)
            .child(Text::with(move || format!("Count: {}", count.get())))
            .child(Button::new("Add").on_click({
                let count = self.count.clone();
                move || count.update(|n| *n += 1)
            }))
    }
}

let _ = Counter {
    count: Signal::new(0),
};
```

`body` must return one view. If a branch produces different types, wrap each
branch in `AnyView`.

## Leaf views

A leaf implements `Leaf` and passes itself to `impl_view_for_leaf!`. The macro
provides the `View` implementation that wires layout, painting, and events to
the leaf methods:

```rust
# use sequoia_ui::prelude::*;
#[derive(Clone)]
struct Dot {
    color: Color,
    diameter: f32,
}

impl Leaf for Dot {
    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle::default()
            .width(Length::px(self.diameter))
            .height(Length::px(self.diameter))
    }

    fn paint(&self, ctx: &mut PaintCtx) {
        let center = Point::new(
            ctx.rect.left() + ctx.rect.size.width * 0.5,
            ctx.rect.top() + ctx.rect.size.height * 0.5,
        );
        ctx.disk(center, self.diameter * 0.5, self.color);
    }
}

impl_view_for_leaf!(Dot);

let dot = Dot {
    color: Color::hex(0x2ea043),
    diameter: 16.0,
};

let _ = dot;
```

`impl_view_for_leaf!` forwards `layout_style`, `intrinsic_size`, `paint`,
`on_event`, `focusable`, `has_visual_state`, and `cursor` from `Leaf` to
`View`. The trait methods all have defaults except `paint`.

### Sizing

`layout_style` controls the rectangle the runtime assigns to the leaf. Return a
fixed size when the leaf has a natural size, or leave the style flexible and
implement `intrinsic_size` when the size depends on content:

```rust
# use sequoia_ui::prelude::*;
# use sequoia_ui::layout::LayoutCtx;
#[derive(Clone)]
struct Label {
    text: String,
}

impl Leaf for Label {
    fn intrinsic_size(&self, ctx: &mut LayoutCtx) -> Option<Size> {
        let width = ctx.text_width(&self.text, 14.0);
        Some(Size::new(width + 16.0, 24.0))
    }

    fn paint(&self, ctx: &mut PaintCtx) {
        ctx.draw_text_tl(
            &self.text,
            (ctx.rect.left() + 8.0, ctx.rect.top() + 6.0),
            14.0,
            Color::WHITE,
        );
    }
}

impl_view_for_leaf!(Label);

let _ = Label {
    text: String::from("Ready"),
};
```

`intrinsic_size` receives a `LayoutCtx`, which exposes text measurement. Return
`None` when the leaf cannot determine a useful size and let the parent decide.

### Painting

`PaintCtx` exposes the leaf's rectangle and the current `VisualState`. Painting
is clipped to the element by the runtime; use `push_clip` and `pop_clip` when a
child-like drawing needs a tighter clip.

Common drawing calls:

- `quad(rect, color, radius)` fills a rounded rectangle.
- `bordered_quad(rect, background, radius, width, border)` fills a rounded
  rectangle with a border.
- `paint_box(style)` paints a full `BoxStyle`.
- `draw_text(text, origin, size, color)` draws a single line. `origin` is the
  text **baseline**, not the top-left corner.
- `draw_text_tl(text, top_left, size, color)` draws a single line from the
  top-left of its line box; use it when positioning text from `ctx.rect`.
- `text_metrics(size)` returns the line-box `ascent`, `descent`, `line_gap`,
  and `height` for vertical centering.
- `measure_text(text, size)` returns the width and height of a specific string.
- `draw_text_wrapped(...)` draws wrapped text inside a width.
- `disk(center, radius, color)` draws a filled circle.
- `image(rect, uv, source)` draws a bitmap.
- `push_backdrop` / `push_rounded_clip` control the paint list below the leaf.

To center text vertically, use the line-box metrics instead of a magic
constant:

```rust
# use sequoia_ui::prelude::*;
# fn center(ctx: &mut PaintCtx, label: &str) {
let metrics = ctx.text_metrics(14.0);
let top = ctx.rect.top() + (ctx.rect.size.height - metrics.height) * 0.5;
ctx.draw_text_tl(label, (ctx.rect.left(), top), 14.0, Color::WHITE);
# }
# let _ = center;
```

Read signals in `paint` with `get_untracked`. Painting must not create reactive
dependencies; subscribe in `track` (or in `body` for a composite) instead.

## Reactive leaves

A leaf that reads a signal should read it once in `track` and use the untracked
value while painting:

```rust
# use sequoia_ui::prelude::*;
#[derive(Clone)]
struct Progress {
    value: Signal<f32>,
}

impl Leaf for Progress {
    fn track(&self) {
        let _ = self.value.get();
    }

    fn paint(&self, ctx: &mut PaintCtx) {
        let t = self.value.get_untracked().clamp(0.0, 1.0);
        ctx.quad(ctx.rect, Color::hex(0x21262d), 4.0);
        let filled = Rect::from_xywh(
            ctx.rect.left(),
            ctx.rect.top(),
            ctx.rect.size.width * t,
            ctx.rect.size.height,
        );
        ctx.quad(filled, Color::hex(0x2ea043), 4.0);
    }
}

impl_view_for_leaf!(Progress);

let _ = Progress {
    value: Signal::new(0.5),
};
```

`track` runs before the leaf is rebuilt. `body` from the macro calls it, so the
leaf is scheduled when any signal read there changes. Without `track`, the leaf
would paint the new value only when something else triggers a frame.

## Input and focus

`on_event` receives routed events. Guard target-only behavior with the event
phase and call `ctx.handled()` for events the leaf consumes:

```rust
# use sequoia_ui::prelude::*;
#[derive(Clone)]
struct Toggle {
    on: Signal<bool>,
}

impl Leaf for Toggle {
    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle::default()
            .width(Length::px(24.0))
            .height(Length::px(24.0))
    }

    fn paint(&self, ctx: &mut PaintCtx) {
        let on = self.on.get_untracked();
        let color = if on {
            Color::hex(0x2ea043)
        } else {
            Color::hex(0x30363d)
        };
        ctx.quad(ctx.rect, color, 6.0);
    }

    fn on_event(&self, event: &Event, ctx: &mut EventCtx) {
        if let Event::Pointer(PointerEvent::Down {
            button: PointerButton::Left,
            ..
        }) = event
        {
            self.on.update(|v| *v = !*v);
            ctx.request_focus();
            ctx.handled();
        }
    }

    fn focusable(&self) -> bool {
        true
    }

    fn has_visual_state(&self) -> bool {
        true
    }

    fn cursor(&self, _state: VisualState) -> CursorIcon {
        CursorIcon::Pointer
    }
}

impl_view_for_leaf!(Toggle);

let _ = Toggle {
    on: Signal::new(false),
};
```

`focusable` lets the leaf receive keyboard events after `request_focus`.
`has_visual_state` opts into repainting when hover or press changes.
`captures_pointer` (on `View`, or via `Canvas`) keeps a drag after the pointer
leaves the bounds. See [Events](../concepts/events.md) for the dispatch phases,
pointer capture, drag and drop, and keyboard routing.

## Custom layout

`View::arrange_override` exists for built-in containers, but the runtime's
child measure and arrange helpers are crate-private. Application code should
compose `Row`, `Column`, `Grid`, `Wrap`, `Stack`, `DockArea`, `ScrollView`, and
`SplitPanel` rather than implement a custom arrangement from outside the
framework. The `CustomLayout` trait is reserved and has no public integration
yet.

If a layout primitive is missing, implement it as a container inside
`sequoia-view`, where the runtime helpers are available, and expose a normal
builder API.

## Testing

A custom view can be tested without a window. Mount it in a `Runtime`, lay it
out, and inspect the resulting `RenderList`:

```rust
# use sequoia_ui::prelude::*;
# use sequoia_ui::text::TextSystem;
# use sequoia_ui::core::RenderList;
# use sequoia_ui::view::Runtime;
#[derive(Clone)]
struct Box;

impl Leaf for Box {
    fn paint(&self, ctx: &mut PaintCtx) {
        ctx.quad(ctx.rect, Color::RED, 4.0);
    }
}

impl_view_for_leaf!(Box);

let rt = Runtime::new();
rt.mount(Row::new().child(Box));
let mut text = TextSystem::new();
rt.layout(Rect::from_xywh(0.0, 0.0, 200.0, 100.0), &mut text);
let mut list = RenderList::new();
rt.paint(&mut list, &mut text);

assert!(!list.is_empty());
assert!(rt.errors().is_empty());
```

For composite views, assert on the child text or the paint list. For leaves,
assert on quads, glyphs, and clip ranges. The runtime records layout and paint
errors in `rt.errors()` rather than panicking.

## Pitfalls

- Do not keep application state in a leaf field that the parent recreates on
  every rebuild. Use a signal or a store.
- `body` must return one view. Use `AnyView` for branches with different types.
- Read signals in `track` or `body`, not in `paint`; painting must not subscribe.
- Return a meaningful `layout_style` or `intrinsic_size`; otherwise the leaf may
  collapse to zero size inside a `Fit` parent.
- Call `ctx.handled()` only for input the leaf fully consumes.
- `impl_view_for_leaf!` must be invoked once per type, at module scope.

## Next steps

- Understand element identity and reconciliation: [View tree](../concepts/view-tree.md).
- Choose containers and sizing rules: [Layout](../concepts/layout.md).
- Route pointer and keyboard input: [Events](../concepts/events.md).
