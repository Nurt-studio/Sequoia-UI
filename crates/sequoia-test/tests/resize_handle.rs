#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_core::Color;
use sequoia_elements::prelude::*;
use sequoia_reactive::Signal;
use sequoia_test::Harness;

struct Demo {
    w: Signal<f32>,
    handle: ResizeHandle,
}

impl Demo {
    fn new(w: Signal<f32>, invert: bool) -> Self {
        let mut handle = ResizeHandle::new(w.clone()).min(100.0).max(400.0);
        if invert {
            handle = handle.invert(true);
        }
        Self { w, handle }
    }
}

impl View for Demo {
    fn body(&self) -> impl View + 'static {
        Row::new()
            .fill_size()
            .child(
                Container::new()
                    .width(self.w.get())
                    .fill_height()
                    .background(Color::hex(0x2C2C2C)),
            )
            .child(self.handle.clone())
            .child_flex(
                Container::new()
                    .background(Color::hex(0x1E1E1E))
                    .child(Text::new("x")),
                1.0,
            )
    }
}

fn handle_rect(h: &Harness) -> sequoia_core::Rect {
    let root = h.root().unwrap();
    let row = h.children(root)[0];
    let kids = h.children(row);
    assert_eq!(kids.len(), 3, "panel + handle + content");
    h.rect(kids[1]).unwrap()
}

#[test]
fn drag_right_widens_panel() {
    let w = Signal::new(200.0);
    let h = Harness::new(Demo::new(w.clone(), false));
    h.layout(800.0, 600.0);

    let r = handle_rect(&h);
    assert_eq!(r.size.width, 7.0);
    let cx = r.left() + r.size.width * 0.5;
    let cy = r.top() + r.size.height * 0.5;

    h.drag((cx, cy), (cx + 60.0, cy));

    h.tick();
    h.layout(800.0, 600.0);

    assert_eq!(w.get_untracked(), 260.0);
    let root = h.root().unwrap();
    let panel = h.children(h.children(root)[0])[0];
    assert_eq!(h.rect(panel).unwrap().size.width, 260.0);
}

#[test]
fn drag_clamps_to_min_and_max() {
    let w = Signal::new(200.0);
    let h = Harness::new(Demo::new(w.clone(), false));
    h.layout(800.0, 600.0);

    let r = handle_rect(&h);
    let cx = r.left() + r.size.width * 0.5;
    let cy = r.top() + r.size.height * 0.5;

    h.drag((cx, cy), (cx - 500.0, cy));
    assert_eq!(w.get_untracked(), 100.0);

    h.tick();
    h.layout(800.0, 600.0);
    let r = handle_rect(&h);
    let cx = r.left() + r.size.width * 0.5;
    let cy = r.top() + r.size.height * 0.5;
    h.drag((cx, cy), (cx + 900.0, cy));
    assert_eq!(w.get_untracked(), 400.0);
}

#[test]
fn invert_moves_opposite() {
    let w = Signal::new(200.0);
    let h = Harness::new(Demo::new(w.clone(), true));
    h.layout(800.0, 600.0);

    let r = handle_rect(&h);
    let cx = r.left() + r.size.width * 0.5;
    let cy = r.top() + r.size.height * 0.5;

    h.drag((cx, cy), (cx - 40.0, cy));
    assert_eq!(w.get_untracked(), 240.0);
}

#[test]
fn clamped_panel_assembly_drags_and_stays_adjacent() {
    struct Asm {
        w: Signal<f32>,
        handle: ResizeHandle,
    }
    impl View for Asm {
        fn body(&self) -> impl View + 'static {
            Row::new()
                .fill_size()
                .child(
                    Container::new()
                        .width(self.w.get())
                        .min_width(160.0)
                        .max_width(400.0)
                        .fill_height()
                        .background(Color::hex(0x2C2C2C)),
                )
                .child(self.handle.clone())
                .child_flex(Container::new().background(Color::hex(0x1E1E1E)), 1.0)
        }
    }

    let w = Signal::new(248.0);
    let asm = Asm {
        w: w.clone(),
        handle: ResizeHandle::new(w.clone()).min(160.0).max(400.0),
    };
    let h = Harness::new(asm);
    h.layout(1000.0, 700.0);

    let root = h.root().unwrap();
    let kids = h.children(h.children(root)[0]);
    let (panel_r, handle_r) = (h.rect(kids[0]).unwrap(), h.rect(kids[1]).unwrap());
    assert_eq!(panel_r.size.width, 248.0);

    assert_eq!(handle_r.left(), panel_r.right());
    assert_eq!(handle_r.size.width, 7.0);

    let cx = handle_r.left() + 3.5;
    let cy = handle_r.top() + handle_r.size.height * 0.5;
    h.drag((cx, cy), (cx + 40.0, cy));
    h.tick();
    h.layout(1000.0, 700.0);

    assert_eq!(w.get_untracked(), 288.0);
    let kids = h.children(h.children(h.root().unwrap())[0]);
    let (panel_r, handle_r) = (h.rect(kids[0]).unwrap(), h.rect(kids[1]).unwrap());
    assert_eq!(panel_r.size.width, 288.0);
    assert_eq!(handle_r.left(), panel_r.right());
}

#[test]
fn gutter_paints_panel_colored_strip() {
    const PANEL: Color = Color::hex(0x2C2C2C);
    struct GDemo {
        w: Signal<f32>,
        handle: ResizeHandle,
    }
    impl View for GDemo {
        fn body(&self) -> impl View + 'static {
            Row::new()
                .fill_size()
                .child(
                    Container::new()
                        .width(self.w.get())
                        .fill_height()
                        .background(PANEL),
                )
                .child(self.handle.clone())
                .child_flex(Container::new().background(Color::hex(0x1E1E1E)), 1.0)
        }
    }

    let w = Signal::new(200.0);
    let demo = GDemo {
        w: w.clone(),
        handle: ResizeHandle::new(w.clone()).gutter(PANEL),
    };
    let h = Harness::new(demo);
    h.layout(800.0, 600.0);

    let list = h.paint();
    let strips: Vec<_> = list
        .quads()
        .filter(|q| q.color == PANEL && (q.rect.size.width - 7.0).abs() < 0.01)
        .collect();
    assert_eq!(
        strips.len(),
        1,
        "exactly one 7px gutter lane in panel color"
    );
    assert_eq!(strips[0].rect.left(), 200.0, "rynna doklejona do panelu");
}

#[test]
fn move_without_down_changes_nothing() {
    let w = Signal::new(200.0);
    let h = Harness::new(Demo::new(w.clone(), false));
    h.layout(800.0, 600.0);

    let r = handle_rect(&h);
    h.pointer_move(r.left() + 3.0, r.top() + 100.0);
    assert_eq!(w.get_untracked(), 200.0, "hover alone does not move");
}

#[test]
fn vertical_axis_resizes_height() {
    struct VDemo {
        h: Signal<f32>,
        handle: ResizeHandle,
    }
    impl View for VDemo {
        fn body(&self) -> impl View + 'static {
            Column::new()
                .fill_size()
                .child(
                    Container::new()
                        .fill_width()
                        .height(self.h.get())
                        .background(Color::hex(0x2C2C2C)),
                )
                .child(self.handle.clone())
                .child_flex(Container::new().background(Color::hex(0x1E1E1E)), 1.0)
        }
    }

    let hv = Signal::new(150.0);
    let demo = VDemo {
        h: hv.clone(),
        handle: ResizeHandle::new(hv.clone())
            .axis(Axis::Vertical)
            .min(50.0)
            .max(500.0),
    };
    let h = Harness::new(demo);
    h.layout(800.0, 600.0);

    let root = h.root().unwrap();
    let col = h.children(root)[0];
    let handle_el = h.children(col)[1];
    let r = h.rect(handle_el).unwrap();
    assert_eq!(r.size.height, 7.0);
    let cx = r.left() + r.size.width * 0.5;
    let cy = r.top() + r.size.height * 0.5;

    h.drag((cx, cy), (cx, cy + 40.0));
    assert_eq!(hv.get_untracked(), 190.0);
}
