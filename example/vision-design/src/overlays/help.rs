use sequoia_ui::prelude::*;

use crate::state::EditorState;
use crate::theme::colors;

#[derive(Clone)]
struct HelpOverlay {
    state: EditorState,
}

impl View for HelpOverlay {
    fn body(&self) -> impl View + 'static {
        let open = self.state.help_open.clone();
        if !open.get() {
            return AnyView::new(Unit);
        }
        let st = self.state.clone();

        let shortcuts = vec![
            ("Select / hand", "V · H"),
            ("Frame / rectangle", "F · R"),
            ("Ellipse / text", "O · T"),
            ("Svelte code", "Ctrl+E"),
            ("Command palette", "Ctrl+K"),
            ("Duplicate / delete", "Ctrl+D / Del"),
            ("Undo / redo", "Ctrl+Z / Ctrl+Shift+Z"),
            ("Zoom / fit", "Ctrl+Wheel / Shift+1"),
            ("Pan canvas", "Middle mouse button"),
            ("Edit text", "Double click"),
            ("Auto layout", "Object → Auto layout"),
            ("Import / export JSON", "File → JSON"),
        ];

        let mut grid = Column::new().gap(4.0).fill_width();
        for (k, v) in shortcuts {
            grid = grid.child(
                Row::new()
                    .fill_width()
                    .padding(Edges::xy(0.0, 4.0))
                    .border_bottom(1.0, colors::active().stroke_soft)
                    .child(Text::new(k).size(11.0).color(Color::hex(0xC9C9C9)))
                    .child(Spacer::flex(1.0))
                    .child(
                        Container::new()
                            .padding(Edges::xy(5.0, 2.0))
                            .background(Color::hex(0x2E2E30))
                            .border(1.0, colors::active().stroke)
                            .radius(4.0)
                            .child(
                                Text::new(v)
                                    .size(9.5)
                                    .font("IBM Plex Mono")
                                    .color(Color::hex(0xAEAEAE)),
                            ),
                    ),
            );
        }

        AnyView::new(
            Container::new()
                .fill_size()
                .background(Color::hexa(0x00000099))
                .center()
                .on_click({
                    let st = st.clone();
                    move || st.help_open.set(false)
                })
                .child(
                    Container::new()
                        .width(540.0)
                        .background(Color::hex(0x232324))
                        .border(1.0, colors::active().stroke)
                        .radius(12.0)
                        .padding(Edges::all(16.0))
                        .on_click(|| {})
                        .child(
                            Column::new()
                                .gap(10.0)
                                .child(
                                    Text::new("Keyboard shortcuts")
                                        .size(14.0)
                                        .semibold()
                                        .color(Color::WHITE),
                                )
                                .child(
                                    Text::new("Esc closes the window")
                                        .size(11.0)
                                        .color(colors::active().icon_inactive),
                                )
                                .child(grid),
                        ),
                ),
        )
    }
}

// -------------------------- root --------------------------

pub(crate) fn help_overlay(state: &EditorState) -> impl View + 'static {
    HelpOverlay {
        state: state.clone(),
    }
}