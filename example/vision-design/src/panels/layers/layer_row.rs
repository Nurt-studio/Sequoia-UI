use std::cell::Cell;
use std::rc::Rc;
use std::time::Instant;

use sequoia_ui::prelude::*;
use sequoia_ui::view::Fill;

use crate::core::layer::Layer;
use crate::state::{EditorState, LayerDrop};
use crate::theme::colors;

use super::layer_tree::filtered_layers;

/// Height of one row in the layers list (keep in sync with `layer_tree`).
pub(super) const ROW_H: f32 = 28.0;

#[derive(Clone)]
pub(crate) struct LayerRow {
    pub(crate) layer: Layer,
    pub(crate) state: EditorState,
    /// Which section the row lives in (page layers vs components) — the drag
    /// & drop indexes are resolved within the same filtered list.
    pub(crate) only_components: bool,
}

impl LayerRow {
    /// Grip handle: pointer-down starts a drag; moving previews the drop
    /// mark, releasing applies it (reorder or reparent into a frame row).
    fn grip(&self) -> impl View + 'static {
        let st = self.state.clone();
        let id = self.layer.id;
        let only_components = self.only_components;
        let dragging: Rc<Cell<bool>> = Rc::new(Cell::new(false));
        let start_y: Rc<Cell<f32>> = Rc::new(Cell::new(0.0));
        let my_index: Rc<Cell<usize>> = Rc::new(Cell::new(0));

        let d_down = dragging.clone();
        let d_move = dragging.clone();
        let d_up = dragging.clone();
        let y_down = start_y.clone();
        let y_move = start_y.clone();
        let i_down = my_index.clone();
        let i_move = my_index.clone();

        Canvas::new()
            .width(12.0)
            .height(ROW_H - 4.0)
            .focusable(false)
            .on_draw(move |p| {
                // 2×3 grip dots.
                let color = Color::hex(0x55555A);
                for row in 0..3 {
                    for col in 0..2 {
                        let x = 3.0 + col as f32 * 5.0;
                        let y = 4.0 + row as f32 * 6.0;
                        p.circle(Point::new(x, y), 1.2, Fill::solid(color));
                    }
                }
            })
            .on_pointer(move |pos, ev, ctx| match ev {
                PointerEvent::Down {
                    button: PointerButton::Left,
                    ..
                } => {
                    let list = filtered_layers(&st, only_components);
                    let Some(idx) = list.iter().position(|l| l.id == id) else {
                        return;
                    };
                    d_down.set(true);
                    y_down.set(pos.y);
                    i_down.set(idx);
                    st.lyr_drag_id.set(Some(id));
                    st.lyr_drop.set(None);
                    ctx.handled();
                }
                PointerEvent::Move { .. } if d_move.get() => {
                    let list = filtered_layers(&st, only_components);
                    let len = list.len();
                    if len == 0 {
                        return;
                    }
                    // Rows are ROW_H tall: the pointer offset converts into a
                    // fractional row position. When the pointer sits near a
                    // row's middle band the drop goes INTO that frame;
                    // otherwise it inserts at the nearest boundary.
                    let raw = i_move.get() as f32 + (pos.y - y_move.get()) / ROW_H;
                    let k = raw.round();
                    let b = (k as i32).clamp(0, len as i32) as usize;
                    let into_zone = (raw - k).abs() < 0.22;
                    let mark = if into_zone && (k as usize) < len {
                        match list.get(k as usize) {
                            Some(l) if l.id != id && l.kind == "FRAME" => {
                                Some(LayerDrop::Into(l.id))
                            }
                            _ => Some(LayerDrop::Before(b)),
                        }
                    } else {
                        Some(LayerDrop::Before(b))
                    };
                    let same_spot =
                        matches!(mark, Some(LayerDrop::Before(b)) if b == i_move.get() || b == i_move.get() + 1);
                    st.lyr_drop.set(if same_spot { None } else { mark });
                    ctx.handled();
                }
                PointerEvent::Up { .. } if d_up.get() => {
                    d_up.set(false);
                    if let Some(mark) = st.lyr_drop.get_untracked() {
                        match mark {
                            LayerDrop::Into(parent) => st.move_node_into(id, parent),
                            LayerDrop::Before(b) => {
                                let list = filtered_layers(&st, only_components);
                                let next = list.get(b).map(|l| l.id);
                                st.move_layer_before(id, next);
                            }
                        }
                    }
                    st.lyr_drag_id.set(None);
                    st.lyr_drop.set(None);
                    ctx.handled();
                }
                _ => {}
            })
    }
}

impl View for LayerRow {
    fn body(&self) -> impl View + 'static {
        let sel = self.state.selected_layers.get().contains(&self.layer.id);
        let is_editing = self.state.editing_layer.get() == Some(self.layer.id);
        let drag_id = self.state.lyr_drag_id.get();
        let mark = self.state.lyr_drop.get();
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
            .child(self.grip())
            .child(Spacer::fixed(if layer.level > 0 { 12.0 } else { 0.0 }))
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
                // The eye edits the DOCUMENT (was: the derived layers signal,
                // which the layout effect immediately overwrote).
                .on_click(move || st_eye.toggle_layer_visible(id))
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
        // Drop indicators: an insertion line at the top edge, a highlight
        // when the layer would land inside this frame.
        let into_here = matches!(mark, Some(LayerDrop::Into(t)) if t == id);
        let insert_before = matches!(mark, Some(LayerDrop::Before(b)) if {
            let list = filtered_layers(&st_click, self.only_components);
            list.get(b).map(|l| l.id) == Some(id)
        });
        let mut row = Container::new()
            .fill_width()
            .padding(Edges::xy(6.0, 3.0))
            .radius(5.0)
            .hover(colors::active().bg_lighter)
            .style({
                let mut bs = BoxStyle::new();
                if into_here {
                    bs = bs
                        .background(colors::active().sel_soft)
                        .border(1.0, Color::hexa(0x0D99FF73));
                } else if sel {
                    bs = bs
                        .background(colors::active().sel_soft)
                        .border(1.0, Color::hexa(0x0D99FF73));
                } else {
                    bs = bs.border(1.0, Color::TRANSPARENT);
                }
                bs
            })
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
                    st_click.selected_layers.set(vec![id]);
                }
            })
            .child(content);
        if insert_before {
            row = row.border_top(1.0, colors::active().sel);
        }

        let hidden = !layer.visible;
        let dragging_this = drag_id == Some(id);
        let row_any = if hidden {
            AnyView::new(Opacity::new(row, 0.45))
        } else if dragging_this {
            AnyView::new(Opacity::new(row, 0.4))
        } else {
            AnyView::new(row)
        };
        if !hidden || self.state.show_hidden_layers.get() {
            row_any
        } else {
            // The filter is on and this row is hidden — it should never be
            // built, but stay safe if the list logic ever drifts.
            AnyView::new(Unit)
        }
    }
}
