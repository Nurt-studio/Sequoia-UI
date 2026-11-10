//! Menu rows and separators (menu bar, context menu, palette).

use sequoia_ui::prelude::*;

use crate::theme::colors;

/// Menu row: clicking closes the menu and runs the action.
pub(crate) fn menu_row(
    label: &'static str,
    hint: Option<&'static str>,
    danger: bool,
    open: Signal<bool>,
    action: impl Fn() + 'static,
) -> AnyView {
    let hover_bg = if danger {
        Color::hexa(0xFA4D56E6)
    } else {
        colors::active().sel
    };
    let mut row = Row::new()
        .fill_width()
        .height(27.0)
        .align_items(Align::Center)
        .padding_xy(8.0, 0.0)
        .child(Text::new(label).size(12.0).color(Color::hex(0xE6E6E6)));
    if let Some(hint) = hint {
        row = row.child(Spacer::flex(1.0)).child(
            Text::new(hint)
                .size(11.0)
                .color(Color::hex(0xE6E6E6).with_alpha(0.6)),
        );
    }
    AnyView::new(
        Container::new()
            .fill_width()
            .radius(5.0)
            .hover(hover_bg)
            .on_click(move || {
                open.set(false);
                action();
            })
            .child(row),
    )
}

/// Like [`menu_row`], but the action receives an `Environment` (clipboard, windows, etc.).
pub(crate) fn menu_row_env(
    label: &'static str,
    hint: Option<&'static str>,
    open: Signal<bool>,
    action: impl Fn(&Environment) + 'static,
) -> AnyView {
    let mut row = Row::new()
        .fill_width()
        .height(27.0)
        .align_items(Align::Center)
        .padding_xy(8.0, 0.0)
        .child(Text::new(label).size(12.0).color(Color::hex(0xE6E6E6)));
    if let Some(hint) = hint {
        row = row.child(Spacer::flex(1.0)).child(
            Text::new(hint)
                .size(11.0)
                .color(Color::hex(0xE6E6E6).with_alpha(0.6)),
        );
    }
    AnyView::new(
        Container::new()
            .fill_width()
            .radius(5.0)
            .hover(colors::active().sel)
            .on_click_env(move |env: &Environment| {
                open.set(false);
                action(env);
            })
            .child(row),
    )
}

/// Horizontal separator in the menu.
pub(crate) fn menu_sep() -> AnyView {
    AnyView::new(
        Container::new()
            .fill_width()
            .padding(Edges::xy(6.0, 4.0))
            .child(
                Container::new()
                    .height(1.0)
                    .fill_width()
                    .background(colors::active().stroke),
            ),
    )
}
