use sequoia_ui::prelude::*;

use crate::state::{AppTheme, EditorState};
use crate::theme::colors;

#[derive(Clone)]
struct SettingsOverlay {
    state: EditorState,
}

impl View for SettingsOverlay {
    fn body(&self) -> impl View + 'static {
        let open = self.state.settings_open.clone();
        if !open.get() {
            return AnyView::new(Unit);
        }
        let st = self.state.clone();
        let current_theme = self.state.theme_mode.get();
        let is_night = self.state.is_night.get();
        let handles_centered = self.state.resize_handles_centered.get();

        let active_style = ButtonStyle::new()
            .background(colors::active().sel)
            .text(Color::WHITE)
            .radius(6.0)
            .padding_xy(12.0, 6.0);

        let inactive_style = ButtonStyle::new()
            .background(colors::active().bg_lighter)
            .text(colors::active().text)
            .radius(6.0)
            .padding_xy(12.0, 6.0);

        AnyView::new(
            Container::new()
                .fill_size()
                .background(Color::hexa(0x00000099))
                .center()
                .on_click({
                    let st = st.clone();
                    move || st.settings_open.set(false)
                })
                .child(
                    Container::new()
                        .width(460.0)
                        .background(colors::active().surface)
                        .border(1.0, colors::active().stroke)
                        .radius(12.0)
                        .padding(Edges::all(20.0))
                        .on_click(|| {})
                        .child(
                            Column::new()
                                .gap(16.0)
                                .child(
                                    Row::new()
                                        .fill_width()
                                        .align_items(Align::Center)
                                        .child(
                                            Text::new("Application Settings")
                                                .size(15.0)
                                                .semibold()
                                                .color(colors::active().text),
                                        )
                                        .child(Spacer::flex(1.0))
                                        .child(
                                            Button::new("Close").style(inactive_style).on_click({
                                                let st = st.clone();
                                                move || st.settings_open.set(false)
                                            }),
                                        ),
                                )
                                .child(
                                    Column::new()
                                        .gap(8.0)
                                        .child(
                                            Text::new("Theme")
                                                .size(12.0)
                                                .semibold()
                                                .color(colors::active().icon_inactive),
                                        )
                                        .child(
                                            Row::new()
                                                .gap(8.0)
                                                .child(
                                                    Button::new("Dark Theme")
                                                        .style(
                                                            if matches!(
                                                                current_theme,
                                                                AppTheme::Dark
                                                            ) {
                                                                active_style
                                                            } else {
                                                                inactive_style
                                                            },
                                                        )
                                                        .on_click({
                                                            let st = st.clone();
                                                            move || {
                                                                st.theme_mode.set(AppTheme::Dark)
                                                            }
                                                        }),
                                                )
                                                .child(
                                                    Button::new("Light Theme")
                                                        .style(
                                                            if matches!(
                                                                current_theme,
                                                                AppTheme::Light
                                                            ) {
                                                                active_style
                                                            } else {
                                                                inactive_style
                                                            },
                                                        )
                                                        .on_click({
                                                            let st = st.clone();
                                                            move || {
                                                                st.theme_mode.set(AppTheme::Light)
                                                            }
                                                        }),
                                                ),
                                        ),
                                )
                                .child(
                                    Column::new()
                                        .gap(8.0)
                                        .child(
                                            Text::new("Mode")
                                                .size(12.0)
                                                .semibold()
                                                .color(colors::active().icon_inactive),
                                        )
                                        .child(
                                            Row::new()
                                                .gap(8.0)
                                                .child(
                                                    Button::new("Night Mode")
                                                        .style(if is_night {
                                                            active_style
                                                        } else {
                                                            inactive_style
                                                        })
                                                        .on_click({
                                                            let st = st.clone();
                                                            move || st.is_night.set(true)
                                                        }),
                                                )
                                                .child(
                                                    Button::new("Day Mode")
                                                        .style(if !is_night {
                                                            active_style
                                                        } else {
                                                            inactive_style
                                                        })
                                                        .on_click({
                                                            let st = st.clone();
                                                            move || st.is_night.set(false)
                                                        }),
                                                ),
                                        ),
                                )
                                .child(
                                    Column::new()
                                        .gap(8.0)
                                        .child(
                                            Text::new("Resize handles")
                                                .size(12.0)
                                                .semibold()
                                                .color(colors::active().icon_inactive),
                                        )
                                        .child(
                                            Row::new()
                                                .gap(8.0)
                                                .child(
                                                    Button::new("Panel edge")
                                                        .style(if handles_centered {
                                                            inactive_style
                                                        } else {
                                                            active_style
                                                        })
                                                        .on_click({
                                                            let st = st.clone();
                                                            move || {
                                                                st.resize_handles_centered
                                                                    .set(false)
                                                            }
                                                        }),
                                                )
                                                .child(
                                                    Button::new("Centered")
                                                        .style(if handles_centered {
                                                            active_style
                                                        } else {
                                                            inactive_style
                                                        })
                                                        .on_click({
                                                            let st = st.clone();
                                                            move || {
                                                                st.resize_handles_centered.set(true)
                                                            }
                                                        }),
                                                ),
                                        ),
                                ),
                        ),
                ),
        )
    }
}

pub(crate) fn settings_overlay(state: &EditorState) -> impl View + 'static {
    SettingsOverlay {
        state: state.clone(),
    }
}
