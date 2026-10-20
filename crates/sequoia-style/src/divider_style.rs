use sequoia_core::Color;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DividerStyle {
    pub color: Option<Color>,
    pub thickness: Option<f32>,
}

impl DividerStyle {
    pub const fn new() -> Self {
        Self {
            color: None,
            thickness: None,
        }
    }

    pub const fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub const fn thickness(mut self, px: f32) -> Self {
        self.thickness = Some(px);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn const_composition_keeps_base() {
        const CUSTOM: DividerStyle = DividerStyle::new().thickness(2.0);
        assert_eq!(CUSTOM.thickness, Some(2.0));
        assert_eq!(CUSTOM.color, None);
    }
}
