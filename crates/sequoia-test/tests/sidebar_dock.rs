use sequoia_core::Color;
use sequoia_elements::Container;
use sequoia_test::Harness;
use sequoia_view::prelude::*;

fn bar(h: f32) -> Container {
    Container::new()
        .fill_width()
        .height(h)
        .background(Color::hex(0x2C2C30))
}

fn sidebar() -> DockArea {
    DockArea::new()
        .width(268.0)
        .fill_height()
        .top(bar(150.0))
        .bottom(bar(80.0))
        .fill(Column::new().child(bar(20.0)).child(bar(20.0)))
}

fn kids(h: &Harness) -> Vec<ElementId> {
    let root = h.root().expect("root");
    h.children(root)
}

fn slots(h: &Harness) -> (sequoia_core::Rect, sequoia_core::Rect, sequoia_core::Rect) {
    let k = kids(h);
    assert_eq!(k.len(), 3, "top + bottom + fill");
    (
        h.rect(k[0]).expect("Mountain"),
        h.rect(k[2]).expect("middle"),
        h.rect(k[1]).expect("stopka"),
    )
}

#[test]
fn footer_is_pinned_to_bottom() {
    let h = Harness::new(sidebar());
    h.layout(268.0, 800.0);
    h.paint();
    h.tick();
    let (_, _, bottom) = slots(&h);
    assert!(
        (bottom.bottom() - 800.0).abs() < 1.0,
        "stopka na samym dole: {:?}",
        bottom
    );
    assert!(
        (bottom.size.height - 80.0).abs() < 1.0,
        "footer height: {:?}",
        bottom
    );
}

#[test]
fn footer_does_not_overlap_middle() {
    let h = Harness::new(sidebar());
    h.layout(268.0, 800.0);
    h.paint();
    h.tick();
    let (_, mid, bottom) = slots(&h);
    assert!(
        mid.bottom() <= bottom.top() + 1.0,
        "center does not go to footer: middle {:?}, footer {:?}",
        mid,
        bottom
    );
}

#[test]
fn fill_before_bottom_still_pins_footer() {
    let area = DockArea::new()
        .width(268.0)
        .fill_height()
        .top(bar(150.0))
        .fill(Column::new().child(bar(20.0)).child(bar(20.0)))
        .bottom(bar(80.0));
    let h = Harness::new(area);
    h.layout(268.0, 800.0);
    h.paint();
    h.tick();
    let root = h.root().expect("root");
    let k = h.children(root);
    let bottom = h.rect(k[2]).expect("stopka");
    let mid = h.rect(k[1]).expect("middle");
    assert!(
        (bottom.bottom() - 800.0).abs() < 1.0,
        "stopka na dole mimo deklaracji po Fill: {:?}",
        bottom
    );
    assert!(
        mid.bottom() <= bottom.top() + 1.0,
        "center does not go to the footer: {:?} vs {:?}",
        mid,
        bottom
    );
}

#[test]
fn legacy_ordered_fill_not_last_starves_footer() {
    let bad = DockArea::new()
        .width(268.0)
        .fill_height()
        .ordered(true)
        .last_child_fill(false)
        .top(bar(150.0))
        .fill(Column::new().child(bar(20.0)).child(bar(20.0)))
        .bottom(bar(80.0));
    let h = Harness::new(bad);
    h.layout(268.0, 800.0);
    h.paint();
    h.tick();
    let root = h.root().expect("root");
    let k = h.children(root);
    let mid = h.rect(k[1]).expect("middle");
    let bottom = h.rect(k[2]).expect("stopka");
    assert!(
        (bottom.top() - mid.top()).abs() < 1.0,
        "ordered: footer at start of center {:?} vs {:?}",
        bottom,
        mid
    );
}
