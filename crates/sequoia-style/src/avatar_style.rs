use sequoia_core::Color;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AvatarStyle {
    pub background: Option<Color>,
    pub text: Option<Color>,
    pub font_size: Option<f32>,
    pub radius: Option<f32>,
}

impl AvatarStyle {
    pub const fn new() -> Self {
        Self {
            background: None,
            text: None,
            font_size: None,
            radius: None,
        }
    }

    pub const fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn const_composition_keeps_base() {
        const CUSTOM: AvatarStyle = AvatarStyle::new().font_size(13.0);
        assert_eq!(CUSTOM.font_size, Some(13.0));
        assert_eq!(CUSTOM.radius, None);
    }
}
