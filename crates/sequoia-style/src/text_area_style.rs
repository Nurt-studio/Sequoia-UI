use sequoia_core::Color;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TextAreaStyle {
    pub text: Option<Color>,
    pub placeholder: Option<Color>,
    pub background: Option<Color>,
    pub border: Option<Color>,
    pub border_width: Option<f32>,
    pub radius: Option<f32>,
    pub font_size: Option<f32>,
    pub font_weight: Option<u16>,
    pub line_height: Option<f32>,
    pub current_line: Option<Color>,
}

impl TextAreaStyle {
    pub const fn new() -> Self {
        Self {
            text: None,
            placeholder: None,
            background: None,
            border: None,
            border_width: None,
            radius: None,
            font_size: None,
            font_weight: None,
            line_height: None,
            current_line: None,
        }
    }

    pub const fn text(mut self, color: Color) -> Self {
        self.text = Some(color);
        self
    }

    pub const fn placeholder(mut self, color: Color) -> Self {
        self.placeholder = Some(color);
        self
    }

    pub const fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    pub const fn border(mut self, color: Color) -> Self {
        self.border = Some(color);
        self
    }

    pub const fn border_width(mut self, width: f32) -> Self {
        self.border_width = Some(width);
        self
    }

    pub const fn radius(mut self, radius: f32) -> Self {
        self.radius = Some(radius);
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

    pub const fn line_height(mut self, factor: f32) -> Self {
        self.line_height = Some(factor);
        self
    }

    pub const fn current_line(mut self, color: Color) -> Self {
        self.current_line = Some(color);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn const_composition_keeps_base() {
        const CUSTOM: TextAreaStyle = TextAreaStyle::new().font_size(13.0);
        assert_eq!(CUSTOM.font_size, Some(13.0));
        assert_eq!(CUSTOM.text, None);
    }
}
