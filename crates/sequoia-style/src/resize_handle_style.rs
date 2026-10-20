use sequoia_core::Color;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ResizeHandleStyle {
    pub idle: Option<Color>,
    pub active: Option<Color>,
    pub idle_width: Option<f32>,
    pub active_width: Option<f32>,
    pub gutter: Option<Color>,
}

impl ResizeHandleStyle {
    pub const fn new() -> Self {
        Self {
            idle: None,
            active: None,
            idle_width: None,
            active_width: None,
            gutter: None,
        }
    }

    pub const fn idle(mut self, color: Color) -> Self {
        self.idle = Some(color);
        self
    }

    pub const fn active(mut self, color: Color) -> Self {
        self.active = Some(color);
        self
    }

    pub const fn idle_width(mut self, width: f32) -> Self {
        self.idle_width = Some(width);
        self
    }

    pub const fn active_width(mut self, width: f32) -> Self {
        self.active_width = Some(width);
        self
    }

    pub const fn gutter(mut self, color: Color) -> Self {
        self.gutter = Some(color);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn const_composition_keeps_base() {
        const CUSTOM: ResizeHandleStyle = ResizeHandleStyle::new().idle_width(2.0);
        assert_eq!(CUSTOM.idle_width, Some(2.0));
        assert_eq!(CUSTOM.idle, None);
    }
}
