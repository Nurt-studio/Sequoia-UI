use sequoia_core::Color;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ChipStyle {
    pub background: Option<Color>,
    pub border: Option<Color>,
    pub text: Option<Color>,
    pub selected_background: Option<Color>,
    pub selected_border: Option<Color>,
    pub selected_text: Option<Color>,
    pub font_size: Option<f32>,
    pub radius: Option<f32>,
    pub padding: Option<(f32, f32)>,
}

impl ChipStyle {
    pub const fn new() -> Self {
        Self {
            background: None,
            border: None,
            text: None,
            selected_background: None,
            selected_border: None,
            selected_text: None,
            font_size: None,
            radius: None,
            padding: None,
        }
    }

    pub const fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    pub const fn border(mut self, color: Color) -> Self {
        self.border = Some(color);
        self
    }

    pub const fn text(mut self, color: Color) -> Self {
        self.text = Some(color);
        self
    }

    pub const fn selected_background(mut self, color: Color) -> Self {
        self.selected_background = Some(color);
        self
    }

    pub const fn selected_border(mut self, color: Color) -> Self {
        self.selected_border = Some(color);
        self
    }

    pub const fn selected_text(mut self, color: Color) -> Self {
        self.selected_text = Some(color);
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
        const CUSTOM: ChipStyle = ChipStyle::new().radius(999.0);
        assert_eq!(CUSTOM.radius, Some(999.0));
        assert_eq!(CUSTOM.selected_text, None);
    }
}
