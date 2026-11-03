#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_elements::{TextArea, TextField};
use sequoia_input::{ImeEvent, Key};
use sequoia_reactive::Signal;
use sequoia_test::Harness;

fn field(value: Signal<String>) -> Harness {
    let h = Harness::new(TextField::new(value).width(300.0).height(30.0));
    h.layout(320.0, 60.0);
    h.runtime().set_focus(h.root());
    h.tick();
    h.paint();
    h
}

fn area(value: Signal<String>) -> Harness {
    let h = Harness::new(TextArea::new(value).width(300.0).height(120.0));
    h.layout(320.0, 140.0);
    h.runtime().set_focus(h.root());
    h.tick();
    h.paint();
    h
}

#[test]
fn preedit_is_not_committed_until_commit() {
    let value = Signal::new("ab".to_string());
    let h = field(value.clone());
    h.key(Key::END);
    h.tick();

    h.ime_preedit("にほん", None);
    h.tick();
    assert_eq!(
        value.get_untracked(),
        "ab",
        "preedit stays out of the model"
    );

    h.ime_commit("日本");
    h.tick();
    assert_eq!(value.get_untracked(), "ab日本");
}

#[test]
fn commit_replaces_selection() {
    let value = Signal::new("abcdef".to_string());
    let h = field(value.clone());
    let root = h.root().unwrap();
    h.view::<TextField>(root).unwrap().select_range(1, 4);
    h.tick();

    h.ime_preedit("Z", None);
    h.tick();
    assert_eq!(value.get_untracked(), "abcdef");

    h.ime_commit("Z");
    h.tick();
    assert_eq!(value.get_untracked(), "aZef");
}

#[test]
fn dead_key_composition_commits_accented_char_once() {
    let value = Signal::new(String::new());
    let h = field(value.clone());

    h.ime_preedit("´", None);
    h.tick();
    assert_eq!(value.get_untracked(), "");

    h.ime_commit("é");
    h.tick();
    assert_eq!(value.get_untracked(), "é");
}

#[test]
fn plain_key_events_during_composition_do_not_insert() {
    let value = Signal::new(String::new());
    let h = field(value.clone());

    h.ime_preedit("n", None);
    h.tick();
    h.key(Key::from_char('n'));
    h.tick();
    assert_eq!(
        value.get_untracked(),
        "",
        "no double insert while composing"
    );

    h.ime_commit("ん");
    h.tick();
    assert_eq!(value.get_untracked(), "ん");
}

#[test]
fn ime_caret_rect_tracks_the_caret_not_the_field() {
    let value = Signal::new(String::new());
    let h = field(value.clone());
    let field_rect = h.rect(h.root().unwrap()).unwrap();

    let start = h.runtime().ime_cursor_rect().expect("caret rect");
    assert!(start.size.width < field_rect.size.width * 0.5);
    assert!(start.size.height > 0.0);
    assert!(start.left() >= field_rect.left() - 0.5);

    h.key(Key::from_char('a'));
    h.key(Key::from_char('b'));
    h.tick();
    h.paint();

    let moved = h.runtime().ime_cursor_rect().unwrap();
    assert!(moved.left() > start.left() + 1.0, "caret moved right");
    assert!(moved.left() < field_rect.right());
}

#[test]
fn preedit_cursor_positions_the_composition_caret() {
    let value = Signal::new(String::new());
    let h = field(value.clone());

    h.ime_preedit("nihon", None);
    h.tick();
    h.paint();
    let end = h.runtime().ime_cursor_rect().unwrap();

    h.ime_preedit("nihon", Some((2, 2)));
    h.tick();
    h.paint();
    let mid = h.runtime().ime_cursor_rect().unwrap();
    assert!(mid.left() < end.left() - 1.0, "caret inside the preedit");

    h.ime_preedit("nihon", None);
    h.tick();
    h.paint();
    let back = h.runtime().ime_cursor_rect().unwrap();
    assert!((back.left() - end.left()).abs() < 0.5);
}

#[test]
fn disabling_ime_drops_preedit_and_restores_caret_rect() {
    let value = Signal::new("ab".to_string());
    let h = field(value.clone());
    h.key(Key::END);
    h.tick();
    h.paint();
    let before = h.runtime().ime_cursor_rect().unwrap();

    h.ime_preedit("xx", None);
    h.tick();
    h.paint();
    let composing = h.runtime().ime_cursor_rect().unwrap();
    assert!(composing.left() > before.left());

    h.ime(ImeEvent::Disabled);
    h.tick();
    h.paint();
    let after = h.runtime().ime_cursor_rect().unwrap();
    assert!((after.left() - before.left()).abs() < 0.5);
    assert_eq!(value.get_untracked(), "ab");
}

#[test]
fn text_area_commit_replaces_selection() {
    let value = Signal::new("abcdef".to_string());
    let h = area(value.clone());
    let root = h.root().unwrap();
    h.view::<TextArea>(root).unwrap().select_range(1, 4);
    h.tick();

    h.ime_preedit("地", None);
    h.tick();
    assert_eq!(value.get_untracked(), "abcdef");

    h.ime_commit("地");
    h.tick();
    assert_eq!(value.get_untracked(), "a地ef");
}

#[test]
fn text_area_caret_rect_is_reported() {
    let value = Signal::new("one\ntwo".to_string());
    let h = area(value.clone());
    h.paint();
    let rect = h.runtime().ime_cursor_rect().expect("caret rect");
    let field_rect = h.rect(h.root().unwrap()).unwrap();
    assert!(rect.left() >= field_rect.left() - 0.5);
    assert!(rect.left() < field_rect.right());
    assert!(rect.top() >= field_rect.top() - 0.5);
    assert!(rect.bottom() <= field_rect.bottom() + 0.5);
}
