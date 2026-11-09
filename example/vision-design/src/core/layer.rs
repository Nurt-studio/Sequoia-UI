//! Layer model and helper constructors (no UI).

use sequoia_ui::prelude::*;

pub(crate) fn next_id(layers: &[Layer]) -> u64 {
    layers.iter().map(|l| l.id).max().unwrap_or(0) + 1
}

pub(crate) fn pick_hex_for(kind: &str) -> (&'static str, Color) {
    match kind {
        "FRAME" => ("#1B1B1B", Color::hex(0x1B1B1B)),
        "RECTANGLE" => ("#0D99FF", Color::hex(0x0D99FF)),
        "ELLIPSE" => ("#42BE65", Color::hex(0x42BE65)),
        "TEXT" => ("#FFFFFF", Color::WHITE),
        _ => ("#0D99FF", Color::hex(0x0D99FF)),
    }
}

#[derive(Clone, PartialEq)]
pub(crate) struct Layer {
    pub(crate) id: u64,
    pub(crate) name: String,
    pub(crate) icon: &'static str,
    pub(crate) kind: &'static str,
    pub(crate) level: u8,
    pub(crate) parent: Option<u64>,
    pub(crate) visible: bool,
    pub(crate) locked: bool,
    pub(crate) component: bool,
    pub(crate) instance: bool,
    /// Slot container of a master — the dashed "hole" overlay on the canvas.
    pub(crate) slot: Option<String>,
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) w: f32,
    pub(crate) h: f32,
    pub(crate) opacity: f32,
    pub(crate) radius: f32,
    pub(crate) fill: Color,
    pub(crate) fill_hex: String,
    pub(crate) stroke: Color,
    pub(crate) stroke_hex: String,
    pub(crate) stroke_width: f32,
    pub(crate) text: String,
    pub(crate) font_size: f32,
    pub(crate) layout: &'static str,
    pub(crate) gap: f32,
    pub(crate) pad_l: f32,
    pub(crate) pad_t: f32,
    pub(crate) pad_r: f32,
    pub(crate) pad_b: f32,
    pub(crate) align_h: &'static str,
    pub(crate) align_v: &'static str,

    // per-child positioning
    pub(crate) position_mode: &'static str, // "RELATIVE" | "ABSOLUTE"
    pub(crate) pin_top: bool,
    pub(crate) pin_right: bool,
    pub(crate) pin_bottom: bool,
    pub(crate) pin_left: bool,
    pub(crate) self_pad_top: f32,
    pub(crate) self_pad_right: f32,
    pub(crate) self_pad_bottom: f32,
    pub(crate) self_pad_left: f32,
}
