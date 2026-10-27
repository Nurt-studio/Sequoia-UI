use sequoia_core::Color;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextStyle {
    pub size: Option<f32>,

    pub color: Option<Color>,

    pub weight: Option<u16>,

    pub family: Option<String>,

    pub italic: Option<bool>,

    pub letter_spacing: Option<f32>,

    /// Line height as a multiple of the font size.
    pub line_height: Option<f32>,

    pub underline: Option<bool>,

    pub strikethrough: Option<bool>,
}

impl TextStyle {
    pub const fn new() -> Self {
        Self {
            size: None,
            color: None,
            weight: None,
            family: None,
            italic: None,
            letter_spacing: None,
            line_height: None,
            underline: None,
            strikethrough: None,
        }
    }

    pub const fn size(mut self, size: f32) -> Self {
        self.size = Some(size);
        self
    }

    pub const fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub const fn weight(mut self, weight: u16) -> Self {
        self.weight = Some(weight);
        self
    }

    pub fn family(mut self, family: impl Into<String>) -> Self {
        self.family = Some(family.into());
        self
    }

    pub const fn italic(mut self, on: bool) -> Self {
        self.italic = Some(on);
        self
    }

    pub const fn letter_spacing(mut self, spacing: f32) -> Self {
        self.letter_spacing = Some(spacing);
        self
    }

    pub const fn line_height(mut self, factor: f32) -> Self {
        self.line_height = Some(factor);
        self
    }

    pub const fn underline(mut self, on: bool) -> Self {
        self.underline = Some(on);
        self
    }

    pub const fn strikethrough(mut self, on: bool) -> Self {
        self.strikethrough = Some(on);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn const_composition_keeps_base() {
        const TITLE: TextStyle = TextStyle::new()
            .size(20.0)
            .color(Color::WHITE)
            .weight(700)
            .italic(true);
        assert_eq!(TITLE.size, Some(20.0));
        assert_eq!(TITLE.color, Some(Color::WHITE));
        assert_eq!(TITLE.weight, Some(700));
        assert_eq!(TITLE.italic, Some(true));
        assert_eq!(TITLE.family, None);
        assert_eq!(TITLE.underline, None);
    }

    #[test]
    fn family_accepts_owned_names() {
        let family = String::from("SequoiaTest");
        assert_eq!(TextStyle::new().family(family.clone()).family, Some(family));
    }

    #[test]
    fn default_is_empty() {
        assert_eq!(TextStyle::default(), TextStyle::new());
    }
}
