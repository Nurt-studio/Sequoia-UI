//! Layers panel: header (with the "hide hidden" eye), search pill and
//! sections.
mod layer_row;
mod layer_tree;

use self::layer_tree::layer_list;

use sequoia_ui::prelude::*;

use crate::components::icon_button::small_icon_button;
use crate::components::search_field::search_field_cancellable;

use crate::state::EditorState;
use crate::theme::colors;

/// Eye in the panel header: ON = hidden layers are filtered out of the
/// browser, OFF = they are listed (dimmed, with the eye-off icon).
#[derive(Clone)]
struct EyeToggle {
    state: EditorState,
}

impl View for EyeToggle {
    fn body(&self) -> impl View + 'static {
        let filtering = !self.state.show_hidden_layers.get();
        let st = self.state.clone();
        Container::new()
            .width(20.0)
            .height(20.0)
            .center()
            .radius(5.0)
            .hover(colors::active().bg_lighter)
            .style(if filtering {
                BoxStyle::new()
                    .background(colors::active().sel_soft)
                    .border(1.0, Color::hexa(0x0D99FF73))
            } else {
                BoxStyle::new()
            })
            .on_click(move || st.show_hidden_layers.update(|v| *v = !*v))
            .child(
                Icon::new(if filtering { "eye" } else { "lyr/eyeoff" })
                    .size(12.0)
                    .color(if filtering {
                        colors::active().icon_active
                    } else {
                        colors::active().icon_inactive
                    }),
            )
    }
}

#[derive(Clone)]
struct Sec {
    title: &'static str,
    open: Signal<bool>,
    cnt: Memo<usize>,
}

impl View for Sec {
    fn body(&self) -> impl View + 'static {
        let open = self.open.get();
        let cnt = self.cnt.clone();
        let toggle = self.open.clone();
        Container::new()
            .fill_width()
            .padding(Edges::xy(6.0, 5.0))
            .hover(colors::active().surface)
            .on_click(move || toggle.update(|o| *o = !*o))
            .child(
                Row::new()
                    .gap(4.0)
                    .align_items(Align::Center)
                    .child(
                        Text::new(if open { "▾" } else { "▸" })
                            .size(14.0)
                            .color(colors::active().icon_inactive),
                    )
                    .child(
                        Text::new(self.title)
                            .size(13.0)
                            .semibold()
                            .letter_spacing(0.6)
                            .color(colors::active().icon_inactive),
                    )
                    .child(Spacer::flex(1.0))
                    .child(
                        Container::new()
                            .padding(Edges::xy(6.0, 1.0))
                            .radius(10.0)
                            .background(colors::active().bg_lighter)
                            .child(
                                Text::with(move || format!("{}", cnt.get()))
                                    .size(13.0)
                                    .font("IBM Plex Mono")
                                    .color(colors::active().icon_inactive),
                            ),
                    ),
            )
    }
}

fn panel_left_body(state: &EditorState, width: f32) -> impl View + 'static {
    let page_cnt = {
        let layers = state.layers.clone();
        memo(move || layers.get().iter().filter(|l| !l.component).count())
    };
    let comp_cnt = {
        let layers = state.layers.clone();
        memo(move || layers.get().iter().filter(|l| l.component).count())
    };

    Column::new()
        .width(width)
        .fill_height()
        .background(colors::active().surface)
        .child(
            Row::new()
                .fill_width()
                .padding(Edges::xy(6.0, 6.0))
                .align_items(Align::Center)
                .child(Spacer::fixed(6.0))
                .child(
                    Text::new("Layers")
                        .size(13.0)
                        .semibold()
                        .letter_spacing(0.3)
                        .color(colors::active().icon_active),
                )
                .child(Spacer::flex(1.0))
                .child(
                    Row::new()
                        .gap(1.0)
                        .align_items(Align::Center)
                        .child(EyeToggle {
                            state: state.clone(),
                        })
                        .child(small_icon_button("plus", || {}))
                        .child(small_icon_button("chevron-down", || {})),
                ),
        )
        .child(
            Container::new()
                .fill_width()
                .margin(Edges::xy(6.0, 0.0))
                .child(search_field_cancellable(state.layer_query.clone())),
        )
        .child(Spacer::fixed(4.0))
        .child(
            Column::new()
                .fill_width()
                .fill_height()
                .padding(Edges::xy(6.0, 2.0))
                .child(Sec {
                    title: "PAGE 1",
                    open: state.sec_page_open.clone(),
                    cnt: page_cnt,
                })
                .child(
                    // The layer list scrolls itself and only builds the rows in
                    // view; with thousands of layers a plain list would paint
                    // every row on every frame.
                    Container::new()
                        .fill_size()
                        .flex(1.0)
                        .child(when(state.sec_page_open.clone(), layer_list(state, false))),
                )
                .child(Sec {
                    title: "COMPONENTS",
                    open: state.sec_comp_open.clone(),
                    cnt: comp_cnt,
                })
                .child(
                    Container::new()
                        .fill_width()
                        .height(160.0)
                        .child(when(state.sec_comp_open.clone(), layer_list(state, true))),
                ),
        )
}

#[derive(Clone)]
struct LeftPanel {
    state: EditorState,
}

impl View for LeftPanel {
    fn body(&self) -> impl View + 'static {
        panel_left_body(&self.state, self.state.left_panel_w.get())
    }
}

pub(crate) fn panel_left(state: &EditorState) -> impl View + 'static {
    LeftPanel {
        state: state.clone(),
    }
}
