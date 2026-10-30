use sequoia_core::Color;

use crate::ButtonStyle;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FormStyle {
    pub error: Option<Color>,

    pub label_size: Option<f32>,

    pub error_size: Option<f32>,

    pub button: Option<ButtonStyle>,
}

impl FormStyle {
    pub const fn new() -> Self {
        Self {
            error: None,
            label_size: None,
            error_size: None,
            button: None,
        }
    }

    pub const fn error(mut self, color: Color) -> Self {
        self.error = Some(color);
        self
    }

    pub const fn label_size(mut self, size: f32) -> Self {
        self.label_size = Some(size);
        self
    }

    pub const fn error_size(mut self, size: f32) -> Self {
        self.error_size = Some(size);
        self
    }

    pub const fn button(mut self, style: ButtonStyle) -> Self {
        self.button = Some(style);
        self
    }
}

pub const FORM_ERROR: Color = Color::rgb8(0xFF, 0x5F, 0x57);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builders_compose() {
        let s = FormStyle::new()
            .label_size(13.0)
            .error_size(10.0)
            .button(ButtonStyle::PRIMARY);
        assert_eq!(s.label_size, Some(13.0));
        assert_eq!(s.error_size, Some(10.0));
        assert_eq!(s.error, None);
        assert_eq!(s.button, Some(ButtonStyle::PRIMARY));
        assert_eq!(FORM_ERROR, Color::rgb8(0xFF, 0x5F, 0x57));
    }
}
