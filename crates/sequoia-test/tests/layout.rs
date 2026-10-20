#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_core::{Color, Length, LengthExt};
use sequoia_test::Harness;
use sequoia_view::prelude::*;

#[test]
fn row_fill_weights_split_proportionally() {
    struct Demo;
    impl View for Demo {
        fn body(&self) -> impl View + 'static {
            Row::new()
                .child(Panel::new().width(1.0.fill()).height(100.0))
                .child(Panel::new().width(3.0.fill()).height(100.0))
        }
    }

    let h = Harness::new(Demo);
    h.layout(400.0, 100.0);
    let row = h.children(h.root().unwrap())[0];
    let kids = h.children(row);
    assert_eq!(kids.len(), 2);
    let a = h.rect(kids[0]).unwrap();
    let b = h.rect(kids[1]).unwrap();
    assert!((a.size.width - 100.0).abs() < 0.01, "weight 1/4 = 100px");
    assert!((b.size.width - 300.0).abs() < 0.01, "weight 3/4 = 300px");
    assert!((b.left() - (a.left() + 100.0)).abs() < 0.01);
}

#[test]
fn stack_overlays_children_on_same_rect_in_order() {
    let h = Harness::new(
        Stack::new()
            .width(200.0)
            .height(100.0)
            .child_at(Panel::new().background(Color::RED), StackPos::fill())
            .child_at(Panel::new().background(Color::BLUE), StackPos::fill()),
    );
    h.layout(400.0, 400.0);
    let stack = h.root().unwrap();
    let kids = h.children(stack);
    assert_eq!(kids.len(), 2);
    let a = h.rect(kids[0]).unwrap();
    let b = h.rect(kids[1]).unwrap();
    assert_eq!(a, b, "layers on the same rect");
    assert_eq!(a, sequoia_core::Rect::from_xywh(0.0, 0.0, 200.0, 100.0));

    let list = h.paint();
    let quads: Vec<_> = list.quads().collect();
    assert_eq!(quads.len(), 2, "both layers paint");
    assert_eq!(quads[0].color, Color::RED);
    assert_eq!(quads[1].color, Color::BLUE);
}

#[test]
fn stack_respects_fixed_child_size_and_centers_it() {
    let h = Harness::new(
        Stack::new()
            .width(200.0)
            .height(100.0)
            .child_at(Panel::new().background(Color::RED), StackPos::fill())
            .child(
                Panel::new()
                    .background(Color::GREEN)
                    .width(200.0)
                    .height(40.0),
            ),
    );
    h.layout(400.0, 400.0);
    let stack = h.root().unwrap();
    let kids = h.children(stack);
    assert_eq!(kids.len(), 2);
    let bg = h.rect(kids[0]).unwrap();
    let bar = h.rect(kids[1]).unwrap();
    assert_eq!(
        bg,
        sequoia_core::Rect::from_xywh(0.0, 0.0, 200.0, 100.0),
        "Fill/Fill background on full rect"
    );
    assert_eq!(
        bar,
        sequoia_core::Rect::from_xywh(0.0, 30.0, 200.0, 40.0),
        "40px bar, full width, vertically centered"
    );

    let list = h.paint();
    let quads: Vec<_> = list.quads().collect();
    assert_eq!(quads.len(), 2, "both layers paint");
    assert_eq!(quads[0].color, Color::RED);
    assert_eq!(quads[1].color, Color::GREEN);
    assert_eq!(quads[1].rect, bar, "green only inside the bar rect");
}

#[test]
fn stack_pins_top_bar_like_css_absolute() {
    let h = Harness::new(
        Stack::new()
            .width(200.0)
            .height(100.0)
            .child_at(Panel::new().background(Color::RED), StackPos::fill())
            .child_at(
                Panel::new()
                    .background(Color::GREEN)
                    .width(200.0)
                    .height(40.0),
                StackPos::top(),
            ),
    );
    h.layout(400.0, 400.0);
    let kids = h.children(h.root().unwrap());
    assert_eq!(kids.len(), 2);
    assert_eq!(
        h.rect(kids[0]).unwrap(),
        sequoia_core::Rect::from_xywh(0.0, 0.0, 200.0, 100.0),
        "background on full rect"
    );
    assert_eq!(
        h.rect(kids[1]).unwrap(),
        sequoia_core::Rect::from_xywh(0.0, 0.0, 200.0, 40.0),
        "bar pinned to the top"
    );
}

#[test]
fn stack_pins_corners_and_stretches_between_pins() {
    let h = Harness::new(
        Stack::new()
            .width(200.0)
            .height(100.0)
            .child(Panel::new().background(Color::RED))
            .child_at(
                Panel::new()
                    .background(Color::BLUE)
                    .width(20.0)
                    .height(20.0),
                StackPos::bottom_right(),
            )
            .child_at(
                Panel::new().background(Color::GREEN),
                StackPos {
                    left: Some(10.0),
                    right: Some(10.0),
                    top: Some(5.0),
                    bottom: Some(5.0),
                    ..Default::default()
                },
            ),
    );
    h.layout(400.0, 400.0);
    let kids = h.children(h.root().unwrap());
    assert_eq!(kids.len(), 3);
    assert_eq!(
        h.rect(kids[1]).unwrap(),
        sequoia_core::Rect::from_xywh(180.0, 80.0, 20.0, 20.0),
        "badge in bottom-right corner"
    );
    assert_eq!(
        h.rect(kids[2]).unwrap(),
        sequoia_core::Rect::from_xywh(10.0, 5.0, 180.0, 90.0),
        "two opposing pins stretch (style ignored)"
    );
}

#[test]
fn stack_negative_offset_overflows_like_relative() {
    let h = Harness::new(
        Stack::new()
            .width(200.0)
            .height(100.0)
            .child(Panel::new().background(Color::RED))
            .child_at(
                Panel::new()
                    .background(Color::BLUE)
                    .width(20.0)
                    .height(20.0),
                StackPos::top_right().offset(-6.0, -6.0),
            ),
    );
    h.layout(400.0, 400.0);
    let kids = h.children(h.root().unwrap());
    assert_eq!(kids.len(), 2);
    assert_eq!(
        h.rect(kids[1]).unwrap(),
        sequoia_core::Rect::from_xywh(174.0, -6.0, 20.0, 20.0),
        "negative offset pushes past the corner"
    );
}

#[test]
fn stack_without_z_keeps_declaration_order() {
    let h = Harness::new(
        Stack::new()
            .width(100.0)
            .height(100.0)
            .child_at(Panel::new().background(Color::RED), StackPos::fill())
            .child_at(Panel::new().background(Color::GREEN), StackPos::fill())
            .child_at(Panel::new().background(Color::BLUE), StackPos::fill().z(0)),
    );
    h.layout(200.0, 200.0);
    let list = h.paint();
    let colors: Vec<Color> = list.quads().map(|q| q.color).collect();
    assert_eq!(
        colors,
        vec![Color::RED, Color::GREEN, Color::BLUE],
        "no z means declaration order as always"
    );
}

#[test]
fn stack_z_reorders_paint_and_hit_test() {
    let h = Harness::new(
        Stack::new()
            .width(100.0)
            .height(100.0)
            .child_at(Panel::new().background(Color::RED), StackPos::fill().z(10))
            .child_at(
                Panel::new().background(Color::GREEN),
                StackPos::fill().z(-5),
            )
            .child_at(Panel::new().background(Color::BLUE), StackPos::fill()),
    );
    h.layout(200.0, 200.0);

    let list = h.paint();
    let colors: Vec<Color> = list.quads().map(|q| q.color).collect();
    assert_eq!(
        colors,
        vec![Color::GREEN, Color::BLUE, Color::RED],
        "ascending by z: -5, 0, 10 - RED declared first but on top"
    );

    let kids = h.children(h.root().unwrap());
    assert_eq!(kids.len(), 3);
    let hit = h.hit_test(50.0, 50.0).expect("hit in the center");
    assert_eq!(hit, kids[2], "click hits RED (highest z)");

    let full = sequoia_core::Rect::from_xywh(0.0, 0.0, 100.0, 100.0);
    for &k in &kids {
        assert_eq!(h.rect(k).unwrap(), full, "z does not move geometry");
    }
}

#[test]
fn stack_z_ties_fall_back_to_declaration_order() {
    let h = Harness::new(
        Stack::new()
            .width(100.0)
            .height(100.0)
            .child_z(
                Panel::new()
                    .background(Color::RED)
                    .width(100.0)
                    .height(100.0),
                3,
            )
            .child_z(
                Panel::new()
                    .background(Color::GREEN)
                    .width(100.0)
                    .height(100.0),
                1,
            )
            .child_z(
                Panel::new()
                    .background(Color::BLUE)
                    .width(100.0)
                    .height(100.0),
                3,
            ),
    );
    h.layout(200.0, 200.0);
    let list = h.paint();
    let colors: Vec<Color> = list.quads().map(|q| q.color).collect();
    assert_eq!(
        colors,
        vec![Color::GREEN, Color::RED, Color::BLUE],
        "z=3 stable: RED before BLUE (declaration order)"
    );
}

#[test]
fn fill_container_in_unbounded_height_sums_content() {
    struct Bar {
        h: f32,
    }
    impl View for Bar {
        fn body(&self) -> impl View + 'static {
            Unit
        }
        fn is_leaf(&self) -> bool {
            true
        }
        fn layout_style(&self) -> LayoutStyle {
            LayoutStyle {
                width: sequoia_core::Length::Fill(1.0),
                height: sequoia_core::Length::Px(self.h),
                ..LayoutStyle::default()
            }
        }
    }

    struct Demo;
    impl View for Demo {
        fn body(&self) -> impl View + 'static {
            ScrollView::new(
                Column::new()
                    .child(Bar { h: 30.0 })
                    .child(Column::new().child(Bar { h: 40.0 }).child(Bar { h: 50.0 })),
            )
        }
    }

    let h = Harness::new(Demo);
    h.layout(300.0, 200.0);
    let scroll_el = h.children(h.root().unwrap())[0];
    let scroll = h.view::<ScrollView>(scroll_el).unwrap();

    assert_eq!(scroll.content_height(&h.runtime(), scroll_el), 120.0);
    let col = h.children(scroll_el)[0];
    let col_rect = h.rect(col).unwrap();
    assert!(col_rect.size.height >= 120.0);
}

#[test]
fn row_defaults_to_shrink_wrap() {
    let h = Harness::new(
        Column::new()
            .width(Length::Fill(1.0))
            .height(Length::Fill(1.0))
            .child(
                Row::new()
                    .child(Panel::new().width(50.0).height(20.0))
                    .child(Panel::new().width(70.0).height(20.0)),
            ),
    );
    h.layout(400.0, 300.0);
    let root = h.root().unwrap();
    let row = h.children(root)[0];
    let r = h.rect(row).unwrap();
    assert!(
        (r.size.width - 120.0).abs() < 0.01,
        "width = children sum: {r:?}"
    );
    assert!(
        (r.size.height - 20.0).abs() < 0.01,
        "height = content: {r:?}"
    );
}

#[test]
fn space_between_pushes_first_and_last_to_edges() {
    let h = Harness::new(
        Row::new()
            .width(400.0)
            .height(40.0)
            .justify_content(Justify::SpaceBetween)
            .child(Panel::new().width(60.0).height(40.0))
            .child(Panel::new().width(80.0).height(40.0)),
    );
    h.layout(400.0, 100.0);
    let root = h.root().unwrap();
    let kids = h.children(root);
    let a = h.rect(kids[0]).unwrap();
    let b = h.rect(kids[1]).unwrap();
    assert!((a.left() - 0.0).abs() < 0.01, "first at start: {a:?}");
    assert!((b.right() - 400.0).abs() < 0.01, "last at the end: {b:?}");
}

#[test]
fn justify_center_and_end_shift_children() {
    fn row_with(justify: Justify) -> Harness {
        Harness::new(
            Row::new()
                .width(300.0)
                .height(30.0)
                .justify_content(justify)
                .child(Panel::new().width(100.0).height(30.0)),
        )
    }
    let h = row_with(Justify::Center);
    h.layout(300.0, 100.0);
    let kid = h.children(h.root().unwrap())[0];
    let r = h.rect(kid).unwrap();
    assert!((r.left() - 100.0).abs() < 0.01, "centering: {r:?}");

    let h = row_with(Justify::End);
    h.layout(300.0, 100.0);
    let kid = h.children(h.root().unwrap())[0];
    let r = h.rect(kid).unwrap();
    assert!((r.left() - 200.0).abs() < 0.01, "to the end: {r:?}");
}

#[test]
fn align_center_offsets_child_cross() {
    let h = Harness::new(
        Row::new()
            .width(300.0)
            .height(100.0)
            .align_items(Align::Center)
            .child(Panel::new().width(50.0).height(20.0)),
    );
    h.layout(300.0, 100.0);
    let kid = h.children(h.root().unwrap())[0];
    let r = h.rect(kid).unwrap();
    assert!((r.top() - 40.0).abs() < 0.01, "vertical centering: {r:?}");
}

#[test]
fn align_stretch_expands_auto_child() {
    struct StretchMe;
    impl View for StretchMe {
        fn body(&self) -> impl View + 'static {
            Unit
        }
        fn is_leaf(&self) -> bool {
            true
        }
        fn layout_style(&self) -> LayoutStyle {
            LayoutStyle {
                width: Length::Px(60.0),
                height: Length::Fit,
                ..LayoutStyle::default()
            }
        }
    }

    let h = Harness::new(Row::new().width(300.0).height(100.0).child(StretchMe));
    h.layout(300.0, 100.0);
    let kid = h.children(h.root().unwrap())[0];
    assert_eq!(h.rect(kid).unwrap().size.height, 0.0);

    let h = Harness::new(
        Row::new()
            .width(300.0)
            .height(100.0)
            .align_items(Align::Stretch)
            .child(StretchMe),
    );
    h.layout(300.0, 100.0);
    let kid = h.children(h.root().unwrap())[0];
    let r = h.rect(kid).unwrap();
    assert!((r.size.height - 100.0).abs() < 0.01, "stretched: {r:?}");
}

#[test]
fn overflow_comes_from_fill_child_not_from_fixed_chrome() {
    let mut tall = Column::new();
    for _ in 0..40 {
        tall = tall.child(Panel::new().width(1.0.fill()).height(30.0));
    }

    let h = Harness::new(
        Column::new()
            .width(Length::Fill(1.0))
            .height(Length::Fill(1.0))
            .child(Panel::new().width(1.0.fill()).height(27.0))
            .child(tall.height(Length::Fill(1.0)))
            .child(Panel::new().width(1.0.fill()).height(178.0))
            .child(Panel::new().width(1.0.fill()).height(28.0)),
    );
    h.layout(400.0, 300.0);

    let root = h.root().unwrap();
    let kids = h.children(root);
    let bar = |i: usize| h.rect(kids[i]).unwrap();
    assert_eq!(bar(0).size.height, 27.0, "menubar: {:?}", bar(0));
    assert_eq!(bar(0).top(), 0.0);
    assert_eq!(bar(2).size.height, 178.0, "command line: {:?}", bar(2));
    assert_eq!(bar(3).size.height, 28.0, "status bar: {:?}", bar(3));
    assert!(
        (bar(3).bottom() - 300.0).abs() < 0.01,
        "status bar at the bottom: {:?}",
        bar(3)
    );

    assert!(
        (bar(1).size.height - 67.0).abs() < 0.01,
        "center squeezed to window rest: {:?}",
        bar(1)
    );
    assert!(
        (bar(1).top() - 27.0).abs() < 0.01,
        "center under the menubar"
    );
}

#[test]
fn text_flex_takes_remaining_row_width() {
    struct Demo;
    impl View for Demo {
        fn body(&self) -> impl View + 'static {
            Row::new()
                .width(400.0)
                .height(40.0)
                .child(Panel::new().width(100.0).height(40.0))
                .child(Text::new("a").flex(1.0))
        }
    }

    let h = Harness::new(Demo);
    h.layout(400.0, 100.0);
    let row = h.children(h.root().unwrap())[0];
    let kids = h.children(row);
    assert_eq!(kids.len(), 2);
    let text_rect = h.rect(kids[1]).unwrap();
    assert!(
        (text_rect.size.width - 300.0).abs() < 1.0,
        "Text.flex(1) takes the rest of the line, it is {}",
        text_rect.size.width
    );
}
