//! Block editor demo.
//!
//! Build & run with:
//!     cargo run --example block_editor -p sequoia-ui

use sequoia_ui::elements::BlockEditor;
use sequoia_ui::prelude::*;
use sequoia_ui::text::{BlockType, Document, HeadingLevel};

fn sample_document() -> Document {
    let mut document = Document::new();
    document.add_block(BlockType::Heading(HeadingLevel::H1), "Block editor");
    document.add_block(
        BlockType::Paragraph,
        "Click anywhere to place the caret. This is a document of typed blocks, not a flat text buffer.",
    );
    document.add_block(BlockType::BulletListItem, "Enter splits the current block");
    document.add_block(
        BlockType::BulletListItem,
        "Backspace merges with the previous block",
    );
    document.add_block(
        BlockType::NumberedListItem { number: 3 },
        "Shift+click selects across blocks",
    );
    document.add_block(
        BlockType::TodoItem { checked: true },
        "Ctrl+Z / Ctrl+Y undo and redo",
    );
    document.add_block(
        BlockType::TodoItem { checked: false },
        "Ctrl+A / C / X / V select, copy, cut, paste",
    );
    document.add_block(BlockType::Quote, "Blocks, not a text buffer.");
    document.add_block(
        BlockType::Code {
            language: "rust".into(),
        },
        "let doc = Document::new();",
    );
    document.add_block(
        BlockType::Callout { icon: "✦".into() },
        "Try IME input, scrolling and dragging a selection.",
    );
    document.add_block(BlockType::Divider, "");
    document.add_block(
        BlockType::Table { rows: 2, cols: 3 },
        "Tables and inline objects are part of the document too.",
    );
    document
}

fn main() {
    let document = Signal::new(sample_document());
    let editor = BlockEditor::new(document.clone())
        .font_size(16.0)
        .padding(18.0, 14.0)
        .background(Color::rgb(1.0, 1.0, 1.0));

    let status_document = document.clone();
    let status_selection = editor.selection_signal();
    let status_focus = editor.focused_block();
    let add_document = document.clone();
    let reset_document = document.clone();

    let view = Column::new()
        .fill_size()
        .gap(12.0)
        .padding(20.0)
        .child(Text::new("Sequoia block editor").size(22.0).bold())
        .child(
            Text::new(
                "Click to focus · type to edit · Enter splits · Backspace merges · drag or Shift+click to select",
            )
            .size(13.0),
        )
        .child_flex(
            Container::new()
                .background(Color::rgb(1.0, 1.0, 1.0))
                .border(1.0, Color::rgb(0.86, 0.88, 0.92))
                .radius(12.0)
                .child(editor),
            1.0,
        )
        .child(
            Row::new()
                .gap(8.0)
                .child(
                    Button::labeled("Add paragraph")
                        .style(ButtonStyle::PRIMARY)
                        .on_click(move || {
                        add_document.update(|doc| {
                            doc.add_block(BlockType::Paragraph, "A new paragraph");
                        });
                    }),
                )
                .child(
                    Button::labeled("Reset document")
                        .style(ButtonStyle::OUTLINE)
                        .on_click(move || reset_document.set(sample_document())),
                ),
        )
        .child(Text::with(move || {
            let document = status_document.get();
            let selection = status_selection.get().map(|selection| {
                document
                    .selection_text(selection)
                    .chars()
                    .take(64)
                    .collect::<String>()
            });
            let selection = selection.filter(|text| !text.is_empty());
            let focus = status_focus
                .get()
                .map(|block| format!("{}", block.0))
                .unwrap_or_else(|| "none".into());
            match selection {
                Some(text) => format!(
                    "{} blocks · focused block {} · selection: {:?}",
                    document.blocks.len(),
                    focus,
                    text
                ),
                None => format!("{} blocks · focused block {}", document.blocks.len(), focus),
            }
        }));

    App::new()
        .title("Block editor")
        .size(920.0, 680.0)
        .theme(Theme::light())
        .root(view)
        .run()
        .expect("event loop failed");
}
