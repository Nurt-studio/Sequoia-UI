use sequoia_core::{Color, Rect};
use sequoia_layout::LayoutCtx;
use sequoia_style::Theme;

pub(crate) const PAD_X: f32 = 6.0;

pub(crate) const LINE_GAP: f32 = 3.0;

pub(crate) const LINE_W: f32 = 2.0;

pub(crate) fn line_rect(rect: Rect, font_size: f32, text_width: f32, line_width: f32) -> Rect {
    line_rect_padded(rect, font_size, text_width, line_width, PAD_X, 0.0)
}

pub(crate) fn line_rect_padded(
    rect: Rect,
    font_size: f32,
    text_width: f32,
    line_width: f32,
    pad_x: f32,
    pad_y: f32,
) -> Rect {
    Rect::from_xywh(
        rect.left() + pad_x,
        rect.top() + pad_y + LayoutCtx::text_line_height(font_size) + LINE_GAP,
        text_width.min((rect.size.width - pad_x * 2.0).max(0.0)),
        line_width,
    )
}

pub(crate) fn baseline(rect: Rect, font_size: f32) -> f32 {
    baseline_padded(rect, font_size, 0.0)
}

pub(crate) fn baseline_padded(rect: Rect, font_size: f32, pad_y: f32) -> f32 {
    rect.top() + pad_y + LayoutCtx::text_line_height(font_size) * 0.5 + font_size * 0.35
}

pub(crate) fn content_height(font_size: f32, line_width: f32) -> f32 {
    LayoutCtx::text_line_height(font_size) + LINE_GAP + line_width
}

pub(crate) struct FieldColors {
    pub text: Color,

    pub line: Color,
}

pub(crate) fn field_colors(
    theme: &Theme,
    _interactive: bool,
    line_full: bool,
    text_color: Option<Color>,
    line_color: Option<Color>,
) -> FieldColors {
    let text = text_color.unwrap_or(theme.text);
    let line = match line_color {
        Some(color) => color,
        None if line_full => theme.accent,
        None => theme.border,
    };
    FieldColors { text, line }
}
