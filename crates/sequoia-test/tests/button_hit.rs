#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_core::Color;
use sequoia_elements::Button;
use sequoia_reactive::Signal;
use sequoia_test::Harness;
use sequoia_view::prelude::*;

#[test]
fn topos_topbar_pattern_fires() {
    let fired = Signal::new(0u32);
    let f = fired.clone();
    let h = Harness::new(AnyView::new(
        Button::new("+ Nowy szkic")
            .on_click(move || f.update(|n| *n += 1))
            .background(Color::WHITE)
            .text_color(Color::WHITE)
            .font_size(12.0)
            .radius(6.0)
            .padding_xy(12.0, 7.0),
    ));
    h.layout(300.0, 200.0);
    h.click(10.0, 10.0);
    h.tick();
    assert_eq!(fired.get_untracked(), 1, "topbar Button w AnyView odpala");
}

#[test]
fn topos_ribbon_pattern_fires() {
    let fired = Signal::new(0u32);
    let f = fired.clone();
    let h = Harness::new(AnyView::new(
        Button::ribbon("layers", "XY").on_click(move || f.update(|n| *n += 1)),
    ));
    h.layout(300.0, 200.0);
    h.click(10.0, 10.0);
    h.tick();
    assert_eq!(fired.get_untracked(), 1, "ribbon Button w AnyView odpala");
}

#[test]
fn bare_button_control_fires() {
    let fired = Signal::new(0u32);
    let f = fired.clone();
    let h = Harness::new(Button::new("go").on_click(move || f.update(|n| *n += 1)));
    h.layout(300.0, 200.0);
    h.click(10.0, 10.0);
    h.tick();
    assert_eq!(fired.get_untracked(), 1, "goly Button odpala (kontrola)");
}
