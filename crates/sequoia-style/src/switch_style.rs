use sequoia_core::Color;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SwitchStyle {
    pub track_on: Option<Color>,
    pub track_off: Option<Color>,
    pub track_disabled: Option<Color>,
    pub thumb: Option<Color>,
    pub thumb_on: Option<Color>,
    pub thumb_disabled: Option<Color>,
    pub border: Option<Color>,
    pub border_on: Option<Color>,
    pub text: Option<Color>,
    pub font_size: Option<f32>,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub thumb_size: Option<f32>,
}

impl SwitchStyle {
    pub const fn new() -> Self {
        Self {
            track_on: None,
            track_off: None,
            track_disabled: None,
            thumb: None,
            thumb_on: None,
            thumb_disabled: None,
            border: None,
            border_on: None,
            text: None,
            font_size: None,
            width: None,
            height: None,
            thumb_size: None,
        }
    }

    pub const fn track_on(mut self, color: Color) -> Self {
        self.track_on = Some(color);
        self
    }

    pub const fn track_off(mut self, color: Color) -> Self {
        self.track_off = Some(color);
        self
    }

    pub const fn track_disabled(mut self, color: Color) -> Self {
        self.track_disabled = Some(color);
        self
    }

    pub const fn thumb(mut self, color: Color) -> Self {
        self.thumb = Some(color);
        self
    }

    pub const fn thumb_on(mut self, color: Color) -> Self {
        self.thumb_on = Some(color);
        self
    }

    pub const fn thumb_disabled(mut self, color: Color) -> Self {
        self.thumb_disabled = Some(color);
        self
    }

    pub const fn border(mut self, color: Color) -> Self {
        self.border = Some(color);
        self
    }

    pub const fn border_on(mut self, color: Color) -> Self {
        self.border_on = Some(color);
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

    pub const fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    pub const fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    pub const fn thumb_size(mut self, size: f32) -> Self {
        self.thumb_size = Some(size);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn const_composition_keeps_base() {
        const CUSTOM: SwitchStyle = SwitchStyle::new().track_on(Color::WHITE);
        assert_eq!(CUSTOM.track_on, Some(Color::WHITE));
        assert_eq!(CUSTOM.track_off, None);
    }

    #[test]
    fn struct_update_syntax_keeps_base() {
        const CUSTOM: SwitchStyle = SwitchStyle {
            track_on: Some(Color::WHITE),
            ..SwitchStyle::new()
        };
        assert_eq!(CUSTOM.track_on, Some(Color::WHITE));
        assert_eq!(CUSTOM.thumb, None);
    }
}
