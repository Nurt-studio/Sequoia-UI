use sequoia_core::{Color, Vec2};

use crate::{ShadowSpec, Theme};

/// Visual tokens for the model-driven docking system.
///
/// `DockTheme` controls the tab strip, split handles, panel chrome, and
/// floating cards drawn by `DockView`, `Dockable`, `DockPanel`, and
/// `DockFloatLayer`. The presets are self-contained so the dock can look
/// correct even when the application uses a different widget theme.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DockTheme {
    pub surface: Color,
    pub tabstrip: Color,
    pub tabstrip_border: Color,
    pub tab: Color,
    pub tab_hover: Color,
    pub tab_active: Color,
    pub tab_text: Color,
    pub tab_text_active: Color,
    pub accent: Color,
    pub split: Color,
    pub split_active: Color,
    pub panel: Color,
    pub panel_border: Color,
    pub panel_radius: f32,
    pub header: Color,
    pub header_text: Color,
    pub header_border: Color,
    pub float: Color,
    pub float_header: Color,
    pub float_border: Color,
    pub float_radius: f32,
    pub float_shadow: ShadowSpec,
    pub action_text: Color,
    pub action_hover: Color,
    pub danger: Color,
    pub tab_font_size: f32,
    pub header_font_size: f32,
    pub tab_padding: f32,
    pub header_padding: f32,
    pub tab_radius: f32,
}

impl DockTheme {
    pub const fn new() -> Self {
        Self::dark()
    }

    pub const fn dark() -> Self {
        Self {
            surface: Color::hex(0x16181D),
            tabstrip: Color::hex(0x1A1D23),
            tabstrip_border: Color::hex(0x262A32),
            tab: Color::TRANSPARENT,
            tab_hover: Color::hex(0x242832),
            tab_active: Color::hex(0x16181D),
            tab_text: Color::hex(0x8D95A3),
            tab_text_active: Color::hex(0xF3F5F9),
            accent: Color::hex(0x4C8DFF),
            split: Color::hex(0x2B303A),
            split_active: Color::hex(0x4C8DFF),
            panel: Color::hex(0x16181D),
            panel_border: Color::hex(0x262A32),
            panel_radius: 10.0,
            header: Color::hex(0x1B1F26),
            header_text: Color::hex(0xE7EBF2),
            header_border: Color::hex(0x262A32),
            float: Color::hex(0x1D2129),
            float_header: Color::hex(0x252A34),
            float_border: Color::hex(0x333947),
            float_radius: 12.0,
            float_shadow: ShadowSpec {
                color: Color::rgba(0.0, 0.0, 0.0, 0.5),
                blur: 24.0,
                spread: 0.0,
                offset: Vec2::new(0.0, 10.0),
            },
            action_text: Color::hex(0xA3ACBB),
            action_hover: Color::hexa(0xFFFFFF14),
            danger: Color::hex(0xFF6B6B),
            tab_font_size: 12.0,
            header_font_size: 12.5,
            tab_padding: 13.0,
            header_padding: 12.0,
            tab_radius: 6.0,
        }
    }

    pub const fn light() -> Self {
        Self {
            surface: Color::hex(0xFFFFFF),
            tabstrip: Color::hex(0xF5F6F9),
            tabstrip_border: Color::hex(0xE4E7ED),
            tab: Color::TRANSPARENT,
            tab_hover: Color::hex(0xEBEEF4),
            tab_active: Color::hex(0xFFFFFF),
            tab_text: Color::hex(0x6B7280),
            tab_text_active: Color::hex(0x111827),
            accent: Color::hex(0x2563EB),
            split: Color::hex(0xE4E7ED),
            split_active: Color::hex(0x2563EB),
            panel: Color::hex(0xFFFFFF),
            panel_border: Color::hex(0xE4E7ED),
            panel_radius: 12.0,
            header: Color::hex(0xF7F8FB),
            header_text: Color::hex(0x1F2937),
            header_border: Color::hex(0xE4E7ED),
            float: Color::hex(0xFFFFFF),
            float_header: Color::hex(0xF7F8FB),
            float_border: Color::hex(0xDDE2EA),
            float_radius: 14.0,
            float_shadow: ShadowSpec {
                color: Color::rgba(0.06, 0.09, 0.16, 0.18),
                blur: 28.0,
                spread: 0.0,
                offset: Vec2::new(0.0, 12.0),
            },
            action_text: Color::hex(0x64748B),
            action_hover: Color::hexa(0x0F172A14),
            danger: Color::hex(0xDC2626),
            tab_font_size: 12.0,
            header_font_size: 12.5,
            tab_padding: 13.0,
            header_padding: 12.0,
            tab_radius: 6.0,
        }
    }

    /// Builds dock tokens from the application theme.
    pub fn from_theme(theme: &Theme) -> Self {
        Self {
            surface: theme.background,
            tabstrip: theme.surface,
            tabstrip_border: theme.border,
            tab: Color::TRANSPARENT,
            tab_hover: theme.surface,
            tab_active: theme.background,
            tab_text: theme.text_secondary,
            tab_text_active: theme.text,
            accent: theme.accent,
            split: theme.border,
            split_active: theme.accent,
            panel: theme.background,
            panel_border: theme.border,
            panel_radius: (theme.radius + 8.0).max(8.0),
            header: theme.surface,
            header_text: theme.text,
            header_border: theme.border,
            float: theme.surface,
            float_header: theme.surface,
            float_border: theme.border,
            float_radius: (theme.radius + 10.0).max(10.0),
            float_shadow: ShadowSpec {
                color: Color::rgba(0.0, 0.0, 0.0, 0.28),
                blur: 22.0,
                spread: 0.0,
                offset: Vec2::new(0.0, 8.0),
            },
            action_text: theme.text_secondary,
            action_hover: theme.text.with_alpha(0.08),
            danger: Color::hex(0xDC2626),
            tab_font_size: theme.font_size,
            header_font_size: theme.font_size + 0.5,
            tab_padding: 13.0,
            header_padding: 12.0,
            tab_radius: (theme.radius + 3.0).max(3.0),
        }
    }

    pub const fn accent(mut self, color: Color) -> Self {
        self.accent = color;
        self
    }

    pub const fn surface(mut self, color: Color) -> Self {
        self.surface = color;
        self
    }

    pub const fn tabstrip(mut self, color: Color) -> Self {
        self.tabstrip = color;
        self
    }

    pub const fn panel(mut self, color: Color) -> Self {
        self.panel = color;
        self
    }

    pub const fn header(mut self, color: Color) -> Self {
        self.header = color;
        self
    }

    pub const fn float(mut self, color: Color) -> Self {
        self.float = color;
        self
    }

    pub const fn radius(mut self, radius: f32) -> Self {
        let radius = if radius.is_finite() && radius >= 0.0 {
            radius
        } else {
            self.panel_radius
        };
        self.panel_radius = radius;
        self.float_radius = radius + 2.0;
        self.tab_radius = radius.max(4.0);
        self
    }

    pub const fn font_size(mut self, size: f32) -> Self {
        if size.is_finite() && size > 0.0 {
            self.tab_font_size = size;
            self.header_font_size = size + 0.5;
        }
        self
    }
}

impl Default for DockTheme {
    fn default() -> Self {
        Self::dark()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_are_distinct() {
        let dark = DockTheme::dark();
        let light = DockTheme::light();
        assert_ne!(dark.surface, light.surface);
        assert_ne!(dark.panel, light.panel);
        assert_ne!(dark.tab_text, light.tab_text);
        assert_ne!(dark.accent, light.accent);
    }

    #[test]
    fn from_theme_maps_core_colors() {
        let theme = Theme::light();
        let dock = DockTheme::from_theme(&theme);
        assert_eq!(dock.surface, theme.background);
        assert_eq!(dock.panel, theme.background);
        assert_eq!(dock.tabstrip, theme.surface);
        assert_eq!(dock.accent, theme.accent);
        assert_eq!(dock.header_text, theme.text);
    }

    #[test]
    fn radius_is_sanitized() {
        let theme = DockTheme::light().radius(f32::NAN);
        assert_eq!(theme.panel_radius, DockTheme::light().panel_radius);
        let theme = DockTheme::light().radius(16.0);
        assert_eq!(theme.panel_radius, 16.0);
        assert_eq!(theme.float_radius, 18.0);
    }
}
