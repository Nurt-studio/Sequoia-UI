#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::cell::Cell;
use std::rc::Rc;

use sequoia_core::{Color, Point, Rect, Vec2};
use sequoia_reactive::Signal;
use sequoia_test::Harness;
use sequoia_view::prelude::*;

const BG: Color = Color::rgb8(0x21, 0x25, 0x2b);
const FG: Color = Color::rgb8(0xe6, 0xe8, 0xeb);
const GRID: Color = Color::rgb8(0x2c, 0x31, 0x39);

#[test]
fn paints_background_and_user_primitives() {
    struct Demo;
    impl View for Demo {
        fn body(&self) -> impl View + 'static {
            Canvas::new().background(BG).on_draw(|ctx| {
                ctx.fill(Rect::from_xywh(10.0, 10.0, 20.0, 20.0), Fill::solid(FG));
            })
        }
    }
    let h = Harness::new(Demo);
    h.layout(200.0, 200.0);
    let list = h.paint();

    assert_eq!(list.quads().count(), 2);
}

#[test]
fn user_culls_with_is_visible() {
    struct Demo;
    impl View for Demo {
        fn body(&self) -> impl View + 'static {
            Canvas::new().on_draw(|ctx| {
                let vp = ctx.rect();
                let a = Rect::from_xywh(10.0, 10.0, 20.0, 20.0);
                if vp.intersects(a) {
                    ctx.fill(a, Fill::solid(FG));
                }
                let b = Rect::from_xywh(500.0, 500.0, 20.0, 20.0);
                if vp.intersects(b) {
                    ctx.fill(b, Fill::solid(FG));
                }
            })
        }
    }
    let h = Harness::new(Demo);
    h.layout(200.0, 200.0);
    let list = h.paint();
    assert_eq!(list.quads().count(), 1, "second primitive culled");
}

#[test]
fn draw_grid_only_visible_lines() {
    struct Demo;
    impl View for Demo {
        fn body(&self) -> impl View + 'static {
            Canvas::new().on_draw(|ctx| {
                ctx.grid_offset(20.0, Vec2::ZERO, GRID);
            })
        }
    }
    let h = Harness::new(Demo);
    h.layout(100.0, 100.0);
    let list = h.paint();

    assert_eq!(list.quads().count(), 10);
    for q in list.quads() {
        assert!(q.rect.left() >= -0.5 && q.rect.right() <= 100.5);
        assert!(q.rect.top() >= -0.5 && q.rect.bottom() <= 100.5);
    }
}

#[test]
fn pointer_gives_local_coords() {
    let seen: Rc<Cell<Option<Point>>> = Rc::new(Cell::new(None));
    struct Demo {
        seen: Rc<Cell<Option<Point>>>,
    }
    impl View for Demo {
        fn body(&self) -> impl View + 'static {
            let seen = self.seen.clone();
            Canvas::new().on_pointer(move |pos, ev, _ctx| {
                if let PointerEvent::Down { .. } = ev {
                    seen.set(Some(pos));
                }
            })
        }
    }
    let h = Harness::new(Demo { seen: seen.clone() });
    h.layout(200.0, 200.0);
    h.pointer_down(30.0, 40.0);

    assert_eq!(seen.get(), Some(Point::new(30.0, 40.0)));
}

#[test]
fn exact_key_binding_fires() {
    let fired: Rc<Cell<u32>> = Rc::new(Cell::new(0));
    struct Demo {
        fired: Rc<Cell<u32>>,
    }
    impl View for Demo {
        fn body(&self) -> impl View + 'static {
            let f = self.fired.clone();
            Canvas::new().key(Key::DELETE, Modifiers::NONE, move || {
                f.set(f.get() + 1);
            })
        }
    }
    let h = Harness::new(Demo {
        fired: fired.clone(),
    });
    h.layout(200.0, 200.0);
    h.pointer_down(10.0, 10.0);
    h.key(Key::DELETE);
    assert_eq!(fired.get(), 1);

    h.key_with_modifiers(Key::DELETE, Modifiers::SHIFT);
    assert_eq!(fired.get(), 1);
}

#[test]
fn on_key_fallback_when_no_exact_binding() {
    let seen: Rc<Cell<bool>> = Rc::new(Cell::new(false));
    struct Demo {
        seen: Rc<Cell<bool>>,
    }
    impl View for Demo {
        fn body(&self) -> impl View + 'static {
            let s = self.seen.clone();
            Canvas::new()
                .key(Key::DELETE, Modifiers::NONE, || {})
                .on_key(move |_| {
                    s.set(true);
                    true
                })
        }
    }
    let h = Harness::new(Demo { seen: seen.clone() });
    h.layout(200.0, 200.0);
    h.pointer_down(10.0, 10.0);
    h.key(Key::from_char('a'));
    assert!(seen.get());
}

#[test]
fn repaint_on_tracked_signal() {
    let dots: Signal<Vec<Point>> = Signal::new(vec![]);
    struct Demo {
        dots: Signal<Vec<Point>>,
    }
    impl View for Demo {
        fn body(&self) -> impl View + 'static {
            let d = self.dots.clone();
            let t = self.dots.clone();
            Canvas::new()
                .on_draw(move |ctx| {
                    for p in d.get_untracked().iter() {
                        ctx.fill(
                            Rect::from_xywh(p.x - 2.0, p.y - 2.0, 4.0, 4.0),
                            Fill::solid(FG),
                        );
                    }
                })
                .track(move || {
                    t.get();
                })
        }
    }
    let h = Harness::new(Demo { dots: dots.clone() });
    h.layout(200.0, 200.0);
    assert_eq!(h.paint().quads().count(), 0);
    h.set(&dots, vec![Point::new(10.0, 10.0), Point::new(50.0, 50.0)]);
    h.layout(200.0, 200.0);
    assert_eq!(h.paint().quads().count(), 2);
}

#[test]
fn primitives_paint_without_panics() {
    struct Demo;
    impl View for Demo {
        fn body(&self) -> impl View + 'static {
            Canvas::new().on_draw(|ctx| {
                let thin = BrushStroke::new(2.0);
                ctx.ellipse(Point::new(30.0, 30.0), 20.0, 10.0, FG);
                ctx.ellipse_outline(Point::new(70.0, 30.0), 20.0, 10.0, thin, FG);
                ctx.rect_outline(Rect::from_xywh(10.0, 60.0, 40.0, 30.0), 0.0, thin, FG);
                ctx.rect_outline(Rect::from_xywh(60.0, 60.0, 40.0, 30.0), 6.0, thin, FG);
                ctx.capsule(Point::new(10.0, 110.0), Point::new(60.0, 110.0), 5.0, FG);
                ctx.arrow(
                    Point::new(70.0, 110.0),
                    Point::new(110.0, 130.0),
                    3.0,
                    10.0,
                    8.0,
                    FG,
                );
                ctx.cross(Point::new(130.0, 30.0), 8.0, 2.0, FG);
                ctx.x_marker(Point::new(160.0, 30.0), 8.0, 2.0, FG);
                ctx.regular_polygon(Point::new(130.0, 70.0), 12.0, 6, 0.0, FG);
                ctx.star(Point::new(160.0, 70.0), 12.0, 5.0, 5, -90.0, FG);

                ctx.ellipse(Point::new(5.0, 5.0), 0.0, 10.0, FG);
                ctx.arrow(
                    Point::new(5.0, 5.0),
                    Point::new(5.0, 5.0),
                    2.0,
                    6.0,
                    5.0,
                    FG,
                );
            })
        }
    }
    let h = Harness::new(Demo);
    h.layout(200.0, 200.0);
    let list = h.paint();
    assert!(
        list.quads().count() >= 10,
        "each primitive adds quads: {}",
        list.quads().count()
    );
}

#[test]
fn dots_grid_count_matches_visible() {
    struct Demo;
    impl View for Demo {
        fn body(&self) -> impl View + 'static {
            Canvas::new().on_draw(|ctx| {
                ctx.dots(20.0, Vec2::ZERO, 1.5, GRID);
            })
        }
    }
    let h = Harness::new(Demo);
    h.layout(100.0, 100.0);
    let list = h.paint();

    assert_eq!(list.quads().count(), 25);
}

#[test]
fn split_grid_marks_every_fifth_line() {
    struct Demo;
    impl View for Demo {
        fn body(&self) -> impl View + 'static {
            Canvas::new().on_draw(|ctx| {
                ctx.grid_split(20.0, 5, Vec2::ZERO, GRID, FG);
            })
        }
    }
    let h = Harness::new(Demo);
    h.layout(100.0, 100.0);
    let list = h.paint();

    assert_eq!(list.quads().count(), 10);
}

#[test]
fn marquee_axes_and_frame_paint() {
    struct Demo;
    impl View for Demo {
        fn body(&self) -> impl View + 'static {
            Canvas::new().on_draw(|ctx| {
                ctx.axes(Point::new(50.0, 50.0), GRID, 1.0);
                ctx.cross(Point::new(20.0, 20.0), 6.0, 2.0, FG);
                ctx.frame(Rect::from_xywh(60.0, 60.0, 30.0, 20.0), FG, 2.0);
                let marquee = Rect::from_xywh(10.0, 120.0, 50.0, 30.0);
                ctx.fill(marquee, Fill::solid(Color::rgba(1.0, 1.0, 1.0, 0.2)));
                ctx.rect_outline(marquee, 0.0, BrushStroke::new(1.0), FG);

                let degenerate = Rect::from_xywh(5.0, 5.0, 0.0, 0.0);
                ctx.fill(degenerate, Fill::solid(FG));
                ctx.rect_outline(degenerate, 0.0, BrushStroke::new(1.0), FG);
            })
        }
    }
    let h = Harness::new(Demo);
    h.layout(200.0, 200.0);
    let list = h.paint();
    assert!(
        list.quads().count() >= 6,
        "axes + crosshair + frame + marquee: {}",
        list.quads().count()
    );
}

#[test]
fn canvas_cursor_reaches_runtime() {
    struct Demo;
    impl View for Demo {
        fn body(&self) -> impl View + 'static {
            Canvas::new().cursor(CursorIcon::Crosshair)
        }
    }
    let h = Harness::new(Demo);
    h.layout(200.0, 200.0);
    h.pointer_move(10.0, 10.0);
    assert_eq!(h.runtime().cursor_icon(), CursorIcon::Crosshair);
}

#[test]
fn lod_tiers_hide_detail_when_zoomed_out() {
    let zoom: Signal<f32> = Signal::new(1.0);
    struct Demo {
        zoom: Signal<f32>,
    }
    impl View for Demo {
        fn body(&self) -> impl View + 'static {
            let z = self.zoom.clone();
            let t = self.zoom.clone();
            Canvas::new()
                .on_draw(move |ctx| {
                    let z = z.get_untracked();

                    ctx.fill(
                        Rect::from_xywh(10.0, 10.0, 168.0 * z, 82.0 * z),
                        Fill::solid(FG),
                    );

                    if Canvas::lod_banded(z, &[1.0, 0.55, 0.3]) <= 1 {
                        ctx.circle(Point::new(10.0, 51.0), 5.0 * z, Fill::solid(FG));
                        ctx.circle(Point::new(178.0 * z, 51.0), 5.0 * z, Fill::solid(FG));
                    }

                    if Canvas::show_detail(z, 168.0, 60.0) {
                        ctx.fill(
                            Rect::from_xywh(20.0, 40.0, 44.0 * z, 44.0 * z),
                            Fill::solid(GRID),
                        );
                    }
                })
                .track(move || {
                    t.get();
                })
        }
    }
    let h = Harness::new(Demo { zoom: zoom.clone() });
    h.layout(300.0, 300.0);

    assert_eq!(h.paint().quads().count(), 4);

    h.set(&zoom, 0.2);
    h.layout(300.0, 300.0);
    assert_eq!(h.paint().quads().count(), 1);

    h.set(&zoom, 0.4);
    h.layout(300.0, 300.0);
    assert_eq!(h.paint().quads().count(), 2);
}
