use sequoia_core::Color;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ContextMenuStyle {
    pub background: Option<Color>,
    pub border: Option<Color>,
    pub radius: Option<f32>,
    pub text: Option<Color>,
    pub font_size: Option<f32>,
    pub padding: Option<(f32, f32)>,
}

impl ContextMenuStyle {
    pub const fn new() -> Self {
        Self {
            background: None,
            border: None,
            radius: None,
            text: None,
            font_size: None,
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

    pub const fn radius(mut self, radius: f32) -> Self {
        self.radius = Some(radius);
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
        const CUSTOM: ContextMenuStyle = ContextMenuStyle::new().radius(10.0);
        assert_eq!(CUSTOM.radius, Some(10.0));
        assert_eq!(CUSTOM.text, None);
    }
}
