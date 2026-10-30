//! Resizable panes built on `SplitPanel`.
//!
//! The window is split twice:
//!
//!   ┌─────────────┬───────────────────────────┐
//!   │             │          Editor           │
//!   │  Explorer   ├───────────────────────────┤
//!   │             │         Terminal          │
//!   └─────────────┴───────────────────────────┘
//!
//! An outer *horizontal* split separates the sidebar from the main area,
//! and an inner *vertical* split divides the main area into an editor and
//! a terminal. Drag any divider to resize the panes.
//!
//! Run with:
//!     cargo run --example split_panel -p sequoia-ui

use sequoia_ui::prelude::*;

/// Builds a simple pane with an icon, a title, and some body text.
fn pane(title: &str, body: &str, icon_name: &str, accent: Color) -> impl View {
    Column::new()
        .fill_size()
        .padding(16.0)
        .gap(8.0)
        .child(
            Row::new()
                .gap(8.0)
                // Using a real icon from the built-in set instead of a colored dot
                .child(Icon::new(icon_name).size(16.0).color(accent))
                .child(Text::new(title).size(15.0).bold()),
        )
        .child(Text::new(body).size(13.0))
}

fn main() {
    // The split fraction is a reactive `Signal`. It stores the share of
    // the available space taken by the *first* pane.
    //
    // When the user drags a divider, `SplitPanel` writes the new fraction
    // back into the signal, so the layout and any other reader stay in
    // sync. Setting the signal programmatically (for example from a
    // "reset layout" button) also moves the divider.
    let outer_split = Signal::new(0.30);
    let inner_split = Signal::new(0.65);

    // Left pane: a fake file tree, just to make the split visible.
    let sidebar = pane(
        "Explorer",
        "src/\n  main.rs\ntests/\nexamples/\nCargo.toml\nREADME.md",
        "folder",
        Color::hex(0x3584E4),
    );

    // Right side contains its own vertical split, stacked top over bottom.
    let editor = pane(
        "Editor",
        "fn main() {\n    println!(\"hello, sequoia\");\n}",
        "file",
        Color::hex(0x26A269),
    );
    let terminal = pane(
        "Terminal",
        "$ cargo run --example split_panel\n\nDrag a divider to resize.",
        "play",
        Color::hex(0xE5A50A),
    );

    // Inner split: editor on top, terminal on the bottom.
    //
    // - `.vertical()`     → split along the vertical axis, so panes are
    //                       stacked top/bottom instead of left/right
    // - `.min_pane(80.0)` → both panes are at least 80 px tall
    let right = SplitPanel::new(inner_split)
        .vertical()
        .min_pane(80.0)
        .first(editor)
        .second(terminal);

    // Outer split: sidebar on the left, everything else on the right.
    //
    // - `.min_first` / `.min_second` clamp the drag range
    // - `.handle_width` controls how wide the draggable divider is
    //   (a wider handle is easier to grab)
    // - `.handle_color` / `.handle_active_color` style the divider and
    //   the highlight it shows while being dragged
    let root = SplitPanel::new(outer_split)
        .horizontal()
        .min_first(140.0)
        .min_second(240.0)
        .handle_width(7.0)
        .handle_color(Color::hex(0x3A3A3A))
        .handle_active_color(Color::hex(0x3584E4))
        .first(sidebar)
        .second(right);

    App::new()
        .title("SplitPanel demo")
        .size(900.0, 600.0)
        .root(root)
        .run()
        .expect("event loop failed");
}
