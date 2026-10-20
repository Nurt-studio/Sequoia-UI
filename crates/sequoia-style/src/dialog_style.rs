use sequoia_core::Color;

use crate::Backdrop;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DialogStyle {
    pub background: Option<Color>,
    pub border: Option<Color>,
    pub border_width: Option<f32>,
    pub radius: Option<f32>,
    pub backdrop: Option<Backdrop>,

    pub title_size: Option<f32>,

    pub danger: Option<Color>,
}

impl DialogStyle {
    pub const fn new() -> Self {
        Self {
            background: None,
            border: None,
            border_width: None,
            radius: None,
            backdrop: None,
            title_size: None,
            danger: None,
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

    pub const fn backdrop(mut self, backdrop: Backdrop) -> Self {
        self.backdrop = Some(backdrop);
        self
    }

    pub const fn title_size(mut self, size: f32) -> Self {
        self.title_size = Some(size);
        self
    }

    pub const fn danger(mut self, color: Color) -> Self {
        self.danger = Some(color);
        self
    }
}

pub const DIALOG_DANGER: Color = Color::rgb8(0xFF, 0x5F, 0x57);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_empty_and_danger_const_is_red() {
        let s = DialogStyle::new().title_size(20.0);
        assert_eq!(s.title_size, Some(20.0));
        assert_eq!(s.danger, None);
        assert_eq!(DIALOG_DANGER, Color::rgb8(0xFF, 0x5F, 0x57));
    }
}
