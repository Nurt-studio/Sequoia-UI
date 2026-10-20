use sequoia_core::Color;

use crate::Backdrop;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ToastStyle {
    pub background: Option<Color>,
    pub border: Option<Color>,
    pub border_width: Option<f32>,
    pub text: Option<Color>,
    pub text_secondary: Option<Color>,
    pub font_size: Option<f32>,
    pub radius: Option<f32>,
    pub padding: Option<(f32, f32)>,

    pub info: Option<Color>,
    pub success: Option<Color>,
    pub warn: Option<Color>,
    pub error: Option<Color>,

    pub backdrop: Option<Backdrop>,
}

impl ToastStyle {
    pub const fn new() -> Self {
        Self {
            background: None,
            border: None,
            border_width: None,
            text: None,
            text_secondary: None,
            font_size: None,
            radius: None,
            padding: None,
            info: None,
            success: None,
            warn: None,
            error: None,
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

    pub const fn text(mut self, color: Color) -> Self {
        self.text = Some(color);
        self
    }

    pub const fn text_secondary(mut self, color: Color) -> Self {
        self.text_secondary = Some(color);
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

    pub const fn padding_xy(mut self, pad_x: f32, pad_y: f32) -> Self {
        self.padding = Some((pad_x, pad_y));
        self
    }

    pub const fn info(mut self, color: Color) -> Self {
        self.info = Some(color);
        self
    }

    pub const fn success(mut self, color: Color) -> Self {
        self.success = Some(color);
        self
    }

    pub const fn warn(mut self, color: Color) -> Self {
        self.warn = Some(color);
        self
    }

    pub const fn error(mut self, color: Color) -> Self {
        self.error = Some(color);
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
        const CUSTOM: ToastStyle = ToastStyle::new().radius(10.0);
        assert_eq!(CUSTOM.radius, Some(10.0));
        assert_eq!(CUSTOM.text, None);
        assert_eq!(CUSTOM.info, None);
    }

    #[test]
    fn kind_colors_chain() {
        const CUSTOM: ToastStyle = ToastStyle::new().info(Color::WHITE).error(Color::BLACK);
        assert_eq!(CUSTOM.info, Some(Color::WHITE));
        assert_eq!(CUSTOM.error, Some(Color::BLACK));
        assert_eq!(CUSTOM.success, None);
    }

    #[test]
    fn backdrop_chain() {
        const CUSTOM: ToastStyle = ToastStyle::new().backdrop(Backdrop::XS);
        assert_eq!(CUSTOM.backdrop, Some(Backdrop::XS));
        assert_eq!(CUSTOM.background, None);
    }
}
