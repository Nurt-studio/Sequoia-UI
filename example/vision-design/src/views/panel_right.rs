use sequoia_ui::prelude::*;

use crate::colors;
use crate::state::{EditorState, Tab};

use crate::state::PickerTarget;
use crate::views::{color_well, parse_hex, rgb_to_hsv, to_hex};

const PANEL_W: f32 = 238.0;

// ---------------------------- helpers ----------------------------

fn mono_lbl(text: &'static str) -> impl View + 'static {
    Container::new().width(16.0).height(28.0).center().child(
        Text::new(text)
            .size(11.0)
            .font("IBM Plex Mono")
            .color(colors::ICON_INACTIVE),
    )
}

fn word_lbl(text: &'static str, width: f32) -> impl View + 'static {
    Container::new()
        .width(width)
        .height(28.0)
        .center()
        .child(Text::new(text).size(11.0).color(Color::hex(0x8E8E93)))
}

fn glyph(icon: &'static str) -> impl View + 'static {
    Container::new()
        .width(16.0)
        .height(28.0)
        .center()
        .child(Icon::new(icon).size(13.0).color(colors::ICON_INACTIVE))
}

fn drag_num(sig: Signal<f32>, min: f32, max: f32, precision: usize) -> impl View + 'static {
    Container::new()
        .height(28.0)
        .background(colors::BG_LIGHTER)
        .radius(6.0)
        .border(1.0, colors::STROKE_SOFT)
        .padding(Edges::xy(8.0, 0.0))
        .center()
        .child(
            DragValue::new(sig)
                .min(min)
                .max(max)
                .precision(precision)
                .trim_zeros(true),
        )
}

// ---------------------------- tab strip ----------------------------

fn tab_button(
    label: &'static str,
    active: bool,
    on_click: impl Fn() + 'static,
) -> impl View + 'static {
    let text_color = if active {
        Color::WHITE
    } else {
        colors::ICON_INACTIVE
    };
    let underline_color = if active {
        Color::hex(0xE8E8E8)
    } else {
        Color::TRANSPARENT
    };
    Container::new().height(32.0).on_click(on_click).child(
        Column::new()
            .align_items(Align::Center)
            .child(
                Container::new()
                    .fill_height()
                    .padding(Edges::xy(6.0, 0.0))
                    .center()
                    .child(Text::new(label).size(12.0).semibold().color(text_color)),
            )
            .child(
                Container::new()
                    .height(2.0)
                    .fill_width()
                    .background(underline_color),
            ),
    )
}

#[derive(Clone)]
struct TabStrip {
    state: EditorState,
}

impl View for TabStrip {
    fn body(&self) -> impl View + 'static {
        let tab = self.state.active_tab.get();
        let kind = self.state.sel_kind.get();
        let td = self.state.active_tab.clone();
        let ti = self.state.active_tab.clone();
        Row::new()
            .fill_width()
            .padding(Edges::xy(12.0, 0.0))
            .border_bottom(1.0, colors::STROKE_SOFT)
            .align_items(Align::Center)
            .gap(16.0)
            .child(tab_button("Design", tab == Tab::Design, move || {
                td.set(Tab::Design)
            }))
            .child(tab_button("Inspect", tab == Tab::Inspect, move || {
                ti.set(Tab::Inspect)
            }))
            .child(Spacer::flex(1.0))
            .child(
                Text::new(kind)
                    .size(11.0)
                    .font("IBM Plex Mono")
                    .color(colors::ICON_INACTIVE),
            )
    }
}

fn tab_strip(state: &EditorState) -> impl View + 'static {
    TabStrip {
        state: state.clone(),
    }
}

fn sel_head(state: &EditorState) -> impl View + 'static {
    let dims = {
        let w = state.sel_w.clone();
        let h = state.sel_h.clone();
        memo(move || format!("{:.0} × {:.0}", w.get(), h.get()))
    };
    Row::new()
        .padding(Edges::xy(12.0, 10.0))
        .border_bottom(1.0, colors::STROKE_SOFT)
        .gap(8.0)
        .align_items(Align::Center)
        .child(
            Container::new()
                .width(28.0)
                .height(28.0)
                .center()
                .radius(6.0)
                .background(colors::BG_LIGHTER)
                .border(1.0, colors::STROKE_SOFT)
                .child(Icon::new("tool/rect").size(14.0).color(colors::ICON_ACTIVE)),
        )
        .child_flex(
            TextField::new(state.sel_name.clone())
                .font_size(13.0)
                .text_color(Color::hex(0xF0F0F0))
                .background(Color::TRANSPARENT)
                .border(1.0, Color::TRANSPARENT),
            1.0,
        )
        .child(
            Text::with(move || dims.get())
                .size(11.0)
                .font("IBM Plex Mono")
                .color(colors::ICON_INACTIVE),
        )
}

#[derive(Clone)]
struct Section {
    title: &'static str,
    open: Signal<bool>,
    body: AnyView,
}

impl View for Section {
    fn body(&self) -> impl View + 'static {
        let open_now = self.open.get();
        let open_cb = self.open.clone();
        let chev = if open_now { "▾" } else { "▸" };
        Column::new()
            .border_bottom(1.0, colors::STROKE_SOFT)
            .padding(Edges::xy(0.0, 4.0))
            .child(
                Container::new()
                    .fill_width()
                    .padding(Edges::xy(12.0, 6.0))
                    .on_click(move || open_cb.update(|b| *b = !*b))
                    .child(
                        Row::new()
                            .gap(8.0)
                            .align_items(Align::Center)
                            .child(Text::new(chev).size(9.0).color(colors::ICON_INACTIVE))
                            .child(
                                Text::new(self.title)
                                    .size(12.0)
                                    .semibold()
                                    .color(Color::hex(0xC9C9CD)),
                            ),
                    ),
            )
            .child(when(
                self.open.clone(),
                Container::new()
                    .padding(Edges::xy(12.0, 4.0))
                    .child(self.body.clone()),
            ))
    }
}

fn section(title: &'static str, open: Signal<bool>, body: impl View + 'static) -> AnyView {
    AnyView::new(Section {
        title,
        open,
        body: AnyView::new(body),
    })
}

// ---------------------------- alignment ----------------------------

fn align_btn(icon: &'static str) -> impl View + 'static {
    Container::new()
        .height(28.0)
        .center()
        .radius(6.0)
        .hover(colors::BG_LIGHTER)
        .on_click(|| {})
        .child(Icon::new(icon).size(14.0).color(Color::hex(0x9A9A9E)))
}

fn body_alignment(_state: &EditorState) -> impl View + 'static {
    Row::new()
        .gap(2.0)
        .child_flex(align_btn("align/left"), 1.0)
        .child_flex(align_btn("align/center-h"), 1.0)
        .child_flex(align_btn("align/right"), 1.0)
        .child_flex(align_btn("align/top"), 1.0)
        .child_flex(align_btn("align/center-v"), 1.0)
        .child_flex(align_btn("align/bottom"), 1.0)
}

// ---------------------------- position / size ----------------------------

fn body_position(state: &EditorState) -> impl View + 'static {
    Column::new().gap(6.0).child(
        Row::new()
            .gap(6.0)
            .align_items(Align::Center)
            .child(mono_lbl("X"))
            .child_flex(drag_num(state.sel_x.clone(), -9999.0, 9999.0, 0), 1.0)
            .child(mono_lbl("Y"))
            .child_flex(drag_num(state.sel_y.clone(), -9999.0, 9999.0, 0), 1.0),
    )
}

fn body_size(state: &EditorState) -> impl View + 'static {
    Column::new().gap(6.0).child(
        Row::new()
            .gap(6.0)
            .align_items(Align::Center)
            .child(mono_lbl("W"))
            .child_flex(drag_num(state.sel_w.clone(), 0.0, 9999.0, 0), 1.0)
            .child(mono_lbl("H"))
            .child_flex(drag_num(state.sel_h.clone(), 0.0, 9999.0, 0), 1.0),
    )
}

fn body_appearance(state: &EditorState) -> impl View + 'static {
    Column::new().gap(6.0).child(
        Row::new()
            .gap(6.0)
            .align_items(Align::Center)
            .child(glyph("ui/opacity"))
            .child_flex(drag_num(state.sel_opacity.clone(), 0.0, 100.0, 0), 1.0)
            .child(glyph("ui/radius"))
            .child_flex(drag_num(state.sel_radius.clone(), 0.0, 999.0, 0), 1.0),
    )
}

// ---------------------------- text ----------------------------

fn body_text(state: &EditorState) -> impl View + 'static {
    let st = state.clone();
    Column::new()
        .gap(8.0)
        .child(
            Container::new().child(
                TextField::new(state.sel_text.clone())
                    .font_size(12.0)
                    .text_color(Color::hex(0xE8E8E8))
                    .background(colors::BG_LIGHTER)
                    .border(1.0, colors::STROKE_SOFT)
                    .radius(6.0)
                    .padding(Edges::xy(8.0, 6.0)),
            ),
        )
        .child(
            Row::new()
                .gap(6.0)
                .align_items(Align::Center)
                .child(mono_lbl("A"))
                .child_flex(drag_num(state.sel_font_size.clone(), 6.0, 200.0, 0), 1.0),
        )
        .child(
            Button::new("Edit on canvas")
                .style(
                    ButtonStyle::new()
                        .background(colors::BG_LIGHTER)
                        .hover(Color::hex(0x2E2E30))
                        .text(Color::hex(0xE8E8E8))
                        .font_size(12.0)
                        .radius(6.0)
                        .padding_xy(10.0, 6.0),
                )
                .on_click(move || st.start_text_edit(true)),
        )
}

// ---------------------------- Auto Layout ----------------------------

fn seg_btn(icon: &'static str, active: bool, on_click: impl Fn() + 'static) -> impl View + 'static {
    Container::new()
        .flex(1.0)
        .height(28.0)
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
        .child(Text::new(icon).size(14.0).color(if active {
            Color::WHITE
        } else {
            colors::ICON_INACTIVE
        }))
}

fn align_dot(active: bool, on_click: impl Fn() + 'static) -> impl View + 'static {
    Container::new()
        .width(24.0)
        .height(24.0)
        .center()
        .radius(4.0)
        .hover(Color::hex(0x2A2A2C))
        .on_click(on_click)
        .child(
            Container::new()
                .width(6.0)
                .height(6.0)
                .radius(1.5)
                .background(if active {
                    Color::hex(0x0D99FF)
                } else {
                    Color::hex(0x4A4A4D)
                }),
        )
}

fn body_auto_layout(state: &EditorState) -> impl View + 'static {
    let layout = state.sel_layout.clone();
    let cur = layout.get();

    let l_h = layout.clone();
    let l_v = layout.clone();
    let l_none = layout.clone();

    let dir_seg = Container::new()
        .padding(3.0)
        .radius(8.0)
        .background(Color::hex(0x141416))
        .border(1.0, colors::STROKE_SOFT)
        .child(
            Row::new()
                .gap(3.0)
                .child(seg_btn("→", cur == "H", move || l_h.set("H".into())))
                .child(seg_btn("↓", cur == "V", move || l_v.set("V".into())))
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

    let active = cur != "NONE";

    let cur_ah = state.sel_align_h.get();
    let cur_av = state.sel_align_v.get();
    let mut grid = Column::new().gap(4.0).align_items(Align::Center);
    let vs = ["start", "center", "end"];
    let hs = ["start", "center", "end"];

    for v in vs {
        let mut row = Row::new().gap(4.0).align_items(Align::Center);
        for h in hs {
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
        .border(1.0, colors::STROKE_SOFT)
        .child(grid);

    let pad_col = Column::new()
        .gap(8.0)
        .child(
            Row::new()
                .gap(6.0)
                .align_items(Align::Center)
                .child(
                    Container::new()
                        .width(24.0)
                        .height(28.0)
                        .center()
                        .child(Text::new("↔").size(14.0).color(colors::ICON_INACTIVE)),
                )
                .child_flex(drag_num(state.sel_pad_h.clone(), 0.0, 200.0, 0), 1.0),
        )
        .child(
            Row::new()
                .gap(6.0)
                .align_items(Align::Center)
                .child(
                    Container::new()
                        .width(24.0)
                        .height(28.0)
                        .center()
                        .child(Text::new("↕").size(14.0).color(colors::ICON_INACTIVE)),
                )
                .child_flex(drag_num(state.sel_pad_v.clone(), 0.0, 200.0, 0), 1.0),
        );

    let bottom_row = Row::new()
        .gap(12.0)
        .align_items(Align::Start)
        .child(align_box)
        .child(Container::new().flex(1.0).child(pad_col));

    let mut col = Column::new().gap(12.0);
    col = col.child(top_row);
    if active {
        col = col.child(bottom_row);
    }

    col
}

// ---------------------------- Fill / Stroke ----------------------------

fn swatch(
    color: Color,
    hex_str: &'static str,
    fill: Signal<Color>,
    hex_sig: Signal<String>,
    hue: Signal<f32>,
) -> impl View + 'static {
    let cur_now = fill.get() == color;
    let cb_fill = fill.clone();
    let cb_hex = hex_sig.clone();
    let cb_hue = hue.clone();
    Container::new()
        .width(22.0)
        .height(22.0)
        .radius(6.0)
        .border(
            if cur_now { 2.0 } else { 1.0 },
            if cur_now {
                Color::hex(0xE8E8E8)
            } else {
                Color::hexa(0xFFFFFF24)
            },
        )
        .background(color)
        .on_click(move || {
            cb_fill.set(color);
            cb_hex.set(hex_str.to_string());
            if let Some((r, g, b)) = parse_hex(hex_str) {
                let (h, s, v) = rgb_to_hsv(r, g, b);
                if s > 0.001 && v > 0.001 {
                    cb_hue.set(h);
                }
            }
        })
}

fn body_fill(state: &EditorState) -> impl View + 'static {
    let fill = state.sel_fill.clone();
    let hex = state.sel_fill_hex.clone();
    let hue = state.picker.hue.clone();

    Column::new()
        .gap(8.0)
        .child(
            Row::new()
                .gap(6.0)
                .align_items(Align::Center)
                .child(color_well(state, fill.clone(), PickerTarget::Fill))
                .child_flex(hex_field(hex.clone(), fill.clone(), hue.clone()), 1.0)
                .child(
                    Text::new("100%")
                        .size(11.0)
                        .font("IBM Plex Mono")
                        .color(colors::ICON_INACTIVE),
                ),
        )
        .child(Row::new().gap(6.0).children(vec![
            AnyView::new(swatch(
                Color::hex(0x0D99FF),
                "#0D99FF",
                fill.clone(),
                hex.clone(),
                hue.clone(),
            )),
            AnyView::new(swatch(
                Color::hex(0x42BE65),
                "#42BE65",
                fill.clone(),
                hex.clone(),
                hue.clone(),
            )),
            AnyView::new(swatch(
                Color::hex(0xF1C21B),
                "#F1C21B",
                fill.clone(),
                hex.clone(),
                hue.clone(),
            )),
            AnyView::new(swatch(
                Color::hex(0xFA4D56),
                "#FA4D56",
                fill.clone(),
                hex.clone(),
                hue.clone(),
            )),
            AnyView::new(swatch(
                Color::hex(0xC792EA),
                "#C792EA",
                fill.clone(),
                hex.clone(),
                hue.clone(),
            )),
            AnyView::new(swatch(
                Color::hex(0xFFFFFF),
                "#FFFFFF",
                fill.clone(),
                hex.clone(),
                hue.clone(),
            )),
            AnyView::new(swatch(
                Color::hex(0x1B1B1B),
                "#1B1B1B",
                fill.clone(),
                hex.clone(),
                hue.clone(),
            )),
        ]))
}

fn body_stroke(state: &EditorState) -> impl View + 'static {
    let stroke = state.sel_stroke.clone();
    let hex = state.sel_stroke_hex.clone();
    let hue = state.picker.hue.clone();

    Column::new().gap(8.0).child(
        Row::new()
            .gap(6.0)
            .align_items(Align::Center)
            .child(color_well(state, stroke.clone(), PickerTarget::Stroke))
            .child_flex(hex_field(hex.clone(), stroke.clone(), hue.clone()), 1.0)
            .child_flex(drag_num(state.sel_stroke_w.clone(), 0.0, 100.0, 0), 1.0),
    )
}

fn hex_field(hex: Signal<String>, fill: Signal<Color>, hue: Signal<f32>) -> impl View + 'static {
    let hex_set = hex.clone();
    TextField::new(hex)
        .font_size(12.0)
        .text_color(Color::hex(0xE8E8E8))
        .background(Color::TRANSPARENT)
        .border(0.0, Color::TRANSPARENT)
        .on_submit(move |text: String| {
            if let Some((r, g, b)) = parse_hex(&text) {
                let q = |x: f32| (x.clamp(0.0, 1.0) * 255.0).round() as u8;
                fill.set(Color::rgb8(q(r), q(g), q(b)));
                hex_set.set(to_hex(r, g, b));
                let (h, s, v) = rgb_to_hsv(r, g, b);
                if s > 0.001 && v > 0.001 {
                    hue.set(h);
                }
            }
        })
}

fn fmt_seg(label: &'static str, active: bool) -> impl View + 'static {
    Container::new()
        .flex(1.0)
        .height(26.0)
        .center()
        .radius(6.0)
        .background(if active {
            Color::hex(0x333336)
        } else {
            Color::TRANSPARENT
        })
        .on_click(|| {})
        .child(
            Text::new(label)
                .size(11.0)
                .font("IBM Plex Mono")
                .color(if active {
                    Color::WHITE
                } else {
                    colors::ICON_INACTIVE
                }),
        )
}

fn body_export(_state: &EditorState) -> impl View + 'static {
    Container::new()
        .padding(3.0)
        .radius(8.0)
        .background(Color::hex(0x141416))
        .border(1.0, colors::STROKE_SOFT)
        .child(
            Row::new()
                .gap(3.0)
                .child(fmt_seg("1x", true))
                .child(fmt_seg("2x", false))
                .child(fmt_seg("3x", false)),
        )
}

// ---------------------------- Inspect tab ----------------------------

fn inspect_body(state: &EditorState) -> impl View + 'static {
    let sx = state.sel_x.clone();
    let sy = state.sel_y.clone();
    let sw = state.sel_w.clone();
    let sh = state.sel_h.clone();
    let sr = state.sel_radius.clone();
    let sop = state.sel_opacity.clone();
    let sfh = state.sel_fill_hex.clone();
    let code = Text::with(move || {
        format!("position: absolute;\nleft: {:.0}px;\ntop: {:.0}px;\nwidth: {:.0}px;\nheight: {:.0}px;\nborder-radius: {:.0}px;\nopacity: {:.2};\nbackground: {};",
            sx.get(), sy.get(), sw.get(), sh.get(), sr.get(), sop.get() / 100.0, sfh.get())
    }).size(11.0).font("IBM Plex Mono").color(Color::hex(0xB9B9BD));

    Column::new()
        .padding(Edges::xy(12.0, 12.0))
        .gap(10.0)
        .child(
            Container::new()
                .padding(3.0)
                .radius(8.0)
                .background(Color::hex(0x141416))
                .border(1.0, colors::STROKE_SOFT)
                .child(
                    Row::new()
                        .gap(3.0)
                        .child(fmt_seg("CSS", true))
                        .child(fmt_seg("Tailwind", false))
                        .child(fmt_seg("Swift", false))
                        .child(fmt_seg("Kotlin", false))
                        .child(fmt_seg("XML", false)),
                ),
        )
        .child(
            Container::new()
                .padding(10.0)
                .radius(8.0)
                .background(Color::hex(0x141416))
                .border(1.0, colors::STROKE_SOFT)
                .child(code),
        )
}

fn empty_box() -> impl View + 'static {
    Container::new()
        .fill_width()
        .padding(Edges::xy(12.0, 24.0))
        .child(
            Column::new()
                .fill_width()
                .gap(8.0)
                .align_items(Align::Center)
                .child(
                    Text::new("Nothing selected")
                        .size(13.0)
                        .semibold()
                        .color(Color::hex(0xB9B9BD)),
                )
                .child(
                    Text::new("Select a layer on the canvas")
                        .size(12.0)
                        .color(colors::ICON_INACTIVE),
                )
                .child(
                    Text::new("to inspect and edit its properties.")
                        .size(12.0)
                        .color(colors::ICON_INACTIVE),
                ),
        )
}

#[derive(Clone)]
struct RightBody {
    state: EditorState,
}

impl View for RightBody {
    fn body(&self) -> impl View + 'static {
        let has_sel = self.state.selected_layer.get().is_some();
        let tab = self.state.active_tab.get();
        if !has_sel {
            AnyView::new(empty_box())
        } else if tab == Tab::Inspect {
            AnyView::new(
                Column::new()
                    .child(sel_head(&self.state))
                    .child(inspect_body(&self.state)),
            )
        } else {
            let kind = self.state.sel_kind.get();
            let is_text = kind == "TEXT";
            let is_frame = kind == "FRAME";

            let mut sections: Vec<AnyView> = vec![
                section(
                    "Alignment",
                    self.state.sections.alignment.clone(),
                    body_alignment(&self.state),
                ),
                section(
                    "Position",
                    self.state.sections.position.clone(),
                    body_position(&self.state),
                ),
                section(
                    "Size",
                    self.state.sections.size.clone(),
                    body_size(&self.state),
                ),
                section(
                    "Appearance",
                    self.state.sections.appearance.clone(),
                    body_appearance(&self.state),
                ),
            ];
            if is_text {
                sections.push(section(
                    "Text",
                    self.state.sections.fill.clone(),
                    body_text(&self.state),
                ));
            }
            if is_frame {
                sections.push(section(
                    "Auto Layout",
                    self.state.sections.auto_layout.clone(),
                    body_auto_layout(&self.state),
                ));
            }
            sections.push(section(
                "Fill",
                self.state.sections.fill.clone(),
                body_fill(&self.state),
            ));
            sections.push(section(
                "Stroke",
                self.state.sections.stroke.clone(),
                body_stroke(&self.state),
            ));
            sections.push(section(
                "Export",
                self.state.sections.export.clone(),
                body_export(&self.state),
            ));

            AnyView::new(
                Column::new()
                    .child(sel_head(&self.state))
                    .children(sections)
                    .child(Spacer::fixed(16.0)),
            )
        }
    }
}

pub(crate) fn panel_right(state: &EditorState) -> impl View + 'static {
    Column::new()
        .width(PANEL_W)
        .fill_height()
        .background(colors::SURFACE)
        .border_left(1.0, colors::STROKE_SOFT)
        .child(tab_strip(state))
        .child(
            ScrollView::new(RightBody {
                state: state.clone(),
            })
            .vertical(),
        )
}
