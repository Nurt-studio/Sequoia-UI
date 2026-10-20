use std::rc::Rc;

use sequoia_core::Edges;
use sequoia_core::{Color, Length, Size};
use sequoia_input::{CursorIcon, Event, EventPhase, PointerButton, PointerEvent};
use sequoia_layout::{LayoutCtx, LayoutStyle};
use sequoia_text::{LetterSpacing, RichAlign, TextSpan, WrapMode};

use crate::view::{EventCtx, PaintCtx, View};
use crate::views::Unit;

pub struct RichText {
    spans: Box<dyn Fn() -> Vec<TextSpan>>,
    font_size: Option<f32>,
    color: Option<Color>,
    wrap_width: Option<f32>,
    wrap_mode: WrapMode,
    align: RichAlign,
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
            self.font_size.unwrap_or(theme_size),
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
        let (size, wrap) = self.resolved(14.0);
        let spans = (self.spans)();
        let pad = self.style.padding;
        let text = ctx.rich_size(
            &spans,
            size,
            wrap,
            self.wrap_mode,
            self.letter_spacing,
            self.align,
        );
        Some(Size::new(
            text.width + pad.horizontal_sum(),
            text.height + pad.vertical_sum(),
        ))
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
            });
            let wrap = self.wrap_width.unwrap_or(0.0);
            let spans = (self.spans)();
            let pad = self.style.padding;
            let rich = ctx.shape_rich_full(
                &spans,
                size,
                wrap,
                self.wrap_mode,
                self.letter_spacing,
                self.align,
            );
            let hit = rich.hit_point(
                pos.x - ctx.rect.left() - pad.left,
                pos.y - ctx.rect.top() - pad.top,
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
        let origin = (ctx.rect.left() + pad.left, ctx.rect.top() + pad.top);
        ctx.draw_rich_spans(
            &spans,
            origin,
            size,
            wrap,
            self.wrap_mode,
            self.letter_spacing,
            self.align,
            color,
        );
    }
}
