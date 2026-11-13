#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::rc::Rc;

use sequoia_elements::{BlockEditor, MemClipboard, SharedClipboard};
use sequoia_input::{Key, Modifiers};
use sequoia_reactive::{Environment, Signal};
use sequoia_test::Harness;
use sequoia_text::{
    Block, BlockId, BlockType, Document, HeadingLevel, InlineContent, InlineElement,
};

fn harness(document: Document) -> (Harness, Signal<Document>) {
    let signal = Signal::new(document);
    let editor = BlockEditor::new(signal.clone())
        .font_size(14.0)
        .padding(0.0, 0.0);
    let h = Harness::new(editor);
    h.layout(400.0, 400.0);
    h.runtime().set_focus(Some(h.root().unwrap()));
    h.tick();
    h.layout(400.0, 400.0);
    h.paint();
    h.tick();
    (h, signal)
}

fn editor(h: &Harness) -> Rc<BlockEditor> {
    h.view::<BlockEditor>(h.root().unwrap()).unwrap()
}

#[test]
fn test_document_block_operations() {
    let mut doc = Document::new();
    let id1 = doc.add_block(BlockType::Paragraph, "Hello Notion");
    let id2 = doc.add_block(BlockType::Heading(HeadingLevel::H1), "Title Heading");
    let id3 = doc.insert_block(1, BlockType::BulletListItem, "List item");

    assert_eq!(doc.blocks.len(), 3);
    assert_eq!(doc.find_block(id1).unwrap().content, "Hello Notion");
    assert_eq!(doc.find_block(id2).unwrap().content, "Title Heading");
    assert_eq!(doc.find_block(id3).unwrap().content, "List item");

    doc.remove_block(id3);
    assert_eq!(doc.blocks.len(), 2);
    assert!(doc.find_block(id3).is_none());
}

#[test]
fn test_block_editor_creation() {
    let doc = Document::new();
    let signal = Signal::new(doc);
    let _editor = BlockEditor::new(signal.clone());
    assert!(signal.get_untracked().blocks.is_empty());
}

#[test]
fn typing_and_enter_create_blocks() {
    let mut document = Document::new();
    document.add_block(BlockType::Paragraph, "");
    let (h, signal) = harness(document);

    for character in "hello".chars() {
        h.key(Key::from_char(character));
    }
    h.key(Key::ENTER);
    for character in "world".chars() {
        h.key(Key::from_char(character));
    }
    h.tick();

    let document = signal.get_untracked();
    assert_eq!(document.blocks.len(), 2);
    assert_eq!(document.blocks[0].plain_text(), "hello");
    assert_eq!(document.blocks[1].plain_text(), "world");
    assert_eq!(editor(&h).text(), "hello\nworld");
}

#[test]
fn typing_into_an_empty_document_creates_a_block() {
    let (h, signal) = harness(Document::new());

    h.key(Key::from_char('a'));
    h.key(Key::ENTER);
    h.key(Key::from_char('b'));
    h.tick();

    let document = signal.get_untracked();
    assert_eq!(document.blocks.len(), 2);
    assert_eq!(document.blocks[0].plain_text(), "a");
    assert_eq!(document.blocks[1].plain_text(), "b");
}

#[test]
fn backspace_merges_blocks_across_boundary() {
    let mut document = Document::new();
    document.add_block(BlockType::Paragraph, "left");
    document.add_block(BlockType::Paragraph, "right");
    let (h, signal) = harness(document);

    editor(&h).set_focus_position(sequoia_text::BlockPosition::new(
        signal.get_untracked().blocks[1].id,
        0,
    ));
    h.key(Key::BACKSPACE);
    h.tick();

    let document = signal.get_untracked();
    assert_eq!(document.blocks.len(), 1);
    assert_eq!(document.blocks[0].plain_text(), "leftright");
}

#[test]
fn dragging_across_blocks_selects_and_deletes() {
    let mut document = Document::new();
    document.add_block(BlockType::Paragraph, "hello");
    document.add_block(BlockType::Paragraph, "world");
    let (h, signal) = harness(document);

    h.drag((2.0, 10.0), (1.0, 40.0));
    h.tick();

    let selection = editor(&h).selection().expect("cross-block selection");
    assert_ne!(selection.anchor.block_id, selection.focus.block_id);
    assert!(editor(&h).selection_text().starts_with("hello\n"));

    h.key(Key::BACKSPACE);
    h.tick();
    let document = signal.get_untracked();
    assert_eq!(document.blocks.len(), 1);
    assert_eq!(document.blocks[0].plain_text(), "world");
}

#[test]
fn layout_and_paint_inline_objects() {
    let mut document = Document::new();
    let block = Block::from_inlines(
        BlockId(1),
        BlockType::Paragraph,
        vec![
            InlineContent::Text("see ".into()),
            InlineContent::Element(InlineElement::Tag("rust".into())),
            InlineContent::Text(" and ".into()),
            InlineContent::Element(InlineElement::Mention("ada".into())),
            InlineContent::Text(" today".into()),
        ],
    );
    document.insert_block_value(0, block);
    document.next_block_id();
    let (_h, signal) = harness(document);

    assert_eq!(
        signal.get_untracked().blocks[0].content,
        "see \u{FFFC} and \u{FFFC} today"
    );
}

#[test]
fn ime_preedit_does_not_commit_and_commit_replaces_selection() {
    let mut document = Document::new();
    let first = document.add_block(BlockType::Paragraph, "hello");
    let second = document.add_block(BlockType::Paragraph, "world");
    let (h, signal) = harness(document);

    editor(&h).set_selection(Some(sequoia_text::DocumentSelection::new(
        sequoia_text::BlockPosition::new(first, 2),
        sequoia_text::BlockPosition::new(second, 3),
    )));
    h.tick();
    h.paint();

    h.ime_preedit("nihon", None);
    h.tick();
    h.paint();
    assert_eq!(
        signal.get_untracked().blocks.len(),
        2,
        "preedit is visual only"
    );
    let caret = h.runtime().ime_cursor_rect().expect("ime caret");
    assert!(caret.left() >= 0.0);

    h.ime_commit("Nihon");
    h.tick();
    let document = signal.get_untracked();
    assert_eq!(document.blocks.len(), 1);
    assert_eq!(document.blocks[0].plain_text(), "heNihonld");
}

#[test]
fn keys_during_composition_do_not_edit_the_document() {
    let mut document = Document::new();
    let id = document.add_block(BlockType::Paragraph, "a");
    let (h, signal) = harness(document);
    editor(&h).set_focus_position(sequoia_text::BlockPosition::new(id, 1));

    h.ime_preedit("ni", None);
    h.key(Key::from_char('x'));
    h.tick();
    assert_eq!(signal.get_untracked().blocks[0].plain_text(), "a");

    h.ime_commit("Nihon");
    h.tick();
    assert_eq!(signal.get_untracked().blocks[0].plain_text(), "aNihon");
}

#[test]
fn ime_caret_tracks_composition_cursor() {
    let mut document = Document::new();
    document.add_block(BlockType::Paragraph, "ab");
    let (h, _signal) = harness(document);
    h.key(Key::END);
    h.tick();
    h.paint();
    let start = h.runtime().ime_cursor_rect().expect("caret rect");

    h.ime_preedit("nihon", None);
    h.tick();
    h.paint();
    let end = h.runtime().ime_cursor_rect().expect("composition caret");
    assert!(end.left() > start.left(), "caret follows preedit");

    h.ime_preedit("nihon", Some((2, 2)));
    h.tick();
    h.paint();
    let middle = h.runtime().ime_cursor_rect().expect("composition midpoint");
    assert!(middle.left() < end.left());

    h.ime_commit("Nihon");
    h.tick();
    h.paint();
    assert!(
        h.view::<BlockEditor>(h.root().unwrap())
            .unwrap()
            .text()
            .ends_with("abNihon")
    );
}

#[test]
fn caret_stays_on_the_focused_visual_line() {
    let mut document = Document::new();
    let id = document.add_block(BlockType::Paragraph, "alpha beta gamma delta epsilon");
    let signal = Signal::new(document);
    let editor = BlockEditor::new(signal).font_size(14.0).padding(0.0, 0.0);
    let h = Harness::new(editor);
    h.layout(60.0, 400.0);
    h.runtime().set_focus(Some(h.root().unwrap()));
    h.view::<BlockEditor>(h.root().unwrap())
        .unwrap()
        .set_focus_position(sequoia_text::BlockPosition::new(id, 0));
    h.tick();
    h.layout(60.0, 400.0);
    h.paint();

    let caret = h.runtime().ime_cursor_rect().expect("caret rect");
    assert!(caret.top() < 20.0, "caret should be on the first line");
}

#[test]
fn wrapped_line_edges_keep_the_visual_line() {
    let mut document = Document::new();
    let id = document.add_block(BlockType::Paragraph, "alpha beta gamma delta epsilon");
    let signal = Signal::new(document);
    let view = BlockEditor::new(signal).font_size(14.0).padding(0.0, 0.0);
    let h = Harness::new(view);
    h.layout(60.0, 400.0);
    h.runtime().set_focus(Some(h.root().unwrap()));
    h.view::<BlockEditor>(h.root().unwrap())
        .unwrap()
        .set_focus_position(sequoia_text::BlockPosition::new(id, 0));
    h.tick();
    h.layout(60.0, 400.0);
    h.paint();

    h.key(Key::END);
    h.tick();
    h.paint();
    h.key(Key::ARROW_DOWN);
    h.tick();
    h.paint();
    h.key(Key::HOME);
    h.tick();
    h.paint();
    let caret = h.runtime().ime_cursor_rect().expect("caret rect");
    assert!(
        caret.top() > 20.0,
        "Home must stay on the wrapped line below the first one"
    );

    h.key(Key::ARROW_LEFT);
    h.tick();
    h.paint();
    let caret = h.runtime().ime_cursor_rect().expect("caret rect");
    assert!(caret.top() < 20.0, "Left must return to the previous line");

    for _ in 0..2 {
        h.key(Key::ARROW_RIGHT);
    }
    h.tick();
    h.paint();
    let caret = h.runtime().ime_cursor_rect().expect("caret rect");
    assert!(
        caret.top() > 20.0,
        "Right at a soft-wrap boundary must move to the next line"
    );
}

#[test]
fn caret_for_a_later_block_uses_its_content_origin() {
    let mut document = Document::new();
    document.add_block(BlockType::Paragraph, "first");
    let second = document.add_block(BlockType::Paragraph, "second");
    let (h, _signal) = harness(document);
    editor(&h).set_focus_position(sequoia_text::BlockPosition::new(second, 0));
    h.tick();
    h.paint();
    h.tick();

    let caret = h.runtime().ime_cursor_rect().expect("caret rect");
    assert!(
        caret.top() > 20.0,
        "the caret must follow the block content origin"
    );
}

#[test]
fn caret_for_a_block_without_lines_stays_on_that_block() {
    let mut document = Document::new();
    document.add_block(BlockType::Paragraph, "first");
    let divider = document.add_block(BlockType::Divider, "");
    let (h, _signal) = harness(document);
    editor(&h).set_focus_position(sequoia_text::BlockPosition::new(divider, 0));
    h.tick();
    h.paint();
    h.tick();

    let caret = h.runtime().ime_cursor_rect().expect("caret rect");
    assert!(
        caret.top() > 10.0,
        "empty block caret must not sit on the first block"
    );
}

#[test]
fn undo_and_redo_restore_document() {
    let mut document = Document::new();
    document.add_block(BlockType::Paragraph, "");
    let (h, signal) = harness(document);

    h.key(Key::from_char('a'));
    h.key(Key::from_char('b'));
    assert_eq!(signal.get_untracked().blocks[0].plain_text(), "ab");

    h.key_with_modifiers(Key::from_char('z'), Modifiers::CONTROL);
    h.tick();
    assert_eq!(signal.get_untracked().blocks[0].plain_text(), "a");

    h.key_with_modifiers(Key::from_char('y'), Modifiers::CONTROL);
    h.tick();
    assert_eq!(signal.get_untracked().blocks[0].plain_text(), "ab");
}

#[test]
fn copying_and_pasting_blocks_keeps_content_and_new_ids() {
    let mut document = Document::new();
    let first = document.add_block(BlockType::Paragraph, "one");
    let second = document.add_block(BlockType::Paragraph, "two");
    let (h, signal) = harness(document);

    let environment = Environment::new();
    environment.provide(SharedClipboard(Rc::new(MemClipboard::new())));
    h.set_environment(environment);

    editor(&h).set_focus_position(sequoia_text::BlockPosition::new(second, 0));
    h.key_with_modifiers(Key::from_char('c'), Modifiers::CONTROL);
    editor(&h).set_focus_position(sequoia_text::BlockPosition::new(first, 0));
    h.key_with_modifiers(Key::from_char('v'), Modifiers::CONTROL);
    h.tick();

    let document = signal.get_untracked();
    assert_eq!(document.blocks.len(), 3);
    assert_eq!(document.blocks[1].plain_text(), "two");
    assert_ne!(document.blocks[1].id, second);
}

#[test]
fn cutting_without_a_selection_removes_and_restores_the_block() {
    let mut document = Document::new();
    let first = document.add_block(BlockType::Paragraph, "one");
    let second = document.add_block(BlockType::Paragraph, "two");
    let (h, signal) = harness(document);
    editor(&h).set_focus_position(sequoia_text::BlockPosition::new(first, 1));

    h.key_with_modifiers(Key::from_char('x'), Modifiers::CONTROL);
    h.tick();
    let document = signal.get_untracked();
    assert_eq!(document.blocks.len(), 1);
    assert_eq!(document.blocks[0].id, second);
    assert_eq!(document.blocks[0].plain_text(), "two");
    assert_eq!(
        editor(&h).focus_position(),
        Some(sequoia_text::BlockPosition::new(second, 0))
    );

    h.key_with_modifiers(Key::from_char('v'), Modifiers::CONTROL);
    h.tick();
    let document = signal.get_untracked();
    assert_eq!(document.blocks.len(), 2);
    assert_eq!(document.blocks[0].plain_text(), "two");
    assert_eq!(document.blocks[1].plain_text(), "one");
    assert_ne!(document.blocks[1].id, first);
}

#[test]
fn cutting_and_pasting_the_whole_document_leaves_no_blank_block() {
    let mut document = Document::new();
    document.add_block(BlockType::Paragraph, "one");
    document.add_block(BlockType::Paragraph, "two");
    let (h, signal) = harness(document);

    h.key_with_modifiers(Key::from_char('a'), Modifiers::CONTROL);
    h.key_with_modifiers(Key::from_char('x'), Modifiers::CONTROL);
    h.tick();
    let document = signal.get_untracked();
    assert_eq!(document.blocks.len(), 1);
    assert_eq!(document.blocks[0].plain_text(), "");

    h.key_with_modifiers(Key::from_char('v'), Modifiers::CONTROL);
    h.tick();
    let document = signal.get_untracked();
    assert_eq!(document.blocks.len(), 2);
    assert_eq!(document.blocks[0].plain_text(), "one");
    assert_eq!(document.blocks[1].plain_text(), "two");
}

#[test]
fn wheel_scrolling_keeps_the_caret_in_view() {
    let mut document = Document::new();
    for index in 0..30 {
        document.add_block(BlockType::Paragraph, format!("line {index}"));
    }
    let last = document.blocks.last().unwrap().id;
    let signal = Signal::new(document);
    let view = BlockEditor::new(signal.clone())
        .font_size(14.0)
        .padding(0.0, 0.0);
    let h = Harness::new(view);
    h.layout(240.0, 120.0);
    h.runtime().set_focus(Some(h.root().unwrap()));
    h.tick();
    h.layout(240.0, 120.0);
    h.paint();

    h.wheel(120.0, 60.0, -120.0);
    h.paint();
    h.tick();
    let scrolled = editor(&h).scroll_offset();
    assert!(scrolled > 0.0, "wheel must scroll the document");

    editor(&h).set_focus_position(sequoia_text::BlockPosition::new(last, 6));
    h.tick();
    h.layout(240.0, 120.0);
    h.paint();
    assert!(editor(&h).scroll_offset() > scrolled);
    let caret = h.runtime().ime_cursor_rect().expect("caret rect");
    assert!(
        caret.top() >= 0.0,
        "scroll={} caret={caret:?}",
        editor(&h).scroll_offset()
    );
    assert!(caret.bottom() <= 120.0);
}

#[test]
fn clicking_after_scrolling_uses_the_scrolled_layout() {
    let mut document = Document::new();
    for index in 0..30 {
        document.add_block(BlockType::Paragraph, format!("line {index}"));
    }
    let first = document.blocks[0].id;
    let signal = Signal::new(document);
    let view = BlockEditor::new(signal.clone())
        .font_size(14.0)
        .padding(0.0, 0.0);
    let h = Harness::new(view);
    h.layout(240.0, 120.0);
    h.runtime().set_focus(Some(h.root().unwrap()));
    h.tick();
    h.layout(240.0, 120.0);
    h.paint();

    h.wheel(120.0, 60.0, -480.0);
    h.paint();
    h.tick();
    assert!(editor(&h).scroll_offset() > 0.0);

    h.click(20.0, 60.0);
    h.tick();
    let focus = editor(&h).focus_position().expect("focused block");
    assert_ne!(focus.block_id, first);
}

#[test]
fn shift_click_extends_from_the_previous_caret() {
    let mut document = Document::new();
    let first = document.add_block(BlockType::Paragraph, "hello");
    let second = document.add_block(BlockType::Paragraph, "world");
    let (h, _signal) = harness(document);

    h.click(2.0, 10.0);
    h.pointer_down_mods(1.0, 40.0, Modifiers::SHIFT);
    h.pointer_up(1.0, 40.0);
    h.tick();

    let selection = editor(&h).selection().expect("shift-click selection");
    assert_eq!(selection.anchor.block_id, first);
    assert_eq!(selection.focus.block_id, second);
}

#[test]
fn internal_copy_paste_works_without_a_clipboard_environment() {
    let mut document = Document::new();
    let first = document.add_block(BlockType::Paragraph, "one");
    let second = document.add_block(BlockType::Paragraph, "two");
    let (h, signal) = harness(document);

    editor(&h).set_focus_position(sequoia_text::BlockPosition::new(second, 0));
    h.key_with_modifiers(Key::from_char('c'), Modifiers::CONTROL);
    editor(&h).set_focus_position(sequoia_text::BlockPosition::new(first, 0));
    h.key_with_modifiers(Key::from_char('v'), Modifiers::CONTROL);
    h.tick();

    let document = signal.get_untracked();
    assert_eq!(document.blocks.len(), 3);
    assert_eq!(document.blocks[1].plain_text(), "two");
}

#[test]
fn copying_selected_inline_object_pastes_an_object() {
    let mut document = Document::new();
    let id = document.add_block(BlockType::Paragraph, "");
    document.find_block_mut(id).unwrap().set_inlines(vec![
        InlineContent::Text("a ".into()),
        InlineContent::Element(InlineElement::Tag("x".into())),
        InlineContent::Text(" b".into()),
    ]);
    let (h, signal) = harness(document);

    let environment = Environment::new();
    environment.provide(SharedClipboard(Rc::new(MemClipboard::new())));
    h.set_environment(environment);

    editor(&h).set_selection(Some(sequoia_text::DocumentSelection::new(
        sequoia_text::BlockPosition::new(id, 2),
        sequoia_text::BlockPosition::new(id, 3),
    )));
    h.key_with_modifiers(Key::from_char('c'), Modifiers::CONTROL);
    editor(&h).set_focus_position(sequoia_text::BlockPosition::new(id, 5));
    h.key_with_modifiers(Key::from_char('v'), Modifiers::CONTROL);
    h.tick();

    let document = signal.get_untracked();
    let objects = document.blocks[0]
        .inlines
        .iter()
        .filter(|item| matches!(item, InlineContent::Element(_)))
        .count();
    assert_eq!(objects, 2);
    assert_eq!(document.blocks[0].plain_text(), "a \u{FFFC} b\u{FFFC}");
}

#[test]
fn pointer_click_moves_caret_between_blocks() {
    let mut document = Document::new();
    let first = document.add_block(BlockType::Paragraph, "hello");
    let second = document.add_block(BlockType::Paragraph, "world");
    let (h, _signal) = harness(document);

    h.click(2.0, 10.0);
    h.tick();
    let focus = editor(&h).focus_position().unwrap();
    assert_eq!(focus.block_id, first);

    h.click(20.0, 40.0);
    h.tick();
    let focus = editor(&h).focus_position().unwrap();
    assert_eq!(focus.block_id, second);
}
