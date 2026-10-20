use sequoia_core::Color;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FocusRingStyle {
    pub color: Option<Color>,
    pub width: Option<f32>,
    pub offset: Option<f32>,
    pub radius: Option<f32>,
}

impl FocusRingStyle {
    pub const fn new() -> Self {
        Self {
            color: None,
            width: None,
            offset: None,
            radius: None,
        }
    }

    pub const fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub const fn width(mut self, px: f32) -> Self {
        self.width = Some(px);
        self
    }

    pub const fn offset(mut self, px: f32) -> Self {
        self.offset = Some(px);
        self
    }

    pub const fn radius(mut self, radius: f32) -> Self {
        self.radius = Some(radius);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn const_composition_keeps_base() {
        const CUSTOM: FocusRingStyle = FocusRingStyle::new().width(2.0);
        assert_eq!(CUSTOM.width, Some(2.0));
        assert_eq!(CUSTOM.color, None);
    }
}
