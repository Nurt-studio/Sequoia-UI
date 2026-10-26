# View tree

A Sequoia interface is a tree of views. Each node describes one piece of the
interface: a label, a button, a container, or a custom widget. The runtime
mounts that description, measures it, paints it, and routes input to it.

Most application code composes built-in views. When a reusable piece needs its
own name, implement `View` and return its contents from `body`.

## Composition

Every view has exactly one required method:

```rust
# use sequoia_ui::prelude::*;
struct Header {
    title: Signal<String>,
}

impl View for Header {
    fn body(&self) -> impl View + 'static {
        let title = self.title.clone();
        Row::new()
            .gap(8.0)
            .child(Text::new("Sequoia"))
            .child(Text::with(move || title.get()).bold())
    }
}

let title = Signal::new(String::from("View tree"));
let _ = Header { title };
```

A composite view returns one child. That child can contain any number of
children of its own, so composition is how larger interfaces are built. The
`Header` above is still one node in the tree even though it renders a row with
two labels.

Views are descriptions, not containers for durable mutable state. The runtime
rebuilds a view's body when its reactive reads change, replacing the view
instance used for that build. Keep state that must survive a rebuild in a
`Signal`, `Store`, or another service instead of in a plain field.

### Reactive reads

`Text::with` takes a closure and runs it in a reactive scope. Reading a signal
inside that closure subscribes the label to the signal. When the value changes,
the label is rebuilt without rebuilding unrelated views.

Reading a signal directly in `body` would subscribe the whole composite view
instead. Both approaches work, but prefer the narrowest closure that can own
the read. This keeps updates local and avoids unnecessary work.

## Leaves and built-in views

A leaf paints itself instead of returning children. The built-in `Text`,
`Canvas`, `Image`, and most widgets are leaves. Layout containers such as `Row`
and `Column` are composites that supply their children directly, so their body
is only a placeholder.

The distinction matters when writing your own view: a composite returns a body,
while a leaf implements the `Leaf` trait and paints its rectangle. The
[Custom views](../guides/custom-views.md) guide covers both forms.

The prelude includes the common building blocks:

- Content: `Text`, `RichText`, `Icon`, `Image`, `Button`, and form widgets.
- Composition: `Row`, `Column`, `Grid`, `Wrap`, `Stack`, `Fragment`, `AnyView`,
  and `Unit`.
- Navigation and scrolling: `DockArea`, `ScrollView`, `SplitPanel`, `List`,
  and `VirtualList`.
- Drawing and input: `Canvas`.

## Element identity

The runtime assigns each mounted view an opaque `ElementId`. The ID stays
stable while the node remains at the same keyed position with the same
concrete view type. Stable identity lets focus, scroll offsets, animations,
and other per-element state survive updates.

When a parent rebuilds, it reconciles its children by key:

- A matching key with a matching view type reuses the element. Its view is
  replaced with the new description and its body is rebuilt.
- A matching key with a different view type unmounts the old subtree and mounts
  a new element.
- A missing key unmounts the old element.
- A new key mounts a fresh element.

Ordinary containers key their children by position. Inserting an item at the
front of a `Column` therefore shifts every child's identity, which can reset
state in the shifted children. Use an explicit keyed list when items can be
reordered, inserted, or removed.

## Keyed lists

`List` runs an item closure inside a reactive scope and reconciles the result
by key:

```rust
# use sequoia_ui::prelude::*;
#[derive(Clone, PartialEq)]
struct Task {
    id: u64,
    title: String,
}

let tasks = Signal::new(vec![
    Task {
        id: 1,
        title: String::from("Write docs"),
    },
    Task {
        id: 2,
        title: String::from("Review layout"),
    },
]);

let list = List::new({
    let tasks = tasks.clone();
    move || tasks.get()
})
.key(|task| task.id)
.item(|task| {
    Row::new()
        .gap(8.0)
        .child(Text::new(task.title.clone()))
});

let _ = list;
```

The key must be unique among siblings and stable for the lifetime of the item.
Do not use the loop index when the order can change: state would follow the
position rather than the item. `VirtualList` uses the same keyed model for large
collections and adds item-height based virtualization.

## Heterogeneous children

A container's generic children must share a concrete type. Use `AnyView` when
a branch or a dynamic collection produces different view types:

```rust
# use sequoia_ui::prelude::*;
enum Shape {
    Circle,
    Square,
}

let shape = Shape::Circle;
let child: AnyView = match shape {
    Shape::Circle => AnyView::new(Text::new("circle")),
    Shape::Square => AnyView::new(Text::new("square")),
};

let _ = Column::new().child(child);
```

`AnyView` erases the Rust type for storage but preserves the concrete type for
reconciliation. A branch that changes the inner type at the same key remounts
the child.

## Pitfalls

- Console different types from `body` or `child`; wrap them in `AnyView`.
- Do not use a loop index as a key for a mutable list; use a stable ID.
- Keep durable state in signals, not in fields that are recreated by a rebuild.
- Remember that `Text::new` copies a string while `Text::with` subscribes to a
  reactive source; choose based on whether the content changes.

## Next steps

- Control how children share space: [Layout](layout.md).
- Handle pointer and keyboard input: [Events](events.md).
- Build a reusable widget: [Custom views](../guides/custom-views.md).
