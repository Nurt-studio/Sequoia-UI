//! Layers panel: header, search and sections.
mod layer_row;
mod layer_tree;

use self::layer_tree::layer_list;

use sequoia_ui::prelude::*;

use crate::components::icon_button::small_icon_button;

use crate::state::EditorState;
use crate::theme::colors;

fn search_box(state: &EditorState) -> impl View + 'static {
    let query = state.layer_query.clone();
    Container::new()
        .fill_width()
        .margin(Edges::xy(6.0, 0.0))
        .padding(Edges::xy(7.0, 4.0))
        .radius(6.0)
        .background(colors::active().bg_lighter)
        .border(1.0, Color::TRANSPARENT)
        .child(
            Row::new()
                .gap(6.0)
                .align_items(Align::Center)
                .child(
                    Icon::new("search")
                        .size(12.0)
                        .color(colors::active().icon_inactive),
                )
                .child_flex(
                    TextField::new(query)
                        .placeholder("Szukaj…")
                        .font_size(11.0)
                        .text_color(colors::active().text)
                        .background(Color::TRANSPARENT)
                        .border(1.0, Color::TRANSPARENT),
                    1.0,
                ),
        )
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

pub(crate) fn panel_left(state: &EditorState) -> impl View + 'static {
    let page_cnt = {
        let layers = state.layers.clone();
        memo(move || layers.get().iter().filter(|l| !l.component).count())
    };
    let comp_cnt = {
        let layers = state.layers.clone();
        memo(move || layers.get().iter().filter(|l| l.component).count())
    };

    Column::new()
        .width(228.0)
        .fill_height()
        .background(colors::active().surface)
        .border_right(1.0, colors::active().stroke_soft)
        .child(
            Row::new()
                .fill_width()
                .padding(Edges::xy(6.0, 6.0))
                .align_items(Align::Center)
                .child(Spacer::fixed(6.0))
                .child(
                    Text::new("Warstwy")
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
                        .child(small_icon_button("plus", || {}))
                        .child(small_icon_button("chevron-down", || {})),
                ),
        )
        .child(search_box(state))
        .child(Spacer::fixed(4.0))
        .child(
            ScrollView::new(
                Column::new()
                    .fill_width()
                    .padding(Edges::xy(6.0, 2.0))
                    .child(Sec {
                        title: "STRONA 1",
                        open: state.sec_page_open.clone(),
                        cnt: page_cnt,
                    })
                    .child(when(state.sec_page_open.clone(), layer_list(state, false)))
                    .child(Sec {
                        title: "KOMPONENTY",
                        open: state.sec_comp_open.clone(),
                        cnt: comp_cnt,
                    })
                    .child(when(state.sec_comp_open.clone(), layer_list(state, true)))
                    .child(Spacer::fixed(12.0)),
            )
            .vertical(),
        )
}