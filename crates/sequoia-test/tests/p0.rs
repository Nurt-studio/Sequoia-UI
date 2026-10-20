#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::cell::Cell;
use std::rc::Rc;

use sequoia_elements::{Clipboard, MemClipboard, SharedClipboard, TextField};
use sequoia_input::{Event, ImeEvent, Key, Modifiers};
use sequoia_reactive::{Environment, Signal};
use sequoia_test::Harness;
use sequoia_text::{FontWeight, TextSystem};
use sequoia_view::prelude::*;

fn focused_field(value: Signal<String>) -> Harness {
    let h = Harness::new(TextField::new(value).width(300.0).height(40.0));
    h.layout(300.0, 80.0);
    h.runtime().set_focus(h.root());
    h.tick();
    h
}

#[test]
fn ime_commit_inserts_at_caret() {
    let value = Signal::new("ab".to_string());
    let h = focused_field(value.clone());
    h.key(Key::END);
    h.tick();
    h.ime_commit("zolc");
    h.tick();
    assert_eq!(value.get_untracked(), "abzolc");
}

#[test]
fn ime_preedit_does_not_touch_signal_but_repaints() {
    let value = Signal::new("ab".to_string());
    let h = focused_field(value.clone());
    let r0 = h.stats().rebuilds;
    h.ime_preedit("nǐ", None);
    h.tick();

    assert_eq!(value.get_untracked(), "ab");

    assert!(h.stats().rebuilds > r0, "preedit schedules a frame");

    h.ime_commit("n");
    h.tick();
    assert_eq!(value.get_untracked(), "nab");
}

#[test]
fn ime_disabled_clears_preedit() {
    let value = Signal::new(String::new());
    let h = focused_field(value.clone());
    h.ime_preedit("x", None);
    h.tick();
    h.ime(ImeEvent::Disabled);
    h.tick();
    h.ime_commit("y");
    h.tick();
    assert_eq!(value.get_untracked(), "y");
}

#[test]
fn shift_arrows_select_and_type_replaces() {
    let value = Signal::new("abcdef".to_string());
    let h = focused_field(value.clone());
    h.key(Key::HOME);
    h.tick();

    h.key_with_shift(Key::ARROW_RIGHT);
    h.key_with_shift(Key::ARROW_RIGHT);
    h.key_with_shift(Key::ARROW_RIGHT);
    h.tick();
    h.key(Key::from_char('X'));
    h.tick();
    assert_eq!(value.get_untracked(), "Xdef");
}

#[test]
fn ctrl_a_selects_all_then_copy() {
    let value = Signal::new("hello".to_string());
    let h = Harness::new(TextField::new(value.clone()).width(300.0).height(40.0));
    let clip = MemClipboard::new();
    let env = Environment::new();
    env.provide(SharedClipboard(Rc::new(clip.clone())));
    h.set_environment(env);
    h.layout(300.0, 80.0);
    h.runtime().set_focus(h.root());
    h.tick();

    h.key_with_modifiers(Key::from_char('a'), Modifiers::CONTROL);
    h.tick();

    h.key(Key::BACKSPACE);
    h.tick();
    assert_eq!(value.get_untracked(), "");
}

#[test]
fn clipboard_copy_uses_selection_not_whole_field() {
    let value = Signal::new("abcdef".to_string());
    let h = Harness::new(TextField::new(value.clone()).width(300.0).height(40.0));
    let clip = MemClipboard::new();
    let env = Environment::new();
    env.provide(SharedClipboard(Rc::new(clip.clone())));
    h.set_environment(env);
    h.layout(300.0, 80.0);
    h.runtime().set_focus(h.root());
    h.tick();

    h.key(Key::HOME);
    h.tick();
    h.key_with_shift(Key::ARROW_RIGHT);
    h.key_with_shift(Key::ARROW_RIGHT);
    h.key_with_shift(Key::ARROW_RIGHT);
    h.key_with_shift(Key::ARROW_RIGHT);
    h.tick();
    h.key_with_modifiers(Key::from_char('c'), Modifiers::CONTROL);
    h.tick();
    assert_eq!(clip.get_text().as_deref(), Some("abcd"));

    h.key_with_modifiers(Key::from_char('x'), Modifiers::CONTROL);
    h.tick();
    assert_eq!(value.get_untracked(), "ef");
    assert_eq!(clip.get_text().as_deref(), Some("abcd"));
}

struct Boom;

impl View for Boom {
    #[allow(unreachable_code)]
    fn body(&self) -> impl View + 'static {
        panic!("boom-body");
        Unit
    }
}

struct Calm;

impl View for Calm {
    fn body(&self) -> impl View + 'static {
        Unit
    }
    fn is_leaf(&self) -> bool {
        true
    }
    fn paint_self(&self, ctx: &mut PaintCtx) {
        ctx.quad(ctx.rect, sequoia_core::Color::WHITE, 0.0);
    }
}

#[test]
fn panicking_sibling_does_not_kill_tree() {
    let h = Harness::new(Column::new().child(Boom).child(Calm));
    h.layout(200.0, 200.0);
    h.tick();
    h.layout(200.0, 200.0);
    let list = h.paint();

    assert_eq!(list.quads().count(), 1, "Calm painted its quad");
    let errs = h.runtime().errors();
    assert!(!errs.is_empty(), "panic stored in errors()");
    assert!(
        errs.iter().any(|e| e.message.contains("body")),
        "error kind: body"
    );
    assert_eq!(h.runtime().stats().errors as usize, errs.len());
}

#[test]
fn provide_requests_redraw() {
    #[derive(Clone, PartialEq, Debug)]
    struct HotTheme(u32);
    let h = Harness::new(Unit);
    let calls = Rc::new(Cell::new(0u32));
    {
        let calls = calls.clone();
        h.runtime()
            .set_redraw_hook(move || calls.set(calls.get() + 1));
    }
    let env = Environment::new();
    env.provide(HotTheme(1));
    h.runtime().set_environment(env.clone());
    let before = calls.get();
    env.provide(HotTheme(2));
    assert!(
        calls.get() > before,
        "provide() requests a frame (hot theme)"
    );
}

#[test]
fn wheel_down_moves_scroll_and_virtual_list_same_way() {
    let s = sequoia_view::ScrollView::new(Canvas::new().height(800.0));
    let hs = Harness::new(s);
    hs.layout(100.0, 100.0);
    let _ = hs.paint();
    hs.tick();
    hs.layout(100.0, 100.0);

    let vl = VirtualList::new(|| (1..=100u64).collect::<Vec<_>>())
        .key(|id: &u64| *id)
        .item(|id: &u64| Text::new(format!("{id}")))
        .item_height(20.0);
    let hv = Harness::new(vl);
    hv.layout(100.0, 100.0);
    let _ = hv.paint();
    hv.tick();
    hv.layout(100.0, 100.0);

    hs.wheel(50.0, 50.0, -60.0);
    hs.tick();
    hv.wheel(50.0, 50.0, -60.0);
    hv.tick();
    let off_s = hs.root().and_then(|r| {
        hs.view::<sequoia_view::ScrollView>(r)
            .map(|v| sequoia_view::ScrollView::offset(&v, &hs.runtime(), r))
    });
    assert_eq!(off_s, Some(60.0), "ScrollView: -delta.y down");
    let off_v = hv.root().and_then(|r| {
        hv.view::<VirtualList<u64>>(r)
            .map(|v| VirtualList::offset(&v))
    });
    assert_eq!(off_v, Some(60.0), "VirtualList: -delta.y down");
}

#[test]
fn button_labeled_on_click_fires() {
    use sequoia_elements::Button;
    let hits = Rc::new(Cell::new(0u32));
    let h = Harness::new({
        let hits = hits.clone();
        Button::labeled("Save").on_click(move || hits.set(hits.get() + 1))
    });
    h.layout(200.0, 80.0);
    let root = h.root().unwrap();
    let r = h.rect(root).unwrap();
    h.click(r.left() + 5.0, r.top() + 5.0);
    h.tick();
    assert_eq!(hits.get(), 1);
}

#[test]
fn caret_positions_cover_every_char_boundary() {
    let mut text = TextSystem::new();
    let pos = text.caret_positions("Ann has a cat", 14.0, FontWeight::NORMAL);
    let chars = "Ann has a cat".chars().count();
    assert_eq!(pos.len(), chars + 1, "granice: znaki + 1");
    assert!((pos[0]).abs() < 0.01, "start w 0");
    for w in pos.windows(2) {
        assert!(w[1] + 0.01 >= w[0], "monotoniczne: {:?})", pos);
    }
    let total = text.text_width_weighted("Ann has a cat", 14.0, FontWeight::NORMAL);
    assert!((pos[chars] - total).abs() < 0.5, "end = width");

    let mid = text.char_index_from_x("Ann has a cat", 14.0, FontWeight::NORMAL, pos[3]);
    assert_eq!(mid, 3);
}

#[test]
fn extended_keys_and_repeat_policy() {
    assert!(Key::BACKSPACE.allows_repeat());
    assert!(Key::ARROW_LEFT.allows_repeat());
    assert!(Key::PAGE_UP.allows_repeat());
    assert!(Key::from_char('a').allows_repeat());
    assert!(!Key::ENTER.allows_repeat());
    assert!(!Key::ESCAPE.allows_repeat());
    assert!(!Key::TAB.allows_repeat());
    assert!(!Key::F1.allows_repeat());

    let ev = Event::Ime(ImeEvent::Commit("a".to_string()));
    assert!(matches!(ev, Event::Ime(_)));
}
