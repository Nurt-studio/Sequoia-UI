use sequoia_core::Color;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SheetStyle {
    pub background: Option<Color>,
    pub border: Option<Color>,
    pub border_width: Option<f32>,
    pub radius: Option<f32>,
}

impl SheetStyle {
    pub const fn new() -> Self {
        Self {
            background: None,
            border: None,
            border_width: None,
            radius: None,
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

    pub const fn border_width(mut self, width: f32) -> Self {
        self.border_width = Some(width);
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
        const CUSTOM: SheetStyle = SheetStyle::new().radius(12.0);
        assert_eq!(CUSTOM.radius, Some(12.0));
        assert_eq!(CUSTOM.background, None);
    }
}
