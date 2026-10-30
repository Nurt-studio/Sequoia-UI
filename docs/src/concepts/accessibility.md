# Accessibility

Sequoia connects accessibility to the two things it already tracks: the view
tree and focus. Any view can describe itself with an `Accessible` value made of
a role and a label, and after layout the runtime derives an accessibility tree
that a platform adapter can hand to assistive technology. Keyboard focus,
activation, and focus traps are routed by the same runtime that handles pointer
input, so a control that works with a mouse has a path to work with a keyboard.

## Semantic tree

Semantics live on the view, not in a parallel registry. A view reports an
`Accessible` through its `accessibility` method:

```rust
# use sequoia_ui::prelude::*;
let save = Accessible::new(AccessibleRole::Button, "Save");
assert_eq!(save.role, AccessibleRole::Button);
assert_eq!(save.label, "Save");
```

The runtime walks the mounted element tree after layout and returns
`AccessibleNode` values from `Runtime::accessibility_tree`:

```text
pub struct AccessibleNode {
    pub element: ElementId,
    pub role: AccessibleRole,
    pub label: String,
    pub children: Vec<AccessibleNode>,
}
```

Each node keeps the `ElementId` of the view that produced it, which lets an
adapter correlate a semantic node with the same element used for hit testing and
focus. `AccessibleNode::flat_labels` flattens a subtree into labels, which is
handy in tests.

`AccessibleRole` is `#[non_exhaustive]`; match it with a wildcard arm:

```rust
# use sequoia_ui::prelude::*;
fn announce(role: AccessibleRole) -> &'static str {
    match role {
        AccessibleRole::Button => "button",
        AccessibleRole::Checkbox => "checkbox",
        AccessibleRole::Dialog => "dialog",
        _ => "element",
    }
}
assert_eq!(announce(AccessibleRole::Button), "button");
```

The variants are `Button`, `Text`, `TextField`, `Checkbox`, `Switch`, `Slider`,
`List`, `ListItem`, `Table`, `Tree`, `TreeItem`, `Dialog`, `Menu`, `MenuItem`,
`Progress`, `Image`, `Group`, `Radio`, and `RadioGroup`.

## What built-in widgets expose

The standard widgets already carry semantics, so wrapping them in layout does
not require extra work:

| Widget | Role | Label |
| --- | --- | --- |
| `Button` | `Button` | button label |
| `Checkbox` | `Checkbox` | checkbox label |
| `Radio` | `Radio` | radio label |
| `RadioGroup` | `RadioGroup` | group label |
| `Switch` | `Switch` | switch label |
| `Slider` | `Slider` | slider label |
| `TextField` | `TextField` | placeholder, or a generic fallback |
| `Text` | `Text` | text content |
| `Dialog` | `Dialog` | dialog title |
| `Table` | `Table` | table label |
| `Tree` | `Tree` | tree label |
| `ColorPicker` | `Group` | "Color picker" |

Layout-only views such as `Row`, `Column`, `Container`, `Spacer`, and `Padding`
report no semantics and are skipped when the tree is built. The accessibility
tree therefore mirrors what a user perceives, not how the layout happens to be
nested.

## Adding semantics to a custom view

A custom control implements `View::accessibility` and returns a role and label.
For a leaf control, also make the view a leaf and paint it as usual:

```rust
# use sequoia_ui::prelude::*;

struct RatingStars {
    stars: u8,
}

impl View for RatingStars {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn is_leaf(&self) -> bool {
        true
    }

    fn accessibility(&self) -> Option<Accessible> {
        Some(Accessible::new(
            AccessibleRole::Slider,
            format!("Rating: {} of 5", self.stars),
        ))
    }

    fn paint_self(&self, ctx: &mut PaintCtx) {
        ctx.quad(ctx.rect, ctx.theme().surface, 4.0);
    }
}

let rating = RatingStars { stars: 4 };
assert_eq!(
    rating.accessibility().unwrap().label,
    "Rating: 4 of 5"
);
```

The role should describe the behavior, not the appearance. A star row that
changes a value is a `Slider`; the same row used only to display an average is
`Text` or `Image`. Composite views that add a label around children should
return `Group`, so readers announce the grouping without losing the children.

## Labels

The tree intentionally stays small: a node carries a role, a label, and its
children. There is no separate checked, expanded, or value field yet, so encode
anything essential in the label itself:

```rust
# use sequoia_ui::prelude::*;
let notifications = Signal::new(true);
let label = if notifications.get() {
    "Notifications: on"
} else {
    "Notifications: off"
};
let toggle = Accessible::new(AccessibleRole::Switch, label);
assert_eq!(toggle.label, "Notifications: on");
```

Keep labels short and free of role words: "Save" is better than "Save button",
because the role is announced separately. Reuse the visible text when there is
one instead of repeating a string. Labels are rebuilt with the view, so a label
derived from a signal updates the next time the tree is queried.

## Focus and keyboard

Only interactive views participate in keyboard focus. That includes `Button`,
`Checkbox`, `Radio`, `Switch`, `Slider`, `TextField`, `TextArea`, `Dropdown`,
focusable `Canvas` views, and focusable `Surface` views. A custom view joins the
cycle by overriding `View::focusable`:

```rust
# use sequoia_ui::prelude::*;

struct Palette;

impl View for Palette {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn is_leaf(&self) -> bool {
        true
    }

    fn focusable(&self) -> bool {
        true
    }
}

assert!(Palette.focusable());
```

The runtime handles Tab and Shift+Tab globally: it walks the focusable views,
skips disabled ones, and moves focus to the next or previous element. Clicking a
descendant moves focus to its nearest focusable ancestor. Focused buttons and
toggles fire on Enter and Space; `TextArea` opts into Tab as a text input instead
of focus navigation.

The focus ring is a view wrapper, not a paint flag:

```rust
# use sequoia_ui::prelude::*;
let submit = FocusRing::wrap(Button::new("Submit"))
    .reserve_space(true)
    .width(2.0);
```

`reserve_space` keeps the layout stable whether the ring is visible or not, and
`width`, `offset`, `color`, and `style` override the theme defaults. Wrap the
control, not the container: the ring is drawn around the focused element, and a
ring on a large container will highlight the whole region.

To keep keyboard focus inside a modal region, use `FocusScope` with a trap.
`Dialog` already does this internally and also closes on Escape:

```rust
# use sequoia_ui::prelude::*;
let name = Signal::new(String::new());
let form = FocusScope::new().trap(true).child(
    Column::new()
        .child(TextField::new(name).placeholder("Name"))
        .child(FocusRing::wrap(Button::new("Continue")).reserve_space(true)),
);
```

When a scope with `trap(true)` contains the focused element, Tab and Shift+Tab
cycle only through that scope. If focus leaves the scope, trapping stops. This is
the same mechanism that keeps `Dialog` and `Popper` content reachable without
tabbing into the page behind them.

## Reduced motion

The runtime exposes a reduced-motion switch. When it is on, animations collapse
to their final state instead of playing:

- animated values driven by `Live::animate` and `Live::spring` jump to the
  target;
- enter, exit, and presence motions complete immediately;
- `Pulse`, `Shake`, and other `Animated` wrappers paint their resting state;
- an `Animator` finishes its current clip instead of advancing it frame by
  frame.

```rust
# use sequoia_ui::prelude::*;
use sequoia_ui::view::Runtime;

let rt = Runtime::new();
assert!(!rt.reduced_motion());

rt.set_reduced_motion(true);
assert!(rt.reduced_motion());
```

Applications that expose a preference toggle can call `set_reduced_motion` on
the runtime, and embedding code can do the same from a settings screen. This is
the switch to use for a "reduce motion" option; there is no per-view opt-out.

## Testing the tree

The test harness mounts a root, lays it out, and exposes the same runtime used
in production. That makes the accessibility tree directly assertable:

```rust
# use sequoia_ui::prelude::*;
# use sequoia_test::Harness;
let h = Harness::new(
    Column::new()
        .child(Text::new("Name"))
        .child(Text::new("Ada")),
);
h.sync(320.0, 240.0);

let tree = h.runtime().accessibility_tree();
let labels: Vec<String> = tree
    .iter()
    .flat_map(AccessibleNode::flat_labels)
    .collect();
assert!(labels.iter().any(|label| label == "Ada"));
```

`AccessibleNode` implements `PartialEq`, so a test can compare the whole tree
when the structure matters, or search it by role when only one control is under
test.

## Guidelines

- Give every custom interactive view a role and a label. A control that is
  invisible to the tree is invisible to assistive technology.
- Keep semantics on the control, not on the layout wrapper. Do not add a
  `Group` around every row.
- Preserve the label when a control is disabled; disabled is conveyed by focus
  behavior, not by removing the label.
- Make the control focusable if it responds to the keyboard, and wrap it in a
  `FocusRing` so sighted keyboard users can see where focus is.
- Use a focus trap only for modal content. A trap on ordinary content makes the
  rest of the window unreachable.
- Test the labels and roles the same way you test clicks. The harness can assert
  both.
