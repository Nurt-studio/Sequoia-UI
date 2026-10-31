use sequoia_ui::prelude::*;

use crate::theme::colors;
use crate::overlays::{parse_hex, rgb_to_hsv, to_hex};

pub(super) use crate::components::number_field::icon_label as glyph;
pub(super) use crate::components::number_field::mono_label as mono_lbl;
pub(super) use crate::components::number_field::number_field as drag_num;
pub(super) use crate::components::segmented::{segment as mode_btn, segment_icon as seg_btn};




// ---------------------------- tab strip ----------------------------



pub(super) fn align_dot(active: bool, on_click: impl Fn() + 'static) -> impl View + 'static {
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



pub(super) fn pin_toggle(sig: Signal<bool>) -> impl View + 'static {
    let on = sig.get();
    let s = sig.clone();
    Container::new()
        .width(18.0)
        .height(18.0)
        .center()
        .radius(4.0)
        .border(
            1.0,
            if on { Color::hex(0x0D99FF) } else { colors::active().stroke_soft },
        )
        .background(if on {
            Color::hex(0x0D99FF)
        } else {
            Color::hex(0x1B1B1D)
        })
        .on_click(move || s.update(|v| *v = !*v))
        .child(if on {
            AnyView::new(Text::new("✓").size(10.0).semibold().color(Color::WHITE))
        } else {
            AnyView::new(Unit)
        })
}

pub(super) fn pin_row(
    label: &'static str,
    pin: Signal<bool>,
    pad: Signal<f32>,
) -> impl View + 'static {
    Row::new()
        .gap(6.0)
        .align_items(Align::Center)
        .child(pin_toggle(pin))
        .child(
            Container::new()
                .width(46.0)
                .child(Text::new(label).size(11.0).color(colors::active().text)),
        )
        .child_flex(drag_num(pad, 0.0, 999.0, 0), 1.0)
}


pub(super) fn swatch(
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


pub(super) fn hex_field(hex: Signal<String>, fill: Signal<Color>, hue: Signal<f32>) -> impl View + 'static {
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




/// Static export format segment (no click action).
pub(super) fn fmt_seg(label: &'static str, active: bool) -> impl View + 'static {
    crate::components::segmented::segment_mono(label, active, || {})
}
