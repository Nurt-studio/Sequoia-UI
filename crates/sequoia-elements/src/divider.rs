use sequoia_core::{Color, Edges, Length, Rect, Size};
use sequoia_layout::{LayoutCtx, LayoutStyle};
use sequoia_style::DividerStyle;
use sequoia_view::{PaintCtx, Unit, View};

const THICKNESS: f32 = 1.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum DividerAxis {
    #[default]
    Horizontal,

    Vertical,
}

pub fn divider_line_rect(rect: Rect, axis: DividerAxis, thickness: f32) -> Rect {
    let t = thickness.max(0.0);
    match axis {
        DividerAxis::Horizontal => {
            let h = t.min(rect.size.height.max(0.0));
            Rect::from_xywh(
                rect.left(),
                rect.top() + (rect.size.height - h) * 0.5,
                rect.size.width.max(0.0),
                h,
            )
        }
        DividerAxis::Vertical => {
            let w = t.min(rect.size.width.max(0.0));
            Rect::from_xywh(
                rect.left() + (rect.size.width - w) * 0.5,
                rect.top(),
                w,
                rect.size.height.max(0.0),
            )
        }
    }
}

pub struct Divider {
    axis: DividerAxis,
    style: Option<DividerStyle>,
    color: Option<Color>,
    thickness: Option<f32>,
    margin: Edges,
}

impl Divider {
    pub fn horizontal() -> Self {
        Self {
            axis: DividerAxis::Horizontal,
            style: None,
            color: None,
            thickness: None,
            margin: Edges::ZERO,
        }
    }

    pub fn vertical() -> Self {
        Self {
            axis: DividerAxis::Vertical,
            style: None,
            color: None,
            thickness: None,
            margin: Edges::ZERO,
        }
    }

    pub fn axis(mut self, axis: DividerAxis) -> Self {
        self.axis = axis;
        self
    }

    pub fn style(mut self, style: DividerStyle) -> Self {
        self.style = Some(style);
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn thickness(mut self, px: f32) -> Self {
        self.thickness = Some(px);
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

    fn resolved(&self, style: DividerStyle, theme: &sequoia_style::Theme) -> (Color, f32) {
        let _ = theme;
        let color = self.color.or(style.color).unwrap_or(Color::hex(0x1F1F1F));
        let thickness = self
            .thickness
            .or(style.thickness)
            .unwrap_or(THICKNESS)
            .max(0.0);
        (color, thickness)
    }
}

impl Default for Divider {
    fn default() -> Self {
        Self::horizontal()
    }
}

fn sanitize_margin(v: f32) -> f32 {
    if v.is_finite() { v.max(0.0) } else { 0.0 }
}

impl View for Divider {
    fn body(&self) -> impl View + 'static {
        Unit
    }
    fn is_leaf(&self) -> bool {
        true
    }
    fn hit_testable(&self) -> bool {
        false
    }
    fn layout_style(&self) -> LayoutStyle {
        match self.axis {
            DividerAxis::Horizontal => LayoutStyle {
                width: Length::Fill(1.0),
                height: Length::Fit,
                margin: self.margin,
                ..LayoutStyle::default().hit_testable(false)
            },
            DividerAxis::Vertical => LayoutStyle {
                width: Length::Fit,
                height: Length::Fill(1.0),
                margin: self.margin,
                ..LayoutStyle::default().hit_testable(false)
            },
        }
    }
    fn intrinsic_size(&self, _ctx: &mut LayoutCtx) -> Option<Size> {
        let style = self.style.unwrap_or_default();
        let t = self
            .thickness
            .or(style.thickness)
            .unwrap_or(THICKNESS)
            .max(0.0);
        Some(match self.axis {
            DividerAxis::Horizontal => Size::new(0.0, t),
            DividerAxis::Vertical => Size::new(t, 0.0),
        })
    }
    fn paint_self(&self, ctx: &mut PaintCtx) {
        let style = self.style.unwrap_or_default();
        let theme = ctx.theme();
        let (color, thickness) = self.resolved(style, &theme);
        if thickness <= 0.0 || color.a <= 0.0 {
            return;
        }
        let line = divider_line_rect(ctx.rect, self.axis, thickness);
        if line.size.width <= 0.0 || line.size.height <= 0.0 {
            return;
        }
        ctx.quad(line, color, 0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn horizontal_centers_thickness() {
        let rect = Rect::from_xywh(0.0, 0.0, 200.0, 21.0);
        assert_eq!(
            divider_line_rect(rect, DividerAxis::Horizontal, 1.0),
            Rect::from_xywh(0.0, 10.0, 200.0, 1.0)
        );
    }

    #[test]
    fn vertical_centers_thickness() {
        let rect = Rect::from_xywh(0.0, 0.0, 21.0, 200.0);
        assert_eq!(
            divider_line_rect(rect, DividerAxis::Vertical, 1.0),
            Rect::from_xywh(10.0, 0.0, 1.0, 200.0)
        );
    }

    #[test]
    fn thickness_never_overflows_rect() {
        let rect = Rect::from_xywh(5.0, 5.0, 10.0, 2.0);
        let line = divider_line_rect(rect, DividerAxis::Horizontal, 8.0);
        assert_eq!(line.size.height, 2.0);
        assert_eq!(line.top(), 5.0);
    }
}
