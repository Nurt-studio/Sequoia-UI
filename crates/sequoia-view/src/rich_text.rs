use std::rc::Rc;

use sequoia_core::Edges;
use sequoia_core::{Color, Length, Size};
use sequoia_input::{CursorIcon, Event, EventPhase, PointerButton, PointerEvent};
use sequoia_layout::{LayoutCtx, LayoutStyle};
use sequoia_text::{LetterSpacing, LineHeight, RichAlign, TextSpan, TextVAlign, WrapMode};

use crate::view::{EventCtx, PaintCtx, View};
use crate::views::{Unit, text_wrap_limit};

pub struct RichText {
    spans: Box<dyn Fn() -> Vec<TextSpan>>,
    font_size: Option<f32>,
    color: Option<Color>,
    wrap_width: Option<f32>,
    wrap_mode: WrapMode,
    align: RichAlign,
    align_y: TextVAlign,
    line_height: LineHeight,
    max_lines: Option<usize>,
    font_scale: f32,
    auto_wrap: bool,
    letter_spacing: f32,
    on_span_click: Option<Rc<dyn Fn(usize)>>,
    style: LayoutStyle,
}

impl RichText {
    pub fn new(spans: Vec<TextSpan>) -> Self {
        Self {
            spans: Box::new(move || spans.clone()),
            font_size: None,
            color: None,
            wrap_width: None,
            wrap_mode: WrapMode::default(),
            align: RichAlign::Left,
            align_y: TextVAlign::Top,
            line_height: LineHeight::default(),
            max_lines: None,
            font_scale: 1.0,
            auto_wrap: false,
            letter_spacing: 0.0,
            on_span_click: None,
            style: LayoutStyle {
                width: Length::Fit,
                height: Length::Fit,
                ..LayoutStyle::default()
            },
        }
    }

    pub fn with(spans: impl Fn() -> Vec<TextSpan> + 'static) -> Self {
        Self {
            spans: Box::new(spans),
            font_size: None,
            color: None,
            wrap_width: None,
            wrap_mode: WrapMode::default(),
            align: RichAlign::Left,
            align_y: TextVAlign::Top,
            line_height: LineHeight::default(),
            max_lines: None,
            font_scale: 1.0,
            auto_wrap: false,
            letter_spacing: 0.0,
            on_span_click: None,
            style: LayoutStyle {
                width: Length::Fit,
                height: Length::Fit,
                ..LayoutStyle::default()
            },
        }
    }

    pub fn spans(&self) -> Vec<TextSpan> {
        (self.spans)()
    }

    pub fn span(mut self, span: TextSpan) -> Self {
        let prev = std::mem::replace(&mut self.spans, Box::new(Vec::new));
        self.spans = Box::new(move || {
            let mut out = prev();
            out.push(span.clone());
            out
        });
        self
    }

    pub fn size(mut self, size: f32) -> Self {
        self.font_size = Some(size);
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn wrap_width(mut self, width: f32) -> Self {
        self.wrap_width = if width > 0.0 { Some(width) } else { None };
        self
    }

    pub fn wrap_mode(mut self, mode: WrapMode) -> Self {
        self.wrap_mode = mode;
        self
    }

    /// Enables wrapping at the width available from the parent container.
    pub fn wrap(mut self) -> Self {
        self.auto_wrap = true;
        self
    }

    /// Sets the line height as a multiple of the font size.
    pub fn line_height(mut self, factor: f32) -> Self {
        if factor.is_finite() && factor > 0.0 {
            self.line_height = LineHeight::Factor(factor);
        }
        self
    }

    /// Sets an absolute line height in logical pixels.
    pub fn line_height_px(mut self, px: f32) -> Self {
        if px.is_finite() && px > 0.0 {
            self.line_height = LineHeight::Absolute(px);
        }
        self
    }

    /// Sets vertical alignment within the text box.
    pub fn align_y(mut self, align: TextVAlign) -> Self {
        self.align_y = align;
        self
    }

    /// Limits visible wrapped text to `lines`, appending an ellipsis.
    pub fn max_lines(mut self, lines: usize) -> Self {
        self.max_lines = if lines == 0 { None } else { Some(lines) };
        self
    }

    /// Scales the resolved font size without changing layout styles.
    pub fn font_scale(mut self, scale: f32) -> Self {
        if scale.is_finite() && scale > 0.0 {
            self.font_scale = scale;
        }
        self
    }

    pub fn align(mut self, align: RichAlign) -> Self {
        self.align = align;
        self
    }

    pub fn centered(mut self) -> Self {
        self.align = RichAlign::Center;
        self
    }

    pub fn right_aligned(mut self) -> Self {
        self.align = RichAlign::Right;
        self
    }

    pub fn letter_spacing(mut self, spacing: LetterSpacing) -> Self {
        self.letter_spacing = if spacing.is_finite() { spacing } else { 0.0 };
        self
    }

    pub fn on_span_click(mut self, handler: impl Fn(usize) + 'static) -> Self {
        self.on_span_click = Some(Rc::new(handler));
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.style = self.style.width(width.into());
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.style = self.style.height(height.into());
        self
    }

    pub fn min_width(mut self, v: f32) -> Self {
        self.style = self.style.min_width(v);
        self
    }

    pub fn max_width(mut self, v: f32) -> Self {
        self.style = self.style.max_width(v);
        self
    }

    pub fn min_height(mut self, v: f32) -> Self {
        self.style = self.style.min_height(v);
        self
    }

    pub fn max_height(mut self, v: f32) -> Self {
        self.style = self.style.max_height(v);
        self
    }

    pub fn flex(mut self, weight: f32) -> Self {
        self.style = self.style.flex(weight);
        self
    }

    pub fn expand(self) -> Self {
        self.flex(1.0)
    }

    pub fn fill_width(mut self) -> Self {
        self.style = self.style.width(Length::Fill(1.0));
        self
    }

    pub fn fill_height(mut self) -> Self {
        self.style = self.style.height(Length::Fill(1.0));
        self
    }

    pub fn fill_size(self) -> Self {
        self.fill_width().fill_height()
    }

    pub fn padding(mut self, padding: impl Into<Edges>) -> Self {
        self.style = self.style.padding(padding);
        self
    }

    pub fn pad(self, padding: impl Into<Edges>) -> Self {
        self.padding(padding)
    }

    pub fn padding_xy(mut self, x: f32, y: f32) -> Self {
        self.style = self.style.padding_xy(x, y);
        self
    }

    fn resolved(&self, theme_size: f32) -> (f32, f32) {
        (
            self.font_size.unwrap_or(theme_size) * self.font_scale,
            self.wrap_width.unwrap_or(0.0),
        )
    }
}

impl View for RichText {
    fn body(&self) -> impl View + 'static {
        let _ = (self.spans)();
        Unit
    }

    fn is_leaf(&self) -> bool {
        true
    }

    fn clip_self(&self) -> bool {
        true
    }

    fn cursor_icon(&self, _state: sequoia_style::VisualState) -> CursorIcon {
        if self.on_span_click.is_some() {
            CursorIcon::Pointer
        } else {
            CursorIcon::Default
        }
    }

    fn layout_style(&self) -> LayoutStyle {
        self.style
    }

    fn intrinsic_size(&self, ctx: &mut LayoutCtx) -> Option<Size> {
        let (size, _wrap) = self.resolved(ctx.default_font_size());
        let spans = (self.spans)();
        let pad = self.style.padding;
        let auto_wrap = if self.auto_wrap {
            text_wrap_limit(self.style, ctx.available_width())
        } else {
            None
        };
        let mut width = self.wrap_width.or(auto_wrap).unwrap_or(0.0);
        if self.max_lines.is_some() && width <= 0.0 {
            width = ctx
                .rich_size_lh(
                    &spans,
                    size,
                    0.0,
                    self.wrap_mode,
                    self.letter_spacing,
                    self.align,
                    self.line_height,
                )
                .width
                .max(1.0);
        }
        let shown = match self.max_lines {
            Some(max_lines) => ctx.rich_elide_lines(
                &spans,
                size,
                width,
                self.wrap_mode,
                self.letter_spacing,
                self.align,
                self.line_height,
                max_lines,
            ),
            None => spans,
        };
        let text = ctx.rich_size_lh(
            &shown,
            size,
            width,
            self.wrap_mode,
            self.letter_spacing,
            self.align,
            self.line_height,
        );
        Some(Size::new(
            text.width + pad.horizontal_sum(),
            text.height + pad.vertical_sum(),
        ))
    }

    fn baseline_offset(&self, ctx: &mut LayoutCtx, width: f32) -> Option<f32> {
        let (size, wrap) = self.resolved(ctx.default_font_size());
        let spans = (self.spans)();
        let pad = self.style.padding;
        let width = if wrap > 0.0 {
            wrap
        } else {
            (width - pad.horizontal_sum()).max(0.0)
        };
        let baseline = ctx.rich_first_baseline(
            &spans,
            size,
            width,
            self.wrap_mode,
            self.letter_spacing,
            self.align,
            self.line_height,
        );
        Some(pad.top + baseline)
    }

    fn on_event(&self, event: &Event, ctx: &mut EventCtx) {
        if ctx.phase != EventPhase::Target {
            return;
        }
        let Some(handler) = &self.on_span_click else {
            return;
        };
        if let Event::Pointer(PointerEvent::Down {
            button: PointerButton::Left,
            pos,
            ..
        }) = event
        {
            let size = self.font_size.unwrap_or_else(|| {
                ctx.environment
                    .get::<sequoia_style::Theme>()
                    .map(|t| t.font_size)
                    .unwrap_or(14.0)
            }) * self.font_scale;
            let spans = (self.spans)();
            let pad = self.style.padding;
            let wrap = self.wrap_width.unwrap_or_else(|| {
                if self.auto_wrap {
                    (ctx.rect.size.width - pad.horizontal_sum()).max(0.0)
                } else {
                    0.0
                }
            });
            let shown = match self.max_lines {
                Some(max_lines) => ctx.text.elide_rich_lines(
                    &spans,
                    size,
                    wrap,
                    self.wrap_mode,
                    self.letter_spacing,
                    self.align,
                    self.line_height,
                    max_lines,
                ),
                None => spans,
            };
            let rich = ctx.text.shape_rich_full_lh(
                &shown,
                size,
                wrap,
                self.wrap_mode,
                self.letter_spacing,
                self.align,
                self.line_height,
            );
            let text_h = rich.height();
            let first_baseline = rich.line(0).map(|line| line.baseline).unwrap_or(0.0);
            let block_top = match self.align_y {
                TextVAlign::Baseline => pad.top - first_baseline,
                TextVAlign::Top => pad.top,
                TextVAlign::Center => {
                    pad.top + (ctx.rect.size.height - pad.vertical_sum() - text_h) * 0.5
                }
                TextVAlign::Bottom => {
                    pad.top + (ctx.rect.size.height - pad.vertical_sum() - text_h)
                }
                _ => pad.top,
            };
            let hit = rich.hit_point(
                pos.x - ctx.rect.left() - pad.left,
                pos.y - ctx.rect.top() - block_top,
            );
            if let Some(span) = hit.span {
                handler(span);
                ctx.handled();
            }
        }
    }

    fn paint_self(&self, ctx: &mut PaintCtx) {
        let theme = ctx.theme();
        let (size, wrap) = self.resolved(theme.font_size);
        let color = self.color.unwrap_or(theme.text);
        let spans = (self.spans)();
        if spans.iter().all(|s| s.text.is_empty()) {
            return;
        }
        let pad = self.style.padding;
        let content_w = (ctx.rect.size.width - pad.horizontal_sum()).max(0.0);
        let wrap = if wrap > 0.0 {
            wrap
        } else if self.auto_wrap {
            content_w
        } else {
            0.0
        };
        let shown = match self.max_lines {
            Some(max_lines) => ctx.elide_rich_lines(
                &spans,
                size,
                wrap,
                self.wrap_mode,
                self.letter_spacing,
                self.align,
                self.line_height,
                max_lines,
            ),
            None => spans,
        };
        let rich = ctx.shape_rich_full_lh(
            &shown,
            size,
            wrap,
            self.wrap_mode,
            self.letter_spacing,
            self.align,
            self.line_height,
        );
        let text_h = rich.height();
        let first_baseline = rich.line(0).map(|line| line.baseline).unwrap_or(0.0);
        let block_top = match self.align_y {
            TextVAlign::Baseline => ctx.rect.top() + pad.top - first_baseline,
            TextVAlign::Top => ctx.rect.top() + pad.top,
            TextVAlign::Center => {
                ctx.rect.top()
                    + pad.top
                    + (ctx.rect.size.height - pad.vertical_sum() - text_h) * 0.5
            }
            TextVAlign::Bottom => {
                ctx.rect.top() + pad.top + (ctx.rect.size.height - pad.vertical_sum() - text_h)
            }
            _ => ctx.rect.top() + pad.top,
        };
        ctx.draw_rich(&rich, (ctx.rect.left() + pad.left, block_top), color);
    }
}
