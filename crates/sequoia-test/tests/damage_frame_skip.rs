#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_core::Color;
use sequoia_elements::Container;
use sequoia_reactive::Signal;
use sequoia_test::Harness;
use sequoia_view::FrameOutcome;
use sequoia_view::prelude::*;

struct Static {
    label: String,
}

impl View for Static {
    fn body(&self) -> impl View + 'static {
        Text::new(self.label.clone())
    }
}

fn pulse_view() -> impl View {
    Pulse::new(Text::new("...").size(14.0))
        .min_opacity(0.2)
        .period_duration(std::time::Duration::from_millis(400))
}

#[test]
fn static_ui_skips_all_frames_after_first() {
    let h = Harness::new(Static {
        label: "hello".into(),
    });
    h.layout(400.0, 300.0);
    h.tick();

    assert_eq!(h.paint_if_dirty(), FrameOutcome::Painted);
    h.tick();

    for _ in 0..9 {
        assert_eq!(h.paint_if_dirty(), FrameOutcome::Skipped);
        h.tick();
    }

    let (painted, skipped) = h.damage_stats();
    assert_eq!(painted, 1);
    assert_eq!(skipped, 9);
}

#[test]
fn signal_change_forces_paint() {
    let signal = Signal::new(0u32);
    let value = signal.clone();
    let h = Harness::new(
        Column::new().child(Text::with(move || format!("n = {}", value.get())).size(14.0)),
    );
    h.layout(400.0, 300.0);
    h.tick();

    assert_eq!(h.paint_if_dirty(), FrameOutcome::Painted);
    h.tick();
    assert_eq!(h.paint_if_dirty(), FrameOutcome::Skipped);
    h.tick();

    signal.set(1);
    h.tick();
    assert_eq!(h.paint_if_dirty(), FrameOutcome::Painted);
    h.tick();
    assert_eq!(h.paint_if_dirty(), FrameOutcome::Skipped);
}

#[test]
fn hover_change_forces_paint() {
    let h = Harness::new(
        Column::new()
            .child(
                Container::new()
                    .width(100.0)
                    .height(40.0)
                    .background(Color::hex(0x2A2A2A))
                    .hover(Color::hex(0x323232))
                    .on_hover(Signal::new(false)),
            )
            .child(Text::new("x")),
    );
    h.layout(400.0, 300.0);
    h.tick();
    assert_eq!(h.paint_if_dirty(), FrameOutcome::Painted);
    h.tick();
    assert_eq!(h.paint_if_dirty(), FrameOutcome::Skipped);

    h.pointer_move(50.0, 20.0);
    assert!(h.damage_rect().is_some());
    h.tick();
    assert_eq!(h.paint_if_dirty(), FrameOutcome::Painted);
    h.tick();
    assert_eq!(h.paint_if_dirty(), FrameOutcome::Skipped);
}

#[test]
fn animation_keeps_painting() {
    let h = Harness::new(pulse_view());
    h.layout(400.0, 300.0);
    h.tick();

    for _ in 0..5 {
        assert_eq!(h.paint_if_dirty(), FrameOutcome::Painted);
        h.tick();
    }

    let (painted, skipped) = h.damage_stats();
    assert_eq!(painted, 5);
    assert_eq!(skipped, 0);
}

#[test]
fn batch_of_signals_marks_one_frame() {
    let a = Signal::new(0u32);
    let b = Signal::new(0u32);
    let a2 = a.clone();
    let b2 = b.clone();
    let h = Harness::new(
        Column::new()
            .child(Text::with(move || format!("a = {}", a2.get())))
            .child(Text::with(move || format!("b = {}", b2.get()))),
    );
    h.layout(400.0, 300.0);
    h.tick();
    assert_eq!(h.paint_if_dirty(), FrameOutcome::Painted);
    h.tick();

    a.set(1);
    b.set(2);
    h.tick();
    assert_eq!(h.paint_if_dirty(), FrameOutcome::Painted);
    let (painted, _) = h.damage_stats();
    assert_eq!(painted, 2);
}

#[test]
fn viewport_change_forces_paint() {
    let h = Harness::new(Text::new("resize"));
    h.layout(400.0, 300.0);
    h.tick();
    assert_eq!(h.paint_if_dirty(), FrameOutcome::Painted);
    h.tick();
    assert_eq!(h.paint_if_dirty(), FrameOutcome::Skipped);

    h.layout(500.0, 320.0);
    h.tick();
    assert_eq!(h.paint_if_dirty(), FrameOutcome::Painted);
    h.tick();
    assert_eq!(h.paint_if_dirty(), FrameOutcome::Skipped);
}

#[test]
fn focus_change_forces_paint() {
    let h = Harness::new(Text::new("focus"));
    h.layout(400.0, 300.0);
    h.tick();
    assert_eq!(h.paint_if_dirty(), FrameOutcome::Painted);
    h.tick();
    assert_eq!(h.paint_if_dirty(), FrameOutcome::Skipped);

    h.runtime().set_focus(h.root());
    h.tick();
    assert_eq!(h.paint_if_dirty(), FrameOutcome::Painted);
    h.tick();
    assert_eq!(h.paint_if_dirty(), FrameOutcome::Skipped);
}
