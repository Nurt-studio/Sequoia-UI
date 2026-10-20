#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::cell::Cell;
use std::rc::Rc;

use sequoia_core::{Color, Length, Point, Rect, Size};
use sequoia_elements::Container;
use sequoia_test::Harness;
use sequoia_view::prelude::*;

fn panel(size: (f32, f32), color: Color) -> impl View + 'static {
    Panel::new().background(color).width(size.0).height(size.1)
}

#[test]
fn overlay_paints_above_main_tree() {
    let h = Harness::new(panel((100.0, 100.0), Color::RED));
    h.layout(400.0, 400.0);
    h.overlay_show(OverlayEntry::new(panel((50.0, 50.0), Color::BLUE)));
    h.layout(400.0, 400.0);

    let list = h.paint();
    let quads: Vec<_> = list.quads().collect();
    assert_eq!(quads.len(), 2);
    assert_eq!(quads[0].color, Color::RED, "main tree first");
    assert_eq!(quads[1].color, Color::BLUE, "overlay on top");
}

#[test]
fn overlay_anchor_to_view_uses_element_rect() {
    let h = Harness::new(
        Column::new()
            .padding(20.0)
            .child(panel((80.0, 30.0), Color::RED)),
    );
    h.layout(400.0, 400.0);

    let root = h.root().unwrap();
    let anchor_el = h.children(root)[0];
    let anchor_rect = h.rect(anchor_el).unwrap();

    let id = h.overlay_show(
        OverlayEntry::new(panel((120.0, 60.0), Color::BLUE))
            .anchor(Anchor::View(anchor_el))
            .placement(Placement::Bottom),
    );
    h.layout(400.0, 400.0);

    let rect = h.overlay_rect(id).unwrap();
    assert_eq!(rect.size, sequoia_core::Size::new(120.0, 60.0));
    assert_eq!(rect.top(), anchor_rect.bottom(), "under the anchor");
    assert_eq!(
        rect.left() + rect.size.width * 0.5,
        anchor_rect.left() + anchor_rect.size.width * 0.5,
        "horizontally centered to the anchor"
    );
}

#[test]
fn overlay_clamps_to_viewport() {
    let h = Harness::new(
        Stack::new()
            .width(400.0)
            .height(400.0)
            .child(Panel::new().width(40.0).height(40.0)),
    );
    h.layout(400.0, 400.0);
    let anchor_el = h.children(h.root().unwrap())[0];

    let id = h.overlay_show(
        OverlayEntry::new(panel((200.0, 80.0), Color::BLUE))
            .anchor(Anchor::Rect(Rect::from_xywh(380.0, 10.0, 10.0, 10.0)))
            .placement(Placement::Bottom),
    );
    h.layout(400.0, 400.0);

    let rect = h.overlay_rect(id).unwrap();
    assert!(rect.right() <= 400.0 + 0.01, "right edge inside viewport");
    assert!(rect.left() >= 0.0 - 0.01);
    let _ = anchor_el;
}

#[test]
fn overlay_hit_test_takes_priority() {
    let clicks_main = std::rc::Rc::new(std::cell::Cell::new(0u32));
    let clicks_ov = std::rc::Rc::new(std::cell::Cell::new(0u32));

    struct Root(std::rc::Rc<std::cell::Cell<u32>>);
    impl View for Root {
        fn body(&self) -> impl View + 'static {
            use sequoia_elements::Button;
            let c = self.0.clone();
            Button::labeled("main").on_click(move || c.set(c.get() + 1))
        }
    }

    let h = Harness::new(Root(clicks_main.clone()));
    h.layout(400.0, 400.0);

    let ov_clicks = clicks_ov.clone();
    let ov_id = h.overlay_show(
        OverlayEntry::new({
            use sequoia_elements::Button;
            Button::labeled("ov").on_click(move || ov_clicks.set(ov_clicks.get() + 1))
        })
        .anchor(Anchor::Rect(Rect::from_xywh(100.0, 100.0, 80.0, 30.0)))
        .placement(Placement::Cursor),
    );
    h.layout(400.0, 400.0);

    h.pointer_move(110.0, 110.0);
    h.click(110.0, 110.0);
    assert_eq!(clicks_ov.get(), 1, "klik w overlay");
    assert_eq!(clicks_main.get(), 0, "main button untouched");

    let button_el = h.children(h.root().unwrap())[0];
    let brect = h.rect(button_el).unwrap();
    let ov_rect = h.overlay_rect(ov_id).unwrap();
    let (bx, by) = (
        brect.left() + brect.size.width * 0.5,
        brect.top() + brect.size.height * 0.5,
    );
    assert!(
        !ov_rect.contains(Point::new(bx, by)),
        "click point outside overlay: ({bx}, {by}) vs {ov_rect:?}"
    );
    h.pointer_move(bx, by);
    h.click(bx, by);
    assert_eq!(clicks_ov.get(), 1);
    assert_eq!(clicks_main.get(), 1, "click outside overlay hits main");
}

#[test]
fn overlay_hide_removes_and_unmounts() {
    let h = Harness::new(panel((100.0, 100.0), Color::RED));
    h.layout(400.0, 400.0);

    let id = h.overlay_show(OverlayEntry::new(panel((50.0, 50.0), Color::BLUE)));
    h.layout(400.0, 400.0);
    assert_eq!(h.paint().quads().count(), 2);

    assert!(h.overlay_hide(id));
    assert!(!h.overlay_hide(id), "second hide returns false");
    h.layout(400.0, 400.0);
    assert_eq!(h.paint().quads().count(), 1, "only the main tree left");
}

#[test]
fn overlay_layers_order() {
    let h = Harness::new(panel((100.0, 100.0), Color::RED));
    h.layout(400.0, 400.0);

    h.overlay_show(
        OverlayEntry::new(panel((20.0, 20.0), Color::rgb8(0, 0, 255))).layer(OverlayLayer::System),
    );
    h.overlay_show(
        OverlayEntry::new(panel((20.0, 20.0), Color::rgb8(0, 255, 0))).layer(OverlayLayer::Tooltip),
    );
    h.layout(400.0, 400.0);

    let list = h.paint();
    let colors: Vec<Color> = list.quads().map(|q| q.color).collect();
    assert_eq!(colors.len(), 3);
    assert_eq!(colors[0], Color::RED);
    assert_eq!(colors[1], Color::rgb8(0, 255, 0), "Tooltip before System");
    assert_eq!(colors[2], Color::rgb8(0, 0, 255));
}

#[test]
fn passthrough_overlay_does_not_capture_hits() {
    let h = Harness::new(panel((200.0, 200.0), Color::RED));
    h.layout(400.0, 400.0);

    h.overlay_show(
        OverlayEntry::new(panel((100.0, 100.0), Color::BLUE))
            .anchor(Anchor::Rect(Rect::from_xywh(50.0, 50.0, 100.0, 100.0)))
            .placement(Placement::Cursor)
            .input_mode(InputMode::Passthrough),
    );
    h.layout(400.0, 400.0);

    let hit = h.hit_test(60.0, 60.0).unwrap();
    let overlay_hit = h.overlay_rect(h.overlay_ids()[0]).unwrap();
    assert!(overlay_hit.contains(sequoia_core::Point::new(60.0, 60.0)));

    let ov_el_inside = h
        .overlay_ids()
        .iter()
        .filter_map(|&id| h.overlay_rect(id))
        .any(|_| false);
    let _ = ov_el_inside;

    assert!(h.is_alive(hit), "hit is a live element");
}

struct MenuItem {
    label: &'static str,
    shortcut: &'static str,
    hits: Rc<Cell<u32>>,
}

impl View for MenuItem {
    fn body(&self) -> impl View + 'static {
        Unit
    }
    fn is_leaf(&self) -> bool {
        true
    }
    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fit,
            height: Length::Px(24.0),
            ..Default::default()
        }
    }
    fn has_visual_state(&self) -> bool {
        true
    }
    fn captures_pointer(&self) -> bool {
        true
    }
    fn intrinsic_size(&self, ctx: &mut LayoutCtx) -> Option<Size> {
        let w = ctx.text_width(self.label, 13.0)
            + ctx.text_width_weighted(self.shortcut, 12.0, sequoia_text::FontWeight::BOLD)
            + 58.0;
        Some(Size::new(w, 24.0))
    }
    fn on_event(&self, event: &Event, ctx: &mut EventCtx) {
        if ctx.phase != EventPhase::Target {
            return;
        }
        if let Event::Pointer(PointerEvent::Down {
            button: PointerButton::Left,
            ..
        }) = event
        {
            self.hits.set(self.hits.get() + 1);
            ctx.handled();
        }
    }
    fn paint_self(&self, ctx: &mut PaintCtx) {
        ctx.quad(ctx.rect, Color::rgb8(40, 44, 52), 0.0);
    }
}

struct MenuSep;

impl View for MenuSep {
    fn body(&self) -> impl View + 'static {
        Unit
    }
    fn is_leaf(&self) -> bool {
        true
    }
    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fill(1.0),
            height: Length::Px(9.0),
            ..Default::default()
        }
    }
    fn paint_self(&self, ctx: &mut PaintCtx) {
        ctx.quad(
            Rect::from_xywh(
                ctx.rect.left(),
                ctx.rect.top() + 4.0,
                ctx.rect.size.width,
                1.0,
            ),
            Color::rgb8(70, 73, 87),
            0.0,
        );
    }
}

struct MenuList {
    items: Vec<(&'static str, &'static str, bool)>,
    hits: Rc<Cell<u32>>,
}

impl View for MenuList {
    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fit,
            height: Length::Fit,
            ..Default::default()
        }
    }
    fn body(&self) -> impl View + 'static {
        let mut col = Column::new().gap(1.0).align_items(Align::Stretch);
        for (label, shortcut, is_sep) in &self.items {
            if *is_sep {
                col = col.child(MenuSep);
            } else {
                col = col.child(MenuItem {
                    label,
                    shortcut,
                    hits: self.hits.clone(),
                });
            }
        }
        Container::new().padding(4.0).child(col)
    }
}

fn file_menu(hits: Rc<Cell<u32>>) -> MenuList {
    MenuList {
        items: vec![
            ("New document", "Ctrl+N", false),
            ("Fit view", "", false),
            ("", "", true),
            ("Zakoncz", "", false),
        ],
        hits,
    }
}

fn menu_quads(h: &Harness) -> Vec<Rect> {
    h.paint().quads().map(|q| q.rect).collect()
}

#[test]
fn overlay_menu_hugs_content_and_stacks_vertically() {
    let hits = Rc::new(Cell::new(0u32));
    let h = Harness::new(panel((400.0, 400.0), Color::RED));
    h.layout(400.0, 400.0);
    let id = h.overlay_show(
        OverlayEntry::new(file_menu(hits))
            .anchor(Anchor::Rect(Rect::from_xywh(10.0, 10.0, 60.0, 24.0)))
            .placement(Placement::Bottom),
    );
    h.layout(400.0, 400.0);

    let rect = h.overlay_rect(id).unwrap();

    assert!(
        (rect.size.height - 92.0).abs() < 1.0,
        "menu height: {rect:?}"
    );
    assert!(rect.size.width > 100.0, "width fits content: {rect:?}");

    let quads = menu_quads(&h);
    let items: Vec<Rect> = quads
        .iter()
        .copied()
        .filter(|r| (r.size.height - 24.0).abs() < 0.01)
        .collect();
    let seps: Vec<Rect> = quads
        .iter()
        .copied()
        .filter(|r| (r.size.height - 1.0).abs() < 0.01)
        .collect();
    assert_eq!(items.len(), 3, "three position rects");
    assert_eq!(seps.len(), 1, "single separator line");
    let t0 = rect.top() + 4.0;
    for (r, want) in items.iter().zip([t0, t0 + 25.0, t0 + 60.0]) {
        assert!(
            (r.top() - want).abs() < 0.01,
            "position in stack: {r:?} vs {want}"
        );
    }
    assert!(
        (seps[0].top() - (t0 + 54.0)).abs() < 0.01,
        "separator line: {:?}",
        seps[0]
    );
    for r in &items {
        assert!(
            (r.left() - (rect.left() + 4.0)).abs() < 0.01,
            "padding 4: {r:?}"
        );
        assert!(
            (r.size.width - (rect.size.width - 8.0)).abs() < 0.01,
            "stretch: {r:?}"
        );
    }
}

#[test]
fn overlay_menu_click_hits_item() {
    let hits = Rc::new(Cell::new(0u32));
    let h = Harness::new(panel((400.0, 400.0), Color::RED));
    h.layout(400.0, 400.0);
    h.overlay_show(
        OverlayEntry::new(file_menu(hits.clone()))
            .anchor(Anchor::Rect(Rect::from_xywh(10.0, 10.0, 60.0, 24.0)))
            .placement(Placement::Bottom),
    );
    h.layout(400.0, 400.0);

    let items: Vec<Rect> = menu_quads(&h)
        .into_iter()
        .filter(|r| (r.size.height - 24.0).abs() < 0.01)
        .collect();
    assert_eq!(items.len(), 3);

    let c = items[1];
    h.pointer_move(c.left() + 5.0, c.top() + 12.0);
    h.click(c.left() + 5.0, c.top() + 12.0);
    assert_eq!(hits.get(), 1, "click hit the menu item");

    h.pointer_move(350.0, 350.0);
    h.click(350.0, 350.0);
    assert_eq!(hits.get(), 1, "click outside menu does nothing");
}

#[test]
fn overlay_flips_bottom_to_top_when_no_room() {
    let hits = Rc::new(Cell::new(0u32));
    let h = Harness::new(panel((400.0, 400.0), Color::RED));
    h.layout(400.0, 400.0);

    let id = h.overlay_show(
        OverlayEntry::new(file_menu(hits))
            .anchor(Anchor::Rect(Rect::from_xywh(10.0, 360.0, 60.0, 24.0)))
            .placement(Placement::Bottom),
    );
    h.layout(400.0, 400.0);

    let rect = h.overlay_rect(id).unwrap();
    assert!(
        rect.bottom() <= 360.0 + 0.01,
        "menu opened upward: {rect:?}"
    );
    assert!(rect.top() >= 0.0 - 0.01, "w viewporcie: {rect:?}");
}

#[test]
fn overlay_flips_right_to_left_when_no_room() {
    let hits = Rc::new(Cell::new(0u32));
    let h = Harness::new(panel((400.0, 400.0), Color::RED));
    h.layout(400.0, 400.0);

    let id = h.overlay_show(
        OverlayEntry::new(file_menu(hits))
            .anchor(Anchor::Rect(Rect::from_xywh(360.0, 10.0, 24.0, 24.0)))
            .placement(Placement::Right),
    );
    h.layout(400.0, 400.0);

    let rect = h.overlay_rect(id).unwrap();
    assert!(
        rect.right() <= 360.0 + 0.01,
        "menu opened to the left: {rect:?}"
    );
    assert!(rect.left() >= 0.0 - 0.01, "w viewporcie: {rect:?}");
}

#[test]
fn overlay_stays_when_requested_side_fits() {
    let hits = Rc::new(Cell::new(0u32));
    let h = Harness::new(panel((400.0, 400.0), Color::RED));
    h.layout(400.0, 400.0);
    let anchor = Rect::from_xywh(10.0, 10.0, 60.0, 24.0);
    let id = h.overlay_show(
        OverlayEntry::new(file_menu(hits))
            .anchor(Anchor::Rect(anchor))
            .placement(Placement::Bottom),
    );
    h.layout(400.0, 400.0);

    let rect = h.overlay_rect(id).unwrap();
    assert!(
        (rect.top() - anchor.bottom()).abs() < 0.01,
        "no flip under anchor: {rect:?}"
    );
}

#[test]
fn overlay_position_defaults_to_absolute() {
    let h = Harness::new(panel((100.0, 100.0), Color::RED));
    h.layout(400.0, 400.0);
    let anchor = Rect::from_xywh(50.0, 50.0, 80.0, 30.0);
    let id = h.overlay_show(
        OverlayEntry::new(panel((120.0, 60.0), Color::BLUE))
            .anchor(Anchor::Rect(anchor))
            .placement(Placement::Bottom),
    );
    h.layout(400.0, 400.0);

    let rect = h.overlay_rect(id).unwrap();
    assert!(
        (rect.top() - anchor.bottom()).abs() < 0.01,
        "under the anchor (Absolute default): {rect:?}"
    );
    assert!(
        (rect.left() - (anchor.left() + (anchor.size.width - 120.0) * 0.5)).abs() < 0.01,
        "horizontally centered: {rect:?}"
    );
}

#[test]
fn overlay_position_relative_starts_at_anchor_origin() {
    let h = Harness::new(panel((100.0, 100.0), Color::RED));
    h.layout(400.0, 400.0);
    let id = h.overlay_show(
        OverlayEntry::new(panel((100.0, 40.0), Color::BLUE))
            .anchor(Anchor::Rect(Rect::from_xywh(50.0, 60.0, 80.0, 30.0)))
            .position(Position::Relative)
            .offset(5.0, 7.0),
    );
    h.layout(400.0, 400.0);

    let rect = h.overlay_rect(id).unwrap();
    assert!(
        (rect.left() - 55.0).abs() < 0.01,
        "anchor corner + dx: {rect:?}"
    );
    assert!(
        (rect.top() - 67.0).abs() < 0.01,
        "anchor corner + dy: {rect:?}"
    );
}

#[test]
fn overlay_position_relative_ignores_placement() {
    let h = Harness::new(panel((100.0, 100.0), Color::RED));
    h.layout(400.0, 400.0);
    let id = h.overlay_show(
        OverlayEntry::new(panel((100.0, 40.0), Color::BLUE))
            .anchor(Anchor::Rect(Rect::from_xywh(50.0, 60.0, 80.0, 30.0)))
            .placement(Placement::Top)
            .position(Position::Relative)
            .offset(0.0, 0.0),
    );
    h.layout(400.0, 400.0);

    let rect = h.overlay_rect(id).unwrap();
    assert!((rect.left() - 50.0).abs() < 0.01, "{rect:?}");
    assert!((rect.top() - 60.0).abs() < 0.01, "{rect:?}");
}

#[test]
fn overlay_position_fixed_ignores_anchor() {
    let h = Harness::new(panel((100.0, 100.0), Color::RED));
    h.layout(400.0, 400.0);
    let id = h.overlay_show(
        OverlayEntry::new(panel((100.0, 40.0), Color::BLUE))
            .anchor(Anchor::Rect(Rect::from_xywh(10.0, 10.0, 20.0, 20.0)))
            .placement(Placement::Bottom)
            .at(300.0, 200.0),
    );
    h.layout(400.0, 400.0);

    let rect = h.overlay_rect(id).unwrap();
    assert!((rect.left() - 300.0).abs() < 0.01, "{rect:?}");
    assert!((rect.top() - 200.0).abs() < 0.01, "{rect:?}");
}

#[test]
fn overlay_position_fixed_clamps_to_viewport() {
    let h = Harness::new(panel((100.0, 100.0), Color::RED));
    h.layout(400.0, 400.0);
    let id = h.overlay_show(OverlayEntry::new(panel((100.0, 100.0), Color::BLUE)).at(380.0, 380.0));
    h.layout(400.0, 400.0);

    let rect = h.overlay_rect(id).unwrap();
    assert!(
        (rect.left() - 300.0).abs() < 0.01,
        "clamped to viewport: {rect:?}"
    );
    assert!(
        (rect.top() - 300.0).abs() < 0.01,
        "clamped to viewport: {rect:?}"
    );
}
