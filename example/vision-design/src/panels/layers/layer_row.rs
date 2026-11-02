use std::time::Instant;

use sequoia_ui::prelude::*;

use crate::core::layer::Layer;
use crate::state::EditorState;
use crate::theme::colors;

#[derive(Clone)]
pub(crate) struct LayerRow {
    pub(crate) layer: Layer,
    pub(crate) state: EditorState,
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
                    .font_size(13.0)
                    .text_color(Color::WHITE)
                    .background(Color::hex(0x101012))
                    .border(1.0, colors::active().sel)
                    .radius(4.0)
                    .padding(Edges::xy(5.0, 1.0))
                    .on_submit(move |_| st_submit.end_layer_rename(true))
                    .on_edit_end(move |_| st_blur.end_layer_rename(true)),
                1.0,
            );
        } else {
            content = content.child_flex(
                Text::new(name).size(13.0).color(text_color).truncate_fill(),
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
                            colors::active().icon_inactive
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
            .hover(colors::active().bg_lighter)
            .style(
                BoxStyle::new()
                    .background(if sel {
                        colors::active().sel_soft
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