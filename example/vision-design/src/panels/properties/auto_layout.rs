use sequoia_ui::prelude::*;

use crate::state::EditorState;
use crate::theme::colors;

use super::controls::{align_dot, drag_num, mode_btn, seg_btn};

pub(super) fn body_auto_layout(state: &EditorState) -> impl View + 'static {
    let layout = state.sel_layout.clone();
    let cur = layout.get();
    let is_flow = cur == "H" || cur == "V";
    let wrap_on = state.sel_wrap.get();

    let st_h = state.clone();
    let st_v = state.clone();
    let st_g = state.clone();
    let st_none = state.clone();

    let dir_seg = Container::new()
        .padding(3.0)
        .radius(8.0)
        .background(Color::hex(0x141416))
        .border(1.0, colors::active().stroke_soft)
        .child(
            Row::new()
                .gap(3.0)
                .child(seg_btn("H", cur == "H", move || st_h.set_layout_mode("H")))
                .child(seg_btn("V", cur == "V", move || st_v.set_layout_mode("V")))
                .child(seg_btn("\u{229E}", cur == "G", move || {
                    st_g.set_layout_mode("G")
                }))
                .child(seg_btn("\u{2715}", cur == "NONE", move || {
                    st_none.set_layout_mode("NONE")
                })),
        );

    // Item spacing along the main axis.
    let top_row = Row::new()
        .gap(8.0)
        .align_items(Align::Center)
        .child(Container::new().flex(1.0).child(dir_seg))
        .child(
            Container::new()
                .width(64.0)
                .child(drag_num(state.sel_gap.clone(), 0.0, 400.0, 0)),
        );

    // Padding, shared by every layout mode.
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

    let mut col = Column::new().gap(12.0).child(top_row);

    if is_flow {
        let wrap_sig = state.sel_wrap.clone();
        let wrap_seg = mode_btn("Wrap", wrap_on, move || wrap_sig.update(|v| *v = !*v));

        let cur_ah = state.sel_align_h.get();
        let cur_av = state.sel_align_v.get();
        let primary_is_sb = if cur == "V" {
            cur_av == "space-between"
        } else {
            cur_ah == "space-between"
        };
        let sb_h = state.sel_align_h.clone();
        let sb_v = state.sel_align_v.clone();
        let column_dir = cur == "V";
        let sb_seg = mode_btn("Space between", primary_is_sb, move || {
            let flip = |v: &mut String| {
                *v = if *v == "space-between" {
                    "start".into()
                } else {
                    "space-between".into()
                };
            };
            if column_dir {
                sb_v.update(flip);
            } else {
                sb_h.update(flip);
            }
        });

        col = col.child(
            Row::new()
                .gap(4.0)
                .child_flex(wrap_seg, 1.0)
                .child_flex(sb_seg, 1.0),
        );

        // Alignment grid, spatial: columns are horizontal, rows vertical. The
        // layout direction decides which one is the primary axis.
        let mut grid = Column::new().gap(4.0).align_items(Align::Center);
        for v in ["start", "center", "end"] {
            let mut row = Row::new().gap(4.0).align_items(Align::Center);
            for h in ["start", "center", "end"] {
                let active = cur_ah == h && cur_av == v;
                let sh = state.sel_align_h.clone();
                let sv = state.sel_align_v.clone();
                let hs: &'static str = match h {
                    "center" => "center",
                    "end" => "end",
                    _ => "start",
                };
                let vs: &'static str = match v {
                    "center" => "center",
                    "end" => "end",
                    _ => "start",
                };
                row = row.child(align_dot(active, move || {
                    sh.set(hs.into());
                    sv.set(vs.into());
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

        col = col.child(
            Row::new()
                .gap(12.0)
                .align_items(Align::Start)
                .child(align_box)
                .child(Container::new().flex(1.0).child(pad_col)),
        );

        if wrap_on {
            // Spacing between wrapped lines.
            col = col.child(
                Row::new()
                    .gap(6.0)
                    .align_items(Align::Center)
                    .child(
                        Container::new().width(60.0).child(
                            Text::new("Row gap")
                                .size(10.0)
                                .color(colors::active().icon_inactive),
                        ),
                    )
                    .child_flex(drag_num(state.sel_gap_row.clone(), 0.0, 400.0, 0), 1.0),
            );

            // Cross-axis alignment of the wrapped lines (`align-content`).
            let ac = state.sel_align_content.get();
            let mut lines = Row::new().gap(4.0).align_items(Align::Center).child(
                Container::new().width(60.0).child(
                    Text::new("Lines")
                        .size(10.0)
                        .color(colors::active().icon_inactive),
                ),
            );
            for (label, value) in [("S", "start"), ("C", "center"), ("E", "end")] {
                let sig = state.sel_align_content.clone();
                let active = ac == value;
                lines = lines.child(
                    Container::new()
                        .padding(Edges::xy(9.0, 3.0))
                        .radius(4.0)
                        .background(if active {
                            Color::hex(0x333336)
                        } else {
                            Color::hex(0x1B1B1D)
                        })
                        .on_click(move || sig.set(value.into()))
                        .child(Text::new(label).size(10.0).color(if active {
                            Color::WHITE
                        } else {
                            colors::active().icon_inactive
                        })),
                );
            }
            col = col.child(lines);
        }
    } else {
        col = col.child(Container::new().child(pad_col));
    }

    let hint: Option<&'static str> = match cur.as_str() {
        "G" => Some("Grid — flows from top-left, wraps by frame width"),
        "H" | "V" => Some("Drag a child to reorder it; set Hug / Fill in Size"),
        _ => None,
    };
    if let Some(h) = hint {
        col = col.child(
            Text::new(h)
                .size(10.0)
                .color(colors::active().icon_inactive),
        );
    }
    col
}
