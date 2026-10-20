#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_core::{Color, Rect, RenderItem, Vec2};
use sequoia_test::Harness;
use sequoia_view::prelude::*;

const STRIPE: Color = Color::hex(0xCACACA);

#[test]
fn row_border_left_replaces_stripe_container() {
    let h = Harness::new(
        Row::new()
            .width(300.0)
            .height(60.0)
            .border_left(8.5, STRIPE)
            .padding_x(12.0)
            .center()
            .child(
                Panel::new()
                    .background(Color::RED)
                    .width(100.0)
                    .height(20.0),
            ),
    );
    h.layout(400.0, 400.0);

    let row = h.root().unwrap();
    assert_eq!(h.rect(row).unwrap(), Rect::from_xywh(0.0, 0.0, 300.0, 60.0));

    let kids = h.children(row);
    assert_eq!(kids.len(), 1);

    assert_eq!(
        h.rect(kids[0]).unwrap(),
        Rect::from_xywh(104.25, 20.0, 100.0, 20.0)
    );

    let list = h.paint();
    let quads: Vec<_> = list.quads().collect();
    assert_eq!(quads.len(), 2, "bar + content, no Container");
    assert_eq!(quads[0].rect, Rect::from_xywh(0.0, 0.0, 8.5, 60.0));
    assert_eq!(quads[0].color, STRIPE);
    assert_eq!(quads[1].rect, Rect::from_xywh(104.25, 20.0, 100.0, 20.0));
    assert_eq!(quads[1].color, Color::RED);
}

#[test]
fn center_shortcut_centers_child_both_axes() {
    let h = Harness::new(
        Row::new().width(200.0).height(100.0).center().child(
            Panel::new()
                .background(Color::BLUE)
                .width(40.0)
                .height(20.0),
        ),
    );
    h.layout(400.0, 400.0);
    let kids = h.children(h.root().unwrap());
    assert_eq!(kids.len(), 1);
    assert_eq!(
        h.rect(kids[0]).unwrap(),
        Rect::from_xywh(80.0, 40.0, 40.0, 20.0)
    );
}

#[test]
fn plain_layouts_paint_nothing_by_default() {
    let h = Harness::new(Row::new().width(100.0).height(50.0));
    h.layout(400.0, 400.0);
    assert!(h.paint().is_empty(), "bare Row paints nothing");

    let h = Harness::new(Column::new().width(100.0).height(50.0));
    h.layout(400.0, 400.0);
    assert!(h.paint().is_empty(), "bare Column paints nothing");

    let h = Harness::new(Stack::new().width(100.0).height(50.0));
    h.layout(400.0, 400.0);
    assert!(h.paint().is_empty(), "bare Stack paints nothing");

    let h = Harness::new(Grid::new([Track::Px(50.0)]).width(100.0).height(50.0));
    h.layout(400.0, 400.0);
    assert!(h.paint().is_empty(), "bare Grid paints nothing");

    let h = Harness::new(DockArea::new().width(100.0).height(50.0));
    h.layout(400.0, 400.0);
    assert!(h.paint().is_empty(), "bare DockArea paints nothing");
}

#[test]
fn shared_box_style_applies_to_every_layout() {
    let card = BoxStyle::new()
        .background(Color::hex(0x252525))
        .border(2.0, Color::WHITE)
        .radius(8.0);

    let h = Harness::new(
        Row::new()
            .width(120.0)
            .height(80.0)
            .box_style(card)
            .child(Panel::new().width(10.0).height(10.0)),
    );
    h.layout(400.0, 400.0);
    let list = h.paint();
    let qs: Vec<_> = list.quads().collect();
    assert_eq!(qs[0].color, Color::hex(0x252525));
    assert_eq!(qs[0].radius, 8.0);
    assert_eq!(qs[0].border_width, 2.0);
    assert_eq!(qs[0].border_color, Color::WHITE);

    let h = Harness::new(
        Column::new()
            .width(120.0)
            .height(80.0)
            .style(card)
            .child(Panel::new().width(10.0).height(10.0)),
    );
    h.layout(400.0, 400.0);
    let list = h.paint();
    let qs: Vec<_> = list.quads().collect();
    assert_eq!(qs[0].color, Color::hex(0x252525));
    assert_eq!(qs[0].border_width, 2.0);

    let h = Harness::new(
        Stack::new()
            .width(120.0)
            .height(80.0)
            .box_style(card)
            .child(Panel::new().width(10.0).height(10.0)),
    );
    h.layout(400.0, 400.0);
    let list = h.paint();
    let qs: Vec<_> = list.quads().collect();
    assert_eq!(qs[0].color, Color::hex(0x252525));
    assert_eq!(qs[0].border_width, 2.0);

    let h = Harness::new(
        Grid::new([Track::Px(60.0)])
            .width(120.0)
            .height(80.0)
            .box_style(card)
            .child(Panel::new().width(10.0).height(10.0)),
    );
    h.layout(400.0, 400.0);
    let list = h.paint();
    let qs: Vec<_> = list.quads().collect();
    assert_eq!(qs[0].color, Color::hex(0x252525));
    assert_eq!(qs[0].border_width, 2.0);

    let h = Harness::new(
        DockArea::new()
            .width(120.0)
            .height(80.0)
            .box_style(card)
            .fill(Panel::new().width(10.0).height(10.0)),
    );
    h.layout(400.0, 400.0);
    let list = h.paint();
    let qs: Vec<_> = list.quads().collect();
    assert_eq!(qs[0].color, Color::hex(0x252525));
    assert_eq!(qs[0].border_width, 2.0);
}

#[test]
fn box_style_shadow_paints_under_and_inset_over_the_background() {
    let h = Harness::new(
        Row::new()
            .width(120.0)
            .height(80.0)
            .style(
                BoxStyle::new()
                    .background(Color::hex(0x252525))
                    .radius(8.0)
                    .shadow(
                        ShadowSpec::new(
                            Color {
                                a: 0.4,
                                ..Color::BLACK
                            },
                            12.0,
                        )
                        .offset(0.0, 4.0),
                    )
                    .inset_shadow(ShadowSpec::new(
                        Color {
                            a: 0.3,
                            ..Color::BLACK
                        },
                        6.0,
                    )),
            )
            .child(Panel::new().width(10.0).height(10.0)),
    );
    h.layout(400.0, 400.0);
    let list = h.paint();

    let items = list.items();

    let RenderItem::Shadow(outer) = items[0] else {
        panic!("outer shadow first")
    };
    assert!(!outer.inset);
    assert_eq!(outer.rect, Rect::from_xywh(0.0, 0.0, 120.0, 80.0));
    assert_eq!(outer.radius, 8.0, "shadow radius from the box");
    assert_eq!(outer.blur, 12.0);
    assert_eq!(outer.offset, Vec2::new(0.0, 4.0));
    let RenderItem::Quad(bg) = items[1] else {
        panic!("box background above the shadow")
    };
    assert_eq!(bg.color, Color::hex(0x252525));
    let RenderItem::Shadow(inner) = items[2] else {
        panic!("inner shadow last")
    };
    assert!(inner.inset);
    assert_eq!(inner.blur, 6.0);
}

#[test]
fn dock_area_border_bottom_paints_stripe_inside() {
    let h = Harness::new(
        DockArea::new()
            .width(200.0)
            .height(100.0)
            .border_bottom(6.0, Color::GREEN)
            .fill(Panel::new()),
    );
    h.layout(400.0, 400.0);
    let list = h.paint();
    let qs: Vec<_> = list.quads().collect();

    let stripe = qs
        .iter()
        .find(|q| q.color == Color::GREEN)
        .expect("bar at the bottom");
    assert_eq!(stripe.rect, Rect::from_xywh(0.0, 94.0, 200.0, 6.0));
}

#[test]
fn panel_per_side_border_keeps_theme_fallback() {
    let h = Harness::new(
        Panel::new()
            .border_left(4.0, Color::RED)
            .width(100.0)
            .height(60.0),
    );
    h.layout(400.0, 400.0);
    let list = h.paint();
    let qs: Vec<_> = list.quads().collect();
    assert_eq!(qs.len(), 2);

    assert_eq!(
        qs[0].color,
        sequoia_style::Theme::dark().surface,
        "background from Theme::dark surface"
    );
    assert_eq!(qs[0].rect, Rect::from_xywh(0.0, 0.0, 100.0, 60.0));
    assert_eq!(qs[1].rect, Rect::from_xywh(0.0, 0.0, 4.0, 60.0));
    assert_eq!(qs[1].color, Color::RED);
}
