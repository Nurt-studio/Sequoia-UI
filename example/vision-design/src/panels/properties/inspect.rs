//! Empty-state box of the right panel (shown when nothing is selected).
//! The old "Inspect" tab (CSS preview) was removed — the Design tab covers
//! the same values in editable form.

use sequoia_ui::prelude::*;

use crate::theme::colors;

pub(super) fn empty_box() -> impl View + 'static {
    Container::new()
        .fill_width()
        .padding(Edges::xy(12.0, 24.0))
        .child(
            Column::new()
                .fill_width()
                .gap(8.0)
                .align_items(Align::Center)
                .child(
                    Text::new("Nothing selected")
                        .size(13.0)
                        .semibold()
                        .color(Color::hex(0xB9B9BD)),
                )
                .child(
                    Text::new("Select a layer on the canvas,")
                        .size(12.0)
                        .color(colors::active().icon_inactive),
                )
                .child(
                    Text::new("to view and edit its properties.")
                        .size(12.0)
                        .color(colors::active().icon_inactive),
                ),
        )
}
