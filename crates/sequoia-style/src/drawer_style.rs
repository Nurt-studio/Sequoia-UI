use sequoia_core::Color;

use crate::Backdrop;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DrawerStyle {
    pub background: Option<Color>,
    pub border: Option<Color>,
    pub border_width: Option<f32>,
    pub radius: Option<f32>,

    pub scrim: Option<Color>,

    pub backdrop: Option<Backdrop>,
}

impl DrawerStyle {
    pub const fn new() -> Self {
        Self {
            background: None,
            border: None,
            border_width: None,
            radius: None,
            scrim: None,
            backdrop: None,
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

    pub const fn scrim(mut self, color: Color) -> Self {
        self.scrim = Some(color);
        self
    }

    pub const fn backdrop(mut self, backdrop: Backdrop) -> Self {
        self.backdrop = Some(backdrop);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn const_composition_keeps_base() {
        const CUSTOM: DrawerStyle = DrawerStyle::new().radius(10.0);
        assert_eq!(CUSTOM.radius, Some(10.0));
        assert_eq!(CUSTOM.background, None);
        assert_eq!(CUSTOM.scrim, None);
    }
}
