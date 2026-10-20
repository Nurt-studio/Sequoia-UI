#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::cell::Cell;
use std::rc::Rc;

use sequoia_core::{Color, Length, Rect, Vec2};
use sequoia_test::Harness;
use sequoia_view::prelude::*;

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
            width: Length::Fill(1.0),
            height: Length::Px(self.h),
            ..LayoutStyle::default()
        }
    }

    fn paint_self(&self, ctx: &mut PaintCtx) {
        ctx.quad(ctx.rect, Color::rgb8(0x33, 0x99, 0xcc), 0.0);
    }
}

struct ScrollViewDemo {
    n: usize,
}

impl View for ScrollViewDemo {
    fn body(&self) -> impl View + 'static {
        let mut col = Column::new();
        for _ in 0..self.n {
            col = col.child(Bar { h: 40.0 });
        }
        ScrollView::new(col)
    }
}

#[test]
fn content_height_known_after_layout() {
    let h = Harness::new(ScrollViewDemo { n: 20 });
    h.layout(300.0, 200.0);

    let root = h.root().unwrap();
    let scroll_el = h.children(root)[0];

    let scroll = h.view::<ScrollView>(scroll_el).unwrap();
    assert_eq!(scroll.content_height(&h.runtime(), scroll_el), 800.0);
}

#[test]
fn child_extends_below_viewport_and_is_clipped() {
    let h = Harness::new(ScrollViewDemo { n: 20 });
    h.layout(300.0, 200.0);

    let root = h.root().unwrap();
    let scroll_el = h.children(root)[0];
    let col = h.children(scroll_el)[0];
    let col_rect = h.rect(col).unwrap();
    assert!(col_rect.size.height >= 800.0);

    let mut list = sequoia_core::RenderList::new();
    let mut text = sequoia_text::TextSystem::new();
    h.runtime().paint(&mut list, &mut text);

    for q in list.quads() {
        assert!(
            q.rect.bottom() <= 200.0 + 0.5,
            "quad leaked past viewport: bottom={}",
            q.rect.bottom()
        );
    }
    let bars = list.quads().filter(|q| q.rect.size.width > 100.0).count();
    assert_eq!(bars, 5, "5 bars fit in 200px (5 x 40)");
    assert_eq!(list.quads().count(), 7, "5 bars + track + scrollbar thumb");
}

#[test]
fn wheel_scrolls_and_reveals_lower_content() {
    let h = Harness::new(ScrollViewDemo { n: 20 });
    h.layout(300.0, 200.0);

    h.wheel(150.0, 100.0, -40.0);
    h.layout(300.0, 200.0);

    let root = h.root().unwrap();
    let scroll_el = h.children(root)[0];
    let scroll = h.view::<ScrollView>(scroll_el).unwrap();
    assert_eq!(
        sequoia_view::ScrollView::offset(&scroll, &h.runtime(), scroll_el),
        40.0
    );

    let mut list = sequoia_core::RenderList::new();
    let mut text = sequoia_text::TextSystem::new();
    h.runtime().paint(&mut list, &mut text);

    for q in list.quads() {
        assert!(q.rect.top() >= -0.5);
        assert!(q.rect.bottom() <= 200.0 + 0.5);
    }
}

#[test]
fn wheel_clamps_at_max_offset() {
    let h = Harness::new(ScrollViewDemo { n: 3 });
    h.layout(300.0, 200.0);

    h.wheel(150.0, 100.0, 500.0);
    h.layout(300.0, 200.0);

    let scroll_el = h.children(h.root().unwrap())[0];
    let scroll = h.view::<ScrollView>(scroll_el).unwrap();
    assert_eq!(
        sequoia_view::ScrollView::offset(&scroll, &h.runtime(), scroll_el),
        0.0,
        "no scrolling when content <= viewport"
    );
}

#[test]
fn scrollbar_paints_only_when_content_exceeds() {
    let h = Harness::new(ScrollViewDemo { n: 3 });
    h.layout(300.0, 200.0);
    let mut list = sequoia_core::RenderList::new();
    let mut text = sequoia_text::TextSystem::new();
    h.runtime().paint(&mut list, &mut text);
    assert_eq!(list.quads().count(), 3, "only 3 bars, no track/thumb");

    let h2 = Harness::new(ScrollViewDemo { n: 20 });
    h2.layout(300.0, 200.0);
    let mut list2 = sequoia_core::RenderList::new();
    let mut text2 = sequoia_text::TextSystem::new();
    h2.runtime().paint(&mut list2, &mut text2);
    assert_eq!(list2.quads().count(), 7, "5 bars + track + thumb");
}

struct BoxLeaf {
    w: f32,
    h: f32,
}

impl View for BoxLeaf {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn is_leaf(&self) -> bool {
        true
    }

    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Px(self.w),
            height: Length::Px(self.h),
            ..LayoutStyle::default()
        }
    }

    fn paint_self(&self, ctx: &mut PaintCtx) {
        ctx.quad(ctx.rect, Color::rgb8(0x33, 0x99, 0xcc), 0.0);
    }
}

fn scroll_of(h: &Harness) -> Rc<ScrollView> {
    h.view::<ScrollView>(scroll_el_of(h)).unwrap()
}

fn scroll_el_of(h: &Harness) -> ElementId {
    let root = h.root().unwrap();
    h.children(root)[0]
}

fn rt_of(h: &Harness) -> Runtime {
    h.runtime()
}

#[test]
fn horizontal_axis_scrolls_wide_content() {
    struct HDemo;
    impl View for HDemo {
        fn body(&self) -> impl View + 'static {
            ScrollView::new(BoxLeaf { w: 800.0, h: 50.0 }).horizontal()
        }
    }
    let h = Harness::new(HDemo);
    h.layout(300.0, 200.0);

    let scroll = scroll_of(&h);
    let el = scroll_el_of(&h);
    let rt = rt_of(&h);
    assert_eq!(scroll.scroll_axis(), ScrollAxis::Horizontal);
    assert_eq!(scroll.content_width(&rt, el), 800.0);
    assert_eq!(scroll.max_offset_x(&rt, el), 500.0);
    assert_eq!(scroll.max_offset(&rt, el), 0.0, "vertical axis disabled");

    h.wheel(150.0, 100.0, -40.0);
    assert_eq!(scroll.offset_x(&rt, el), 0.0);

    h.wheel_xy(150.0, 100.0, -60.0, 0.0);
    assert_eq!(scroll.offset_x(&rt, el), 60.0);
    h.wheel_with_modifiers(150.0, 100.0, 0.0, -40.0, Modifiers::SHIFT);
    assert_eq!(scroll.offset_x(&rt, el), 100.0);

    scroll.scroll_to_x(&rt, el, 120.0);
    h.layout(300.0, 200.0);
    let root = h.root().unwrap();
    let child = h.children(h.children(root)[0])[0];
    let rect = h.rect(child).unwrap();
    assert_eq!(rect.left(), -120.0);
}

#[test]
fn both_axes_scroll_and_paint_corner() {
    struct BothDemo;
    impl View for BothDemo {
        fn body(&self) -> impl View + 'static {
            ScrollView::new(BoxLeaf { w: 800.0, h: 800.0 }).both()
        }
    }
    let h = Harness::new(BothDemo);
    h.layout(300.0, 200.0);

    let scroll = scroll_of(&h);
    let el = scroll_el_of(&h);
    let rt = rt_of(&h);
    assert_eq!(scroll.max_offset_vec(&rt, el), Vec2::new(500.0, 600.0));

    scroll.scroll_to_xy(&rt, el, 100.0, 200.0);
    h.layout(300.0, 200.0);
    let root = h.root().unwrap();
    let child = h.children(h.children(root)[0])[0];
    let rect = h.rect(child).unwrap();
    assert_eq!(rect.origin, sequoia_core::Point::new(-100.0, -200.0));

    assert_eq!(h.paint().quads().count(), 6);
}

#[test]
fn scrollbar_visibility_never_and_always() {
    struct Tall;
    impl View for Tall {
        fn body(&self) -> impl View + 'static {
            ScrollView::new(BoxLeaf { w: 300.0, h: 800.0 }).show_scrollbar(false)
        }
    }
    let h = Harness::new(Tall);
    h.layout(300.0, 200.0);
    assert_eq!(
        sequoia_view::ScrollView::offset(&scroll_of(&h), &h.runtime(), scroll_el_of(&h)),
        0.0,
        "hidden bar does not block the wheel"
    );
    h.wheel(150.0, 100.0, -40.0);
    assert_eq!(
        sequoia_view::ScrollView::offset(&scroll_of(&h), &h.runtime(), scroll_el_of(&h)),
        40.0
    );
    assert_eq!(
        h.paint().quads().count(),
        1,
        "content without track and thumb"
    );

    struct Short;
    impl View for Short {
        fn body(&self) -> impl View + 'static {
            ScrollView::new(BoxLeaf { w: 300.0, h: 50.0 })
                .scrollbar_visibility(ScrollbarVisibility::Always)
        }
    }
    let h2 = Harness::new(Short);
    h2.layout(300.0, 200.0);
    assert_eq!(
        h2.paint().quads().count(),
        3,
        "content + track + thumb full height"
    );
}

#[test]
fn keyboard_arrows_pages_and_home_end() {
    struct KDemo;
    impl View for KDemo {
        fn body(&self) -> impl View + 'static {
            ScrollView::new(Canvas::new().height(800.0))
        }
    }
    let h = Harness::new(KDemo);
    h.layout(300.0, 200.0);
    h.pointer_down(150.0, 100.0);

    let scroll = scroll_of(&h);
    let el = scroll_el_of(&h);
    let rt = rt_of(&h);
    assert_eq!(scroll.max_offset(&rt, el), 600.0);

    h.key(Key::ARROW_DOWN);
    assert_eq!(sequoia_view::ScrollView::offset(&scroll, &rt, el), 40.0);
    h.key(Key::ARROW_UP);
    assert_eq!(sequoia_view::ScrollView::offset(&scroll, &rt, el), 0.0);
    h.key(Key::PAGE_DOWN);
    assert_eq!(sequoia_view::ScrollView::offset(&scroll, &rt, el), 200.0);
    h.key(Key::PAGE_UP);
    assert_eq!(sequoia_view::ScrollView::offset(&scroll, &rt, el), 0.0);
    h.key(Key::END);
    assert_eq!(sequoia_view::ScrollView::offset(&scroll, &rt, el), 600.0);
    h.key(Key::HOME);
    assert_eq!(sequoia_view::ScrollView::offset(&scroll, &rt, el), 0.0);

    h.key(Key::ARROW_LEFT);
    assert_eq!(sequoia_view::ScrollView::offset(&scroll, &rt, el), 0.0);
}

#[test]
fn thumb_drag_moves_offset() {
    let h = Harness::new(ScrollViewDemo { n: 20 });
    h.layout(300.0, 200.0);
    let scroll = scroll_of(&h);
    let el = scroll_el_of(&h);
    let rt = rt_of(&h);
    assert_eq!(scroll.max_offset(&rt, el), 600.0);

    h.pointer_down(294.0, 10.0);
    assert_eq!(
        sequoia_view::ScrollView::offset(&scroll, &rt, el),
        0.0,
        "thumb grab does not jump"
    );
    h.pointer_move(294.0, 100.0);
    let off = sequoia_view::ScrollView::offset(&scroll, &rt, el);
    assert!(off > 300.0 && off < 600.0, "drag moves down: offset={off}");
    h.pointer_up(294.0, 100.0);
    assert_eq!(
        sequoia_view::ScrollView::offset(&scroll, &rt, el),
        off,
        "release does not revert"
    );

    let h2 = Harness::new(ScrollViewDemo { n: 20 });
    h2.layout(300.0, 200.0);
    let scroll2 = scroll_of(&h2);
    let el2 = scroll_el_of(&h2);
    let rt2 = rt_of(&h2);
    h2.pointer_down(294.0, 190.0);
    assert!(
        sequoia_view::ScrollView::offset(&scroll2, &rt2, el2) > 400.0,
        "jump to track bottom: offset={}",
        sequoia_view::ScrollView::offset(&scroll2, &rt2, el2)
    );
    h2.pointer_up(294.0, 190.0);
}

#[test]
fn on_scroll_fires_on_change_only() {
    let seen: Rc<Cell<Vec2>> = Rc::new(Cell::new(Vec2::ZERO));
    let count: Rc<Cell<u32>> = Rc::new(Cell::new(0));
    struct CDemo {
        seen: Rc<Cell<Vec2>>,
        count: Rc<Cell<u32>>,
    }
    impl View for CDemo {
        fn body(&self) -> impl View + 'static {
            let seen = self.seen.clone();
            let count = self.count.clone();
            ScrollView::new(BoxLeaf { w: 300.0, h: 800.0 }).on_scroll(move |off| {
                seen.set(off);
                count.set(count.get() + 1);
            })
        }
    }
    let h = Harness::new(CDemo {
        seen: seen.clone(),
        count: count.clone(),
    });
    h.layout(300.0, 200.0);

    let scroll = scroll_of(&h);
    let el = scroll_el_of(&h);
    let rt = rt_of(&h);
    scroll.scroll_to_y(&rt, el, 100.0);
    assert_eq!(count.get(), 1);
    assert_eq!(seen.get(), Vec2::new(0.0, 100.0));
    scroll.scroll_to_y(&rt, el, 100.0);
    assert_eq!(count.get(), 1, "same offset does not call again");
    h.wheel(150.0, 100.0, -40.0);
    assert_eq!(count.get(), 2);
}

#[test]
fn programmatic_api_pages_and_ensure_visible() {
    let h = Harness::new(ScrollViewDemo { n: 20 });
    h.layout(300.0, 200.0);
    let scroll = scroll_of(&h);
    let el = scroll_el_of(&h);
    let rt = rt_of(&h);

    scroll.scroll_to_bottom(&rt, el);
    assert_eq!(sequoia_view::ScrollView::offset(&scroll, &rt, el), 600.0);
    scroll.page_up(&rt, el);
    assert_eq!(sequoia_view::ScrollView::offset(&scroll, &rt, el), 400.0);
    scroll.page_down(&rt, el);
    assert_eq!(sequoia_view::ScrollView::offset(&scroll, &rt, el), 600.0);
    scroll.scroll_to_top(&rt, el);
    assert_eq!(sequoia_view::ScrollView::offset(&scroll, &rt, el), 0.0);

    assert_eq!(scroll.scroll_fraction_y(&rt, el), 0.0);
    scroll.scroll_to_bottom(&rt, el);
    assert_eq!(scroll.scroll_fraction_y(&rt, el), 1.0);

    assert!(scroll.ensure_visible(&rt, el, Rect::from_xywh(0.0, 0.0, 300.0, 40.0)));
    assert_eq!(sequoia_view::ScrollView::offset(&scroll, &rt, el), 0.0);
    assert!(
        !scroll.ensure_visible(&rt, el, Rect::from_xywh(0.0, 0.0, 300.0, 40.0)),
        "visible rect keeps offset"
    );

    assert!(scroll.ensure_visible(&rt, el, Rect::from_xywh(0.0, 400.0, 300.0, 400.0)));
    assert_eq!(sequoia_view::ScrollView::offset(&scroll, &rt, el), 400.0);
}

struct HoverDemo {
    n: usize,
}

impl View for HoverDemo {
    fn body(&self) -> impl View + 'static {
        let mut col = Column::new();
        for _ in 0..self.n {
            col = col.child(Bar { h: 40.0 });
        }
        ScrollView::new(col)
            .scrollbar_visibility(ScrollbarVisibility::OnHover)
            .scrollbar_thumb(Color::rgb8(0xFF, 0x00, 0x00))
            .scrollbar_track(Color::rgb8(0x00, 0x00, 0xFF))
    }
}

fn thumb_track_counts(h: &Harness) -> (usize, usize) {
    let list = h.paint();
    let red = Color::rgb8(0xFF, 0x00, 0x00);
    let blue = Color::rgb8(0x00, 0x00, 0xFF);
    let mut r = 0;
    let mut b = 0;
    for q in list.quads() {
        if q.color == red {
            r += 1;
        }
        if q.color == blue {
            b += 1;
        }
    }
    (r, b)
}

#[test]
fn onhover_bar_hidden_until_edge_hover() {
    let h = Harness::new(HoverDemo { n: 20 });
    h.layout(300.0, 200.0);
    h.tick();

    assert_eq!(thumb_track_counts(&h), (0, 0), "bez hovera pasek schowany");

    h.pointer_move(292.0, 100.0);
    h.tick();
    assert_eq!(
        thumb_track_counts(&h),
        (1, 1),
        "edge hover shows track and knob"
    );

    h.pointer_move(50.0, 100.0);
    h.tick();
    assert_eq!(
        thumb_track_counts(&h),
        (0, 0),
        "beyond the edge the bar disappears"
    );
}

#[test]
fn onhover_bar_stays_hidden_without_overflow() {
    let h = Harness::new(HoverDemo { n: 3 });
    h.layout(300.0, 200.0);
    h.tick();
    h.pointer_move(292.0, 100.0);
    h.tick();
    assert_eq!(thumb_track_counts(&h), (0, 0));
}

#[test]
fn touch_drag_pans_content() {
    let h = Harness::new(ScrollViewDemo { n: 20 });
    h.layout(300.0, 200.0);
    h.tick();

    h.touch_down(1, 150.0, 150.0, None);
    h.touch_move(1, 150.0, 120.0, None);
    h.touch_move(1, 150.0, 90.0, None);
    h.touch_up(1, 150.0, 90.0);
    h.tick();
    h.layout(300.0, 200.0);

    let scroll_el = h.children(h.root().unwrap())[0];
    let scroll = h.view::<ScrollView>(scroll_el).unwrap();
    assert_eq!(
        sequoia_view::ScrollView::offset(&scroll, &h.runtime(), scroll_el),
        60.0,
        "drag scrolls content"
    );
}

#[test]
fn touch_tap_does_not_scroll() {
    let h = Harness::new(ScrollViewDemo { n: 20 });
    h.layout(300.0, 200.0);
    h.tick();

    h.touch_down(1, 150.0, 150.0, None);
    h.touch_move(1, 152.0, 151.0, None);
    h.touch_up(1, 152.0, 151.0);
    h.tick();
    h.layout(300.0, 200.0);

    let scroll_el = h.children(h.root().unwrap())[0];
    let scroll = h.view::<ScrollView>(scroll_el).unwrap();
    assert_eq!(
        sequoia_view::ScrollView::offset(&scroll, &h.runtime(), scroll_el),
        0.0,
        "tap nie rusza offsetu"
    );
}

#[test]
fn touch_drag_clamps_at_max_offset() {
    let h = Harness::new(ScrollViewDemo { n: 20 });
    h.layout(300.0, 200.0);
    h.tick();

    h.touch_down(1, 150.0, 180.0, None);
    h.touch_move(1, 150.0, 20.0, None);
    h.touch_move(1, 150.0, -500.0, None);
    h.touch_up(1, 150.0, -500.0);
    h.tick();
    h.layout(300.0, 200.0);

    let scroll_el = h.children(h.root().unwrap())[0];
    let scroll = h.view::<ScrollView>(scroll_el).unwrap();
    assert_eq!(
        sequoia_view::ScrollView::offset(&scroll, &h.runtime(), scroll_el),
        600.0,
        "pan clampuje do max offsetu"
    );
}

#[test]
fn touch_horizontal_drag_ignored_in_vertical_mode() {
    let h = Harness::new(ScrollViewDemo { n: 20 });
    h.layout(300.0, 200.0);
    h.tick();

    h.touch_down(1, 150.0, 100.0, None);
    h.touch_move(1, 50.0, 100.0, None);
    h.touch_up(1, 50.0, 100.0);
    h.tick();
    h.layout(300.0, 200.0);

    let scroll_el = h.children(h.root().unwrap())[0];
    let scroll = h.view::<ScrollView>(scroll_el).unwrap();
    assert_eq!(
        sequoia_view::ScrollView::offset(&scroll, &h.runtime(), scroll_el),
        0.0,
        "poziomy ruch nie scrolluje pionu"
    );
}
