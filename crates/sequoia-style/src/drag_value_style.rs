use sequoia_core::Color;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DragValueStyle {
    pub text: Option<Color>,
    pub line: Option<Color>,
    pub line_width: Option<f32>,
    pub line_full: Option<bool>,
    pub background: Option<Color>,
    pub hover_background: Option<Color>,
    pub pressed_background: Option<Color>,
    pub border: Option<Color>,
    pub border_width: Option<f32>,
    pub radius: Option<f32>,
    pub padding: Option<(f32, f32)>,
    pub font_size: Option<f32>,
    pub font_weight: Option<u16>,
}

impl DragValueStyle {
    pub const fn new() -> Self {
        Self {
            text: None,
            line: None,
            line_width: None,
            line_full: None,
            background: None,
            hover_background: None,
            pressed_background: None,
            border: None,
            border_width: None,
            radius: None,
            padding: None,
            font_size: None,
            font_weight: None,
        }
    }

    pub const fn text(mut self, color: Color) -> Self {
        self.text = Some(color);
        self
    }

    pub const fn line(mut self, color: Color) -> Self {
        self.line = Some(color);
        self
    }

    pub const fn line_width(mut self, width: f32) -> Self {
        self.line_width = Some(width);
        self
    }

    pub const fn line_full(mut self, full: bool) -> Self {
        self.line_full = Some(full);
        self
    }

    pub const fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    pub const fn hover_background(mut self, color: Color) -> Self {
        self.hover_background = Some(color);
        self
    }

    pub const fn pressed_background(mut self, color: Color) -> Self {
        self.pressed_background = Some(color);
        self
    }

    pub const fn border_color(mut self, color: Color) -> Self {
        self.border = Some(color);
        self
    }

    pub const fn border_width(mut self, width: f32) -> Self {
        self.border_width = Some(width);
        self
    }

    pub const fn border(mut self, width: f32, color: Color) -> Self {
        self.border_width = Some(width);
        self.border = Some(color);
        self
    }

    pub const fn radius(mut self, radius: f32) -> Self {
        self.radius = Some(radius);
        self
    }

    pub const fn padding_xy(mut self, pad_x: f32, pad_y: f32) -> Self {
        self.padding = Some((pad_x, pad_y));
        self
    }

    pub const fn font_size(mut self, size: f32) -> Self {
        self.font_size = Some(size);
        self
    }

    pub const fn font_weight(mut self, weight: u16) -> Self {
        self.font_weight = Some(weight);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn const_composition_keeps_base() {
        const CUSTOM: DragValueStyle = DragValueStyle::new().font_size(12.0);
        assert_eq!(CUSTOM.font_size, Some(12.0));
        assert_eq!(CUSTOM.text, None);
    }

    #[test]
    fn surface_builders_set_all_fields() {
        const CUSTOM: DragValueStyle = DragValueStyle::new()
            .background(Color::RED)
            .hover_background(Color::GREEN)
            .pressed_background(Color::BLUE)
            .border(1.0, Color::WHITE)
            .radius(4.0)
            .padding_xy(3.0, 2.0)
            .line_full(true);
        assert_eq!(CUSTOM.background, Some(Color::RED));
        assert_eq!(CUSTOM.hover_background, Some(Color::GREEN));
        assert_eq!(CUSTOM.pressed_background, Some(Color::BLUE));
        assert_eq!(CUSTOM.border_width, Some(1.0));
        assert_eq!(CUSTOM.border, Some(Color::WHITE));
        assert_eq!(CUSTOM.radius, Some(4.0));
        assert_eq!(CUSTOM.padding, Some((3.0, 2.0)));
        assert_eq!(CUSTOM.line_full, Some(true));
    }
}
