#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use sequoia_core::Length;
use sequoia_elements::Button;
use sequoia_input::{Event, Key, PointerEvent};
use sequoia_reactive::Signal;
use sequoia_test::Harness;
use sequoia_view::prelude::*;

struct Logged {
    tag: &'static str,
    log: Rc<RefCell<Vec<String>>>,
    child: AnyView,
}

impl Logged {
    fn new(tag: &'static str, log: Rc<RefCell<Vec<String>>>, child: impl View + 'static) -> Self {
        Self {
            tag,
            log,
            child: AnyView::new(child),
        }
    }
}

impl View for Logged {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn children_override(&self) -> Option<Vec<AnyView>> {
        Some(vec![self.child.clone()])
    }

    fn on_event(&self, _event: &Event, ctx: &mut EventCtx) {
        let phase = match ctx.phase {
            EventPhase::Capture => "C",
            EventPhase::Target => "T",
            EventPhase::Bubble => "B",
        };
        self.log.borrow_mut().push(format!("{}:{phase}", self.tag));
    }
}

struct Fixed {
    w: f32,
    h: f32,
}

impl View for Fixed {
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
}

#[test]
fn hit_test_picks_deepest_element() {
    let h = Harness::new(
        Column::new()
            .width(Length::Fill(1.0))
            .height(Length::Fill(1.0))
            .child(Fixed { w: 200.0, h: 100.0 })
            .child(Fixed { w: 200.0, h: 100.0 }),
    );
    h.layout(400.0, 400.0);

    let root = h.root().unwrap();
    let kids = h.children(root);
    assert_eq!(kids.len(), 2);

    assert_eq!(h.hit_test(100.0, 50.0), Some(kids[0]));
    assert_eq!(h.hit_test(100.0, 150.0), Some(kids[1]));

    assert_eq!(h.hit_test(350.0, 350.0), Some(root));
}

#[test]
fn click_dispatches_capture_target_bubble_through_layers() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let h = Harness::new(Logged::new(
        "outer",
        log.clone(),
        Logged::new("inner", log.clone(), Fixed { w: 200.0, h: 100.0 }),
    ));
    h.layout(400.0, 400.0);

    h.pointer_down(100.0, 50.0);

    assert_eq!(
        *log.borrow(),
        vec![
            "outer:C".to_string(),
            "inner:C".to_string(),
            "inner:B".to_string(),
            "outer:B".to_string(),
        ],
        "Capture→Target(pusty log)→Bubble"
    );
}

#[test]
fn stop_propagation_blocks_outer_bubble() {
    struct Stopper {
        child: AnyView,
    }

    impl View for Stopper {
        fn body(&self) -> impl View + 'static {
            Unit
        }

        fn children_override(&self) -> Option<Vec<AnyView>> {
            Some(vec![self.child.clone()])
        }

        fn on_event(&self, event: &Event, ctx: &mut EventCtx) {
            if ctx.phase == EventPhase::Capture
                && matches!(event, Event::Pointer(PointerEvent::Down { .. }))
            {
                ctx.stop_propagation();
            }
        }
    }

    let log = Rc::new(RefCell::new(Vec::new()));
    let h = Harness::new(Logged::new(
        "outer",
        log.clone(),
        Stopper {
            child: AnyView::new(Fixed { w: 200.0, h: 100.0 }),
        },
    ));
    h.layout(400.0, 400.0);

    h.pointer_down(100.0, 50.0);

    let l = log.borrow();
    assert!(
        l.contains(&"outer:C".to_string()),
        "capture reached the root"
    );
    assert!(
        !l.contains(&"outer:B".to_string()),
        "bubble stopped by stop_propagation"
    );
}

#[test]
fn button_click_fires_and_outside_does_not() {
    struct Root(Rc<Cell<u32>>);

    impl View for Root {
        fn body(&self) -> impl View + 'static {
            let hits = self.0.clone();
            Column::new()
                .child(Button::labeled("Save").on_click(move || hits.set(hits.get() + 1)))
                .child(Fixed { w: 200.0, h: 100.0 })
        }
    }

    let hits = Rc::new(Cell::new(0u32));
    let h = Harness::new(Root(hits.clone()));
    h.layout(400.0, 400.0);

    let root = h.root().unwrap();
    let col = h.children(root)[0];
    let btn = h.children(col)[0];
    let r = h.rect(btn).unwrap();

    h.pointer_move(r.left() + 10.0, r.top() + 10.0);
    h.click(r.left() + 10.0, r.top() + 10.0);
    assert_eq!(hits.get(), 1);

    h.pointer_move(r.left() + 10.0, r.bottom() + 50.0);
    h.click(r.left() + 10.0, r.bottom() + 50.0);
    assert_eq!(hits.get(), 1, "click outside the button fires no action");
}

#[test]
fn button_release_outside_does_not_fire() {
    struct Root(Rc<Cell<u32>>);

    impl View for Root {
        fn body(&self) -> impl View + 'static {
            let hits = self.0.clone();
            Column::new()
                .child(Button::labeled("Save").on_click(move || hits.set(hits.get() + 1)))
                .child(Fixed { w: 200.0, h: 100.0 })
        }
    }

    let hits = Rc::new(Cell::new(0u32));
    let h = Harness::new(Root(hits.clone()));
    h.layout(400.0, 400.0);

    let btn = h.children(h.children(h.root().unwrap())[0])[0];
    let r = h.rect(btn).unwrap();

    h.pointer_move(r.left() + 10.0, r.top() + 10.0);
    h.pointer_down(r.left() + 10.0, r.top() + 10.0);
    h.pointer_up(r.left() + 10.0, r.bottom() + 50.0);
    assert_eq!(hits.get(), 0, "release outside the button is not a click");

    h.pointer_move(r.left() + 10.0, r.top() + 10.0);
    h.pointer_down(r.left() + 10.0, r.top() + 10.0);
    h.pointer_up(r.left() + 10.0, r.top() + 10.0);
    assert_eq!(hits.get(), 1);
}

#[test]
fn pointer_capture_receives_moves_outside_bounds() {
    struct CaptureBox {
        moves: Rc<Cell<u32>>,
    }

    impl View for CaptureBox {
        fn body(&self) -> impl View + 'static {
            Unit
        }

        fn is_leaf(&self) -> bool {
            true
        }

        fn layout_style(&self) -> LayoutStyle {
            LayoutStyle {
                width: Length::Px(100.0),
                height: Length::Px(100.0),
                ..LayoutStyle::default()
            }
        }

        fn captures_pointer(&self) -> bool {
            true
        }

        fn on_event(&self, event: &Event, _ctx: &mut EventCtx) {
            if matches!(event, Event::Pointer(PointerEvent::Move { .. })) {
                self.moves.set(self.moves.get() + 1);
            }
        }
    }

    let moves = Rc::new(Cell::new(0u32));
    let h = Harness::new(CaptureBox {
        moves: moves.clone(),
    });
    h.layout(400.0, 400.0);

    let box_el = h.root().unwrap();

    h.pointer_down(50.0, 50.0);
    assert_eq!(h.interact().captured, Some(box_el));

    h.pointer_move(380.0, 380.0);
    h.pointer_move(390.0, 390.0);
    assert_eq!(moves.get(), 2, "capture delivers Move outside bounds");

    h.pointer_up(390.0, 390.0);
    assert_eq!(h.interact().captured, None);
}

#[test]
fn hover_follows_pointer() {
    let h = Harness::new(
        Column::new()
            .width(Length::Fill(1.0))
            .height(Length::Fill(1.0))
            .child(Fixed { w: 100.0, h: 100.0 }),
    );
    h.layout(400.0, 400.0);

    let root = h.root().unwrap();
    let panel = h.children(root)[0];

    h.pointer_move(50.0, 50.0);
    assert_eq!(h.interact().hovered, Some(panel));

    h.pointer_move(350.0, 350.0);
    assert_eq!(h.interact().hovered, Some(root));
}

#[test]
fn tab_moves_focus_and_wraps() {
    struct FocusBox;

    impl View for FocusBox {
        fn body(&self) -> impl View + 'static {
            Unit
        }

        fn is_leaf(&self) -> bool {
            true
        }

        fn focusable(&self) -> bool {
            true
        }

        fn layout_style(&self) -> LayoutStyle {
            LayoutStyle {
                width: Length::Px(40.0),
                height: Length::Px(40.0),
                ..LayoutStyle::default()
            }
        }
    }

    let h = Harness::new(Column::new().child(FocusBox).child(FocusBox));
    h.layout(400.0, 400.0);

    let root = h.root().unwrap();
    let a = h.children(root)[0];
    let b = h.children(root)[1];

    assert_eq!(h.interact().focus, None);
    h.key(Key::TAB);
    assert_eq!(h.interact().focus, Some(a));
    h.key(Key::TAB);
    assert_eq!(h.interact().focus, Some(b));
    h.key(Key::TAB);
    assert_eq!(h.interact().focus, Some(a), "wrap to start");
    h.key_with_shift(Key::TAB);
    assert_eq!(h.interact().focus, Some(b), "Shift+Tab backward");
}

#[test]
fn key_reaches_focused_element() {
    struct KeyBox {
        got: Rc<Cell<Option<Key>>>,
    }

    impl View for KeyBox {
        fn body(&self) -> impl View + 'static {
            Unit
        }

        fn is_leaf(&self) -> bool {
            true
        }

        fn focusable(&self) -> bool {
            true
        }

        fn layout_style(&self) -> LayoutStyle {
            LayoutStyle {
                width: Length::Px(40.0),
                height: Length::Px(40.0),
                ..LayoutStyle::default()
            }
        }

        fn on_event(&self, event: &Event, ctx: &mut EventCtx) {
            if let Event::Key(k) = event
                && ctx.phase == EventPhase::Target
            {
                self.got.set(Some(k.key));
                ctx.handled();
            }
        }
    }

    let got = Rc::new(Cell::new(None));
    let h = Harness::new(KeyBox { got: got.clone() });
    h.layout(400.0, 400.0);

    h.key(Key::TAB);
    h.key(Key::from_char('x'));
    assert_eq!(got.get(), Some(Key::from_char('x')));
}

#[test]
fn focus_and_pressed_survive_reactive_change() {
    use sequoia_reactive::Signal;

    let show = Signal::new(true);

    struct Root(Signal<bool>);

    impl View for Root {
        fn body(&self) -> impl View + 'static {
            if self.0.get() {
                AnyView::new(Button::labeled("Temp").on_click(|| {}))
            } else {
                AnyView::new(Fixed { w: 100.0, h: 100.0 })
            }
        }
    }

    let h = Harness::new(Root(show.clone()));
    h.layout(400.0, 400.0);

    let root = h.root().unwrap();
    let btn = h.children(root)[0];

    let pressable = h.children(btn)[0];
    assert!(
        h.type_name_of(pressable)
            .is_some_and(|n| n.contains("Container"))
    );
    h.pointer_down(20.0, 20.0);
    assert_eq!(h.interact().pressed, Some(pressable));
    assert_eq!(h.interact().focus, Some(pressable));

    h.pointer_up(20.0, 20.0);
    h.set(&show, false);

    let now = h.children(root)[0];
    assert_ne!(pressable, now, "remount after type change");
    h.key(Key::from_char('a'));
    assert_eq!(
        h.interact().focus,
        Some(pressable),
        "old focus stays (ignored at dispatch)"
    );
}

#[test]
fn event_sees_same_text_width_as_layout() {
    struct WidthProbe {
        seen: Rc<Cell<f32>>,
        seen_bold: Rc<Cell<f32>>,
    }

    impl View for WidthProbe {
        fn body(&self) -> impl View + 'static {
            Unit
        }

        fn is_leaf(&self) -> bool {
            true
        }

        fn layout_style(&self) -> LayoutStyle {
            LayoutStyle {
                width: Length::Px(300.0),
                height: Length::Px(40.0),
                ..LayoutStyle::default()
            }
        }

        fn on_event(&self, event: &Event, ctx: &mut EventCtx) {
            if matches!(event, Event::Pointer(_)) {
                self.seen.set(ctx.text_width("Hello", 14.0));
                self.seen_bold.set(ctx.text_width_weighted(
                    "Hello",
                    14.0,
                    sequoia_text::FontWeight::BOLD,
                ));
            }
        }
    }

    let seen = Rc::new(Cell::new(0.0f32));
    let seen_bold = Rc::new(Cell::new(0.0f32));
    let h = Harness::new(WidthProbe {
        seen: seen.clone(),
        seen_bold: seen_bold.clone(),
    });
    h.layout(400.0, 400.0);
    h.pointer_down(10.0, 10.0);

    let mut text = sequoia_text::TextSystem::new();
    assert!((seen.get() - text.text_width("Hello", 14.0)).abs() < 1e-4);
    assert!(
        (seen_bold.get() - text.text_width_weighted("Hello", 14.0, sequoia_text::FontWeight::BOLD))
            .abs()
            < 1e-4
    );
}

#[test]
fn button_activates_with_enter_and_space_when_focused() {
    let clicks = Signal::new(0u32);
    let ok_clicks = clicks.clone();
    let cancel_clicks = clicks.clone();
    let h = Harness::new(
        Column::new()
            .child(Button::labeled("OK").on_click(move || ok_clicks.update(|n| *n += 1)))
            .child(Button::labeled("Anuluj").on_click(move || cancel_clicks.update(|n| *n += 10))),
    );
    h.layout(400.0, 200.0);

    h.key(Key::TAB);
    h.key(Key::ENTER);
    assert_eq!(clicks.get_untracked(), 1, "Enter klika sfokusowany Button");
    h.key(Key::SPACE);
    assert_eq!(clicks.get_untracked(), 2, "Space clicks too");

    h.key(Key::TAB);
    h.key(Key::ENTER);
    assert_eq!(clicks.get_untracked(), 12);
}

#[test]
fn disabled_button_ignores_keyboard() {
    let clicks = Signal::new(0u32);
    let c = clicks.clone();
    let h = Harness::new(
        Button::labeled("Inactive")
            .on_click(move || c.update(|n| *n += 1))
            .disabled(true),
    );
    h.layout(400.0, 200.0);

    h.key(Key::TAB);
    assert_eq!(h.interact().focus, None);
    h.key(Key::ENTER);
    h.key(Key::SPACE);
    assert_eq!(clicks.get_untracked(), 0);
}
