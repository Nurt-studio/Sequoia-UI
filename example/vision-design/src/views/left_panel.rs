use std::time::Instant;

use sequoia_ui::prelude::*;

use crate::colors;
use crate::state::{EditorState, Layer};

fn head_btn(icon: &'static str, on_click: impl Fn() + 'static) -> impl View + 'static {
    Container::new()
        .width(20.0)
        .height(20.0)
        .center()
        .radius(5.0)
        .hover(colors::BG_LIGHTER)
        .on_click(on_click)
        .child(Icon::new(icon).size(12.0).color(colors::ICON_INACTIVE))
}

fn search_box(state: &EditorState) -> impl View + 'static {
    let query = state.layer_query.clone();
    Container::new()
        .fill_width()
        .margin(Edges::xy(6.0, 0.0))
        .padding(Edges::xy(7.0, 4.0))
        .radius(6.0)
        .background(colors::BG_LIGHTER)
        .border(1.0, Color::TRANSPARENT)
        .child(
            Row::new()
                .gap(6.0)
                .align_items(Align::Center)
                .child(Icon::new("search").size(12.0).color(colors::ICON_INACTIVE))
                .child_flex(
                    TextField::new(query)
                        .placeholder("Search…")
                        .font_size(11.0)
                        .text_color(colors::TEXT)
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
            .hover(colors::SURFACE)
            .on_click(move || toggle.update(|o| *o = !*o))
            .child(
                Row::new()
                    .gap(4.0)
                    .align_items(Align::Center)
                    .child(
                        Text::new(if open { "▾" } else { "▸" })
                            .size(8.0)
                            .color(colors::ICON_INACTIVE),
                    )
                    .child(
                        Text::new(self.title)
                            .size(10.0)
                            .semibold()
                            .letter_spacing(0.6)
                            .color(colors::ICON_INACTIVE),
                    )
                    .child(Spacer::flex(1.0))
                    .child(
                        Container::new()
                            .padding(Edges::xy(6.0, 1.0))
                            .radius(10.0)
                            .background(colors::BG_LIGHTER)
                            .child(
                                Text::with(move || format!("{}", cnt.get()))
                                    .size(10.0)
                                    .font("IBM Plex Mono")
                                    .color(colors::ICON_INACTIVE),
                            ),
                    ),
            )
    }
}

#[derive(Clone)]
struct LayerRow {
    layer: Layer,
    state: EditorState,
}

impl View for LayerRow {
    fn body(&self) -> impl View + 'static {
        let sel = self.state.selected_layer.get() == Some(self.layer.id);
        let is_editing = self.state.editing_layer.get() == Some(self.layer.id);
        let id = self.layer.id;
        let layer = self.layer.clone();
        let st = self.state.clone();

        let name = if layer.instance {
            format!("◈ {}", layer.name)
        } else {
            layer.name.clone()
        };
        let icon_color = if layer.component {
            Color::hex(0xCFA6F5)
        } else if sel {
            Color::WHITE
        } else {
            Color::hex(0x9A9A9E)
        };
        let text_color = if sel {
            Color::WHITE
        } else {
            Color::hex(0x9A9A9E)
        };

        let mut content = Row::new()
            .gap(6.0)
            .align_items(Align::Center)
            .child(Spacer::fixed(if layer.level > 0 { 16.0 } else { 0.0 }))
            .child(Icon::new(layer.icon).size(13.0).color(icon_color));

        if layer.locked {
            content = content.child(Icon::new("lyr/lock").size(11.0).color(Color::hex(0x8E8E93)));
        }

        if is_editing {
            let st_submit = st.clone();
            let st_blur = st.clone();
            content = content.child_flex(
                TextField::new(st.rename_buffer.clone())
                    .font_size(12.0)
                    .text_color(Color::WHITE)
                    .background(Color::hex(0x101012))
                    .border(1.0, colors::SEL)
                    .radius(4.0)
                    .padding(Edges::xy(5.0, 1.0))
                    .on_submit(move |_| st_submit.end_layer_rename(true))
                    .on_edit_end(move |_| st_blur.end_layer_rename(true)),
                1.0,
            );
        } else {
            content = content.child_flex(
                Text::new(name).size(12.0).color(text_color).truncate_fill(),
                1.0,
            );
        }

        if layer.kind == "FRAME" && layer.layout != "NONE" {
            content = content.child(
                Container::new()
                    .padding(Edges::xy(4.0, 1.0))
                    .radius(4.0)
                    .background(Color::hexa(0x0D99FF24))
                    .child(
                        Text::new("AL")
                            .size(8.5)
                            .semibold()
                            .font("IBM Plex Mono")
                            .color(Color::hex(0x9ECDF5)),
                    ),
            );
        }

        let st_eye = st.clone();
        content = content.child(
            Container::new()
                .width(18.0)
                .height(18.0)
                .center()
                .radius(4.0)
                .hover(Color::hex(0x353537))
                .on_click(move || {
                    let id = id;
                    st_eye.layers.update(|ls| {
                        for l in ls.iter_mut() {
                            if l.id == id {
                                l.visible = !l.visible;
                            }
                        }
                    });
                })
                .child(
                    Icon::new(if layer.visible { "eye" } else { "lyr/eyeoff" })
                        .size(12.0)
                        .color(if layer.visible {
                            colors::ICON_INACTIVE
                        } else {
                            Color::hex(0x5A5A5E)
                        }),
                ),
        );

        let st_click = st.clone();
        let row = Container::new()
            .fill_width()
            .padding(Edges::xy(6.0, 3.0))
            .radius(5.0)
            .hover(colors::BG_LIGHTER)
            .style(
                BoxStyle::new()
                    .background(if sel {
                        colors::SEL_SOFT
                    } else {
                        Color::TRANSPARENT
                    })
                    .border(
                        1.0,
                        if sel {
                            Color::hexa(0x0D99FF73)
                        } else {
                            Color::TRANSPARENT
                        },
                    ),
            )
            .on_click(move || {
                let now = Instant::now();
                let lcc = st_click.last_canvas_click.clone();
                let is_dbl = {
                    let c = lcc.borrow();
                    c.0 == Some(id) && now.duration_since(c.1).as_millis() < 400
                };
                if is_dbl {
                    *lcc.borrow_mut() = (None, now);
                    st_click.open_layer_rename(id);
                } else {
                    *lcc.borrow_mut() = (Some(id), now);
                    st_click.selected_layer.set(Some(id));
                }
            })
            .child(content);

        if layer.visible {
            AnyView::new(row)
        } else {
            AnyView::new(Opacity::new(row, 0.45))
        }
    }
}

fn layer_list(state: &EditorState, only_components: bool) -> impl View + 'static {
    let layers = state.layers.clone();
    let query = state.layer_query.clone();
    let st = state.clone();
    List::new({
        let layers = layers.clone();
        let query = query.clone();
        move || {
            let q = query.get().to_lowercase();
            let all = layers.get();
            all.into_iter()
                .filter(|l| l.component == only_components)
                .filter(|l| q.is_empty() || l.name.to_lowercase().contains(&q))
                .collect::<Vec<_>>()
        }
    })
    .key(|l: &Layer| l.id)
    .item({
        let st = st.clone();
        move |l: &Layer| LayerRow {
            layer: l.clone(),
            state: st.clone(),
        }
    })
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
        .background(colors::SURFACE)
        .border_right(1.0, colors::STROKE_SOFT)
        .child(
            Row::new()
                .fill_width()
                .padding(Edges::xy(6.0, 6.0))
                .align_items(Align::Center)
                .child(Spacer::fixed(6.0))
                .child(
                    Text::new("Layers")
                        .size(11.0)
                        .semibold()
                        .letter_spacing(0.3)
                        .color(colors::ICON_ACTIVE),
                )
                .child(Spacer::flex(1.0))
                .child(
                    Row::new()
                        .gap(1.0)
                        .align_items(Align::Center)
                        .child(head_btn("plus", || {}))
                        .child(head_btn("chevron-down", || {})),
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
                        title: "PAGE 1",
                        open: state.sec_page_open.clone(),
                        cnt: page_cnt,
                    })
                    .child(when(state.sec_page_open.clone(), layer_list(state, false)))
                    .child(Sec {
                        title: "COMPONENTS",
                        open: state.sec_comp_open.clone(),
                        cnt: comp_cnt,
                    })
                    .child(when(state.sec_comp_open.clone(), layer_list(state, true)))
                    .child(Spacer::fixed(12.0)),
            )
            .vertical(),
        )
}
