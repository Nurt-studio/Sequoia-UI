use sequoia_core::{Color, Edges, Size};
use sequoia_layout::{Axis, ContainerStyle, LayoutCtx, LayoutStyle};
use sequoia_style::TooltipStyle;
use sequoia_view::{
    Anchor, AnyView, InputMode, OverlayEntry, OverlayLayer, PaintCtx, Placement, Unit, View,
};

const PAD_X: f32 = 8.0;
const PAD_Y: f32 = 5.0;

pub struct Tooltip {
    child: Option<AnyView>,
    tip: String,
    font_size: Option<f32>,
    style: Option<TooltipStyle>,
    background: Option<Color>,
    border: Option<Color>,
    text_color: Option<Color>,
    radius: Option<f32>,
    padding: Option<(f32, f32)>,
    margin: Edges,
}

impl Tooltip {
    pub fn wrap<V: View + 'static>(child: V, tip: impl Into<String>) -> Self {
        Self::new(child, tip)
    }

    pub fn new<V: View + 'static>(child: V, tip: impl Into<String>) -> Self {
        Self {
            child: Some(AnyView::new(child)),
            tip: tip.into(),
            font_size: None,
            style: None,
            background: None,
            border: None,
            text_color: None,
            radius: None,
            padding: None,
            margin: Edges::ZERO,
        }
    }

    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = Some(size);
        self
    }

    pub fn style(mut self, style: TooltipStyle) -> Self {
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
}

impl View for Tooltip {
    fn body(&self) -> impl View + 'static {
        Unit
    }
    fn children_override(&self) -> Option<Vec<AnyView>> {
        self.child.clone().map(|c| vec![c])
    }
    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: sequoia_core::Length::Fit,
            height: sequoia_core::Length::Fit,
            margin: self.margin,
            ..LayoutStyle::default()
        }
    }
    fn container(&self) -> Option<ContainerStyle> {
        Some(ContainerStyle::new(Axis::Vertical, 0.0))
    }
    fn has_visual_state(&self) -> bool {
        true
    }
    fn paint_self(&self, ctx: &mut PaintCtx) {
        if !ctx.hovered_subtree || self.tip.is_empty() {
            return;
        }
        let theme = ctx.theme();
        let style = self.style.unwrap_or_default();
        let size = self
            .font_size
            .or(style.font_size)
            .unwrap_or(theme.font_size);
        let background = self
            .background
            .or(style.background)
            .unwrap_or(Color::rgb8(0x2B, 0x2B, 0x2B));
        let border = self.border.or(style.border).unwrap_or(theme.border);
        let text = self.text_color.or(style.text).unwrap_or(theme.text);
        let radius = self.radius.or(style.radius).unwrap_or(3.0);
        let padding = self.padding.or(style.padding).unwrap_or((PAD_X, PAD_Y));
        ctx.show_overlay(
            OverlayEntry::new(TooltipBubble {
                tip: self.tip.clone(),
                font_size: size,
                background,
                border,
                text,
                radius,
                padding,
            })
            .anchor(Anchor::Rect(ctx.rect))
            .placement(Placement::Bottom)
            .layer(OverlayLayer::Tooltip)
            .input_mode(InputMode::Passthrough)
            .transient(true)
            .offset(0.0, 4.0),
        );
    }
}

struct TooltipBubble {
    tip: String,
    font_size: f32,
    background: Color,
    border: Color,
    text: Color,
    radius: f32,
    padding: (f32, f32),
}

impl View for TooltipBubble {
    fn body(&self) -> impl View + 'static {
        Unit
    }
    fn is_leaf(&self) -> bool {
        true
    }
    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: sequoia_core::Length::Fit,
            height: sequoia_core::Length::Fit,
            ..LayoutStyle::default()
        }
    }
    fn intrinsic_size(&self, ctx: &mut LayoutCtx) -> Option<Size> {
        let w = ctx.text_width(&self.tip, self.font_size);
        let (pad_x, pad_y) = self.padding;
        Some(Size::new(
            w + pad_x * 2.0,
            LayoutCtx::text_line_height(self.font_size) + pad_y * 2.0,
        ))
    }
    fn paint_self(&self, ctx: &mut PaintCtx) {
        let rect = ctx.rect;
        if rect.size.width <= 0.0 || rect.size.height <= 0.0 {
            return;
        }
        ctx.quad(rect, self.background, self.radius);
        ctx.quad(
            sequoia_core::Rect::from_xywh(
                rect.left() + 1.0,
                rect.top() + 1.0,
                (rect.size.width - 2.0).max(0.0),
                (rect.size.height - 2.0).max(0.0),
            ),
            self.border,
            (self.radius - 1.0).max(0.0),
        );
        let (pad_x, _) = self.padding;
        let x = rect.left() + pad_x;
        let y = rect.top() + rect.size.height * 0.5 + self.font_size * 0.35;
        ctx.draw_text(&self.tip, (x, y), self.font_size, self.text);
    }
}

fn sanitize_margin(v: f32) -> f32 {
    if v.is_finite() { v.max(0.0) } else { 0.0 }
}
