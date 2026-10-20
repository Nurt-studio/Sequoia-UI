#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_elements::prelude::*;
use sequoia_reactive::Signal;
use sequoia_test::Harness;

#[test]
fn conditional_children_do_not_need_anyview() {
    let row = Row::new()
        .child(Text::new("a"))
        .child_if(true, || Text::new("b"))
        .child_if(false, || Text::new("x"))
        .child_opt(Some(Text::new("c")))
        .child_opt(None::<Text>)
        .children(Some(Text::new("d")));
    let h = Harness::new(row);
    h.layout(400.0, 200.0);

    let kids = h.children(h.root().unwrap());
    assert_eq!(kids.len(), 4, "a, b, c, d - no AnyView and no if branches");
}

#[test]
fn fit_overrides_fill_size() {
    let c = Container::new()
        .fill_size()
        .fit()
        .child(Panel::new().width(30.0).height(10.0));
    let h = Harness::new(c);
    h.layout(400.0, 400.0);

    let r = h.rect(h.root().unwrap()).unwrap();
    assert_eq!(r.size.width, 30.0, "fit = content size");
    assert_eq!(r.size.height, 10.0);
}

#[test]
fn hit_testable_false_is_pointer_transparent() {
    let opaque = Container::new().width(50.0).height(50.0);
    let h = Harness::new(opaque);
    h.layout(100.0, 100.0);
    assert!(h.hit_test(10.0, 10.0).is_some(), "captures by default");

    let transparent = Container::new()
        .width(50.0)
        .height(50.0)
        .hit_testable(false);
    let h2 = Harness::new(transparent);
    h2.layout(100.0, 100.0);
    assert!(
        h2.hit_test(10.0, 10.0).is_none(),
        "pointer-events: none -> hit falls through"
    );
}

#[test]
fn icon_button_has_square_intrinsic_size() {
    let h = Harness::new(Button::icon("nowyszkic").on_click(|| {}));
    h.layout(200.0, 200.0);

    let r = h.rect(h.root().unwrap()).unwrap();
    assert_eq!(r.size.width, 16.0 + 4.0 * 2.0, "16px icon + 4 padding");
    assert_eq!(r.size.height, 16.0 + 4.0 * 2.0);
}

#[test]
fn ribbon_button_stacks_label_under_icon() {
    let h = Harness::new(Button::ribbon("move", "Move"));
    h.layout(200.0, 200.0);

    let r = h.rect(h.root().unwrap()).unwrap();
    assert!(
        r.size.height >= 28.0 + 2.0 + 14.0,
        "28px icon + gap + text line, got {r:?}"
    );
}

#[derive(Clone)]
struct OverrideProbe {
    flag: Signal<bool>,
}

impl View for OverrideProbe {
    fn body(&self) -> impl View + 'static {
        let _ = self.flag.get();
        Unit
    }

    fn children_override(&self) -> Option<Vec<AnyView>> {
        let mut kids = vec![AnyView::new(Text::new("x"))];
        if self.flag.get_untracked() {
            kids.push(AnyView::new(Text::new("y")));
        }
        Some(kids)
    }
}

#[test]
fn body_reads_are_tracked_even_with_children_override() {
    let flag = Signal::new(false);
    let h = Harness::new(OverrideProbe { flag: flag.clone() });
    h.layout(100.0, 100.0);

    let root = h.root().unwrap();
    assert_eq!(h.children(root).len(), 1, "initial state");

    h.set(&flag, true);
    h.layout(100.0, 100.0);

    assert_eq!(
        h.children(h.root().unwrap()).len(),
        2,
        "odczyt z body() subskrybuje mimo children_override"
    );
}
