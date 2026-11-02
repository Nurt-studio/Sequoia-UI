use sequoia_ui::prelude::*;

use crate::state::EditorState;
use crate::theme::colors;

use super::controls::fmt_seg;

pub(super) fn inspect_body(state: &EditorState) -> impl View + 'static {
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
                .border(1.0, colors::active().stroke_soft)
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
                .border(1.0, colors::active().stroke_soft)
                .child(code),
        )
}

pub(super) fn empty_box() -> impl View + 'static {
    Container::new()
        .fill_width()
        .padding(Edges::xy(12.0, 24.0))
        .child(
            Column::new()
                .fill_width()
                .gap(8.0)
                .align_items(Align::Center)
                .child(
                    Text::new("Nic nie jest zaznaczone")
                        .size(13.0)
                        .semibold()
                        .color(Color::hex(0xB9B9BD)),
                )
                .child(
                    Text::new("Zaznacz warstwę na canvasie,")
                        .size(12.0)
                        .color(colors::active().icon_inactive),
                )
                .child(
                    Text::new("aby oglądać i edytować jej właściwości.")
                        .size(12.0)
                        .color(colors::active().icon_inactive),
                ),
        )
}