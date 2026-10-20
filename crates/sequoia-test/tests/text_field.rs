#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_core::{Color, RenderItem};
use sequoia_elements::{Clipboard, DragValue, MemClipboard, SharedClipboard, TextField};
use sequoia_reactive::{Environment, Signal};
use sequoia_test::Harness;
use sequoia_text::TextSystem;
use sequoia_view::prelude::*;

const FONT: f32 = 14.0;
const PAD_X: f32 = 6.0;
const LINE_W: f32 = 2.0;

const LINE_Y: f32 = (40.0 - (FONT * 1.35 + 3.0 + LINE_W)) / 2.0 + FONT * 1.35 + 3.0;

fn has_fonts() -> bool {
    TextSystem::new().shape("0", FONT).glyph_count() > 0
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

fn field(value: Signal<String>) -> Harness {
    Harness::new(
        TextField::new(value)
            .font_size(FONT)
            .width(200.0)
            .height(40.0),
    )
}

#[test]
fn typing_appends_to_the_signal() {
    let value = Signal::new(String::new());
    let h = field(value.clone());
    h.layout(200.0, 80.0);
    h.runtime().set_focus(h.root());

    h.key(Key::from_char('a'));
    h.key(Key::from_char('b'));
    h.key(Key::from_char('z'));
    assert_eq!(value.get_untracked(), "abz");

    h.key(Key::BACKSPACE);
    assert_eq!(value.get_untracked(), "ab");

    h.key(Key::DELETE);
    assert_eq!(value.get_untracked(), "ab");

    h.key(Key::SPACE);
    assert_eq!(value.get_untracked(), "ab ");
}

#[test]
fn arrows_and_home_end_move_the_caret() {
    let value = Signal::new("abc".to_string());
    let h = field(value.clone());
    h.layout(200.0, 80.0);
    h.runtime().set_focus(h.root());

    h.key(Key::from_char('X'));
    assert_eq!(value.get_untracked(), "Xabc");

    h.key(Key::END);
    h.key(Key::from_char('!'));
    assert_eq!(value.get_untracked(), "Xabc!");

    h.key(Key::ARROW_LEFT);
    h.key(Key::ARROW_LEFT);
    h.key(Key::from_char('Y'));
    assert_eq!(value.get_untracked(), "XabYc!");

    h.key(Key::HOME);
    h.key(Key::from_char('Z'));
    assert_eq!(value.get_untracked(), "ZXabYc!");
}

#[test]
fn click_places_the_caret() {
    let value = Signal::new("abcdef".to_string());
    let h = field(value.clone());
    h.layout(200.0, 80.0);

    h.pointer_move(20.0, 20.0);
    let _ = h.paint();

    h.pointer_down(PAD_X + 1.0, 20.0);
    h.pointer_up(PAD_X + 1.0, 20.0);
    h.key(Key::from_char('X'));
    assert_eq!(value.get_untracked(), "Xabcdef");

    h.pointer_down(PAD_X + 150.0, 20.0);
    h.pointer_up(PAD_X + 150.0, 20.0);
    h.key(Key::from_char('!'));
    assert_eq!(value.get_untracked(), "Xabcdef!");
}

#[test]
fn paints_text_only_and_caret_only_when_focused() {
    let value = Signal::new("abc".to_string());
    let h = field(value.clone());
    h.layout(200.0, 80.0);

    let list = h.paint();
    let quads: Vec<_> = list.quads().collect();
    assert_eq!(quads.len(), 0, "sam tekst, bez kreski");

    h.runtime().set_focus(h.root());
    h.tick();
    h.layout(200.0, 80.0);
    let list = h.paint();
    let quads: Vec<_> = list.quads().collect();
    assert_eq!(quads.len(), 1, "caret only");
    let caret = quads
        .iter()
        .find(|q| q.rect.size.width < LINE_W)
        .expect("belka");
    assert_eq!(caret.color.a, 1.0);
    assert_eq!(caret.rect.left(), PAD_X, "caret at text start");

    h.key(Key::END);
    h.tick();
    h.layout(200.0, 80.0);
    let list = h.paint();
    let caret = list
        .quads()
        .find(|q| q.rect.size.width < LINE_W)
        .expect("belka");
    if has_fonts() {
        assert!(caret.rect.left() > PAD_X, "caret moved: {:?}", caret.rect);
    }
}

#[test]
fn underline_is_opt_in_via_line_width() {
    let value = Signal::new("abc".to_string());
    let h = Harness::new(
        TextField::new(value.clone())
            .font_size(FONT)
            .line_width(LINE_W)
            .width(200.0)
            .height(40.0),
    );
    h.layout(200.0, 80.0);

    let list = h.paint();
    let quads: Vec<_> = list.quads().collect();
    assert_eq!(quads.len(), 1, "line without caret");
    assert_eq!(quads[0].rect.top(), LINE_Y);
    assert_eq!(quads[0].rect.size.height, LINE_W);
    assert_eq!(quads[0].rect.left(), PAD_X);
    assert_eq!(quads[0].rect.size.width, 200.0 - PAD_X * 2.0);
}

#[test]
fn placeholder_shows_until_text_appears() {
    if !has_fonts() {
        eprintln!("skip: no fonts in environment");
        return;
    }
    let value = Signal::new(String::new());
    let h = Harness::new(
        TextField::new(value.clone())
            .placeholder("Nazwa sceny")
            .font_size(FONT)
            .width(200.0)
            .height(40.0),
    );
    h.layout(200.0, 80.0);
    assert_eq!(glyph_count(&h.paint()), "Nazwa sceny".chars().count());

    h.set(&value, "Las".to_string());
    h.layout(200.0, 80.0);
    let list = h.paint();
    assert_eq!(glyph_count(&list), 3);
    let color = list
        .items()
        .iter()
        .find_map(|i| match i {
            RenderItem::Text(t) => Some(t.color),
            RenderItem::Quad(_)
            | RenderItem::Image(_)
            | RenderItem::Shadow(_)
            | RenderItem::Waveform(_) => None,

            _ => None,
        })
        .expect("text");
    assert_ne!(color, Color::TRANSPARENT);
}

#[test]
fn max_length_blocks_further_input() {
    let value = Signal::new(String::new());
    let h = Harness::new(
        TextField::new(value.clone())
            .max_length(3)
            .width(200.0)
            .height(40.0),
    );
    h.layout(200.0, 80.0);
    h.runtime().set_focus(h.root());
    for c in ['a', 'b', 'c', 'd'] {
        h.key(Key::from_char(c));
    }
    assert_eq!(value.get_untracked(), "abc");
}

#[test]
fn typing_requests_redraw() {
    use std::cell::Cell;
    use std::rc::Rc;

    let value = Signal::new(String::new());
    let h = field(value.clone());
    h.layout(200.0, 80.0);
    h.runtime().set_focus(h.root());

    let frames = Rc::new(Cell::new(0u32));
    let counter = frames.clone();
    h.runtime()
        .set_redraw_hook(move || counter.set(counter.get() + 1));

    let before = frames.get();
    h.key(Key::from_char('x'));
    assert!(frames.get() > before, "typing schedules a frame");
}

#[test]
fn cursor_over_the_field_is_a_text_caret() {
    let value = Signal::new(String::new());
    let h = field(value);
    h.layout(200.0, 80.0);

    assert_eq!(h.runtime().cursor_icon(), CursorIcon::Default);
    h.pointer_move(100.0, 20.0);
    assert_eq!(h.runtime().cursor_icon(), CursorIcon::Text);
}

#[test]
fn font_weight_reaches_text_and_widgets() {
    if !has_fonts() {
        eprintln!("skip: no fonts in environment");
        return;
    }

    let regular = Harness::new(Text::new("Speed").size(16.0));
    regular.layout(200.0, 60.0);
    let bold = Harness::new(Text::new("Speed").size(16.0).bold());
    bold.layout(200.0, 60.0);
    let w_regular = regular.rect(regular.root().unwrap()).unwrap().size.width;
    let w_bold = bold.rect(bold.root().unwrap()).unwrap().size.width;
    assert!(w_bold >= w_regular, "bold {w_bold} vs regular {w_regular}");

    let drag = Harness::new(
        DragValue::new(Signal::new(24.0))
            .font_size(FONT)
            .precision(1)
            .weight(FontWeight::SEMIBOLD),
    );
    drag.layout(200.0, 60.0);
    assert!(glyph_count(&drag.paint()) > 0);

    let name = Harness::new(
        TextField::new(Signal::new("Las".to_string()))
            .font_size(FONT)
            .bold(true)
            .width(200.0)
            .height(40.0),
    );
    name.layout(200.0, 60.0);
    assert_eq!(glyph_count(&name.paint()), 3);
}

fn field_with_clipboard(value: Signal<String>) -> (Harness, MemClipboard) {
    use std::rc::Rc;

    let h = field(value);
    let clip = MemClipboard::new();
    let env = Environment::new();
    env.provide(SharedClipboard(Rc::new(clip.clone())));
    h.set_environment(env);
    h.layout(200.0, 80.0);
    h.runtime().set_focus(h.root());
    (h, clip)
}

#[test]
fn ctrl_c_copies_ctrl_x_cuts_ctrl_v_pastes() {
    use sequoia_input::Modifiers;

    let value = Signal::new("hello".to_string());
    let (h, clip) = field_with_clipboard(value.clone());

    h.key_with_modifiers(Key::from_char('c'), Modifiers::CONTROL);
    assert_eq!(clip.content(), "hello", "C copies the whole field");
    assert_eq!(value.get_untracked(), "hello", "copy keeps text intact");

    h.key_with_modifiers(Key::from_char('x'), Modifiers::CONTROL);
    assert_eq!(clip.content(), "hello", "X kopiuje");
    assert_eq!(value.get_untracked(), "", "X clears the field");

    h.key(Key::from_char('!'));
    h.key(Key::HOME);
    h.key_with_modifiers(Key::from_char('v'), Modifiers::CONTROL);
    assert_eq!(value.get_untracked(), "hello!", "V pastes at caret");
}

#[test]
fn paste_filters_control_chars_and_respects_max_length() {
    use sequoia_input::Modifiers;

    let value = Signal::new(String::new());
    let h = Harness::new(
        TextField::new(value.clone())
            .max_length(4)
            .width(200.0)
            .height(40.0),
    );
    let clip = MemClipboard::new();
    let env = Environment::new();
    env.provide(SharedClipboard(std::rc::Rc::new(clip.clone())));
    h.set_environment(env);
    h.layout(200.0, 80.0);
    h.runtime().set_focus(h.root());

    clip.set_text("ab\ncd\x01ef");
    h.key_with_modifiers(Key::from_char('v'), Modifiers::CONTROL);
    assert_eq!(
        value.get_untracked(),
        "ab c",
        "newline becomes space, controls dropped, limit cuts"
    );
}

#[test]
fn clipboard_shortcuts_without_clipboard_are_noop() {
    use sequoia_input::Modifiers;

    let value = Signal::new("hello".to_string());
    let h = field(value.clone());
    h.layout(200.0, 80.0);
    h.runtime().set_focus(h.root());

    h.key_with_modifiers(Key::from_char('c'), Modifiers::CONTROL);
    h.key_with_modifiers(Key::from_char('x'), Modifiers::CONTROL);
    h.key_with_modifiers(Key::from_char('v'), Modifiers::CONTROL);
    assert_eq!(value.get_untracked(), "hello");
}

#[test]
fn select_all_paints_a_highlight() {
    use sequoia_input::Modifiers;
    use sequoia_style::Theme;

    let value = Signal::new("abcdef".to_string());
    let h = field(value.clone());
    h.layout(200.0, 80.0);
    h.runtime().set_focus(h.root());

    h.tick();
    h.layout(200.0, 80.0);
    assert_eq!(h.paint().quads().count(), 1);

    h.key_with_modifiers(Key::from_char('a'), Modifiers::CONTROL);
    h.tick();
    h.layout(200.0, 80.0);
    let list = h.paint();
    let quads: Vec<_> = list.quads().collect();
    assert_eq!(quads.len(), 2, "selection + caret: {quads:?}");
    assert!(
        quads
            .iter()
            .any(|q| q.color == Theme::dark().accent_pressed),
        "missing selection quad: {quads:?}"
    );
}

#[test]
fn selection_visible_without_focus_or_hover() {
    use sequoia_input::Modifiers;
    use sequoia_style::Theme;

    let value = Signal::new("abcdef".to_string());
    let h = field(value.clone());
    h.layout(200.0, 80.0);
    h.runtime().set_focus(h.root());

    h.key_with_modifiers(Key::from_char('a'), Modifiers::CONTROL);
    h.runtime().set_focus(None);
    h.pointer_move(199.0, 79.0);
    h.tick();
    h.layout(200.0, 80.0);
    let list = h.paint();
    let quads: Vec<_> = list.quads().collect();
    assert!(
        quads
            .iter()
            .any(|q| q.color == Theme::dark().accent_pressed),
        "selection exists but no highlight visible: {quads:?}"
    );
}

#[test]
fn mouse_drag_selects_text() {
    let value = Signal::new("abcdef".to_string());
    let h = field(value.clone());
    h.layout(200.0, 80.0);

    h.pointer_move(20.0, 20.0);
    let _ = h.paint();

    h.pointer_down(PAD_X + 1.0, 20.0);
    h.pointer_move(PAD_X + 150.0, 20.0);
    h.pointer_up(PAD_X + 150.0, 20.0);

    h.runtime().set_focus(h.root());
    h.key(Key::from_char('X'));
    assert_eq!(value.get_untracked(), "X");
}

#[test]
fn drag_from_middle_selects_a_range() {
    let value = Signal::new("abcdef".to_string());
    let h = Harness::new(
        TextField::new(value.clone())
            .font_size(FONT)
            .width(200.0)
            .height(40.0),
    );
    h.layout(200.0, 80.0);
    h.pointer_move(20.0, 20.0);
    let _ = h.paint();

    h.pointer_down(PAD_X + 150.0, 20.0);
    h.pointer_move(PAD_X + 1.0, 20.0);
    h.pointer_up(PAD_X + 1.0, 20.0);

    h.runtime().set_focus(h.root());
    h.key(Key::BACKSPACE);
    assert_eq!(value.get_untracked(), "", "backward drag selected all");
}

#[test]
fn selection_api_select_range_and_clear() {
    let value = Signal::new("abcdef".to_string());
    let h = field(value.clone());
    h.layout(200.0, 80.0);

    let view = h.view::<TextField>(h.root().unwrap()).expect("pole");
    assert_eq!(view.selection(), None);

    view.select_range(1, 4);
    assert_eq!(view.selection(), Some((1, 4)));
    assert_eq!(view.selected_text().as_deref(), Some("bcd"));

    view.select_range(4, 1);
    assert_eq!(view.selection(), Some((1, 4)));

    view.select_range(0, 99);
    assert_eq!(view.selection(), Some((0, 6)));
    view.select_range(2, 2);
    assert_eq!(view.selection(), None);

    view.select_all();
    assert_eq!(view.selection(), Some((0, 6)));
    assert!(view.clear_selection());
    assert_eq!(view.selection(), None);
    assert!(!view.clear_selection());
}

#[test]
fn ctrl_z_undoes_typing_and_ctrl_y_redoes_it() {
    use sequoia_input::Modifiers;

    let value = Signal::new(String::new());
    let h = field(value.clone());
    h.layout(200.0, 80.0);
    h.runtime().set_focus(h.root());

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
    let h = field(value.clone());
    h.layout(200.0, 80.0);
    h.runtime().set_focus(h.root());

    let view = h.view::<TextField>(h.root().unwrap()).expect("pole");
    h.key(Key::ARROW_RIGHT);
    assert_eq!(view.caret(), 1);
    h.key(Key::ARROW_RIGHT);
    assert_eq!(view.caret(), 6);
    h.key(Key::ARROW_LEFT);
    assert_eq!(view.caret(), 1);
    h.key(Key::ARROW_RIGHT);
    h.key(Key::BACKSPACE);
    assert_eq!(value.get_untracked(), "ab");
    assert_eq!(view.caret(), 1);
}

#[test]
fn enter_fires_on_submit() {
    use std::cell::RefCell;
    use std::rc::Rc;

    let value = Signal::new(String::new());
    let fired: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let fired2 = fired.clone();
    let h = Harness::new(
        TextField::new(value.clone())
            .width(200.0)
            .height(40.0)
            .on_submit(move |s| fired2.borrow_mut().push(s)),
    );
    h.layout(200.0, 80.0);
    h.runtime().set_focus(h.root());
    h.tick();
    h.layout(200.0, 80.0);
    let _ = h.paint();

    h.key(Key::from_char('a'));
    h.key(Key::from_char('b'));
    h.key(Key::ENTER);
    assert_eq!(*fired.borrow(), vec!["ab".to_string()]);

    assert_eq!(value.get_untracked(), "ab");
}

#[test]
fn long_text_stays_inside_and_caret_follows() {
    let value = Signal::new(String::new());
    let h = Harness::new(
        TextField::new(value.clone())
            .font_size(FONT)
            .width(120.0)
            .height(40.0),
    );
    h.layout(120.0, 80.0);
    h.runtime().set_focus(h.root());
    h.tick();

    for c in "abcdefghijklmnopqrstuvwxyz0123456789".chars() {
        h.key(Key::from_char(c));
    }
    assert_eq!(value.get_untracked().len(), 36);
    h.tick();
    h.layout(120.0, 80.0);
    let list = h.paint();

    let fr = h.rect(h.root().unwrap()).unwrap();

    let caret = list
        .quads()
        .find(|q| q.rect.size.width < LINE_W)
        .expect("belka");
    assert!(
        caret.rect.left() >= fr.left() - 0.5,
        "caret nie przed polem: {:?}",
        caret.rect
    );
    assert!(
        caret.rect.right() <= fr.right() + 0.5,
        "caret nie za polem: {:?} vs {fr:?}",
        caret.rect
    );

    for q in list.quads() {
        assert!(
            q.rect.left() >= fr.left() - 0.5 && q.rect.right() <= fr.right() + 0.5,
            "quad w polu: {:?} vs {fr:?}",
            q.rect
        );
    }
}
