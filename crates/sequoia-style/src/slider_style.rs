use sequoia_core::Color;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SliderStyle {
    pub track: Option<Color>,
    pub fill: Option<Color>,
    pub thumb_center: Option<Color>,
    pub thumb_outline: Option<Color>,
    pub track_height: Option<f32>,
    pub thumb_size: Option<f32>,
}

impl SliderStyle {
    pub const fn new() -> Self {
        Self {
            track: None,
            fill: None,
            thumb_center: None,
            thumb_outline: None,
            track_height: None,
            thumb_size: None,
        }
    }

    pub const fn track(mut self, color: Color) -> Self {
        self.track = Some(color);
        self
    }

    pub const fn fill(mut self, color: Color) -> Self {
        self.fill = Some(color);
        self
    }

    pub const fn thumb_center(mut self, color: Color) -> Self {
        self.thumb_center = Some(color);
        self
    }

    pub const fn thumb_outline(mut self, color: Color) -> Self {
        self.thumb_outline = Some(color);
        self
    }

    pub const fn track_height(mut self, height: f32) -> Self {
        self.track_height = Some(height);
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
        const CUSTOM: SliderStyle = SliderStyle::new().track_height(6.0);
        assert_eq!(CUSTOM.track_height, Some(6.0));
        assert_eq!(CUSTOM.track, None);
    }
}
