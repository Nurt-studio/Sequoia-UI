use std::time::Instant;

use sequoia_ui::prelude::*;
use sequoia_ui::view::Fill;

use crate::model::tree_ops;
use crate::state::{EditorState, LayerDrop};
use crate::theme::colors;

use super::layer_tree::{ROW_H, TreeRow, expand, toggle_collapsed, tree_rows};

/// Wraps a row so a right click can open the layer context menu. `PointerEvent`
/// positions are window coordinates, the same ones the canvas menu uses.
#[derive(Clone)]
struct LayerHit {
    state: EditorState,
    id: u64,
    child: AnyView,
}

impl View for LayerHit {
    fn body(&self) -> impl View + 'static {
        self.child.clone()
    }

    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fill(1.0),
            ..LayoutStyle::default()
        }
    }

    fn on_event(&self, event: &Event, ctx: &mut EventCtx) {
        if let Event::Pointer(PointerEvent::Down {
            button: PointerButton::Right,
            pos,
            ..
        }) = event
        {
            // Right-clicking an already selected row keeps a multi-selection
            // (so Group acts on everything that is selected); otherwise the
            // row becomes the selection.
            if !self
                .state
                .selected_layers
                .get_untracked()
                .contains(&self.id)
            {
                self.state.selected_layers.set(vec![self.id]);
            }
            self.state.selected_layer.set(Some(self.id));
            self.state.ctx_menu_x.set(pos.x + 6.0);
            self.state.ctx_menu_y.set(pos.y + 6.0);
            self.state.ctx_menu_open.set(true);
            ctx.handled();
        }
    }
}

#[derive(Clone)]
pub(crate) struct LayerRow {
    pub(crate) row: TreeRow,
    pub(crate) state: EditorState,
    /// Which section the row lives in (page layers vs components) — the drag
    /// & drop rows are resolved within the same visible tree.
    pub(crate) only_components: bool,
}

/// Converts the pointer offset into a drop target: the middle band of a row
/// drops INTO it, anywhere else inserts before/after it. Ids (not list
/// indexes) travel in the mark, so collapsing or filtering mid-drag cannot
/// retarget the drop.
pub(super) fn drop_mark(
    st: &EditorState,
    rows: &[TreeRow],
    dragged: u64,
    raw: f32,
) -> Option<LayerDrop> {
    if rows.is_empty() {
        return None;
    }
    let k = raw.round();
    let idx = k as i32;
    if (raw - k).abs() < 0.3
        && (0..rows.len() as i32).contains(&idx)
        && let Some(row) = rows.get(idx as usize)
    {
        let into = row.layer.id;
        if into != dragged && row.layer.kind != "TEXT" && !st.is_descendant(dragged, into) {
            return Some(LayerDrop::Into(into));
        }
    }
    let b = idx.clamp(0, rows.len() as i32) as usize;
    if let Some(row) = rows.get(b) {
        if row.layer.id != dragged {
            return Some(LayerDrop::Before(Some(row.layer.id)));
        }
        if let Some(next) = rows.get(b + 1) {
            return Some(LayerDrop::Before(Some(next.layer.id)));
        }
    }
    rows.iter()
        .rev()
        .find(|r| r.layer.id != dragged)
        .map(|r| LayerDrop::After(r.layer.id))
}

/// The drop that would leave the document unchanged (avoids history entries
/// and flickering "same spot" indicators).
pub(super) fn is_noop(st: &EditorState, id: u64, mark: LayerDrop) -> bool {
    let root = st.root.get_untracked();
    let Some(r) = root.as_ref() else {
        return true;
    };
    let parent = tree_ops::parent_of(r, id).map(|p| p.id()).unwrap_or(r.id());
    let siblings = tree_ops::find(r, parent)
        .map(|p| p.children().to_vec())
        .unwrap_or_default();
    let pos = siblings.iter().position(|c| c.id() == id);
    match mark {
        LayerDrop::Into(p) => {
            tree_ops::parent_of(r, id).map(|x| x.id()) == Some(p)
                && siblings.last().map(|c| c.id()) == Some(id)
        }
        LayerDrop::After(prev) => {
            let prev = siblings.iter().position(|c| c.id() == prev);
            matches!((prev, pos), (Some(a), Some(b)) if a + 1 == b)
        }
        LayerDrop::Before(Some(next)) => {
            let next = siblings.iter().position(|c| c.id() == next);
            matches!((next, pos), (Some(a), Some(b)) if a == b + 1)
        }
        LayerDrop::Before(None) => false,
    }
}

/// Runs a drop mark and expands the container it landed in.
pub(super) fn apply_drop(st: &EditorState, id: u64, mark: LayerDrop) {
    match mark {
        LayerDrop::Into(parent) => {
            st.move_node_into(id, parent);
            expand(st, parent);
        }
        LayerDrop::Before(next) => st.move_layer_before(id, next),
        LayerDrop::After(prev) => st.move_layer_after(id, prev),
    }
}

pub(super) fn end_drag(st: &EditorState) {
    st.lyr_drag_id.set(None);
    st.lyr_drop.set(None);
}

impl LayerRow {
    /// Grip handle: starts the drag. Tracking happens in the full-window
    /// overlay, so it also works in the browser build where pointer capture
    /// on a small canvas is not reliable.
    fn grip(&self) -> impl View + 'static {
        let st = self.state.clone();
        let id = self.row.layer.id;
        let only_components = self.only_components;

        Canvas::new()
            .width(16.0)
            .height(ROW_H - 4.0)
            .focusable(false)
            .capture(false)
            .on_draw(move |p| {
                // 2×3 grip dots.
                let color = Color::hex(0x6A6A70);
                for row in 0..3 {
                    for col in 0..2 {
                        let x = 4.0 + col as f32 * 6.0;
                        let y = 5.0 + row as f32 * 7.0;
                        p.circle(Point::new(x, y), 1.5, Fill::solid(color));
                    }
                }
            })
            .on_pointer(move |_pos, ev, ctx| {
                if let PointerEvent::Down {
                    button: PointerButton::Left,
                    ..
                } = ev
                {
                    let rows = tree_rows(&st, only_components);
                    let Some(idx) = rows.iter().position(|r| r.layer.id == id) else {
                        return;
                    };
                    st.lyr_drag_index.set(idx);
                    st.lyr_drag_start_y.set(ctx.pos.y);
                    st.lyr_drag_section.set(only_components);
                    st.lyr_drag_id.set(Some(id));
                    st.lyr_drop.set(None);
                    ctx.handled();
                }
            })
    }
}

impl View for LayerRow {
    fn body(&self) -> impl View + 'static {
        let layer = &self.row.layer;
        let sel = self.state.selected_layers.get().contains(&layer.id);
        let is_editing = self.state.editing_layer.get() == Some(layer.id);
        let drag_id = self.state.lyr_drag_id.get();
        let mark = self.state.lyr_drop.get();
        let id = layer.id;
        let layer_owned = layer.clone();
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
            .gap(5.0)
            .align_items(Align::Center)
            .child(self.grip());

        // Tree chevron: only containers are collapsible, leaves keep the space.
        let chevron = if self.row.has_children {
            let st_chev = st.clone();
            let open = self.row.expanded;
            Container::new()
                .width(16.0)
                .height(20.0)
                .center()
                .radius(3.0)
                .hover(colors::active().bg_lighter)
                .on_click(move || toggle_collapsed(&st_chev, id))
                .child(
                    Text::new(if open { "▾" } else { "▸" })
                        .size(14.0)
                        .color(colors::active().icon_inactive),
                )
        } else {
            Container::new().width(16.0)
        };
        content = content
            .child(chevron)
            .child(Spacer::fixed(self.row.depth as f32 * 16.0))
            .child(Icon::new(layer.icon).size(14.0).color(icon_color));

        if layer.locked {
            content = content.child(Icon::new("lyr/lock").size(14.0).color(Color::hex(0x8E8E93)));
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
                            .size(10.0)
                            .semibold()
                            .font("IBM Plex Mono")
                            .color(Color::hex(0x9ECDF5)),
                    ),
            );
        }

        let st_eye = st.clone();
        content = content.child(
            Container::new()
                .width(20.0)
                .height(20.0)
                .center()
                .radius(4.0)
                .hover(Color::hex(0x353537))
                // The eye edits the DOCUMENT (was: the derived layers signal,
                // which the layout effect immediately overwrote).
                .on_click(move || st_eye.toggle_layer_visible(id))
                .child(
                    Icon::new(if layer.visible { "eye" } else { "lyr/eyeoff" })
                        .size(14.0)
                        .color(if layer.visible {
                            colors::active().icon_inactive
                        } else {
                            Color::hex(0x5A5A5E)
                        }),
                ),
        );

        let st_click = st.clone();
        // Drop indicators: a line at the edge the layer would land on, a
        // highlight when it would land inside this container.
        let into_here = matches!(mark, Some(LayerDrop::Into(t)) if t == id);
        let insert_before = matches!(mark, Some(LayerDrop::Before(Some(next))) if next == id);
        let insert_after = matches!(mark, Some(LayerDrop::After(prev)) if prev == id);
        let mut row = Container::new()
            .fill_width()
            .padding(Edges::xy(6.0, 3.0))
            .radius(5.0)
            .hover(colors::active().bg_lighter)
            .style({
                let mut bs = BoxStyle::new();
                if into_here || sel {
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
        if insert_after {
            row = row.border_bottom(1.0, colors::active().sel);
        }

        let hidden = !layer_owned.visible;
        let dragging_this = drag_id == Some(id);
        let row_any = if hidden {
            AnyView::new(Opacity::new(row, 0.45))
        } else if dragging_this {
            AnyView::new(Opacity::new(row, 0.4))
        } else {
            AnyView::new(row)
        };
        if !hidden || self.state.show_hidden_layers.get() {
            AnyView::new(LayerHit {
                state: st.clone(),
                id,
                child: row_any,
            })
        } else {
            // The filter is on and this row is hidden — it should never be
            // built, but stay safe if the list logic ever drifts.
            AnyView::new(Unit)
        }
    }
}
