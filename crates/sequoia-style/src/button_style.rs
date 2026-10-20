use sequoia_core::Color;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ButtonStyle {
    pub background: Option<Color>,

    pub hover: Option<Color>,

    pub pressed: Option<Color>,

    pub disabled: Option<Color>,

    pub text: Option<Color>,

    pub text_disabled: Option<Color>,

    pub font_size: Option<f32>,

    pub font_weight: Option<u16>,

    pub radius: Option<f32>,

    pub padding: Option<(f32, f32)>,

    pub border_width: Option<f32>,

    pub border_color: Option<Color>,
}

impl ButtonStyle {
    pub const fn new() -> Self {
        Self {
            background: None,
            hover: None,
            pressed: None,
            disabled: None,
            text: None,
            text_disabled: None,
            font_size: None,
            font_weight: None,
            radius: None,
            padding: None,
            border_width: None,
            border_color: None,
        }
    }

    pub const PRIMARY: Self = Self {
        background: Some(Color::rgb8(0x35, 0x6F, 0xD1)),
        hover: Some(Color::rgb8(0x45, 0x82, 0xE0)),
        pressed: Some(Color::rgb8(0x2C, 0x5A, 0xA8)),
        disabled: None,
        text: Some(Color::WHITE),
        text_disabled: None,
        font_size: None,
        font_weight: Some(600),
        radius: None,
        padding: None,
        border_width: None,
        border_color: None,
    };

    pub const SECONDARY: Self = Self {
        background: Some(Color::rgb8(0x33, 0x36, 0x3D)),
        hover: Some(Color::rgb8(0x40, 0x44, 0x4E)),
        pressed: Some(Color::rgb8(0x2A, 0x2D, 0x33)),
        disabled: None,
        text: Some(Color::rgb8(0xE6, 0xE6, 0xE6)),
        text_disabled: None,
        font_size: None,
        font_weight: None,
        radius: None,
        padding: None,
        border_width: None,
        border_color: None,
    };

    pub const DANGER: Self = Self {
        background: Some(Color::rgb8(0xB3, 0x26, 0x1E)),
        hover: Some(Color::rgb8(0xD3, 0x2F, 0x2F)),
        pressed: Some(Color::rgb8(0x8A, 0x1A, 0x14)),
        disabled: None,
        text: Some(Color::WHITE),
        text_disabled: None,
        font_size: None,
        font_weight: Some(600),
        radius: None,
        padding: None,
        border_width: None,
        border_color: None,
    };

    pub const GHOST: Self = Self {
        background: Some(Color::TRANSPARENT),
        hover: Some(Color::rgb8(0x33, 0x36, 0x3D)),
        pressed: Some(Color::rgb8(0x2A, 0x2D, 0x33)),
        disabled: None,
        text: Some(Color::rgb8(0x35, 0x6F, 0xD1)),
        text_disabled: None,
        font_size: None,
        font_weight: None,
        radius: None,
        padding: None,
        border_width: None,
        border_color: None,
    };

    pub const DARK_FILLED: Self = Self {
        background: Some(Color::hex(0x1F1F1F)),
        hover: Some(Color::hex(0x2B2B2B)),
        pressed: Some(Color::hex(0x141414)),
        disabled: None,
        text: Some(Color::hex(0xEDEDED)),
        text_disabled: None,
        font_size: Some(12.0),
        font_weight: Some(500),
        radius: None,
        padding: Some((12.0, 6.0)),
        border_width: None,
        border_color: None,
    };

    pub const OUTLINE_ROSE: Self = Self {
        background: Some(Color::TRANSPARENT),
        hover: Some(Color::hexa(0xB0868626)),
        pressed: Some(Color::hexa(0xB086864D)),
        disabled: None,
        text: Some(Color::hex(0xB08686)),
        text_disabled: None,
        font_size: Some(12.0),
        font_weight: Some(600),
        radius: None,
        padding: Some((12.0, 6.0)),
        border_width: Some(1.0),
        border_color: Some(Color::hex(0xB08686)),
    };

    pub const fn primary() -> Self {
        Self::PRIMARY
    }

    pub const fn secondary() -> Self {
        Self::SECONDARY
    }

    pub const fn danger() -> Self {
        Self::DANGER
    }

    pub const fn ghost() -> Self {
        Self::GHOST
    }

    pub const fn dark_filled() -> Self {
        Self::DARK_FILLED
    }

    pub const fn outline_rose() -> Self {
        Self::OUTLINE_ROSE
    }

    pub const fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    pub const fn hover(mut self, color: Color) -> Self {
        self.hover = Some(color);
        self
    }

    pub const fn pressed(mut self, color: Color) -> Self {
        self.pressed = Some(color);
        self
    }

    pub const fn disabled(mut self, color: Color) -> Self {
        self.disabled = Some(color);
        self
    }

    pub const fn text(mut self, color: Color) -> Self {
        self.text = Some(color);
        self
    }

    pub const fn text_disabled(mut self, color: Color) -> Self {
        self.text_disabled = Some(color);
        self
    }

    pub const fn font_size(mut self, size: f32) -> Self {
        self.font_size = Some(size);
        self
    }

    pub const fn font_weight(mut self, weight: u16) -> Self {
        self.font_weight = Some(weight);
        self
    }

    pub const fn radius(mut self, radius: f32) -> Self {
        self.radius = Some(radius);
        self
    }

    pub const fn padding_xy(mut self, pad_x: f32, pad_y: f32) -> Self {
        self.padding = Some((pad_x, pad_y));
        self
    }

    pub const fn border_width(mut self, width: f32) -> Self {
        self.border_width = Some(width);
        self
    }

    pub const fn border_color(mut self, color: Color) -> Self {
        self.border_color = Some(color);
        self
    }

    pub const fn border(mut self, width: f32, color: Color) -> Self {
        self.border_width = Some(width);
        self.border_color = Some(color);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_differ_per_state() {
        assert_ne!(ButtonStyle::PRIMARY.background, ButtonStyle::PRIMARY.hover);
        assert_ne!(ButtonStyle::PRIMARY.hover, ButtonStyle::PRIMARY.pressed);
        assert_ne!(
            ButtonStyle::DANGER.background,
            ButtonStyle::PRIMARY.background
        );
    }

    #[test]
    fn const_composition_in_const_context() {
        const CUSTOM: ButtonStyle = ButtonStyle::PRIMARY.radius(12.0);
        assert_eq!(CUSTOM.radius, Some(12.0));
        assert_eq!(CUSTOM.background, ButtonStyle::PRIMARY.background);
    }

    #[test]
    fn struct_update_syntax_keeps_base() {
        const CUSTOM: ButtonStyle = ButtonStyle {
            background: Some(Color::hex(0xB3261E)),
            ..ButtonStyle::PRIMARY
        };
        assert_eq!(CUSTOM.background, Some(Color::hex(0xB3261E)));
        assert_eq!(CUSTOM.hover, ButtonStyle::PRIMARY.hover);
    }

    #[test]
    fn border_builders_set_both_fields() {
        const S: ButtonStyle = ButtonStyle::new().border(1.0, Color::WHITE);
        assert_eq!(S.border_width, Some(1.0));
        assert_eq!(S.border_color, Some(Color::WHITE));
    }

    #[test]
    fn new_variants_share_metrics_but_differ_in_look() {
        assert_eq!(
            ButtonStyle::DARK_FILLED.padding,
            ButtonStyle::OUTLINE_ROSE.padding
        );
        assert_eq!(ButtonStyle::DARK_FILLED.padding, Some((12.0, 6.0)));
        assert_eq!(ButtonStyle::DARK_FILLED.font_size, Some(12.0));
        assert_eq!(ButtonStyle::OUTLINE_ROSE.font_size, Some(12.0));

        assert_eq!(ButtonStyle::DARK_FILLED.font_weight, Some(500));
        assert_eq!(ButtonStyle::OUTLINE_ROSE.font_weight, Some(600));
        assert_eq!(
            ButtonStyle::DARK_FILLED.background,
            Some(Color::hex(0x1F1F1F))
        );
        assert_eq!(ButtonStyle::DARK_FILLED.text, Some(Color::hex(0xEDEDED)));
        assert_eq!(ButtonStyle::DARK_FILLED.border_width, None);
        assert_eq!(
            ButtonStyle::OUTLINE_ROSE.background,
            Some(Color::TRANSPARENT)
        );
        assert_eq!(ButtonStyle::OUTLINE_ROSE.text, Some(Color::hex(0xB08686)));
        assert_eq!(ButtonStyle::OUTLINE_ROSE.border_width, Some(1.0));
        assert_eq!(
            ButtonStyle::OUTLINE_ROSE.border_color,
            Some(Color::hex(0xB08686))
        );
        assert_ne!(
            ButtonStyle::DARK_FILLED.hover,
            ButtonStyle::DARK_FILLED.background
        );
        assert_ne!(
            ButtonStyle::OUTLINE_ROSE.hover,
            ButtonStyle::OUTLINE_ROSE.background
        );
    }
}
