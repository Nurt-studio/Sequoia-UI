#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_core::{Color, Length, Rect};
use sequoia_elements::Container;
use sequoia_test::Harness;
use sequoia_view::prelude::*;

#[test]
fn paints_background_and_insets_child_by_padding() {
    let h = Harness::new(
        Container::new()
            .background(Color::RED)
            .padding(10.0)
            .width(200.0)
            .height(100.0)
            .child(
                Panel::new()
                    .background(Color::BLUE)
                    .width(Length::Fill(1.0))
                    .height(Length::Fill(1.0)),
            ),
    );
    h.layout(400.0, 400.0);
    let list = h.paint();

    let quads: Vec<_> = list.quads().collect();
    assert_eq!(quads.len(), 2);
    assert_eq!(quads[0].color, Color::RED);
    assert_eq!(quads[0].rect, Rect::from_xywh(0.0, 0.0, 200.0, 100.0));

    assert_eq!(quads[1].color, Color::BLUE);
    assert_eq!(quads[1].rect, Rect::from_xywh(10.0, 10.0, 180.0, 80.0));

    let root = h.root().unwrap();
    let kids = h.children(root);
    assert_eq!(kids.len(), 1);
    assert_eq!(
        h.rect(kids[0]).unwrap(),
        Rect::from_xywh(10.0, 10.0, 180.0, 80.0)
    );
}

#[test]
fn border_paints_ring_over_background() {
    let h = Harness::new(
        Container::new()
            .background(Color::BLUE)
            .border(2.0, Color::WHITE)
            .radius(8.0)
            .width(100.0)
            .height(60.0),
    );
    h.layout(400.0, 400.0);
    let list = h.paint();

    let quads: Vec<_> = list.quads().collect();
    assert_eq!(quads.len(), 1, "background + border in one bordered quad");
    assert_eq!(quads[0].color, Color::BLUE);
    assert_eq!(quads[0].rect, Rect::from_xywh(0.0, 0.0, 100.0, 60.0));
    assert_eq!(quads[0].radius, 8.0);
    assert_eq!(quads[0].border_color, Color::WHITE);
    assert_eq!(quads[0].border_width, 2.0);
    assert!(quads[0].has_border());
}

#[test]
fn translucent_background_keeps_border_out_of_fill() {
    let bg = Color::rgba(154. / 255., 214. / 255., 248. / 255., 24. / 255.);
    let h = Harness::new(
        Container::new()
            .background(bg)
            .border(1.0, Color::RED)
            .width(120.0)
            .height(70.0),
    );
    h.layout(400.0, 400.0);
    let list = h.paint();

    let quads: Vec<_> = list.quads().collect();
    assert_eq!(quads.len(), 1, "one quad instead of a two-quad ring");
    assert_eq!(
        quads[0].color, bg,
        "interior is the background, not the border"
    );
    assert_eq!(quads[0].rect, Rect::from_xywh(0.0, 0.0, 120.0, 70.0));
    assert_eq!(quads[0].border_color, Color::RED);
    assert_eq!(quads[0].border_width, 1.0);
}

#[test]
fn transparent_background_with_border_paints_ring() {
    let h = Harness::new(
        Container::new()
            .border(2.0, Color::GREEN)
            .radius(8.0)
            .width(100.0)
            .height(60.0),
    );
    h.layout(400.0, 400.0);
    let list = h.paint();

    let quads: Vec<_> = list.quads().collect();
    assert_eq!(quads.len(), 1, "ring alone in one quad");
    assert_eq!(quads[0].color, Color::TRANSPARENT);
    assert_eq!(quads[0].rect, Rect::from_xywh(0.0, 0.0, 100.0, 60.0));
    assert_eq!(quads[0].radius, 8.0);
    assert_eq!(quads[0].border_color, Color::GREEN);
    assert_eq!(quads[0].border_width, 2.0);
}
#[test]
fn empty_transparent_container_paints_nothing() {
    let h = Harness::new(Container::new().width(50.0).height(50.0));
    h.layout(400.0, 400.0);
    let list = h.paint();
    assert!(list.is_empty());

    let kids = h.children(h.root().unwrap());
    assert_eq!(kids.len(), 1);
    assert!(h.type_name_of(kids[0]).unwrap().contains("Unit"));
}

#[test]
fn single_child_keeps_explicit_size_under_plain_composite() {
    struct Wrap;
    impl View for Wrap {
        fn body(&self) -> impl View + 'static {
            Container::new()
                .background(Color::RED)
                .width(80.0)
                .height(40.0)
                .child(Panel::new().background(Color::BLUE))
        }
    }
    let h = Harness::new(Wrap);
    h.layout(400.0, 400.0);
    let kids = h.children(h.root().unwrap());
    assert_eq!(kids.len(), 1);
    assert_eq!(
        h.rect(kids[0]).unwrap(),
        Rect::from_xywh(0.0, 0.0, 80.0, 40.0)
    );
}

#[test]
fn fit_container_hugs_text_child() {
    struct Wrap;
    impl View for Wrap {
        fn body(&self) -> impl View + 'static {
            Container::new()
                .background(Color::RED)
                .child(Text::new("Hello World").size(16.0).color(Color::WHITE))
        }
    }
    let h = Harness::new(Wrap);
    h.layout(800.0, 600.0);
    let kids = h.children(h.root().unwrap());
    assert_eq!(kids.len(), 1);
    let r = h.rect(kids[0]).unwrap();
    assert!(
        r.size.width > 0.0 && r.size.width < 800.0,
        "hugs text width, got {r:?}"
    );
    assert!(
        r.size.height > 0.0 && r.size.height < 600.0,
        "hugs text height, got {r:?}"
    );
}

#[test]
fn rounded_clip_cuts_header_to_card_shape() {
    struct Card;
    impl View for Card {
        fn body(&self) -> impl View + 'static {
            Container::new()
                .background(Color::RED)
                .radius(8.0)
                .width(240.0)
                .height(280.0)
                .clip(true)
                .child(
                    Column::new().child(
                        Row::new()
                            .background(Color::GREEN)
                            .fill_width()
                            .child(Text::new("Settings")),
                    ),
                )
        }
    }
    let h = Harness::new(Card);
    h.layout(400.0, 400.0);
    let list = h.paint();
    let quads: Vec<_> = list.quads().collect();
    assert_eq!(quads.len(), 2);
    let (red, green) = (quads[0], quads[1]);
    assert_eq!(red.color, Color::RED);
    assert_eq!(red.radius, 8.0);
    assert_eq!(green.color, Color::GREEN);

    assert_eq!(green.rect.top(), red.rect.top());
    assert_eq!(green.rect.left(), red.rect.left());
    assert_eq!(green.rect.size.width, red.rect.size.width);
    assert_eq!(green.radius, 0.0);

    let green_clip = list.item_rounded_clip(1).expect("header clipped");
    assert_eq!(green_clip.rect, red.rect);
    assert_eq!(green_clip.radius, 8.0);
}
