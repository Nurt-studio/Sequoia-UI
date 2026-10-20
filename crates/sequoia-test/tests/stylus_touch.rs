#![allow(clippy::unwrap_used)]

use std::cell::RefCell;
use std::rc::Rc;

use sequoia_test::Harness;
use sequoia_ui::prelude::*;

#[derive(Clone)]
struct Probe {
    log: Rc<RefCell<Vec<String>>>,
}

impl View for Probe {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn children_override(&self) -> Option<Vec<AnyView>> {
        Some(vec![AnyView::new(Text::new("cel"))])
    }

    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fill(1.0),
            height: Length::Fill(1.0),
            ..LayoutStyle::default()
        }
    }

    fn on_event(&self, event: &Event, ctx: &mut EventCtx) {
        match event {
            Event::Pointer(PointerEvent::Down { .. }) => {
                self.log.borrow_mut().push(format!(
                    "down p={:.2} tool={:?}",
                    ctx.pressure(),
                    ctx.tool()
                ));
            }
            Event::Pointer(PointerEvent::TouchDown { id, pos, .. }) => {
                self.log
                    .borrow_mut()
                    .push(format!("touch {id} @{}", pos.x as i32));
            }
            Event::Pointer(PointerEvent::Pinch { delta, .. }) => {
                self.log.borrow_mut().push(format!("pinch {delta:.2}"));
            }
            _ => {}
        }
    }
}

fn probe_harness(log: Rc<RefCell<Vec<String>>>) -> Harness {
    Harness::new(Column::new().fill_size().child(Probe { log }))
}

#[test]
fn pen_click_acts_like_mouse_click_with_pressure() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let h = probe_harness(log.clone());
    h.sync(400.0, 300.0);
    assert_eq!(h.pointer_pressure(), 1.0, "myszka startuje z 1.0");
    h.pen_down(50.0, 50.0, 0.5);
    assert_eq!(h.pointer_pressure(), 0.5);
    h.pen_up(50.0, 50.0);
    h.tick();
    let log = log.borrow();
    assert!(
        log.iter().any(|l| l.starts_with("down p=0.50")),
        "Down normalized + force in ctx: {log:?}"
    );
}

#[test]
fn mouse_reports_tool_mouse_and_full_pressure() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let h = probe_harness(log.clone());
    h.sync(400.0, 300.0);
    h.click(60.0, 60.0);
    h.tick();
    let log = log.borrow();
    assert!(
        log.iter()
            .any(|l| l.contains("p=1.00") && l.contains("Mouse")),
        "{log:?}"
    );
}

#[test]
fn touch_tracks_fingers_and_dispatches_touch_events() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let h = probe_harness(log.clone());
    h.sync(400.0, 300.0);
    h.touch_down(7, 100.0, 100.0, Some(0.8));
    assert_eq!(h.active_touch_count(), 1);
    assert_eq!(h.pointer_pressure(), 0.8);
    h.touch_down(9, 200.0, 200.0, None);
    assert_eq!(h.active_touch_count(), 2);
    h.touch_move(7, 110.0, 110.0, None);
    h.touch_up(7, 110.0, 110.0);
    assert_eq!(h.active_touch_count(), 1);
    h.touch_up(9, 200.0, 200.0);
    assert_eq!(h.active_touch_count(), 0);
    h.tick();
    let log = log.borrow();
    assert!(log.iter().any(|l| l == "touch 7 @100"), "{log:?}");
}

#[test]
fn pinch_reaches_hit_target() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let h = probe_harness(log.clone());
    h.sync(400.0, 300.0);
    h.pinch(150.0, 150.0, 0.25);
    h.tick();
    let log = log.borrow();
    assert!(log.iter().any(|l| l == "pinch 0.25"), "{log:?}");
}

#[test]
fn pen_move_updates_pressure_without_click() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let h = probe_harness(log.clone());
    h.sync(400.0, 300.0);
    h.pen_move(10.0, 10.0, 0.3);
    assert_eq!(h.pointer_pressure(), 0.3);
    assert!(log.borrow().is_empty(), "sam ruch nie klika");
}

#[test]
fn touch_without_force_defaults_to_full_pressure() {
    let h = probe_harness(Rc::new(RefCell::new(Vec::new())));
    h.sync(400.0, 300.0);
    h.touch_down(1, 50.0, 50.0, None);
    assert_eq!(h.pointer_pressure(), 1.0);
    assert_eq!(
        PointerEvent::TouchDown {
            id: 1,
            pos: sequoia_core::Point::new(0.0, 0.0),
            force: None,
            modifiers: Modifiers::NONE,
        }
        .pressure(),
        1.0
    );
}
