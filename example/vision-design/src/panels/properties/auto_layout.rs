use sequoia_ui::prelude::*;

use crate::state::EditorState;
use crate::theme::colors;

use super::controls::{align_dot, drag_num, seg_btn};

pub(super) fn body_auto_layout(state: &EditorState) -> impl View + 'static {
    let layout = state.sel_layout.clone();
    let cur = layout.get();

    let l_h = layout.clone();
    let l_v = layout.clone();
    let l_z = layout.clone();
    let l_g = layout.clone();
    let l_none = layout.clone();

    let dir_seg = Container::new()
        .padding(3.0)
        .radius(8.0)
        .background(Color::hex(0x141416))
        .border(1.0, colors::active().stroke_soft)
        .child(
            Row::new()
                .gap(3.0)
                .child(seg_btn("H", cur == "H", move || l_h.set("H".into())))
                .child(seg_btn("V", cur == "V", move || l_v.set("V".into())))
                .child(seg_btn("Z", cur == "Z", move || l_z.set("Z".into())))
                .child(seg_btn("⊞", cur == "G", move || l_g.set("G".into())))
                .child(seg_btn("✕", cur == "NONE", move || {
                    l_none.set("NONE".into())
                })),
        );

    let top_row = Row::new()
        .gap(8.0)
        .align_items(Align::Center)
        .child(Container::new().flex(1.0).child(dir_seg))
        .child(
            Container::new()
                .width(64.0)
                .child(drag_num(state.sel_gap.clone(), 0.0, 200.0, 0)),
        );

    let show_align = cur == "H" || cur == "V";

    let cur_ah = state.sel_align_h.get();
    let cur_av = state.sel_align_v.get();
    let mut grid = Column::new().gap(4.0).align_items(Align::Center);
    for v in ["start", "center", "end"] {
        let mut row = Row::new().gap(4.0).align_items(Align::Center);
        for h in ["start", "center", "end"] {
            let active_dot = cur_ah == h && cur_av == v;
            let sh = state.sel_align_h.clone();
            let sv = state.sel_align_v.clone();
            let h_s: &'static str = match h {
                "center" => "center",
                "end" => "end",
                _ => "start",
            };
            let v_s: &'static str = match v {
                "center" => "center",
                "end" => "end",
                _ => "start",
            };
            row = row.child(align_dot(active_dot, move || {
                sh.set(h_s.into());
                sv.set(v_s.into());
            }));
        }
        grid = grid.child(row);
    }

    let align_box = Container::new()
        .padding(4.0)
        .radius(8.0)
        .background(Color::hex(0x141416))
        .border(1.0, colors::active().stroke_soft)
        .child(grid);

    let pad_col = Column::new()
        .gap(8.0)
        .child(
            Row::new()
                .gap(6.0)
                .align_items(Align::Center)
                .child(
                    Container::new().width(24.0).height(28.0).center().child(
                        Text::new("↔")
                            .size(14.0)
                            .color(colors::active().icon_inactive),
                    ),
                )
                .child_flex(drag_num(state.sel_pad_h.clone(), 0.0, 200.0, 0), 1.0),
        )
        .child(
            Row::new()
                .gap(6.0)
                .align_items(Align::Center)
                .child(
                    Container::new().width(24.0).height(28.0).center().child(
                        Text::new("↕")
                            .size(14.0)
                            .color(colors::active().icon_inactive),
                    ),
                )
                .child_flex(drag_num(state.sel_pad_v.clone(), 0.0, 200.0, 0), 1.0),
        );

    let bottom = if show_align {
        Row::new()
            .gap(12.0)
            .align_items(Align::Start)
            .child(align_box)
            .child(Container::new().flex(1.0).child(pad_col))
    } else {
        Row::new()
            .gap(12.0)
            .child(Container::new().flex(1.0).child(pad_col))
    };

    let hint: Option<&'static str> = match cur.as_str() {
        "Z" => Some("ZStack — children keep own position unless pinned (see Constraints)"),
        "G" => Some("Grid — flows from top-left, wraps by frame width"),
        _ => None,
    };

    let active = cur != "NONE";
    let mut col = Column::new().gap(12.0);
    col = col.child(top_row);
    if active {
        col = col.child(bottom);
    }
    if let Some(h) = hint {
        col = col.child(
            Text::new(h)
                .size(10.0)
                .color(colors::active().icon_inactive),
        );
    }
    col
}

// ---------------------------- Constraints (child in auto-layout parent) ---
