use sequoia_core::Size;
use sequoia_text::{
    ElideMode, FontStyle, FontWeight, LetterSpacing, RichAlign, TextSpan, TextSystem, WrapMode,
};

pub struct LayoutCtx<'a> {
    text: &'a mut TextSystem,
}

impl<'a> LayoutCtx<'a> {
    pub fn new(text: &'a mut TextSystem) -> Self {
        Self { text }
    }

    pub fn text_width(&mut self, text: &str, size: f32) -> f32 {
        self.text.text_width(text, size)
    }

    pub fn text_width_weighted(&mut self, text: &str, size: f32, weight: FontWeight) -> f32 {
        self.text.text_width_weighted(text, size, weight)
    }

    pub fn text_width_spaced(&mut self, text: &str, size: f32, spacing: LetterSpacing) -> f32 {
        self.text.text_width_spaced(text, size, spacing)
    }

    pub fn text_width_weighted_spaced(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
        spacing: LetterSpacing,
    ) -> f32 {
        self.text
            .text_width_weighted_spaced(text, size, weight, spacing)
    }

    pub fn text_width_styled_spaced(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
        spacing: LetterSpacing,
        family: Option<&str>,
        style: FontStyle,
    ) -> f32 {
        self.text
            .text_width_styled_spaced(text, size, weight, spacing, family, style)
    }

    pub fn caret_positions(&mut self, text: &str, size: f32, weight: FontWeight) -> Vec<f32> {
        self.text.caret_positions(text, size, weight)
    }

    pub fn caret_positions_spaced(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
        spacing: LetterSpacing,
    ) -> Vec<f32> {
        self.text
            .caret_positions_spaced(text, size, weight, spacing)
    }

    pub fn text_wrapped_size(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
        width: f32,
        mode: WrapMode,
    ) -> Size {
        self.text_wrapped_size_spaced(text, size, weight, width, mode, 0.0)
    }

    pub fn text_wrapped_size_spaced(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
        width: f32,
        mode: WrapMode,
        spacing: LetterSpacing,
    ) -> Size {
        let shaped = self
            .text
            .shape_wrapped_spaced(text, size, weight, width, mode, spacing);
        Size::new(shaped.width(), shaped.height())
    }

    pub fn text_wrapped_size_styled_spaced(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
        width: f32,
        mode: WrapMode,
        spacing: LetterSpacing,
        family: Option<&str>,
        style: FontStyle,
    ) -> Size {
        let shaped = self
            .text
            .shape_wrapped_styled_spaced(text, size, weight, width, mode, spacing, family, style);
        Size::new(shaped.width(), shaped.height())
    }

    pub fn text_elided_size(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
        max_width: f32,
        mode: ElideMode,
    ) -> Size {
        self.text_elided_size_spaced(text, size, weight, max_width, mode, 0.0)
    }

    pub fn text_elided_size_spaced(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
        max_width: f32,
        mode: ElideMode,
        spacing: LetterSpacing,
    ) -> Size {
        let shown = self
            .text
            .elide_spaced(text, size, weight, max_width, mode, spacing);
        let w = self
            .text
            .text_width_weighted_spaced(&shown, size, weight, spacing)
            .min(max_width.max(0.0));
        Size::new(w, Self::text_line_height(size))
    }

    pub fn text_elided_size_styled_spaced(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
        max_width: f32,
        mode: ElideMode,
        spacing: LetterSpacing,
        family: Option<&str>,
        style: FontStyle,
    ) -> Size {
        let shown = self
            .text
            .elide_styled_spaced(text, size, weight, max_width, mode, spacing, family, style);
        let w = self
            .text
            .text_width_styled_spaced(&shown, size, weight, spacing, family, style)
            .min(max_width.max(0.0));
        Size::new(w, Self::text_line_height(size))
    }

    pub fn text_line_height(size: f32) -> f32 {
        size * 1.35
    }

    pub fn rich_size(
        &mut self,
        spans: &[TextSpan],
        size: f32,
        width: f32,
        mode: WrapMode,
        spacing: LetterSpacing,
        align: RichAlign,
    ) -> Size {
        self.text
            .shape_rich_full(spans, size, width, mode, spacing, align)
            .size()
    }
}
