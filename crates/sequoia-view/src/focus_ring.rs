use sequoia_core::{Color, Edges, Length, Rect};
use sequoia_layout::LayoutStyle;
use sequoia_style::FocusRingStyle;

use crate::any::AnyView;
use crate::view::{PaintCtx, View};
use crate::views::Unit;

pub const RING_WIDTH: f32 = 2.0;
pub const RING_OFFSET: f32 = 2.0;

pub fn ring_rect(rect: Rect, offset: f32) -> Rect {
    let o = if offset.is_finite() {
        offset.max(0.0)
    } else {
        0.0
    };
    rect.inflate(o, o)
}

pub(crate) fn ring_margin(offset: f32, width: f32) -> f32 {
    let width = if width.is_finite() {
        width.max(0.0)
    } else {
        0.0
    };
    let offset = if offset.is_finite() {
        offset.max(0.0)
    } else {
        0.0
    };
    offset + width * 0.5
}

fn child_box(rect: Rect, margin: f32) -> Rect {
    let m = if margin.is_finite() {
        margin.max(0.0)
    } else {
        0.0
    };
    Rect::from_xywh(
        rect.left() + m,
        rect.top() + m,
        (rect.size.width - m * 2.0).max(0.0),
        (rect.size.height - m * 2.0).max(0.0),
    )
}

#[derive(Clone)]
pub struct FocusRing {
    child: AnyView,
    style: Option<FocusRingStyle>,
    color: Option<Color>,
    width: Option<f32>,
    offset: Option<f32>,
    radius: Option<f32>,
    enabled: bool,

    reserve_space: bool,
}

impl FocusRing {
    pub fn wrap<V: View + 'static>(child: V) -> Self {
        Self {
            child: AnyView::new(child),
            style: None,
            color: None,
            width: None,
            offset: None,
            radius: None,
            enabled: true,
            reserve_space: false,
        }
    }

    pub fn style(mut self, style: FocusRingStyle) -> Self {
        self.style = Some(style);
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn width(mut self, px: f32) -> Self {
        self.width = Some(px);
        self
    }

    pub fn offset(mut self, px: f32) -> Self {
        self.offset = Some(px);
        self
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = Some(radius);
        self
    }

    pub fn enabled(mut self, on: bool) -> Self {
        self.enabled = on;
        self
    }

    pub fn reserve_space(mut self, on: bool) -> Self {
        self.reserve_space = on;
        self
    }

    fn resolved(&self, theme: &sequoia_style::Theme) -> (Color, f32, f32, f32) {
        let style = self.style.unwrap_or_default();
        let color = self.color.or(style.color).unwrap_or(theme.accent);
        let width = self.width.or(style.width).unwrap_or(RING_WIDTH).max(0.0);
        let offset = self.offset.or(style.offset).unwrap_or(RING_OFFSET).max(0.0);
        let radius = self
            .radius
            .or(style.radius)
            .unwrap_or(theme.radius)
            .max(0.0);
        (color, width, offset, radius)
    }

    pub fn reserved_margin(&self) -> f32 {
        if !self.enabled || !self.reserve_space {
            return 0.0;
        }
        let style = self.style.unwrap_or_default();
        let width = self.width.or(style.width).unwrap_or(RING_WIDTH).max(0.0);
        let offset = self.offset.or(style.offset).unwrap_or(RING_OFFSET).max(0.0);
        ring_margin(offset, width)
    }

    pub fn should_paint(&self, focus_within: bool) -> bool {
        self.enabled && focus_within
    }
}

impl View for FocusRing {
    fn body(&self) -> impl View + 'static {
        Unit
    }
    fn children_override(&self) -> Option<Vec<AnyView>> {
        Some(vec![self.child.clone()])
    }
    fn layout_style(&self) -> LayoutStyle {
        let m = self.reserved_margin();
        LayoutStyle {
            width: Length::Fit,
            height: Length::Fit,

            padding: if m > 0.0 {
                Edges::uniform(m)
            } else {
                Edges::ZERO
            },
            ..LayoutStyle::default()
        }
    }

    fn hit_testable(&self) -> bool {
        false
    }

    fn paint_self(&self, ctx: &mut PaintCtx) {
        if !self.should_paint(ctx.has_focus_within()) {
            return;
        }
        let theme = ctx.theme();
        let (color, width, offset, radius) = self.resolved(&theme);
        if width <= 0.0 || color.a <= 0.0 {
            return;
        }

        let rect = ring_rect(child_box(ctx.rect, self.reserved_margin()), offset);
        if rect.size.width <= 0.0 || rect.size.height <= 0.0 {
            return;
        }
        ctx.bordered_quad(rect, Color::TRANSPARENT, radius + offset, width, color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_rect_inflates_by_offset() {
        let r = Rect::from_xywh(10.0, 20.0, 100.0, 40.0);
        assert_eq!(ring_rect(r, 2.0), Rect::from_xywh(8.0, 18.0, 104.0, 44.0));
        assert_eq!(ring_rect(r, 0.0), r);
        assert_eq!(ring_rect(r, -5.0), r, "ujemny offset = brak");
    }

    #[test]
    fn should_paint_needs_enabled_and_focus() {
        let ring = FocusRing::wrap(Unit);
        assert!(ring.should_paint(true));
        assert!(!ring.should_paint(false));
        assert!(!ring.enabled(false).should_paint(true));
    }

    #[test]
    fn builder_overrides_style_over_theme() {
        let theme = sequoia_style::Theme::dark();
        let ring = FocusRing::wrap(Unit)
            .style(FocusRingStyle::new().width(3.0))
            .width(5.0);
        let (_, w, _, _) = ring.resolved(&theme);
        assert_eq!(w, 5.0, "builder wygrywa ze stylem");
        let ring = FocusRing::wrap(Unit).style(FocusRingStyle::new().width(3.0));
        let (_, w, _, _) = ring.resolved(&theme);
        assert_eq!(w, 3.0, "styl wygrywa z theme");
        let ring = FocusRing::wrap(Unit);
        let (c, w, o, _) = ring.resolved(&theme);
        assert_eq!(c, theme.accent);
        assert_eq!(w, RING_WIDTH);
        assert_eq!(o, RING_OFFSET);
    }

    #[test]
    fn ring_margin_is_offset_plus_half_width() {
        assert!((ring_margin(RING_OFFSET, RING_WIDTH) - 3.0).abs() < 1e-6);
        assert!((ring_margin(1.0, 2.0) - 2.0).abs() < 1e-6);

        assert_eq!(ring_margin(f32::NAN, f32::NAN), 0.0);
        assert_eq!(ring_margin(-5.0, -1.0), 0.0);
    }

    #[test]
    fn child_box_insets_by_margin() {
        let r = Rect::from_xywh(0.0, 0.0, 14.0, 14.0);
        assert_eq!(child_box(r, 3.0), Rect::from_xywh(3.0, 3.0, 8.0, 8.0));
        assert_eq!(child_box(r, 0.0), r);

        assert_eq!(child_box(r, 100.0).size.width, 0.0);
    }

    #[test]
    fn layout_reserves_only_when_opted_in() {
        use sequoia_core::Edges;

        let ring = FocusRing::wrap(Unit);
        assert_eq!(ring.reserved_margin(), 0.0);
        assert_eq!(ring.layout_style().padding, Edges::ZERO);
        assert_eq!(ring.layout_style().width, Length::Fit);

        let ring = FocusRing::wrap(Unit).reserve_space(true);
        assert_eq!(ring.reserved_margin(), 3.0);
        assert_eq!(ring.layout_style().padding, Edges::uniform(3.0));

        let ring = FocusRing::wrap(Unit)
            .reserve_space(true)
            .offset(1.0)
            .width(1.0);
        assert_eq!(ring.reserved_margin(), 1.5);

        assert_eq!(FocusRing::wrap(Unit).enabled(false).reserved_margin(), 0.0);
        assert_eq!(
            FocusRing::wrap(Unit)
                .reserve_space(true)
                .enabled(false)
                .reserved_margin(),
            0.0
        );
    }

    #[test]
    fn margin_is_click_through() {
        assert!(!FocusRing::wrap(Unit).hit_testable());
        assert!(!FocusRing::wrap(Unit).enabled(false).hit_testable());
    }

    #[test]
    fn optin_ink_fits_inside_reserved_box() {
        let ring = FocusRing::wrap(Unit).reserve_space(true);
        let wrapper = Rect::from_xywh(0.0, 0.0, 14.0, 14.0);
        let child = child_box(wrapper, ring.reserved_margin());
        let path = ring_rect(child, 2.0);
        let ink_outer = path.left() - 2.0 * 0.5;
        assert!(
            ink_outer >= wrapper.left() - 1e-6,
            "atrament nie wystaje poza box wrappera: {ink_outer}"
        );
    }

    #[test]
    fn default_ring_flush_with_child() {
        let ring = FocusRing::wrap(Unit);
        let wrapper = Rect::from_xywh(0.0, 0.0, 8.0, 8.0);
        assert_eq!(child_box(wrapper, ring.reserved_margin()), wrapper);
    }
}
