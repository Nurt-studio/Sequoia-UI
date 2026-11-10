use sequoia_ui::prelude::*;

use crate::state::EditorState;
use crate::theme::colors;

use super::controls::{drag_num, seg_btn};

fn align_icon_btn(
    icon: &'static str,
    active: bool,
    on_click: impl Fn() + 'static,
) -> impl View + 'static {
    Container::new()
        .flex(1.0)
        .height(28.0)
        .center()
        .radius(5.0)
        .background(if active {
            Color::hex(0x333336)
        } else {
            Color::TRANSPARENT
        })
        .hover(Color::hex(0x232325))
        .on_click(on_click)
        .child(Icon::new(icon).size(14.0).color(if active {
            colors::active().sel
        } else {
            colors::active().icon_inactive
        }))
}

fn axis_group(
    label: &'static str,
    current: &str,
    sig: Signal<String>,
    options: [(&'static str, &'static str); 3],
) -> impl View + 'static {
    let mut buttons = Row::new().gap(2.0);
    for (icon, value) in options {
        let s = sig.clone();
        buttons = buttons.child(align_icon_btn(icon, current == value, move || {
            s.set(value.into())
        }));
    }
    Row::new()
        .gap(6.0)
        .align_items(Align::Center)
        .child(
            Container::new().width(14.0).child(
                Text::new(label)
                    .size(11.0)
                    .color(colors::active().icon_inactive),
            ),
        )
        .child_flex(
            Container::new()
                .padding(3.0)
                .radius(8.0)
                .background(Color::hex(0x141416))
                .border(1.0, colors::active().stroke_soft)
                .child(buttons),
            1.0,
        )
}

fn choice_chip(
    label: &'static str,
    active: bool,
    on_click: impl Fn() + 'static,
) -> impl View + 'static {
    Container::new()
        .height(26.0)
        .padding(Edges::xy(6.0, 0.0))
        .center()
        .radius(6.0)
        .background(if active {
            Color::hex(0x333336)
        } else {
            Color::TRANSPARENT
        })
        .hover(if active {
            Color::hex(0x3E3E40)
        } else {
            Color::hex(0x232325)
        })
        .on_click(on_click)
        .child(Text::new(label).size(12.0).semibold().color(if active {
            Color::WHITE
        } else {
            colors::active().icon_inactive
        }))
}

fn flow_chip(
    label: &'static str,
    active: bool,
    on_click: impl Fn() + 'static,
) -> impl View + 'static {
    Container::new()
        .height(26.0)
        .padding(Edges::xy(10.0, 0.0))
        .center()
        .radius(6.0)
        .background(if active {
            Color::hex(0x333336)
        } else {
            colors::active().bg_darker
        })
        .border(
            1.0,
            if active {
                colors::active().stroke
            } else {
                colors::active().stroke_soft
            },
        )
        .hover(if active {
            Color::hex(0x3E3E40)
        } else {
            Color::hex(0x232325)
        })
        .on_click(on_click)
        .child(Text::new(label).size(12.0).semibold().color(if active {
            Color::WHITE
        } else {
            colors::active().icon_inactive
        }))
}

fn spacing_row(label: impl View + 'static, control: impl View + 'static) -> impl View + 'static {
    Row::new()
        .gap(6.0)
        .align_items(Align::Center)
        .child(
            Container::new()
                .width(40.0)
                .height(28.0)
                .center()
                .child(label),
        )
        .child_flex(control, 1.0)
}

fn spacing_controls(state: &EditorState) -> impl View + 'static {
    Column::new()
        .gap(2.0)
        .child(spacing_row(
            Text::new("Gap")
                .size(11.0)
                .color(colors::active().icon_inactive),
            drag_num(state.sel_gap.clone(), 0.0, 400.0, 0),
        ))
        .child(spacing_row(
            Text::new("Pad H")
                .size(11.0)
                .color(colors::active().icon_inactive),
            drag_num(state.sel_pad_h.clone(), 0.0, 200.0, 0),
        ))
        .child(spacing_row(
            Text::new("Pad V")
                .size(11.0)
                .color(colors::active().icon_inactive),
            drag_num(state.sel_pad_v.clone(), 0.0, 200.0, 0),
        ))
}

pub(super) fn body_auto_layout(state: &EditorState) -> impl View + 'static {
    let layout = state.sel_layout.clone();
    let cur = layout.get();
    let is_flow = cur == "H" || cur == "V";
    let wrap_on = state.sel_wrap.get();
    let is_text = state.sel_kind.get() == "TEXT";
    let parent_layout = state.sel_parent_layout.get();

    let st_h = state.clone();
    let st_v = state.clone();
    let st_g = state.clone();
    let st_none = state.clone();

    let dir_seg = Container::new()
        .fill_width()
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

    let mut col = Column::new().gap(12.0).child(dir_seg);

    if is_flow {
        let cur_ah = state.sel_align_h.get();
        let cur_av = state.sel_align_v.get();

        let align_h = axis_group(
            "H",
            &cur_ah,
            state.sel_align_h.clone(),
            [
                ("align/left", "start"),
                ("align/center-h", "center"),
                ("align/right", "end"),
            ],
        );
        let align_v = axis_group(
            "V",
            &cur_av,
            state.sel_align_v.clone(),
            [
                ("align/top", "start"),
                ("align/center-v", "center"),
                ("align/bottom", "end"),
            ],
        );

        col = col.child(
            Row::new()
                .gap(12.0)
                .align_items(Align::Center)
                .child_flex(Column::new().gap(6.0).child(align_h).child(align_v), 1.0)
                .child(Container::new().width(118.0).child(spacing_controls(state))),
        );
    } else {
        if cur == "G" {
            let label = |text: &'static str| {
                Container::new().width(48.0).child(
                    Text::new(text)
                        .size(11.0)
                        .color(colors::active().icon_inactive),
                )
            };
            col = col.child(
                Row::new()
                    .gap(6.0)
                    .align_items(Align::Center)
                    .child(label("Columns"))
                    .child_flex(drag_num(state.sel_grid_cols.clone(), 1.0, 24.0, 0), 1.0),
            );
            col = col.child(
                Row::new()
                    .gap(6.0)
                    .align_items(Align::Center)
                    .child(label("Rows"))
                    .child_flex(drag_num(state.sel_grid_rows.clone(), 0.0, 24.0, 0), 1.0),
            );
            col = col.child(
                Row::new()
                    .gap(6.0)
                    .align_items(Align::Center)
                    .child(label("Row gap"))
                    .child_flex(drag_num(state.sel_gap_row.clone(), 0.0, 400.0, 0), 1.0),
            );
        }
        col = col.child(spacing_controls(state));
    }

    if !is_text && parent_layout != "NONE" {
        let current = state.sel_align_self.get();
        let mut segments = Row::new().gap(2.0);
        for (label, value) in [
            ("Auto", "auto"),
            ("S", "start"),
            ("C", "center"),
            ("E", "end"),
            ("Stretch", "stretch"),
        ] {
            let sig = state.sel_align_self.clone();
            segments = segments.child(choice_chip(label, current == value, move || {
                sig.set(value.into())
            }));
        }
        col = col.child(
            Row::new()
                .gap(4.0)
                .align_items(Align::Center)
                .child(
                    Container::new().width(40.0).child(
                        Text::new("Self")
                            .size(11.0)
                            .color(colors::active().icon_inactive),
                    ),
                )
                .child_flex(segments, 1.0),
        );
    }

    if is_flow {
        let wrap_sig = state.sel_wrap.clone();
        let wrap_chip = flow_chip("Wrap", wrap_on, move || wrap_sig.update(|v| *v = !*v));

        let primary_is_sb = if cur == "V" {
            state.sel_align_v.get() == "space-between"
        } else {
            state.sel_align_h.get() == "space-between"
        };
        let sb_h = state.sel_align_h.clone();
        let sb_v = state.sel_align_v.clone();
        let column_dir = cur == "V";
        let sb_chip = flow_chip("Space between", primary_is_sb, move || {
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
                .align_items(Align::Center)
                .child(
                    Container::new().width(40.0).child(
                        Text::new("Flow")
                            .size(11.0)
                            .color(colors::active().icon_inactive),
                    ),
                )
                .child(wrap_chip)
                .child(sb_chip)
                .child(Spacer::flex(1.0)),
        );

        if wrap_on {
            col = col.child(
                Row::new()
                    .gap(4.0)
                    .align_items(Align::Center)
                    .child(
                        Container::new().width(44.0).child(
                            Text::new("Row gap")
                                .size(11.0)
                                .color(colors::active().icon_inactive),
                        ),
                    )
                    .child_flex(drag_num(state.sel_gap_row.clone(), 0.0, 400.0, 0), 1.0),
            );

            let ac = state.sel_align_content.get();
            let mut lines = Row::new().gap(4.0).align_items(Align::Center).child(
                Container::new().width(40.0).child(
                    Text::new("Lines")
                        .size(11.0)
                        .color(colors::active().icon_inactive),
                ),
            );
            for (label, value) in [("S", "start"), ("C", "center"), ("E", "end")] {
                let sig = state.sel_align_content.clone();
                let active = ac == value;
                lines = lines.child(choice_chip(label, active, move || sig.set(value.into())));
            }
            col = col.child(lines);
        }
    }

    let hint: Option<&'static str> = match cur.as_str() {
        "G" | "H" | "V" => Some("Drag a child to reorder it"),
        _ => None,
    };
    if let Some(h) = hint {
        col = col.child(
            Text::new(h)
                .size(11.0)
                .color(colors::active().icon_inactive),
        );
    }
    col
}
