use sequoia_core::{Color, Edges, Length, Size};
use sequoia_layout::{LayoutCtx, LayoutStyle};
use sequoia_style::KbdStyle;
use sequoia_view::{FontWeight, PaintCtx, Unit, View};

const PAD_X: f32 = 8.0;
const PAD_Y: f32 = 4.0;
const RADIUS: f32 = 6.0;
const FONT_SIZE: f32 = 12.0;
const BORDER_W: f32 = 1.0;

pub struct Kbd {
    label: String,
    style: Option<KbdStyle>,
    background: Option<Color>,
    border: Option<Color>,
    text_color: Option<Color>,
    font_size: Option<f32>,
    radius: Option<f32>,
    padding: Option<(f32, f32)>,
    margin: Edges,
}

impl Kbd {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            style: None,
            background: None,
            border: None,
            text_color: None,
            font_size: None,
            radius: None,
            padding: None,
            margin: Edges::ZERO,
        }
    }

    pub fn style(mut self, style: KbdStyle) -> Self {
        self.style = Some(style);
        self
    }

    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    pub fn border(mut self, color: Color) -> Self {
        self.border = Some(color);
        self
    }

    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color = Some(color);
        self
    }

    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = Some(size);
        self
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = Some(radius);
        self
    }

    pub fn padding_xy(mut self, pad_x: f32, pad_y: f32) -> Self {
        self.padding = Some((pad_x, pad_y));
        self
    }

    pub fn margin(mut self, margin: impl Into<Edges>) -> Self {
        let m = margin.into();
        self.margin = Edges::only(
            sanitize_margin(m.left),
            sanitize_margin(m.top),
            sanitize_margin(m.right),
            sanitize_margin(m.bottom),
        );
        self
    }

    pub fn margin_xy(self, x: f32, y: f32) -> Self {
        self.margin(Edges::xy(x, y))
    }

    pub fn margin_x(mut self, x: f32) -> Self {
        let x = sanitize_margin(x);
        self.margin.left = x;
        self.margin.right = x;
        self
    }

    pub fn margin_y(mut self, y: f32) -> Self {
        let y = sanitize_margin(y);
        self.margin.top = y;
        self.margin.bottom = y;
        self
    }

    pub fn margin_left(mut self, v: f32) -> Self {
        self.margin.left = sanitize_margin(v);
        self
    }

    pub fn margin_right(mut self, v: f32) -> Self {
        self.margin.right = sanitize_margin(v);
        self
    }

    pub fn margin_top(mut self, v: f32) -> Self {
        self.margin.top = sanitize_margin(v);
        self
    }

    pub fn margin_bottom(mut self, v: f32) -> Self {
        self.margin.bottom = sanitize_margin(v);
        self
    }

    fn font_size_resolved(&self, style: KbdStyle) -> f32 {
        self.font_size.or(style.font_size).unwrap_or(FONT_SIZE)
    }

    fn padding_resolved(&self, style: KbdStyle) -> (f32, f32) {
        self.padding.or(style.padding).unwrap_or((PAD_X, PAD_Y))
    }
}

fn sanitize_margin(v: f32) -> f32 {
    if v.is_finite() { v.max(0.0) } else { 0.0 }
}

impl View for Kbd {
    fn body(&self) -> impl View + 'static {
        Unit
    }
    fn is_leaf(&self) -> bool {
        true
    }
    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fit,
            height: Length::Fit,
            margin: self.margin,
            ..LayoutStyle::default()
        }
    }
    fn intrinsic_size(&self, ctx: &mut LayoutCtx) -> Option<Size> {
        let style = self.style.unwrap_or_default();
        let size = self.font_size_resolved(style);
        let (pad_x, pad_y) = self.padding_resolved(style);
        let w = ctx.text_width_weighted(&self.label, size, FontWeight::SEMIBOLD);
        Some(Size::new(
            w + pad_x * 2.0,
            LayoutCtx::text_line_height(size) + pad_y * 2.0,
        ))
    }
    fn paint_self(&self, ctx: &mut PaintCtx) {
        if self.label.is_empty() {
            return;
        }
        let rect = ctx.rect;
        if rect.size.width <= 0.0 || rect.size.height <= 0.0 {
            return;
        }
        let theme = ctx.theme();
        let style = self.style.unwrap_or_default();
        let size = self.font_size_resolved(style);
        let (pad_x, _) = self.padding_resolved(style);
        let background = self
            .background
            .or(style.background)
            .unwrap_or(theme.surface);
        let border = self.border.or(style.border).unwrap_or(theme.border);
        let text = self.text_color.or(style.text).unwrap_or(theme.text);
        let radius = self.radius.or(style.radius).unwrap_or(RADIUS);
        ctx.bordered_quad(rect, background, radius, BORDER_W, border);
        let x = rect.left() + pad_x;
        let y = rect.top() + rect.size.height * 0.5 + size * 0.35;
        ctx.draw_text_weighted(&self.label, (x, y), size, FontWeight::SEMIBOLD, text);
    }
}
