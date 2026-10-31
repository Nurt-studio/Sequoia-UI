use std::cell::Cell;

use sequoia_ui::prelude::Color;

#[derive(Clone, Copy, PartialEq)]
pub(crate) struct Palette {
    pub text: Color,
    pub surface: Color,
    pub bg_lighter: Color,
    pub bg_darker: Color,
    pub icon_inactive: Color,
    pub icon_active: Color,
    pub stroke: Color,
    pub stroke_soft: Color,
    pub accent: Color,
    pub accent_soft: Color,
    pub sel: Color,
    pub sel_soft: Color,
    pub danger: Color,
    pub ok: Color,
    pub warn: Color,
}

/// 🌙 Night — deep dark, current appearance
pub(crate) const NIGHT: Palette = Palette {
    text: Color::hex(0xC5CBCD),
    surface: Color::hex(0x1C1C1E),
    bg_lighter: Color::hex(0x262628),
    bg_darker: Color::hex(0x1B1B1B),
    icon_inactive: Color::hex(0x5A5A5E),
    icon_active: Color::hex(0xAEAEAE),
    stroke: Color::hex(0x2A2A2C),
    stroke_soft: Color::hex(0x232325),
    accent: Color::hex(0x0F62FE),
    accent_soft: Color::hexa(0x0F62FE2E),
    sel: Color::hex(0x0D99FF),
    sel_soft: Color::hexa(0x0D99FF29),
    danger: Color::hex(0xFA4D56),
    ok: Color::hex(0x3ECF8E),
    warn: Color::hex(0xF1C21B),
};

/// ☀️ Day (Comfortable Dark) — lighter gray theme for daytime work
pub(crate) const DAY: Palette = Palette {
    text: Color::hex(0xDFE4E7),          // Lighter, highly readable text
    surface: Color::hex(0x323236),       // Clearly lighter main surface
    bg_lighter: Color::hex(0x3F3F44),    // Highlighted elements / lighter background
    bg_darker: Color::hex(0x29292D),     // Deeper backgrounds / side panels
    icon_inactive: Color::hex(0x8C8C93), // Visible but muted icons
    icon_active: Color::hex(0xE0E0E0),   // Clear active icons
    stroke: Color::hex(0x4E4E54),        // Clearly visible borders
    stroke_soft: Color::hex(0x404046),   // Softer borders
    accent: Color::hex(0x3388FF),        // Slightly lighter accent (more readable on gray)
    accent_soft: Color::hexa(0x3388FF33),
    sel: Color::hex(0x3388FF),
    sel_soft: Color::hexa(0x3388FF33),
    danger: Color::hex(0xFF6B72),
    ok: Color::hex(0x4ADE80),
    warn: Color::hex(0xFACC15),
};

thread_local! {
    static ACTIVE: Cell<Palette> = const { Cell::new(NIGHT) };
}

/// Returns the active theme palette (read when building views).
pub(crate) fn active() -> Palette {
    ACTIVE.with(Cell::get)
}

/// Sets the palette used by subsequent view builds.
pub(crate) fn set_active(palette: Palette) {
    ACTIVE.with(|current| current.set(palette));
}
