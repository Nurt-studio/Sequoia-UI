#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_core::Color;
use sequoia_elements::{ClickTrigger, Container};
use sequoia_input::{CursorIcon, Key};
use sequoia_reactive::Signal;
use sequoia_test::Harness;
use sequoia_view::prelude::*;

fn counter_button(count: &Signal<u32>) -> Container {
    let c = count.clone();
    Container::new()
        .child(Text::new("icon"))
        .width(200.0)
        .height(60.0)
        .on_click(move || c.update(|n| *n += 1))
}

#[test]
fn click_on_child_fires_once() {
    let count = Signal::new(0u32);
    let h = Harness::new(counter_button(&count));
    h.layout(300.0, 200.0);
    h.click(10.0, 10.0);
    h.tick();
    assert_eq!(count.get_untracked(), 1, "click on a child fires once");
}

#[test]
fn press_inside_release_outside_does_not_fire() {
    let count = Signal::new(0u32);
    let h = Harness::new(counter_button(&count));
    h.layout(300.0, 200.0);
    h.pointer_down(10.0, 10.0);
    h.pointer_up(500.0, 500.0);
    h.tick();
    assert_eq!(
        count.get_untracked(),
        0,
        "release outside the rect is not a click"
    );
}

#[test]
fn down_outside_up_inside_does_not_fire() {
    let count = Signal::new(0u32);
    let h = Harness::new(counter_button(&count));
    h.layout(300.0, 200.0);
    h.pointer_down(500.0, 500.0);
    h.pointer_up(10.0, 10.0);
    h.tick();
    assert_eq!(
        count.get_untracked(),
        0,
        "press started outside is not a click"
    );
}

#[test]
fn disabled_ignores_click_and_keyboard() {
    let count = Signal::new(0u32);
    let c = count.clone();
    let h = Harness::new(
        Container::new()
            .child(Text::new("icon"))
            .disabled(true)
            .on_click(move || c.update(|n| *n += 1)),
    );
    h.layout(300.0, 200.0);
    h.click(10.0, 10.0);
    h.key(Key::ENTER);
    h.tick();
    assert_eq!(count.get_untracked(), 0);
}

#[test]
fn keyboard_activates_focused() {
    let count = Signal::new(0u32);
    let h = Harness::new(counter_button(&count));
    h.layout(300.0, 200.0);

    h.click(10.0, 10.0);
    h.key(Key::ENTER);
    h.key(Key::SPACE);
    h.tick();
    assert_eq!(count.get_untracked(), 3);
}

#[test]
fn hover_mirror_tracks_pointer() {
    let hov = Signal::new(false);
    let h = Harness::new(
        Container::new()
            .child(Text::new("icon"))
            .width(200.0)
            .height(60.0)
            .hovered(hov.clone())
            .on_click(|| {}),
    );
    h.layout(300.0, 200.0);
    h.paint();
    h.tick();
    assert!(!hov.get_untracked());

    h.pointer_move(10.0, 10.0);
    h.paint();
    h.tick();
    assert!(hov.get_untracked(), "hover sets the mirror");

    h.pointer_move(500.0, 500.0);
    h.paint();
    h.tick();
    assert!(!hov.get_untracked(), "leave clears the mirror");
}

#[test]
fn cursor_is_pointer_over_child() {
    let h = Harness::new(counter_button(&Signal::new(0u32)));
    h.layout(300.0, 200.0);

    h.pointer_move(10.0, 10.0);
    assert_eq!(h.runtime().cursor_icon(), CursorIcon::Pointer);
    h.pointer_move(500.0, 500.0);
    assert_eq!(h.runtime().cursor_icon(), CursorIcon::Default);
}

#[test]
fn static_container_has_default_cursor_and_no_focus() {
    let h = Harness::new(
        Container::new()
            .width(200.0)
            .height(60.0)
            .child(Text::new("box")),
    );
    h.layout(300.0, 200.0);
    h.pointer_move(10.0, 10.0);
    assert_eq!(h.runtime().cursor_icon(), CursorIcon::Default);
    h.click(10.0, 10.0);
    h.key(Key::ENTER);
    h.tick();
}

#[test]
fn nested_button_wins_over_outer_container() {
    use sequoia_elements::Button;
    let outer = Signal::new(0u32);
    let inner = Signal::new(0u32);
    let (o, i) = (outer.clone(), inner.clone());
    let h = Harness::new(
        Container::new()
            .child(
                Column::new().child(Button::labeled("inner").on_click(move || {
                    i.update(|n| *n += 1);
                })),
            )
            .width(200.0)
            .height(60.0)
            .on_click(move || o.update(|n| *n += 1)),
    );
    h.layout(300.0, 200.0);
    h.click(5.0, 5.0);
    h.tick();
    assert_eq!(inner.get_untracked(), 1, "inner Button fires");
    assert_eq!(outer.get_untracked(), 0, "outer does not duplicate");
}

#[test]
fn icon_button_pattern_from_docs() {
    use sequoia_core::Length;
    fn icon_button(path: &str) -> impl View + 'static {
        Container::new()
            .background(Color::hex(0x0FA4FA))
            .border(0.1, Color::WHITE)
            .radius(4.)
            .padding(4.)
            .width(Length::Fit)
            .height(Length::Fit)
            .child(Icon::new(path).size(16.).color(Color::WHITE))
    }
    let fired = Signal::new(false);
    let f = fired.clone();
    let h = Harness::new(
        Container::new()
            .child(icon_button("play"))
            .on_click(move || f.set(true)),
    );
    h.layout(300.0, 200.0);
    let list = h.paint();
    assert!(!list.is_empty(), "container background paints");
    h.click(10.0, 10.0);
    h.tick();
    assert!(fired.get_untracked());
}

#[test]
fn button_with_icon_row_fires() {
    let fired = Signal::new(false);
    let f = fired.clone();
    let h = Harness::new(
        Container::new()
            .padding_xy(10.0, 5.0)
            .radius(6.0)
            .background(Color::hex(0x2A2A2A))
            .child(
                Row::new()
                    .gap(8.0)
                    .child(Icon::new("copy").size(16.0))
                    .child(Text::new("Duplikuj")),
            )
            .on_click(move || f.set(true)),
    );
    h.layout(300.0, 200.0);
    h.click(10.0, 10.0);
    h.tick();
    assert!(fired.get_untracked());
}

#[test]
fn on_click_wraps_container_gpui_style() {
    let fired = Signal::new(0u32);
    let f = fired.clone();
    let h = Harness::new(
        Container::new()
            .background(Color::hex(0x0FA4FA))
            .radius(4.0)
            .padding(4.0)
            .width(200.0)
            .height(60.0)
            .child(Icon::new("play").size(16.0).color(Color::WHITE))
            .on_click(move || f.update(|n| *n += 1)),
    );
    h.layout(300.0, 200.0);
    h.click(10.0, 10.0);
    h.pointer_down(10.0, 10.0);
    h.pointer_up(500.0, 500.0);
    h.tick();
    assert_eq!(fired.get_untracked(), 1, "click yes, outside release no");
}

#[test]
fn container_wraps_any_content() {
    let fired = Signal::new(false);
    let f = fired.clone();
    let h = Harness::new(
        Container::new()
            .width(200.0)
            .height(60.0)
            .child(
                Row::new()
                    .width(200.0)
                    .height(60.0)
                    .child(Text::new("row-button")),
            )
            .on_click(move || f.set(true)),
    );
    h.layout(300.0, 200.0);
    h.click(10.0, 10.0);
    h.tick();
    assert!(fired.get_untracked());
}

#[test]
fn on_hover_mirrors_like_hovered() {
    let hov = Signal::new(false);
    let h = Harness::new(
        Container::new()
            .width(200.0)
            .height(60.0)
            .child(Text::new("x"))
            .on_hover(hov.clone()),
    );
    h.layout(300.0, 200.0);
    h.pointer_move(10.0, 10.0);
    h.paint();
    h.tick();
    assert!(hov.get_untracked());
    h.pointer_move(500.0, 500.0);
    h.paint();
    h.tick();
    assert!(!hov.get_untracked());
}

#[test]
fn release_default_fires_only_on_up() {
    let fired = Signal::new(0u32);
    let f = fired.clone();
    let h = Harness::new(
        Container::new()
            .child(Text::new("icon"))
            .width(200.0)
            .height(60.0)
            .trigger(ClickTrigger::Release)
            .on_click(move || f.update(|n| *n += 1)),
    );
    h.layout(300.0, 200.0);
    h.pointer_down(10.0, 10.0);
    h.tick();
    assert_eq!(fired.get_untracked(), 0, "press does not fire in Release");
    h.pointer_up(10.0, 10.0);
    h.tick();
    assert_eq!(fired.get_untracked(), 1, "dopiero Up odpala");
}

#[test]
fn press_fires_on_down_without_up() {
    let fired = Signal::new(0u32);
    let f = fired.clone();
    let h = Harness::new(
        Container::new()
            .child(Text::new("icon"))
            .width(200.0)
            .height(60.0)
            .on_press(move || f.update(|n| *n += 1)),
    );
    h.layout(300.0, 200.0);

    h.pointer_down(10.0, 10.0);
    h.tick();
    assert_eq!(fired.get_untracked(), 1, "press fires on Down");
}

#[test]
fn press_does_not_double_fire_on_up() {
    let fired = Signal::new(0u32);
    let f = fired.clone();
    let h = Harness::new(
        Container::new()
            .child(Text::new("icon"))
            .width(200.0)
            .height(60.0)
            .on_press(move || f.update(|n| *n += 1)),
    );
    h.layout(300.0, 200.0);

    h.click(10.0, 10.0);
    h.tick();
    assert_eq!(fired.get_untracked(), 1, "exactly once per gesture");
}

#[test]
fn press_outside_down_does_not_fire_on_up() {
    let fired = Signal::new(0u32);
    let f = fired.clone();
    let h = Harness::new(
        Container::new()
            .child(Text::new("icon"))
            .width(200.0)
            .height(60.0)
            .on_press(move || f.update(|n| *n += 1)),
    );
    h.layout(300.0, 200.0);

    h.pointer_down(500.0, 500.0);
    h.pointer_up(10.0, 10.0);
    h.tick();
    assert_eq!(fired.get_untracked(), 0);
}

#[test]
fn press_disabled_ignores_down() {
    let fired = Signal::new(0u32);
    let f = fired.clone();
    let h = Harness::new(
        Container::new()
            .child(Text::new("icon"))
            .width(200.0)
            .height(60.0)
            .disabled(true)
            .on_press(move || f.update(|n| *n += 1)),
    );
    h.layout(300.0, 200.0);
    h.pointer_down(10.0, 10.0);
    h.key(Key::ENTER);
    h.tick();
    assert_eq!(fired.get_untracked(), 0);
}

#[test]
fn state_colors_are_opt_in() {
    use sequoia_style::{Theme, VisualState};
    let theme = Theme::dark();
    let idle_state = VisualState::default();
    let mut hovered = VisualState::default();
    hovered.hovered = true;
    let mut pressed = VisualState::default();
    pressed.pressed = true;

    let plain = Container::new().background(Color::RED).on_click(|| {});
    assert_eq!(plain.target_background(&theme, idle_state), Color::RED);
    assert_eq!(plain.target_background(&theme, hovered), Color::RED);
    assert_eq!(plain.target_background(&theme, pressed), Color::RED);

    let styled = Container::new()
        .background(Color::RED)
        .hover(Color::GREEN)
        .pressed(Color::BLUE)
        .disabled_background(Color::WHITE)
        .on_click(|| {});
    assert_eq!(styled.target_background(&theme, idle_state), Color::RED);
    assert_eq!(styled.target_background(&theme, hovered), Color::GREEN);
    assert_eq!(styled.target_background(&theme, pressed), Color::BLUE);

    let dis = Container::new().disabled(true).on_click(|| {});
    assert_eq!(
        dis.target_background(&theme, idle_state),
        theme.disabled,
        "disabled bez jawnego koloru = theme.disabled"
    );
}
