use sequoia_core::Color;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SplitPanelStyle {
    pub handle: Option<Color>,
    pub handle_active: Option<Color>,
    pub handle_width: Option<f32>,
}

impl SplitPanelStyle {
    pub const fn new() -> Self {
        Self {
            handle: None,
            handle_active: None,
            handle_width: None,
        }
    }

    pub const fn handle(mut self, color: Color) -> Self {
        self.handle = Some(color);
        self
    }

    pub const fn handle_active(mut self, color: Color) -> Self {
        self.handle_active = Some(color);
        self
    }

    pub const fn handle_width(mut self, width: f32) -> Self {
        self.handle_width = Some(width);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn const_composition_keeps_base() {
        const CUSTOM: SplitPanelStyle = SplitPanelStyle::new().handle_width(9.0);
        assert_eq!(CUSTOM.handle_width, Some(9.0));
        assert_eq!(CUSTOM.handle, None);
    }
}
