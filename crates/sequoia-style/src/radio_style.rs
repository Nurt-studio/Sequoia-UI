use sequoia_core::Color;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RadioStyle {
    pub background: Option<Color>,
    pub checked: Option<Color>,
    pub border: Option<Color>,
    pub checked_border: Option<Color>,
    pub dot: Option<Color>,
    pub text: Option<Color>,
    pub font_size: Option<f32>,
    pub diameter: Option<f32>,
}

impl RadioStyle {
    pub const fn new() -> Self {
        Self {
            background: None,
            checked: None,
            border: None,
            checked_border: None,
            dot: None,
            text: None,
            font_size: None,
            diameter: None,
        }
    }

    pub const fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    pub const fn checked(mut self, color: Color) -> Self {
        self.checked = Some(color);
        self
    }

    pub const fn border(mut self, color: Color) -> Self {
        self.border = Some(color);
        self
    }

    pub const fn checked_border(mut self, color: Color) -> Self {
        self.checked_border = Some(color);
        self
    }

    pub const fn dot(mut self, color: Color) -> Self {
        self.dot = Some(color);
        self
    }

    pub const fn text(mut self, color: Color) -> Self {
        self.text = Some(color);
        self
    }

    pub const fn font_size(mut self, size: f32) -> Self {
        self.font_size = Some(size);
        self
    }

    pub const fn diameter(mut self, size: f32) -> Self {
        self.diameter = Some(size);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn const_composition_keeps_base() {
        const CUSTOM: RadioStyle = RadioStyle::new().diameter(18.0);
        assert_eq!(CUSTOM.diameter, Some(18.0));
        assert_eq!(CUSTOM.background, None);
    }

    #[test]
    fn struct_update_syntax_keeps_base() {
        const CUSTOM: RadioStyle = RadioStyle {
            dot: Some(Color::WHITE),
            ..RadioStyle::new()
        };
        assert_eq!(CUSTOM.dot, Some(Color::WHITE));
        assert_eq!(CUSTOM.checked, None);
    }
}
