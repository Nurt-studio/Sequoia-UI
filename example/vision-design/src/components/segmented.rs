//! Segments (grouped toggles) used in the properties panel.

use sequoia_ui::prelude::*;

use crate::theme::colors;

fn segment_bg(active: bool) -> Color {
    if active {
        Color::hex(0x333336)
    } else {
        Color::TRANSPARENT
    }
}

fn segment_hover(active: bool) -> Color {
    if active {
        Color::hex(0x3E3E40)
    } else {
        Color::hex(0x232325)
    }
}

/// 28 px segment with a glyph (auto-layout directions).
pub(crate) fn segment_icon(
    icon: &'static str,
    active: bool,
    on_click: impl Fn() + 'static,
) -> impl View + 'static {
    Container::new()
        .flex(1.0)
        .height(28.0)
        .center()
        .radius(6.0)
        .background(segment_bg(active))
        .hover(segment_hover(active))
        .on_click(on_click)
        .child(Text::new(icon).size(14.0).color(if active {
            Color::WHITE
        } else {
            colors::active().icon_inactive
        }))
}

/// 26 px segment with a bold label (Relative / Absolute).
pub(crate) fn segment(
    label: &'static str,
    active: bool,
    on_click: impl Fn() + 'static,
) -> impl View + 'static {
    Container::new()
        .flex(1.0)
        .height(26.0)
        .center()
        .radius(6.0)
        .background(segment_bg(active))
        .hover(segment_hover(active))
        .on_click(on_click)
        .child(Text::new(label).size(11.0).semibold().color(if active {
            Color::WHITE
        } else {
            colors::active().icon_inactive
        }))
}

/// 26 px segment with a monospace label (export formats, CSS/Tailwind).
pub(crate) fn segment_mono(
    label: &'static str,
    active: bool,
    on_click: impl Fn() + 'static,
) -> impl View + 'static {
    Container::new()
        .flex(1.0)
        .height(26.0)
        .center()
        .radius(6.0)
        .background(segment_bg(active))
        .on_click(on_click)
        .child(
            Text::new(label)
                .size(11.0)
                .font("IBM Plex Mono")
                .color(if active {
                    Color::WHITE
                } else {
                    colors::active().icon_inactive
                }),
        )
}