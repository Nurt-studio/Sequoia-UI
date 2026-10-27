//! Text measurement context used during layout.

use sequoia_core::Size;
use sequoia_text::{
    ElideMode, FontStyle, FontWeight, LetterSpacing, LineHeight, RichAlign, TextSpan, TextSystem,
    WrapMode,
};

/// Borrowed text system used by layout code to shape and measure text.
///
/// Measurements are not cached here; the underlying [`TextSystem`] owns any
/// caching that may occur.
pub struct LayoutCtx<'a> {
    text: &'a mut TextSystem,
    default_font_size: f32,
    available_width: f32,
}

impl<'a> LayoutCtx<'a> {
    /// Creates a context that borrows `text` and uses `default_font_size` for
    /// callers that do not specify a size.
    pub fn new(text: &'a mut TextSystem, default_font_size: f32) -> Self {
        Self {
            text,
            default_font_size,
            available_width: f32::INFINITY,
        }
    }

    /// Returns the default font size for this layout pass.
    pub fn default_font_size(&self) -> f32 {
        self.default_font_size
    }

    /// Sets the width available to the view currently being measured.
    pub fn set_available_width(&mut self, width: f32) {
        self.available_width = if width.is_finite() {
            width.max(0.0)
        } else {
            f32::INFINITY
        };
    }

    /// Returns the width available to the view currently being measured.
    pub fn available_width(&self) -> f32 {
        self.available_width
    }

    /// Measures the width of a single line of text.
    pub fn text_width(&mut self, text: &str, size: f32) -> f32 {
        self.text.text_width(text, size)
    }

    /// Measures the width of a single line using the given font weight.
    pub fn text_width_weighted(&mut self, text: &str, size: f32, weight: FontWeight) -> f32 {
        self.text.text_width_weighted(text, size, weight)
    }

    /// Measures the width of a single line with letter spacing.
    pub fn text_width_spaced(&mut self, text: &str, size: f32, spacing: LetterSpacing) -> f32 {
        self.text.text_width_spaced(text, size, spacing)
    }

    /// Measures the width of a single line with weight and letter spacing.
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

    /// Measures the width of a single line with weight, spacing, family, and
    /// style.
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

    /// Returns caret positions along a single line of text.
    pub fn caret_positions(&mut self, text: &str, size: f32, weight: FontWeight) -> Vec<f32> {
        self.text.caret_positions(text, size, weight)
    }

    /// Returns caret positions along a single line with letter spacing.
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

    /// Measures wrapped text within `width`, returning its size.
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

    /// Measures wrapped text within `width` with letter spacing.
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

    /// Measures wrapped text with weight, spacing, family, and style.
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

    /// Measures wrapped text with a custom line height.
    #[allow(clippy::too_many_arguments)]
    pub fn text_wrapped_size_styled_spaced_lh(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
        width: f32,
        mode: WrapMode,
        spacing: LetterSpacing,
        family: Option<&str>,
        style: FontStyle,
        line_height: LineHeight,
    ) -> Size {
        let shaped = self.text.shape_wrapped_styled_spaced_lh(
            text,
            size,
            weight,
            width,
            mode,
            spacing,
            family,
            style,
            line_height,
        );
        Size::new(shaped.width(), shaped.height())
    }

    /// Measures the longest unbreakable unit of `text` for the given wrap mode.
    #[allow(clippy::too_many_arguments)]
    pub fn text_min_content_width_styled_spaced(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
        spacing: LetterSpacing,
        family: Option<&str>,
        style: FontStyle,
        mode: WrapMode,
    ) -> f32 {
        self.text
            .min_content_width_styled_spaced(text, size, weight, spacing, family, style, mode)
    }

    /// Elides wrapped text to `max_lines`, returning the visible string.
    #[allow(clippy::too_many_arguments)]
    pub fn text_elide_lines_styled_spaced(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
        width: f32,
        mode: WrapMode,
        spacing: LetterSpacing,
        family: Option<&str>,
        style: FontStyle,
        line_height: LineHeight,
        max_lines: usize,
    ) -> String {
        self.text.elide_lines_styled_spaced(
            text,
            size,
            weight,
            width,
            mode,
            spacing,
            family,
            style,
            line_height,
            max_lines,
        )
    }

    /// Measures text elided to `max_width` with the given elision mode.
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

    /// Measures elided text with letter spacing.
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

    /// Measures elided text with weight, spacing, family, and style.
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

    /// Measures elided text with a custom line height.
    #[allow(clippy::too_many_arguments)]
    pub fn text_elided_size_styled_spaced_lh(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
        max_width: f32,
        mode: ElideMode,
        spacing: LetterSpacing,
        family: Option<&str>,
        style: FontStyle,
        line_height: LineHeight,
    ) -> Size {
        let shown = self
            .text
            .elide_styled_spaced(text, size, weight, max_width, mode, spacing, family, style);
        let w = self
            .text
            .text_width_styled_spaced(&shown, size, weight, spacing, family, style)
            .min(max_width.max(0.0));
        Size::new(w, Self::text_line_height_with(size, line_height))
    }

    /// Returns the line height used for a font size with the default factor.
    pub fn text_line_height(size: f32) -> f32 {
        Self::text_line_height_with(size, LineHeight::default())
    }

    /// Returns the line height used for a font size.
    pub fn text_line_height_with(size: f32, line_height: LineHeight) -> f32 {
        line_height.resolve(size).max(1.0)
    }

    /// Returns the first baseline offset of a shaped single/wrapped line.
    #[allow(clippy::too_many_arguments)]
    pub fn text_first_baseline_styled_spaced_lh(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
        width: f32,
        mode: WrapMode,
        spacing: LetterSpacing,
        family: Option<&str>,
        style: FontStyle,
        line_height: LineHeight,
    ) -> f32 {
        self.text
            .shape_wrapped_styled_spaced_lh(
                text,
                size,
                weight,
                width,
                mode,
                spacing,
                family,
                style,
                line_height,
            )
            .first_baseline()
    }

    /// Measures rich text spans, returning their shaped size.
    pub fn rich_size(
        &mut self,
        spans: &[TextSpan],
        size: f32,
        width: f32,
        mode: WrapMode,
        spacing: LetterSpacing,
        align: RichAlign,
    ) -> Size {
        self.rich_size_lh(
            spans,
            size,
            width,
            mode,
            spacing,
            align,
            LineHeight::default(),
        )
    }

    /// Measures rich text spans with a custom line height.
    #[allow(clippy::too_many_arguments)]
    pub fn rich_size_lh(
        &mut self,
        spans: &[TextSpan],
        size: f32,
        width: f32,
        mode: WrapMode,
        spacing: LetterSpacing,
        align: RichAlign,
        line_height: LineHeight,
    ) -> Size {
        self.text
            .shape_rich_full_lh(spans, size, width, mode, spacing, align, line_height)
            .size()
    }

    /// Elides rich text to `max_lines`.
    #[allow(clippy::too_many_arguments)]
    pub fn rich_elide_lines(
        &mut self,
        spans: &[TextSpan],
        size: f32,
        width: f32,
        mode: WrapMode,
        spacing: LetterSpacing,
        align: RichAlign,
        line_height: LineHeight,
        max_lines: usize,
    ) -> Vec<TextSpan> {
        self.text.elide_rich_lines(
            spans,
            size,
            width,
            mode,
            spacing,
            align,
            line_height,
            max_lines,
        )
    }

    /// Returns the first baseline of shaped rich text.
    #[allow(clippy::too_many_arguments)]
    pub fn rich_first_baseline(
        &mut self,
        spans: &[TextSpan],
        size: f32,
        width: f32,
        mode: WrapMode,
        spacing: LetterSpacing,
        align: RichAlign,
        line_height: LineHeight,
    ) -> f32 {
        self.text
            .shape_rich_full_lh(spans, size, width, mode, spacing, align, line_height)
            .line(0)
            .map(|line| line.baseline)
            .unwrap_or_else(|| Self::text_line_height_with(size, line_height) * 0.5)
    }
}
