use sequoia_core::Color;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TextFieldStyle {
    pub text: Option<Color>,
    pub placeholder: Option<Color>,
    pub line: Option<Color>,
    pub line_width: Option<f32>,
    pub font_size: Option<f32>,
    pub font_weight: Option<u16>,
    pub background: Option<Color>,
    pub border: Option<Color>,
    pub border_width: Option<f32>,
    pub radius: Option<f32>,
    pub padding: Option<(f32, f32)>,
}

impl TextFieldStyle {
    pub const fn new() -> Self {
        Self {
            text: None,
            placeholder: None,
            line: None,
            line_width: None,
            font_size: None,
            font_weight: None,
            background: None,
            border: None,
            border_width: None,
            radius: None,
            padding: None,
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

    pub const fn padding_xy(mut self, pad_x: f32, pad_y: f32) -> Self {
        self.padding = Some((pad_x, pad_y));
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn const_composition_keeps_base() {
        const CUSTOM: TextFieldStyle = TextFieldStyle::new().font_size(13.0);
        assert_eq!(CUSTOM.font_size, Some(13.0));
        assert_eq!(CUSTOM.text, None);
    }
}
