#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_elements::Button;
use sequoia_test::Harness;
use sequoia_view::prelude::*;

#[test]
fn fit_text_keeps_line_height_on_main_axis_deficit() {
    let h = Harness::new(
        Column::new()
            .width(200.0)
            .height(30.0)
            .child(Text::new("Hello").size(14.0))
            .child(Text::new("World").size(14.0)),
    );
    h.layout(400.0, 400.0);
    let kids = h.children(h.root().unwrap());
    assert_eq!(kids.len(), 2);
    for k in kids {
        let r = h.rect(k).unwrap();
        assert!(
            r.size.height >= 18.0,
            "Fit Text must not shrink below line height 18.9, got {r:?}"
        );
    }
}

#[test]
fn button_keeps_intrinsic_height_so_background_matches_text() {
    let h = Harness::new(
        Column::new()
            .width(200.0)
            .height(40.0)
            .child(Button::labeled("OK"))
            .child(Button::labeled("Cancel")),
    );
    h.layout(400.0, 400.0);
    let kids = h.children(h.root().unwrap());
    assert_eq!(kids.len(), 2);
    for k in kids {
        let r = h.rect(k).unwrap();
        assert!(
            r.size.height >= 26.0,
            "Button must not become a thin strip, got {r:?}"
        );
    }
}

#[test]
fn fit_text_keeps_height_on_cross_axis_deficit() {
    let h = Harness::new(
        Row::new()
            .width(200.0)
            .height(10.0)
            .child(Text::new("Hello").size(14.0)),
    );
    h.layout(400.0, 400.0);
    let kid = h.children(h.root().unwrap())[0];
    let r = h.rect(kid).unwrap();
    assert!(
        r.size.height >= 18.0,
        "Cross-axis Fit must not squash below intrinsic, got {r:?}"
    );
}

#[test]
fn flex_child_absorbs_deficit_while_fixed_chrome_keeps_size() {
    let h = Harness::new(
        Column::new()
            .width(200.0)
            .height(100.0)
            .child(Text::new("Header").size(14.0))
            .child(Panel::new().fill_width().flex(1.0))
            .child(Text::new("Footer").size(14.0)),
    );
    h.layout(400.0, 400.0);
    let kids = h.children(h.root().unwrap());
    assert_eq!(kids.len(), 3);
    let header = h.rect(kids[0]).unwrap();
    let footer = h.rect(kids[2]).unwrap();
    assert!(
        header.size.height >= 18.0,
        "header keeps line height: {header:?}"
    );
    assert!(
        footer.size.height >= 18.0,
        "footer keeps line height: {footer:?}"
    );
}
