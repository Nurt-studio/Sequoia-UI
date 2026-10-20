use sequoia_core::Color;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TableStyle {
    pub header_size: Option<f32>,

    pub header_color: Option<Color>,

    pub row_stripe: Option<Color>,
}

impl TableStyle {
    pub const fn new() -> Self {
        Self {
            header_size: None,
            header_color: None,
            row_stripe: None,
        }
    }

    pub const fn header_size(mut self, size: f32) -> Self {
        self.header_size = Some(size);
        self
    }

    pub const fn header_color(mut self, color: Color) -> Self {
        self.header_color = Some(color);
        self
    }

    pub const fn row_stripe(mut self, color: Color) -> Self {
        self.row_stripe = Some(color);
        self
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TreeStyle {
    pub arrow_color: Option<Color>,

    pub font_size: Option<f32>,
}

impl TreeStyle {
    pub const fn new() -> Self {
        Self {
            arrow_color: None,
            font_size: None,
        }
    }

    pub const fn arrow_color(mut self, color: Color) -> Self {
        self.arrow_color = Some(color);
        self
    }

    pub const fn font_size(mut self, size: f32) -> Self {
        self.font_size = Some(size);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_builders_compose() {
        let s = TableStyle::new().header_size(13.0);
        assert_eq!(s.header_size, Some(13.0));
        assert_eq!(s.row_stripe, None);
    }

    #[test]
    fn tree_builders_compose() {
        let s = TreeStyle::new().font_size(14.0);
        assert_eq!(s.font_size, Some(14.0));
        assert_eq!(s.arrow_color, None);
    }
}
