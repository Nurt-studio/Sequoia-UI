#![forbid(unsafe_code)]

mod avatar_style;
mod badge_style;
mod box_style;
mod button_style;
mod checkbox_style;
mod chip_style;
mod context_menu_style;
mod dialog_style;
mod divider_style;
mod drag_value_style;
mod drawer_style;
mod effect;
mod filter;
mod floating_menu_style;
mod focus_ring_style;
mod form_style;
mod kbd_style;
mod modal_style;
mod radio_style;
mod resize_handle_style;
mod sheet_style;
mod slider_style;
mod split_panel_style;
mod switch_style;
mod table_tree_style;
mod text_area_style;
mod text_field_style;
mod toast_style;
mod tokens;
mod tooltip_style;

pub use avatar_style::AvatarStyle;
pub use badge_style::BadgeStyle;
pub use box_style::{BorderSide, BoxStyle, ShadowSpec};
pub use button_style::ButtonStyle;
pub use checkbox_style::CheckboxStyle;
pub use chip_style::ChipStyle;
pub use context_menu_style::ContextMenuStyle;
pub use dialog_style::{DIALOG_DANGER, DialogStyle};
pub use divider_style::DividerStyle;
pub use drag_value_style::DragValueStyle;
pub use drawer_style::DrawerStyle;
pub use effect::Backdrop;
pub use filter::{
    Effect, Filter, GradientAlphas, Mask, VERTICAL, backdrop_blur, blur, gradient, gradient_angled,
    gradient_mask,
};
pub use floating_menu_style::FloatingMenuStyle;
pub use focus_ring_style::FocusRingStyle;
pub use form_style::{FORM_ERROR, FormStyle};
pub use kbd_style::KbdStyle;
pub use modal_style::ModalStyle;
pub use radio_style::RadioStyle;
pub use resize_handle_style::ResizeHandleStyle;
pub use sheet_style::SheetStyle;
pub use slider_style::SliderStyle;
pub use split_panel_style::SplitPanelStyle;
pub use switch_style::SwitchStyle;
pub use table_tree_style::{TableStyle, TreeStyle};
pub use text_area_style::TextAreaStyle;
pub use text_field_style::TextFieldStyle;
pub use toast_style::ToastStyle;
pub use tokens::TokensError;
pub use tooltip_style::TooltipStyle;

use sequoia_core::Color;

#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct Style {
    pub background: Option<Color>,

    pub radius: f32,

    pub color: Color,

    pub font_size: f32,

    pub line_height_factor: f32,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            background: None,
            radius: 0.0,
            color: Color::rgb8(0xE6, 0xE6, 0xE6),
            font_size: 14.0,
            line_height_factor: 1.35,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct Theme {
    pub background: Color,

    pub surface: Color,

    pub border: Color,

    pub text: Color,

    pub text_secondary: Color,

    pub accent: Color,

    pub accent_hover: Color,

    pub accent_pressed: Color,

    pub disabled: Color,

    pub radius: f32,

    pub font_size: f32,
}

impl Theme {
    pub const fn dark() -> Self {
        Self {
            background: Color::rgb8(0x19, 0x19, 0x19),
            surface: Color::rgb8(0x26, 0x26, 0x26),
            border: Color::rgb8(0x45, 0x45, 0x45),
            text: Color::rgb8(0xD4, 0xD4, 0xD4),
            text_secondary: Color::rgb8(0x9a, 0x9a, 0x9a),
            accent: Color::rgb8(0x35, 0xC2, 0xF1),
            accent_hover: Color::rgb8(0x5a, 0xD3, 0xF5),
            accent_pressed: Color::rgb8(0x1e, 0x9a, 0xCD),
            disabled: Color::rgb8(0x6B, 0x6B, 0x6B),
            radius: 3.0,
            font_size: 12.0,
        }
    }

    pub const fn dcc_dark_panel() -> Self {
        Self::dark()
    }

    pub const fn light() -> Self {
        Self {
            background: Color::rgb8(0xF5, 0xF5, 0xF5),
            surface: Color::rgb8(0xFF, 0xFF, 0xFF),
            border: Color::rgb8(0xD4, 0xD4, 0xD4),
            text: Color::rgb8(0x1A, 0x1A, 0x1A),
            text_secondary: Color::rgb8(0x5A, 0x5A, 0x5A),
            accent: Color::rgb8(0x0D, 0x6E, 0xD8),
            accent_hover: Color::rgb8(0x2B, 0x88, 0xE5),
            accent_pressed: Color::rgb8(0x0B, 0x5C, 0xB5),
            disabled: Color::rgb8(0xA8, 0xA8, 0xA8),
            radius: 3.0,
            font_size: 12.0,
        }
    }
}

pub const ACCENT_ACTIVE: Color = Color::rgb8(0xE8, 0x84, 0x2C);

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct VisualState {
    pub hovered: bool,

    pub pressed: bool,

    pub focused: bool,

    pub disabled: bool,

    pub selected: bool,
}

pub fn resolve_accent(theme: &Theme, state: VisualState) -> Color {
    if state.disabled {
        theme.disabled
    } else if state.pressed {
        theme.accent_pressed
    } else if state.hovered {
        theme.accent_hover
    } else {
        theme.accent
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_dark_is_sane() {
        let t = Theme::dark();
        assert_ne!(t.background, t.surface);
        assert_ne!(t.accent, t.accent_hover);
    }

    #[test]
    fn theme_light_is_sane_and_distinct() {
        let l = Theme::light();
        let d = Theme::dark();
        assert_ne!(l.background, d.background);
        assert_ne!(l.text, d.text);
        assert_ne!(l.background, l.surface);
        assert_ne!(l.accent, l.accent_hover);
    }

    #[test]
    fn resolve_accent_by_state() {
        let t = Theme::dark();
        assert_eq!(resolve_accent(&t, VisualState::default()), t.accent);
        assert_eq!(
            resolve_accent(
                &t,
                VisualState {
                    hovered: true,
                    ..Default::default()
                }
            ),
            t.accent_hover
        );
        assert_eq!(
            resolve_accent(
                &t,
                VisualState {
                    pressed: true,
                    ..Default::default()
                }
            ),
            t.accent_pressed
        );
        assert_eq!(
            resolve_accent(
                &t,
                VisualState {
                    disabled: true,
                    ..Default::default()
                }
            ),
            t.disabled
        );
    }
}
