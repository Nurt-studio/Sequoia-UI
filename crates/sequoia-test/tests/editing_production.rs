#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeSet;
use std::rc::Rc;

use sequoia_elements::{Clipboard, MemClipboard, SharedClipboard, TextArea, TextField};
use sequoia_input::{Key, Modifiers};
use sequoia_reactive::{Environment, Signal};
use sequoia_style::FORM_ERROR;
use sequoia_test::Harness;
use sequoia_view::AccessibleRole;
use sequoia_view::prelude::*;

const FONT: f32 = 14.0;

fn field(value: Signal<String>) -> Harness {
    let h = Harness::new(TextField::new(value).width(240.0).height(30.0));
    h.layout(260.0, 60.0);
    h.runtime().set_focus(h.root());
    h.tick();
    h.paint();
    h
}

fn wrapped(value: Signal<String>) -> Harness {
    let h = Harness::new(
        TextArea::new(value)
            .font_size(FONT)
            .soft_wrap(true)
            .width(110.0)
            .height(140.0),
    );
    h.layout(130.0, 160.0);
    h.runtime().set_focus(h.root());
    h.tick();
    h.paint();
    h
}

fn area(value: Signal<String>, width: f32, height: f32) -> Harness {
    let h = Harness::new(
        TextArea::new(value)
            .font_size(FONT)
            .width(width)
            .height(height),
    );
    h.layout(width + 20.0, height + 20.0);
    h.runtime().set_focus(h.root());
    h.tick();
    h.paint();
    h
}

fn area_view(h: &Harness) -> Rc<TextArea> {
    h.view::<TextArea>(h.root().unwrap()).expect("area")
}

fn glyphs(list: &sequoia_core::RenderList) -> usize {
    list.items()
        .iter()
        .map(|i| match i {
            sequoia_core::RenderItem::Text(t) => t.glyphs.len(),
            _ => 0,
        })
        .sum()
}

fn glyph_rows(list: &sequoia_core::RenderList) -> usize {
    let mut rows = BTreeSet::new();
    for item in list.items() {
        if let sequoia_core::RenderItem::Text(t) = item {
            for g in &t.glyphs {
                rows.insert(g.rect.top().round() as i32);
            }
        }
    }
    rows.len()
}

#[test]
fn soft_wrap_moves_caret_between_visual_rows() {
    let text = "aaaa bbbb cccc dddd eeee ffff gggg hhhh";
    let h = wrapped(Signal::new(text.to_string()));
    let view = area_view(&h);
    view.set_caret(0);
    h.tick();

    h.key(Key::ARROW_DOWN);
    assert!(view.caret() > 0, "caret moved to the next visual row");
    assert_eq!(view.caret_line_col().0, 0, "still the same hard line");

    h.key(Key::ARROW_UP);
    assert_eq!(view.caret(), 0);
}

#[test]
fn home_and_end_stop_at_visual_row_edges() {
    let text = "aaaa bbbb cccc dddd eeee ffff";
    let total = text.chars().count();
    let h = wrapped(Signal::new(text.to_string()));
    let view = area_view(&h);
    view.set_caret(0);
    h.tick();

    h.key(Key::END);
    let first_end = view.caret();
    assert!(first_end > 0 && first_end < total);

    h.key_with_modifiers(Key::END, Modifiers::CONTROL);
    assert_eq!(view.caret(), total);

    h.key(Key::HOME);
    assert!(view.caret() > 0, "start of the last visual row");
    assert!(view.caret() < total);
}

#[test]
fn wrapped_text_paints_on_multiple_rows() {
    let h = wrapped(Signal::new("aaaa bbbb cccc dddd eeee ffff".to_string()));
    let list = h.paint();
    assert!(
        glyph_rows(&list) >= 2,
        "soft-wrapped glyphs use several rows"
    );
}

#[test]
fn click_on_wrapped_row_places_caret_there() {
    let h = wrapped(Signal::new("aaaa bbbb cccc dddd eeee ffff".to_string()));
    let rect = h.rect(h.root().unwrap()).unwrap();
    let line_h = FONT * 1.35;
    let y = rect.top() + 6.0 + line_h + 4.0;
    h.pointer_down(rect.left() + 20.0, y);
    h.pointer_up(rect.left() + 20.0, y);

    let view = area_view(&h);
    assert!(view.caret() > 0);
    assert_eq!(view.caret_line_col().0, 0);
}

#[test]
fn character_wrap_breaks_long_words() {
    use sequoia_view::WrapMode;

    let h = Harness::new(
        TextArea::new(Signal::new("abcdefghijklmnopqrstuvwxyz".to_string()))
            .font_size(FONT)
            .soft_wrap(true)
            .wrap_mode(WrapMode::Character)
            .width(70.0)
            .height(120.0),
    );
    h.layout(90.0, 140.0);
    h.runtime().set_focus(h.root());
    h.tick();

    let view = area_view(&h);
    view.set_caret(0);
    h.tick();
    h.key(Key::ARROW_DOWN);
    assert!(view.caret() > 0);
}

#[test]
fn secure_field_masks_and_blocks_copy() {
    let value = Signal::new("secret".to_string());
    let clip = MemClipboard::new();
    let env = Environment::new();
    env.provide(SharedClipboard(Rc::new(clip.clone())));
    let h = Harness::new(
        TextField::new(value.clone())
            .secure(true)
            .width(240.0)
            .height(30.0),
    );
    h.set_environment(env);
    h.layout(260.0, 60.0);
    h.runtime().set_focus(h.root());
    h.tick();

    let view = h.view::<TextField>(h.root().unwrap()).unwrap();
    let a11y = view.accessibility().expect("accessible field");
    assert_eq!(a11y.value.as_deref(), Some("••••••"));
    assert_eq!(
        glyphs(&h.paint()),
        6,
        "masked display has one glyph per char"
    );

    h.key_with_modifiers(Key::from_char('a'), Modifiers::CONTROL);
    h.key_with_modifiers(Key::from_char('c'), Modifiers::CONTROL);
    assert_eq!(clip.get_text(), None, "copy is blocked");

    h.key_with_modifiers(Key::from_char('x'), Modifiers::CONTROL);
    assert_eq!(value.get_untracked(), "secret", "cut is blocked");
}

#[test]
fn readonly_field_blocks_edits_but_allows_selection() {
    let value = Signal::new("abc".to_string());
    let h = Harness::new(
        TextField::new(value.clone())
            .readonly(true)
            .width(240.0)
            .height(30.0),
    );
    h.layout(260.0, 60.0);
    h.runtime().set_focus(h.root());
    h.tick();

    h.key(Key::from_char('x'));
    h.ime_commit("日");
    h.key(Key::BACKSPACE);
    assert_eq!(value.get_untracked(), "abc");

    h.key(Key::HOME);
    h.key_with_shift(Key::ARROW_RIGHT);
    let view = h.view::<TextField>(h.root().unwrap()).unwrap();
    assert_eq!(view.selection(), Some((0, 1)));
}

#[test]
fn disabled_field_is_not_focusable_and_ignores_input() {
    let value = Signal::new("abc".to_string());
    let h = Harness::new(
        TextField::new(value.clone())
            .disabled(true)
            .width(240.0)
            .height(30.0),
    );
    h.layout(260.0, 60.0);
    h.runtime().set_focus(h.root());
    h.tick();

    let view = h.view::<TextField>(h.root().unwrap()).unwrap();
    assert!(!view.focusable());
    assert!(!view.accepts_text_input());
    h.key(Key::from_char('x'));
    assert_eq!(value.get_untracked(), "abc");

    let a11y = view.accessibility().expect("accessible field");
    assert!(a11y.disabled);
}

#[test]
fn error_state_draws_error_border() {
    let h = Harness::new(
        TextField::new(Signal::new("x".to_string()))
            .error(true)
            .border(1.0, sequoia_core::Color::BLACK)
            .width(200.0)
            .height(30.0),
    );
    h.layout(220.0, 60.0);
    let list = h.paint();
    assert!(
        list.quads().any(|q| q.border_color == FORM_ERROR),
        "field error"
    );

    let h = Harness::new(
        TextArea::new(Signal::new("x".to_string()))
            .error(true)
            .width(200.0)
            .height(80.0),
    );
    h.layout(220.0, 100.0);
    let list = h.paint();
    assert!(
        list.quads().any(|q| q.border_color == FORM_ERROR),
        "area error"
    );
}

#[test]
fn text_field_ctrl_arrows_move_by_word() {
    let value = Signal::new("foo bar".to_string());
    let h = field(value.clone());
    let view = h.view::<TextField>(h.root().unwrap()).unwrap();
    h.key(Key::END);
    h.key_with_modifiers(Key::ARROW_LEFT, Modifiers::CONTROL);
    assert_eq!(view.caret(), 4);

    h.key_with_modifiers(Key::BACKSPACE, Modifiers::CONTROL);
    assert_eq!(value.get_untracked(), "bar");
}

#[test]
fn text_area_disabled_blocks_input_and_focus() {
    let value = Signal::new("abc".to_string());
    let h = Harness::new(
        TextArea::new(value.clone())
            .font_size(FONT)
            .disabled(true)
            .width(200.0)
            .height(80.0),
    );
    h.layout(220.0, 100.0);
    let _ = h.paint();
    h.pointer_down(20.0, 20.0);
    h.pointer_up(20.0, 20.0);
    h.key(Key::from_char('x'));

    assert_eq!(value.get_untracked(), "abc");
    let view = area_view(&h);
    assert!(!view.focusable());
    assert!(!view.accepts_text_input());
}

#[test]
fn accessibility_tree_exposes_value_caret_and_selection() {
    let value = Signal::new("abc".to_string());
    let field = TextField::new(value).width(200.0).height(30.0);
    let h = Harness::new(Column::new().child(field));
    h.layout(220.0, 60.0);
    h.runtime().set_focus(h.root());
    h.tick();

    let ctx = h
        .runtime()
        .accessibility_tree()
        .into_iter()
        .find(|n| n.role == AccessibleRole::TextField)
        .expect("field node");
    assert_eq!(ctx.value.as_deref(), Some("abc"));
    assert_eq!(ctx.caret, Some(0));
    assert_eq!(ctx.selection, None);
}

#[test]
fn auto_size_tracks_content_and_max_lines() {
    let h = Harness::new(
        TextArea::new(Signal::new("a\nb\nc\nd\ne".to_string()))
            .font_size(FONT)
            .auto_size(true)
            .width(200.0),
    );
    h.layout(220.0, 400.0);
    let five = h.rect(h.root().unwrap()).unwrap().size.height;
    assert!(five > 0.0);

    let long = (0..20)
        .map(|i| format!("line{i}"))
        .collect::<Vec<_>>()
        .join("\n");
    let capped = Harness::new(
        TextArea::new(Signal::new(long))
            .font_size(FONT)
            .auto_size(true)
            .max_lines(5)
            .width(200.0),
    );
    capped.layout(220.0, 400.0);
    let height = capped.rect(capped.root().unwrap()).unwrap().size.height;
    assert!(
        height <= five + 0.5,
        "max_lines caps the measured height: {height} vs {five}"
    );
}

#[test]
fn large_document_paints_only_visible_rows() {
    let text = (0..4000)
        .map(|i| format!("line {i} with some content"))
        .collect::<Vec<_>>()
        .join("\n");
    let h = area(Signal::new(text), 200.0, 80.0);
    let painted = glyphs(&h.paint());
    assert!(
        painted < 1000,
        "only visible rows are painted, got {painted} glyphs"
    );
}
