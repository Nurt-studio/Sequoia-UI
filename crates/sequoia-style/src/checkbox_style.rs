use sequoia_core::Color;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CheckboxStyle {
    pub background: Option<Color>,
    pub checked: Option<Color>,
    pub border: Option<Color>,
    pub checked_border: Option<Color>,
    pub check: Option<Color>,
    pub text: Option<Color>,
    pub font_size: Option<f32>,
    pub radius: Option<f32>,
    pub box_size: Option<f32>,
}

impl CheckboxStyle {
    pub const fn new() -> Self {
        Self {
            background: None,
            checked: None,
            border: None,
            checked_border: None,
            check: None,
            text: None,
            font_size: None,
            radius: None,
            box_size: None,
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

    pub const fn check(mut self, color: Color) -> Self {
        self.check = Some(color);
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

    pub const fn radius(mut self, radius: f32) -> Self {
        self.radius = Some(radius);
        self
    }

    pub const fn box_size(mut self, size: f32) -> Self {
        self.box_size = Some(size);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn const_composition_keeps_base() {
        const CUSTOM: CheckboxStyle = CheckboxStyle::new().radius(4.0);
        assert_eq!(CUSTOM.radius, Some(4.0));
        assert_eq!(CUSTOM.background, None);
    }

    #[test]
    fn struct_update_syntax_keeps_base() {
        const CUSTOM: CheckboxStyle = CheckboxStyle {
            background: Some(Color::WHITE),
            ..CheckboxStyle::new()
        };
        assert_eq!(CUSTOM.background, Some(Color::WHITE));
        assert_eq!(CUSTOM.check, None);
    }
}
