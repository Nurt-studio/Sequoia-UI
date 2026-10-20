use sequoia_core::Color;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DragValueStyle {
    pub text: Option<Color>,
    pub line: Option<Color>,
    pub line_width: Option<f32>,
    pub font_size: Option<f32>,
    pub font_weight: Option<u16>,
}

impl DragValueStyle {
    pub const fn new() -> Self {
        Self {
            text: None,
            line: None,
            line_width: None,
            font_size: None,
            font_weight: None,
        }
    }

    pub const fn text(mut self, color: Color) -> Self {
        self.text = Some(color);
        self
    }

    pub const fn line(mut self, color: Color) -> Self {
        self.line = Some(color);
        self
    }

    pub const fn line_width(mut self, width: f32) -> Self {
        self.line_width = Some(width);
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn const_composition_keeps_base() {
        const CUSTOM: DragValueStyle = DragValueStyle::new().font_size(12.0);
        assert_eq!(CUSTOM.font_size, Some(12.0));
        assert_eq!(CUSTOM.text, None);
    }
}
