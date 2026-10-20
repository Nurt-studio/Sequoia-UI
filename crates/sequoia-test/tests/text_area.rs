#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_core::RenderItem;
use sequoia_elements::{Clipboard, MemClipboard, SharedClipboard, TextArea, TextField};
use sequoia_reactive::{Environment, Signal};
use sequoia_test::Harness;
use sequoia_view::prelude::*;

const FONT: f32 = 14.0;

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

fn area(value: Signal<String>) -> Harness {
    Harness::new(
        TextArea::new(value)
            .font_size(FONT)
            .width(200.0)
            .height(80.0),
    )
}

fn focused(h: &Harness) {
    h.layout(200.0, 120.0);
    h.runtime().set_focus(h.root());
}

fn view_of(h: &Harness) -> std::rc::Rc<TextArea> {
    h.view::<TextArea>(h.root().unwrap()).expect("area")
}

#[test]
fn enter_creates_new_lines() {
    let value = Signal::new(String::new());
    let h = area(value.clone());
    focused(&h);

    for c in ['a', 'b'] {
        h.key(Key::from_char(c));
    }
    h.key(Key::ENTER);
    for c in ['c', 'd'] {
        h.key(Key::from_char(c));
    }
    assert_eq!(value.get_untracked(), "ab\ncd");
    let view = view_of(&h);
    assert_eq!(view.caret_line_col(), (1, 2));
    assert_eq!(view.line_count(), 2);
}

#[test]
fn enter_copies_current_line_indent() {
    let value = Signal::new("    foo".to_string());
    let h = area(value.clone());
    focused(&h);

    h.key(Key::END);
    h.key(Key::ENTER);
    assert_eq!(value.get_untracked(), "    foo\n    ");
}

#[test]
fn backspace_at_line_start_joins_lines() {
    use sequoia_input::Modifiers;

    let value = Signal::new("ab\ncd".to_string());
    let h = area(value.clone());
    focused(&h);

    h.key_with_modifiers(Key::END, Modifiers::CONTROL);
    h.key(Key::ARROW_LEFT);
    h.key(Key::ARROW_LEFT);
    assert_eq!(view_of(&h).caret(), 3);
    h.key(Key::BACKSPACE);
    assert_eq!(value.get_untracked(), "abcd");
    assert_eq!(view_of(&h).caret(), 2);
}

#[test]
fn delete_at_line_end_joins_lines() {
    use sequoia_input::Modifiers;

    let value = Signal::new("ab\ncd".to_string());
    let h = area(value.clone());
    focused(&h);

    h.key_with_modifiers(Key::HOME, Modifiers::CONTROL);
    h.key(Key::ARROW_RIGHT);
    h.key(Key::ARROW_RIGHT);
    assert_eq!(view_of(&h).caret(), 2);
    h.key(Key::DELETE);
    assert_eq!(value.get_untracked(), "abcd");
}

#[test]
fn up_and_down_move_between_lines() {
    let value = Signal::new("abcd\nef".to_string());
    let h = area(value.clone());
    focused(&h);

    h.key(Key::ARROW_DOWN);
    assert_eq!(view_of(&h).caret_line_col(), (1, 0));
    h.key(Key::from_char('X'));
    assert_eq!(value.get_untracked(), "abcd\nXef");

    h.key(Key::ARROW_UP);

    assert_eq!(view_of(&h).caret_line_col().0, 0);
}

#[test]
fn home_end_line_and_document() {
    use sequoia_input::Modifiers;

    let value = Signal::new("ab\ncde".to_string());
    let h = area(value.clone());
    focused(&h);

    h.key(Key::END);
    assert_eq!(view_of(&h).caret_line_col(), (0, 2));
    h.key_with_modifiers(Key::END, Modifiers::CONTROL);
    assert_eq!(view_of(&h).caret_line_col(), (1, 3));
    h.key(Key::HOME);
    assert_eq!(view_of(&h).caret_line_col(), (1, 0));
    h.key_with_modifiers(Key::HOME, Modifiers::CONTROL);
    assert_eq!(view_of(&h).caret(), 0);
}

#[test]
fn tab_inserts_indent_and_shift_tab_removes_it() {
    use sequoia_input::Modifiers;

    let value = Signal::new(String::new());
    let h = area(value.clone());
    focused(&h);

    h.key(Key::from_char('a'));
    h.key(Key::TAB);
    assert_eq!(value.get_untracked(), "a    ");
    h.key_with_modifiers(Key::TAB, Modifiers::SHIFT);
    assert_eq!(value.get_untracked(), "a");
}

#[test]
fn tab_keeps_focus_instead_of_moving_to_next_field() {
    let value = Signal::new(String::new());
    let other = Signal::new(String::new());
    let h = Harness::new(
        Column::new()
            .child(TextArea::new(value.clone()).font_size(FONT).height(60.0))
            .child(TextField::new(other.clone()).width(200.0)),
    );
    h.layout(200.0, 120.0);
    let _ = h.paint();

    h.pointer_down(10.0, 10.0);
    h.pointer_up(10.0, 10.0);
    h.key(Key::TAB);
    h.key(Key::from_char('b'));
    assert_eq!(value.get_untracked(), "    b");
    assert_eq!(other.get_untracked(), "");
}

#[test]
fn tab_indents_every_line_of_a_multiline_selection() {
    use sequoia_input::Modifiers;

    let value = Signal::new("a\nb".to_string());
    let h = area(value.clone());
    focused(&h);

    view_of(&h).select_all();
    h.key(Key::TAB);
    assert_eq!(value.get_untracked(), "    a\n    b");
    assert_eq!(view_of(&h).selection(), Some((4, 11)));

    h.key_with_modifiers(Key::TAB, Modifiers::SHIFT);
    assert_eq!(value.get_untracked(), "a\nb");
    assert_eq!(view_of(&h).selection(), Some((0, 3)));
}

#[test]
fn ctrl_word_jumps_and_ctrl_backspace() {
    use sequoia_input::Modifiers;

    let value = Signal::new("foo bar".to_string());
    let h = area(value.clone());
    focused(&h);

    h.key(Key::END);
    h.key_with_modifiers(Key::ARROW_LEFT, Modifiers::CONTROL);
    assert_eq!(view_of(&h).caret(), 4);

    h.key_with_modifiers(Key::BACKSPACE, Modifiers::CONTROL);
    assert_eq!(value.get_untracked(), "bar");
}

#[test]
fn max_lines_truncates_insertions() {
    let value = Signal::new(String::new());
    let h = Harness::new(
        TextArea::new(value.clone())
            .font_size(FONT)
            .max_lines(2)
            .width(200.0)
            .height(80.0),
    );
    focused(&h);

    h.key(Key::from_char('a'));
    h.key(Key::ENTER);
    h.key(Key::from_char('b'));
    h.key(Key::ENTER);
    h.key(Key::from_char('c'));
    assert_eq!(view_of(&h).line_count(), 2);
    assert_eq!(value.get_untracked(), "a\nbc");
}

fn area_with_clipboard(value: Signal<String>) -> (Harness, MemClipboard) {
    use std::rc::Rc;

    let h = area(value);
    let clip = MemClipboard::new();
    let env = Environment::new();
    env.provide(SharedClipboard(Rc::new(clip.clone())));
    h.set_environment(env);
    h.layout(200.0, 120.0);
    h.runtime().set_focus(h.root());
    (h, clip)
}

#[test]
fn clipboard_preserves_line_breaks() {
    use sequoia_input::Modifiers;

    let value = Signal::new("a\nb".to_string());
    let (h, clip) = area_with_clipboard(value.clone());

    view_of(&h).select_all();
    h.key_with_modifiers(Key::from_char('c'), Modifiers::CONTROL);
    assert_eq!(clip.content(), "a\nb");

    h.key_with_modifiers(Key::from_char('x'), Modifiers::CONTROL);
    assert_eq!(value.get_untracked(), "");

    h.key_with_modifiers(Key::from_char('v'), Modifiers::CONTROL);
    assert_eq!(value.get_untracked(), "a\nb");

    clip.set_text("x\r\ny");
    h.key_with_modifiers(Key::from_char('v'), Modifiers::CONTROL);
    assert_eq!(value.get_untracked(), "a\nbx\ny");
}

#[test]
fn read_only_blocks_edits_but_keeps_navigation_and_copy() {
    use sequoia_input::Modifiers;

    let value = Signal::new("hello".to_string());
    let h = Harness::new(
        TextArea::new(value.clone())
            .font_size(FONT)
            .editable(false)
            .width(200.0)
            .height(80.0),
    );
    let clip = MemClipboard::new();
    let env = Environment::new();
    env.provide(SharedClipboard(std::rc::Rc::new(clip.clone())));
    h.set_environment(env);
    h.layout(200.0, 120.0);
    h.runtime().set_focus(h.root());

    h.key(Key::from_char('x'));
    h.key(Key::ENTER);
    h.key(Key::BACKSPACE);
    h.key(Key::TAB);
    assert_eq!(value.get_untracked(), "hello");

    clip.set_text("zzz");
    h.key_with_modifiers(Key::from_char('v'), Modifiers::CONTROL);
    assert_eq!(value.get_untracked(), "hello");

    h.key(Key::END);
    assert_eq!(view_of(&h).caret_line_col(), (0, 5));

    h.key_with_modifiers(Key::from_char('c'), Modifiers::CONTROL);
    assert_eq!(clip.content(), "hello");
}

#[test]
fn click_moves_caret_to_the_clicked_line() {
    let value = Signal::new("ab\ncde".to_string());
    let h = area(value.clone());
    h.layout(200.0, 120.0);
    let _ = h.paint();

    h.pointer_down(10.0, 32.0);
    h.pointer_up(10.0, 32.0);
    assert_eq!(view_of(&h).caret_line_col().0, 1);

    h.runtime().set_focus(h.root());
    h.key(Key::from_char('X'));
    let text = value.get_untracked();
    let lines: Vec<&str> = text.split('\n').collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], "ab");
    assert!(lines[1].contains('X'));
}

#[test]
fn wheel_scrolls_long_content() {
    let text = (0..20)
        .map(|i| format!("line{i}"))
        .collect::<Vec<_>>()
        .join("\n");
    let value = Signal::new(text);
    let h = area(value);
    h.layout(200.0, 80.0);
    let _ = h.paint();
    assert_eq!(view_of(&h).scroll_offset().y, 0.0);

    h.wheel(100.0, 40.0, -40.0);
    assert_eq!(view_of(&h).scroll_offset().y, 40.0);

    h.wheel(100.0, 40.0, 10_000.0);
    assert_eq!(view_of(&h).scroll_offset().y, 0.0);
}

#[test]
fn selection_api_spans_lines() {
    let value = Signal::new("ab\ncd".to_string());
    let h = area(value.clone());
    h.layout(200.0, 120.0);

    let view = view_of(&h);
    assert_eq!(view.selection(), None);
    view.select_range(1, 4);
    assert_eq!(view.selection(), Some((1, 4)));
    assert_eq!(view.selected_text().as_deref(), Some("b\nc"));
    view.select_range(4, 1);
    assert_eq!(view.selection(), Some((1, 4)));
    view.select_all();
    assert_eq!(view.selection(), Some((0, 5)));
    assert!(view.clear_selection());
    assert_eq!(view.selection(), None);
}

#[test]
fn placeholder_shows_until_text_appears() {
    let value = Signal::new(String::new());
    let h = Harness::new(
        TextArea::new(value.clone())
            .placeholder("Wpisz kod…")
            .font_size(FONT)
            .width(200.0)
            .height(80.0),
    );
    h.layout(200.0, 120.0);
    assert_eq!(glyph_count(&h.paint()), "Wpisz kod…".chars().count());

    h.set(&value, "x".to_string());
    h.layout(200.0, 120.0);
    assert_eq!(glyph_count(&h.paint()), 1);
}

#[test]
fn ime_commit_keeps_line_breaks() {
    let value = Signal::new(String::new());
    let h = area(value.clone());
    focused(&h);

    h.ime_commit("a\nb");
    assert_eq!(value.get_untracked(), "a\nb");
    assert_eq!(view_of(&h).caret_line_col(), (1, 1));
}

#[test]
fn typing_keeps_painting_background_and_caret() {
    use sequoia_style::Theme;

    let value = Signal::new("hi".to_string());
    let h = area(value.clone());
    h.layout(200.0, 120.0);
    let list = h.paint();

    assert_eq!(list.quads().count(), 1);

    h.runtime().set_focus(h.root());
    h.tick();
    h.layout(200.0, 120.0);
    let list = h.paint();
    let quads: Vec<_> = list.quads().collect();
    assert_eq!(quads.len(), 2, "background + caret: {quads:?}");
    assert!(
        quads.iter().any(|q| q.color == Theme::dark().surface),
        "missing background: {quads:?}"
    );
}

#[test]
fn ctrl_z_undoes_typing_and_ctrl_y_redoes_it() {
    use sequoia_input::Modifiers;

    let value = Signal::new(String::new());
    let h = area(value.clone());
    focused(&h);

    for c in ['a', 'b'] {
        h.key(Key::from_char(c));
    }
    assert_eq!(value.get_untracked(), "ab");

    h.key_with_modifiers(Key::from_char('z'), Modifiers::CONTROL);
    h.key_with_modifiers(Key::from_char('z'), Modifiers::CONTROL);
    assert_eq!(value.get_untracked(), "");
    h.key_with_modifiers(Key::from_char('y'), Modifiers::CONTROL);
    h.key_with_modifiers(Key::from_char('y'), Modifiers::CONTROL);
    assert_eq!(value.get_untracked(), "ab");
}

#[test]
fn arrows_and_backspace_step_over_graphemes() {
    let value = Signal::new("a👨‍👩‍👧b".to_string());
    let h = area(value.clone());
    focused(&h);

    h.key(Key::ARROW_RIGHT);
    assert_eq!(view_of(&h).caret(), 1);
    h.key(Key::ARROW_RIGHT);
    assert_eq!(view_of(&h).caret(), 6);
    h.key(Key::ARROW_LEFT);
    assert_eq!(view_of(&h).caret(), 1);

    h.key(Key::ARROW_RIGHT);
    h.key(Key::BACKSPACE);
    assert_eq!(value.get_untracked(), "ab");
    assert_eq!(view_of(&h).caret(), 1);
}
