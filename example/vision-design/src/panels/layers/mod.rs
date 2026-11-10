//! Layers panel: header (with the "hide hidden" eye), search pill and
//! sections.
mod layer_row;
mod layer_tree;

use self::layer_tree::{ROW_H, layer_list, tree_rows};

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
                    .size(14.0)
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
                            .size(11.0)
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
                                    .size(10.0)
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

/// Full-window drag layer: appears while a layer row is being dragged and
/// resolves the drop from the pointer position. It does not rely on pointer
/// capture on the tiny grip, so drag & drop behaves the same in the browser.
#[derive(Clone)]
struct DragOverlay {
    state: EditorState,
}

impl View for DragOverlay {
    fn body(&self) -> impl View + 'static {
        if self.state.lyr_drag_id.get().is_none() {
            return AnyView::new(Unit);
        }
        let st = self.state.clone();
        AnyView::new(
            Canvas::new()
                .fill_size()
                .focusable(false)
                .capture(false)
                .on_pointer(move |pos, ev, ctx| match ev {
                    PointerEvent::Move { .. }
                    | PointerEvent::PenMove { .. }
                    | PointerEvent::TouchMove { .. } => {
                        let Some(id) = st.lyr_drag_id.get_untracked() else {
                            return;
                        };
                        let section = st.lyr_drag_section.get_untracked();
                        let rows = tree_rows(&st, section);
                        if rows.is_empty() {
                            return;
                        }
                        let raw = st.lyr_drag_index.get_untracked() as f32
                            + (pos.y - st.lyr_drag_start_y.get_untracked()) / ROW_H;
                        let mark = self::layer_row::drop_mark(&st, &rows, id, raw);
                        st.lyr_drop
                            .set(mark.filter(|m| !self::layer_row::is_noop(&st, id, *m)));
                        ctx.handled();
                    }
                    PointerEvent::Up { .. }
                    | PointerEvent::PenUp { .. }
                    | PointerEvent::TouchUp { .. } => {
                        if let (Some(id), Some(mark)) =
                            (st.lyr_drag_id.get_untracked(), st.lyr_drop.get_untracked())
                        {
                            self::layer_row::apply_drop(&st, id, mark);
                        }
                        self::layer_row::end_drag(&st);
                        ctx.handled();
                    }
                    PointerEvent::Cancel => {
                        self::layer_row::end_drag(&st);
                        ctx.handled();
                    }
                    _ => {}
                }),
        )
    }
}

pub(crate) fn drag_overlay(state: &EditorState) -> impl View + 'static {
    DragOverlay {
        state: state.clone(),
    }
}
