#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::cell::Cell;
use std::rc::Rc;

use sequoia_core::{Color, Length, Point, RenderItem};
use sequoia_elements::{Container, DragValue};
use sequoia_reactive::Signal;
use sequoia_test::Harness;
use sequoia_text::TextSystem;
use sequoia_view::prelude::*;

const FONT: f32 = 14.0;
const LINE_W: f32 = 2.0;
const PAD_X: f32 = 6.0;

const LINE_Y: f32 = FONT * 1.35 + 3.0;

fn has_fonts() -> bool {
    TextSystem::new().shape("0", FONT).glyph_count() > 0
}

#[test]
fn paints_value_with_underline_under_the_text() {
    let value = Signal::new(24.0);
    let h = Harness::new(
        DragValue::new(value)
            .font_size(FONT)
            .precision(1)
            .color(Color::RED)
            .line_color(Color::BLUE),
    );
    h.layout(200.0, 80.0);
    let rect = h.rect(h.root().unwrap()).unwrap();
    let list = h.paint();

    let quads: Vec<_> = list.quads().collect();
    assert_eq!(quads.len(), 1, "line alone");
    assert_eq!(quads[0].color, Color::BLUE);
    assert_eq!(quads[0].rect.left(), PAD_X);
    assert_eq!(quads[0].rect.top(), LINE_Y);
    assert_eq!(quads[0].rect.size.height, LINE_W);
    assert_eq!(quads[0].radius, LINE_W * 0.5, "rounded ends");

    assert_eq!(quads[0].rect.size.width, rect.size.width - PAD_X * 2.0);
    assert_eq!(rect.size.height, 24.0, "default height");

    if let Some(RenderItem::Text(draw)) = list
        .items()
        .iter()
        .find(|i| matches!(i, RenderItem::Text(_)))
    {
        assert_eq!(draw.color, Color::RED);
        let bottom = draw
            .glyphs
            .iter()
            .map(|g| g.rect.bottom())
            .fold(f32::MIN, f32::max);
        assert!(
            bottom <= LINE_Y,
            "glyphs above the line ({bottom} > {LINE_Y})"
        );
    }
}

#[test]
fn underline_follows_the_value_not_the_field() {
    let wide = Harness::new(
        DragValue::new(Signal::new(24.0))
            .font_size(FONT)
            .line_width(LINE_W)
            .width(240.0),
    );
    wide.layout(300.0, 80.0);
    let line = wide.paint().quads().next().unwrap().rect;

    let auto = Harness::new(
        DragValue::new(Signal::new(24.0))
            .font_size(FONT)
            .line_width(LINE_W),
    );
    auto.layout(300.0, 80.0);
    let auto_line = auto.paint().quads().next().unwrap().rect;

    assert_eq!(line.left(), PAD_X);
    assert_eq!(
        line.size.width, auto_line.size.width,
        "line follows the value, not the field width"
    );
    assert!(line.size.width < 240.0 - PAD_X * 2.0);
}

#[test]
fn drag_changes_value_and_clamps() {
    let value = Signal::new(50.0);
    let h = Harness::new(
        DragValue::new(value.clone())
            .min(0.0)
            .max(100.0)
            .sensitivity(1.0)
            .width(200.0),
    );
    h.layout(200.0, 80.0);

    h.pointer_move(150.0, 12.0);
    assert_eq!(value.get_untracked(), 50.0);

    h.pointer_down(100.0, 12.0);
    assert_eq!(value.get_untracked(), 50.0, "press alone changes nothing");

    h.pointer_move(130.0, 12.0);
    assert_eq!(value.get_untracked(), 80.0);
    h.pointer_move(90.0, 12.0);
    assert_eq!(value.get_untracked(), 40.0);
    h.pointer_up(90.0, 12.0);

    h.pointer_down(100.0, 12.0);
    h.pointer_move(400.0, 12.0);
    assert_eq!(value.get_untracked(), 100.0);
    h.pointer_up(400.0, 12.0);
    assert_eq!(value.get_untracked(), 100.0);
}

#[test]
fn drag_requests_redraw_and_shows_new_value() {
    let value = Signal::new(0.5);
    let h = Harness::new(
        DragValue::new(value.clone())
            .min(0.0)
            .max(100.0)
            .sensitivity(1.0)
            .width(200.0),
    );
    h.layout(200.0, 80.0);

    let frames = Rc::new(Cell::new(0u32));
    let counter = frames.clone();
    h.runtime()
        .set_redraw_hook(move || counter.set(counter.get() + 1));

    h.pointer_down(100.0, 12.0);
    let before = frames.get();
    h.pointer_move(140.0, 12.0);
    assert_eq!(value.get_untracked(), 40.5);
    assert!(frames.get() > before, "drag schedules a frame");

    h.tick();
    h.layout(200.0, 80.0);
    let list = h.paint();
    if has_fonts() {
        let glyphs: usize = list
            .items()
            .iter()
            .map(|i| match i {
                RenderItem::Text(t) => t.glyphs.len(),
                RenderItem::Quad(_)
                | RenderItem::Image(_)
                | RenderItem::Shadow(_)
                | RenderItem::Waveform(_) => 0,

                _ => 0,
            })
            .sum();
        assert_eq!(glyphs, "40.50".chars().count(), "new value is visible");
    }
}

#[test]
fn drag_works_inside_a_row_like_in_the_example() {
    let value = Signal::new(0.6);
    let h = Harness::new(
        Container::new().padding(12.0).width(280.0).child(
            Column::new()
                .gap(8.0)
                .child(Text::new("Speed").size(14.0))
                .child(
                    Row::new()
                        .gap(8.0)
                        .child(Text::new("Speed").size(14.0))
                        .child(DragValue::new(value.clone()).min(0.0).max(5.0).precision(1)),
                ),
        ),
    );
    h.layout(400.0, 300.0);

    let dv = find_drag_value(&h).expect("DragValue in the tree");
    let rect = h.rect(dv).unwrap();
    assert!(
        rect.size.width > 0.0 && rect.size.height > 0.0,
        "rect {rect:?}"
    );
    assert_eq!(
        h.type_name_of(dv)
            .as_deref()
            .map(|n| n.contains("DragValue")),
        Some(true)
    );

    let (x, y) = (rect.left() + 5.0, rect.top() + rect.size.height * 0.5);
    h.pointer_move(x, y);
    assert_eq!(value.get_untracked(), 0.6, "hover changes nothing");
    h.pointer_down(x, y);
    h.pointer_move(x + 20.0, y);
    assert!(
        (value.get_untracked() - 1.1).abs() < 1e-4,
        "value after drag: {}",
        value.get_untracked()
    );
    h.pointer_up(x + 20.0, y);
}

struct Host {
    value: Signal<f32>,
    frame: Signal<u32>,
}

impl View for Host {
    fn body(&self) -> impl View + 'static {
        let _ = self.frame.get();
        Column::new().child(
            DragValue::new(self.value.clone())
                .min(0.0)
                .max(100.0)
                .sensitivity(1.0)
                .width(200.0),
        )
    }
}

#[test]
fn drag_survives_a_parent_rebuild() {
    let value = Signal::new(50.0);
    let frame = Signal::new(0u32);
    let h = Harness::new(Host {
        value: value.clone(),
        frame: frame.clone(),
    });
    h.layout(300.0, 200.0);
    let dv = find_drag_value(&h).expect("DragValue in the tree");
    let rect = h.rect(dv).unwrap();
    let (x, y) = (rect.left() + 5.0, rect.top() + rect.size.height * 0.5);

    h.pointer_down(x, y);
    h.pointer_move(x + 10.0, y);
    assert_eq!(value.get_untracked(), 60.0);

    h.set(&frame, 1);
    h.layout(300.0, 200.0);

    h.pointer_move(x + 20.0, y);
    assert_eq!(value.get_untracked(), 70.0, "drag survives parent rebuild");
    h.pointer_up(x + 20.0, y);
}

fn find_drag_value(h: &Harness) -> Option<ElementId> {
    find_view::<DragValue>(h)
}

fn find_view<T: View + 'static>(h: &Harness) -> Option<ElementId> {
    fn walk<T: View + 'static>(h: &Harness, id: ElementId) -> Option<ElementId> {
        if h.view::<T>(id).is_some() {
            return Some(id);
        }
        h.children(id).into_iter().find_map(|c| walk::<T>(h, c))
    }
    walk::<T>(h, h.root()?)
}

#[test]
fn row_defaults_to_content_height() {
    fn panel(value: Signal<f32>, row_height: Option<Length>) -> Harness {
        let row = match row_height {
            Some(h) => Row::new().gap(8.0).height(h),
            None => Row::new().gap(8.0),
        };
        Harness::new(
            Container::new()
                .padding(12.0)
                .width(280.0)
                .height(300.0)
                .child(
                    Column::new()
                        .gap(8.0)
                        .child(row.child(DragValue::new(value).min(0.0).max(5.0).precision(1)))
                        .child(Text::new("reszta panelu").size(14.0)),
                ),
        )
    }

    let auto = panel(Signal::new(0.6), None);
    auto.layout(400.0, 400.0);
    let row = auto.rect(find_view::<Row>(&auto).expect("Row")).unwrap();
    assert_eq!(
        row.size.height, 24.0,
        "default Auto: row sized to content: {row:?}"
    );

    let stretched = panel(Signal::new(0.6), Some(Length::Fill(1.0)));
    stretched.layout(400.0, 400.0);
    let row = stretched
        .rect(find_view::<Row>(&stretched).expect("Row"))
        .unwrap();
    assert!(
        row.size.height > 100.0,
        "explicit height(Fill) stretches the row: {row:?}"
    );
}

fn glyph_count(list: &sequoia_core::RenderList) -> usize {
    list.items()
        .iter()
        .map(|i| match i {
            RenderItem::Text(t) => t.glyphs.len(),
            RenderItem::Quad(_)
            | RenderItem::Image(_)
            | RenderItem::Shadow(_)
            | RenderItem::Waveform(_) => 0,

            _ => 0,
        })
        .sum()
}

#[test]
fn click_then_typing_sets_the_value() {
    let value = Signal::new(0.6);
    let h = Harness::new(
        DragValue::new(value.clone())
            .min(0.0)
            .max(100.0)
            .precision(1)
            .width(200.0)
            .height(40.0),
    );
    h.layout(200.0, 80.0);

    h.click(100.0, 20.0);
    assert!(h.runtime().interact().focus.is_some(), "field gets focus");

    h.key(Key::from_char('2'));
    h.key(Key::from_char('4'));
    h.key(Key::ENTER);
    assert_eq!(value.get_untracked(), 24.0);
}

#[test]
fn editing_is_visible_and_escape_cancels() {
    let value = Signal::new(1.5);
    let h = Harness::new(
        DragValue::new(value.clone())
            .min(0.0)
            .max(100.0)
            .precision(1)
            .font_size(FONT)
            .height(40.0),
    );
    h.layout(200.0, 80.0);
    let before = glyph_count(&h.paint());

    h.click(PAD_X + 4.0, 20.0);
    h.tick();
    h.layout(200.0, 80.0);

    if has_fonts() {
        assert_eq!(glyph_count(&h.paint()), before + 1, "edit caret");
    }

    h.key(Key::from_char('7'));
    h.key(Key::ESCAPE);
    assert_eq!(value.get_untracked(), 1.5, "Esc = no change");
}

#[test]
fn leaving_the_field_commits_editing() {
    let value = Signal::new(1.5);
    let h = Harness::new(
        DragValue::new(value.clone())
            .min(0.0)
            .max(100.0)
            .precision(1)
            .width(200.0)
            .height(40.0),
    );
    h.layout(200.0, 80.0);

    h.click(100.0, 20.0);
    h.key(Key::from_char('9'));
    assert_eq!(value.get_untracked(), 1.5, "no change before commit");

    h.runtime().set_focus(None);
    h.tick();
    h.layout(200.0, 80.0);
    let _ = h.paint();
    assert_eq!(value.get_untracked(), 9.0);
}

#[test]
fn typing_a_digit_starts_editing_when_focused() {
    let value = Signal::new(2.0);
    let h = Harness::new(
        DragValue::new(value.clone())
            .min(0.0)
            .max(100.0)
            .precision(1)
            .width(200.0)
            .height(40.0),
    );
    h.layout(200.0, 80.0);
    h.runtime().set_focus(h.root());

    h.key(Key::from_char('5'));
    h.key(Key::ENTER);
    assert_eq!(value.get_untracked(), 5.0);
}

#[test]
fn ctrl_click_restores_the_reset_value() {
    let value = Signal::new(3.0);
    let h = Harness::new(
        DragValue::new(value.clone())
            .reset_to(0.5)
            .min(0.0)
            .max(5.0)
            .precision(2)
            .width(200.0)
            .height(40.0),
    );
    h.layout(200.0, 80.0);

    let rt = h.runtime();
    let at = Point::new(100.0, 20.0);
    let mut text = sequoia_text::TextSystem::new();
    rt.pointer_down(at, PointerButton::Left, Modifiers::CONTROL, &mut text);
    rt.pointer_up(at, PointerButton::Left, Modifiers::CONTROL, &mut text);
    assert_eq!(value.get_untracked(), 0.5);
    assert!(
        rt.interact().captured.is_none(),
        "Ctrl+click does not steal the caret"
    );

    h.pointer_down(100.0, 20.0);
    h.pointer_move(110.0, 20.0);
    assert!(
        (value.get_untracked() - 0.75).abs() < 1e-4,
        "{}",
        value.get_untracked()
    );
    h.pointer_up(110.0, 20.0);
}

#[test]
fn default_sensitivity_is_range_per_200px() {
    let value = Signal::new(50.0);
    let h = Harness::new(
        DragValue::new(value.clone())
            .min(0.0)
            .max(200.0)
            .width(200.0),
    );
    h.layout(200.0, 80.0);

    h.pointer_down(50.0, 12.0);
    h.pointer_move(60.0, 12.0);
    assert_eq!(value.get_untracked(), 60.0);
    h.pointer_up(60.0, 12.0);
}

#[test]
fn precision_rounds_dragged_value() {
    let value = Signal::new(0.0);
    let h = Harness::new(
        DragValue::new(value.clone())
            .min(0.0)
            .max(10.0)
            .sensitivity(0.25)
            .precision(1)
            .width(200.0),
    );
    h.layout(200.0, 80.0);

    h.pointer_down(50.0, 12.0);
    h.pointer_move(55.0, 12.0);
    assert!((value.get_untracked() - 1.3).abs() < 1e-4);
    h.pointer_up(55.0, 12.0);
}

#[test]
fn vertical_drag_increases_upwards() {
    let value = Signal::new(10.0);
    let h = Harness::new(
        DragValue::new(value.clone())
            .min(0.0)
            .max(100.0)
            .sensitivity(1.0)
            .orientation(Axis::Vertical)
            .width(200.0)
            .height(40.0),
    );
    h.layout(200.0, 80.0);

    h.pointer_down(50.0, 20.0);
    h.pointer_move(50.0, 5.0);
    assert_eq!(value.get_untracked(), 25.0);
    h.pointer_move(50.0, 30.0);
    assert_eq!(value.get_untracked(), 0.0);
    h.pointer_up(50.0, 30.0);
}

#[test]
fn cursor_is_resize_while_dragging() {
    let value = Signal::new(0.5);
    let h = Harness::new(DragValue::new(value).width(200.0).height(40.0));
    h.layout(200.0, 80.0);

    assert_eq!(
        h.runtime().cursor_icon(),
        CursorIcon::Default,
        "outside the field"
    );
    h.pointer_move(100.0, 20.0);
    assert_eq!(h.runtime().cursor_icon(), CursorIcon::Grab);
    h.pointer_down(100.0, 20.0);
    assert_eq!(h.runtime().cursor_icon(), CursorIcon::ColResize);
    h.pointer_move(120.0, 20.0);
    assert_eq!(h.runtime().cursor_icon(), CursorIcon::ColResize);
    h.pointer_up(120.0, 20.0);
    assert_eq!(h.runtime().cursor_icon(), CursorIcon::Grab, "after release");

    let v = Signal::new(0.5);
    let h = Harness::new(
        DragValue::new(v)
            .orientation(Axis::Vertical)
            .width(200.0)
            .height(40.0),
    );
    h.layout(200.0, 80.0);
    h.pointer_move(100.0, 20.0);
    h.pointer_down(100.0, 20.0);
    assert_eq!(h.runtime().cursor_icon(), CursorIcon::RowResize);
}

#[test]
fn arrows_step_when_focused() {
    let value = Signal::new(5.0);
    let h = Harness::new(
        DragValue::new(value.clone())
            .min(0.0)
            .max(10.0)
            .precision(1)
            .width(200.0)
            .height(40.0),
    );
    h.layout(200.0, 80.0);

    h.key(Key::ARROW_RIGHT);
    assert_eq!(value.get_untracked(), 5.0);

    h.runtime().set_focus(h.root());

    h.key(Key::ARROW_UP);
    assert_eq!(value.get_untracked(), 5.5);
    h.key(Key::ARROW_LEFT);
    h.key(Key::ARROW_LEFT);
    assert_eq!(value.get_untracked(), 4.5);
}

#[test]
fn value_and_measure_follow_the_signal() {
    let value = Signal::new(1.0);
    let h = Harness::new(
        DragValue::new(value.clone())
            .min(0.0)
            .max(1_000_000.0)
            .line_width(LINE_W)
            .font_size(FONT),
    );
    h.layout(200.0, 80.0);
    let before = h.rect(h.root().unwrap()).unwrap().size.width;

    let rebuilds = h.stats().rebuilds;
    h.set(&value, 123456.0);
    assert!(
        h.stats().rebuilds > rebuilds,
        "value change schedules rebuild"
    );

    h.layout(200.0, 80.0);
    let after = h.rect(h.root().unwrap()).unwrap().size.width;
    let line = h.paint().quads().next().unwrap().rect.size.width;
    assert_eq!(line, after - PAD_X * 2.0, "line sized to the value");
    if has_fonts() {
        assert!(
            after > before,
            "width follows longer text: {before} -> {after}"
        );
    }
}

#[test]
fn nan_bounds_do_not_poison_the_signal() {
    let value = Signal::new(0.5);
    let h = Harness::new(
        DragValue::new(value.clone())
            .min(f32::NAN)
            .max(f32::NAN)
            .sensitivity(1.0)
            .width(200.0),
    );
    h.layout(200.0, 80.0);

    h.pointer_down(100.0, 12.0);
    h.pointer_move(160.0, 12.0);
    assert!(
        value.get_untracked().is_finite(),
        "po draggu z NaN granicami: {}",
        value.get_untracked()
    );
    h.pointer_up(160.0, 12.0);
}

#[test]
fn nan_pointer_move_does_not_poison_the_signal() {
    let value = Signal::new(50.0);
    let h = Harness::new(
        DragValue::new(value.clone())
            .min(0.0)
            .max(100.0)
            .sensitivity(1.0)
            .width(200.0),
    );
    h.layout(200.0, 80.0);

    h.pointer_down(100.0, 12.0);
    h.pointer_move(f32::NAN, 12.0);
    assert!(
        value.get_untracked().is_finite(),
        "po ruchu NaN: {}",
        value.get_untracked()
    );
    h.pointer_up(100.0, 12.0);
}

#[test]
fn wheel_steps_value_like_slider() {
    let value = Signal::new(24.0);
    let h = Harness::new(DragValue::new(value.clone()).min(0.0).max(100.0));
    h.layout(200.0, 80.0);
    let r = h.rect(h.root().unwrap()).unwrap();

    h.wheel(r.left() + 4.0, r.top() + 4.0, -120.0);
    assert_eq!(value.get_untracked(), 25.0);

    h.wheel(r.left() + 4.0, r.top() + 4.0, 120.0);
    assert_eq!(value.get_untracked(), 24.0);

    h.wheel_with_modifiers(r.left() + 4.0, r.top() + 4.0, 0.0, -120.0, Modifiers::SHIFT);
    assert!((value.get_untracked() - 24.1).abs() < 1e-4);

    h.wheel_with_modifiers(
        r.left() + 4.0,
        r.top() + 4.0,
        0.0,
        -120.0,
        Modifiers::CONTROL,
    );
    assert!((value.get_untracked() - 34.1).abs() < 1e-4);
}
