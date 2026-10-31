//! Icon and glyph buttons used in bars and panel headers.

use sequoia_ui::prelude::*;

use crate::theme::colors;

/// 26×26, radius 6; disabled = 32% opacity.
pub(crate) fn icon_button(
    icon: &'static str,
    disabled: bool,
    on_click: impl Fn() + 'static,
) -> AnyView {
    let btn = Container::new()
        .width(26.0)
        .height(26.0)
        .center()
        .radius(6.0)
        .hover(colors::active().bg_lighter)
        .on_click(move || {
            if !disabled {
                on_click();
            }
        })
        .child(Icon::new(icon).size(15.0).color(colors::active().icon_inactive));
    if disabled {
        AnyView::new(Opacity::new(btn, 0.32))
    } else {
        AnyView::new(btn)
    }
}

/// 26×26 with access to the shared environment (e.g. the `Windows` service).
pub(crate) fn icon_button_env(
    icon: &'static str,
    disabled: bool,
    on_click: impl Fn(&Environment) + 'static,
) -> AnyView {
    let btn = Container::new()
        .width(26.0)
        .height(26.0)
        .center()
        .radius(6.0)
        .hover(colors::active().bg_lighter)
        .on_click_env(move |env| {
            if !disabled {
                on_click(env);
            }
        })
        .child(Icon::new(icon).size(15.0).color(colors::active().icon_inactive));
    if disabled {
        AnyView::new(Opacity::new(btn, 0.32))
    } else {
        AnyView::new(btn)
    }
}

/// 20×20, radius 5; panel headers.
pub(crate) fn small_icon_button(
    icon: &'static str,
    on_click: impl Fn() + 'static,
) -> impl View + 'static {
    Container::new()
        .width(20.0)
        .height(20.0)
        .center()
        .radius(5.0)
        .hover(colors::active().bg_lighter)
        .on_click(on_click)
        .child(Icon::new(icon).size(12.0).color(colors::active().icon_inactive))
}

/// 26×26 with a text glyph (e.g. zoom −/+).
pub(crate) fn glyph_button(
    label: &'static str,
    disabled: bool,
    on_click: impl Fn() + 'static,
) -> AnyView {
    let btn = Container::new()
        .width(26.0)
        .height(26.0)
        .center()
        .hover(colors::active().bg_lighter)
        .on_click(move || {
            if !disabled {
                on_click();
            }
        })
        .child(
            Text::new(label)
                .size(11.0)
                .font("IBM Plex Mono")
                .color(colors::active().icon_inactive),
        );
    if disabled {
        AnyView::new(Opacity::new(btn, 0.32))
    } else {
        AnyView::new(btn)
    }
}
