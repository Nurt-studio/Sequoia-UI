use sequoia_core::{Color, Rect, RenderList, Shadow, Vec2};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShadowSpec {
    pub color: Color,

    pub blur: f32,

    pub spread: f32,

    pub offset: Vec2,
}

impl ShadowSpec {
    pub fn new(color: Color, blur: f32) -> Self {
        Self {
            color,
            blur,
            spread: 0.0,
            offset: Vec2::ZERO,
        }
    }

    pub fn spread(mut self, spread: f32) -> Self {
        self.spread = spread;
        self
    }

    pub fn offset(mut self, x: f32, y: f32) -> Self {
        self.offset = Vec2::new(x, y);
        self
    }

    pub fn is_visible(self) -> bool {
        self.color.a > 0.0 && self.blur > 0.0 && self.blur.is_finite()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BorderSide {
    pub width: f32,

    pub color: Color,
}

impl Default for BorderSide {
    fn default() -> Self {
        Self {
            width: 0.0,
            color: Color::TRANSPARENT,
        }
    }
}

impl BorderSide {
    pub fn new(width: f32, color: Color) -> Self {
        Self {
            width: width.max(0.0),
            color,
        }
    }

    pub fn is_visible(self) -> bool {
        self.width > 0.0 && self.color.a > 0.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct BoxStyle {
    pub background: Color,

    pub border_top: BorderSide,

    pub border_right: BorderSide,

    pub border_bottom: BorderSide,

    pub border_left: BorderSide,

    pub radius: f32,

    pub shadow: Option<ShadowSpec>,

    pub inset_shadow: Option<ShadowSpec>,
}

impl Default for BoxStyle {
    fn default() -> Self {
        Self {
            background: Color::TRANSPARENT,
            border_top: BorderSide::default(),
            border_right: BorderSide::default(),
            border_bottom: BorderSide::default(),
            border_left: BorderSide::default(),
            radius: 0.0,
            shadow: None,
            inset_shadow: None,
        }
    }
}

impl BoxStyle {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn border_edges(self) -> sequoia_core::Edges {
        sequoia_core::Edges::only(
            self.border_left.width.max(0.0),
            self.border_top.width.max(0.0),
            self.border_right.width.max(0.0),
            self.border_bottom.width.max(0.0),
        )
    }

    pub fn background(mut self, color: Color) -> Self {
        self.background = color;
        self
    }

    pub fn border(mut self, width: f32, color: Color) -> Self {
        let side = BorderSide::new(width, color);
        self.border_top = side;
        self.border_right = side;
        self.border_bottom = side;
        self.border_left = side;
        self
    }

    pub fn border_left(mut self, width: f32, color: Color) -> Self {
        self.border_left = BorderSide::new(width, color);
        self
    }

    pub fn border_right(mut self, width: f32, color: Color) -> Self {
        self.border_right = BorderSide::new(width, color);
        self
    }

    pub fn border_top(mut self, width: f32, color: Color) -> Self {
        self.border_top = BorderSide::new(width, color);
        self
    }

    pub fn border_bottom(mut self, width: f32, color: Color) -> Self {
        self.border_bottom = BorderSide::new(width, color);
        self
    }

    pub fn border_x(mut self, width: f32, color: Color) -> Self {
        let side = BorderSide::new(width, color);
        self.border_left = side;
        self.border_right = side;
        self
    }

    pub fn border_y(mut self, width: f32, color: Color) -> Self {
        let side = BorderSide::new(width, color);
        self.border_top = side;
        self.border_bottom = side;
        self
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius.max(0.0);
        self
    }

    pub fn shadow(mut self, spec: ShadowSpec) -> Self {
        self.shadow = Some(spec);
        self
    }

    pub fn inset_shadow(mut self, spec: ShadowSpec) -> Self {
        self.inset_shadow = Some(spec);
        self
    }

    pub fn is_empty(self) -> bool {
        self.background.a <= 0.0
            && !self.border_top.is_visible()
            && !self.border_right.is_visible()
            && !self.border_bottom.is_visible()
            && !self.border_left.is_visible()
            && !self.shadow.is_some_and(ShadowSpec::is_visible)
            && !self.inset_shadow.is_some_and(ShadowSpec::is_visible)
    }

    fn paint_shadow(&self, list: &mut RenderList, rect: Rect, spec: ShadowSpec, inset: bool) {
        if !spec.is_visible() {
            return;
        }

        let radius = if inset {
            (self.radius - spec.spread).max(0.0)
        } else {
            (self.radius + spec.spread).max(0.0)
        };
        list.push_shadow(Shadow {
            rect,
            color: spec.color,
            radius,
            blur: spec.blur,
            spread: spec.spread,
            offset: spec.offset,
            inset,
        });
    }

    pub fn uniform_border(self) -> Option<(f32, Color)> {
        let t = self.border_top;
        if !t.is_visible() {
            return None;
        }
        for s in [self.border_right, self.border_bottom, self.border_left] {
            if !s.is_visible() || s.width != t.width || s.color != t.color {
                return None;
            }
        }
        Some((t.width, t.color))
    }

    pub fn paint(self, list: &mut RenderList, rect: Rect) {
        if self.is_empty() {
            return;
        }
        if let Some(spec) = self.shadow {
            self.paint_shadow(list, rect, spec, false);
        }
        if let Some((w, c)) = self.uniform_border() {
            list.push_bordered_quad(rect, self.background, self.radius, w, c);
            if let Some(spec) = self.inset_shadow {
                self.paint_shadow(list, rect, spec, true);
            }
            return;
        }
        if self.background.a > 0.0 {
            list.push_quad(rect, self.background, self.radius);
        }
        let x = rect.left();
        let y = rect.top();
        let w = rect.size.width;
        let h = rect.size.height;

        if self.border_left.is_visible() {
            let lw = self.border_left.width.min(w.max(0.0));
            list.push_quad(Rect::from_xywh(x, y, lw, h), self.border_left.color, 0.0);
        }
        if self.border_right.is_visible() {
            let rw = self.border_right.width.min(w.max(0.0));
            list.push_quad(
                Rect::from_xywh(x + w - rw, y, rw, h),
                self.border_right.color,
                0.0,
            );
        }

        let lw = if self.border_left.is_visible() {
            self.border_left.width.min(w.max(0.0))
        } else {
            0.0
        };
        let rw = if self.border_right.is_visible() {
            self.border_right.width.min(w.max(0.0))
        } else {
            0.0
        };
        let inner_w = (w - lw - rw).max(0.0);
        if self.border_top.is_visible() {
            let th = self.border_top.width;
            list.push_quad(
                Rect::from_xywh(x + lw, y, inner_w, th),
                self.border_top.color,
                0.0,
            );
        }
        if self.border_bottom.is_visible() {
            let bh = self.border_bottom.width;
            list.push_quad(
                Rect::from_xywh(x + lw, y + h - bh, inner_w, bh),
                self.border_bottom.color,
                0.0,
            );
        }
        if let Some(spec) = self.inset_shadow {
            self.paint_shadow(list, rect, spec, true);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sequoia_core::Vec2;
    use sequoia_core::{Rect, RenderItem};

    #[test]
    fn shadow_is_painted_under_and_inset_over_the_decoration() {
        let r = Rect::from_xywh(0.0, 0.0, 100.0, 50.0);
        let mut list = RenderList::new();
        BoxStyle::new()
            .background(Color::WHITE)
            .radius(8.0)
            .shadow(ShadowSpec::new(Color::BLACK, 12.0).offset(0.0, 4.0))
            .inset_shadow(ShadowSpec::new(Color::BLACK, 6.0))
            .paint(&mut list, r);
        let items = list.items();

        let RenderItem::Shadow(outer) = items[0] else {
            panic!("outer shadow first")
        };
        assert!(!outer.inset);
        assert_eq!(outer.rect, r, "caster shape is the box");
        assert_eq!(outer.radius, 8.0, "radius from the box");
        assert_eq!(outer.blur, 12.0);
        assert_eq!(outer.offset, Vec2::new(0.0, 4.0));
        assert!(matches!(items[1], RenderItem::Quad(_)));
        let RenderItem::Shadow(inner) = items[2] else {
            panic!("inner shadow last")
        };
        assert!(inner.inset);
        assert_eq!(inner.blur, 6.0);
    }

    #[test]
    fn shadow_alone_is_not_empty() {
        let style = BoxStyle::new().shadow(ShadowSpec::new(Color::BLACK, 4.0));
        assert!(!style.is_empty(), "shadow alone is not an empty style");
        let mut list = RenderList::new();
        style.paint(&mut list, Rect::from_xywh(0.0, 0.0, 10.0, 10.0));
        assert_eq!(list.shadows().count(), 1);

        assert!(
            BoxStyle::new()
                .shadow(ShadowSpec::new(Color::TRANSPARENT, 4.0))
                .is_empty()
        );
        assert!(
            BoxStyle::new()
                .inset_shadow(ShadowSpec::new(Color::BLACK, 0.0))
                .is_empty()
        );
    }

    #[test]
    fn empty_paints_nothing() {
        let mut list = RenderList::new();
        BoxStyle::new().paint(&mut list, Rect::from_xywh(0.0, 0.0, 100.0, 50.0));
        assert!(list.is_empty());
    }

    #[test]
    fn background_only_is_single_quad() {
        let mut list = RenderList::new();
        BoxStyle::new()
            .background(Color::RED)
            .radius(6.0)
            .paint(&mut list, Rect::from_xywh(0.0, 0.0, 100.0, 50.0));
        let qs: Vec<_> = list.quads().collect();
        assert_eq!(qs.len(), 1);
        assert_eq!(qs[0].color, Color::RED);
        assert_eq!(qs[0].radius, 6.0);
        assert!(!qs[0].has_border());
    }

    #[test]
    fn uniform_border_is_single_bordered_quad() {
        let mut list = RenderList::new();
        BoxStyle::new()
            .background(Color::BLUE)
            .border(2.0, Color::WHITE)
            .paint(&mut list, Rect::from_xywh(0.0, 0.0, 100.0, 60.0));
        let qs: Vec<_> = list.quads().collect();
        assert_eq!(qs.len(), 1, "background + frame is one quad");
        assert_eq!(qs[0].border_width, 2.0);
        assert_eq!(qs[0].border_color, Color::WHITE);
    }

    #[test]
    fn left_border_is_background_plus_stripe() {
        let mut list = RenderList::new();
        BoxStyle::new()
            .border_left(8.5, Color::hex(0xCACACA))
            .paint(&mut list, Rect::from_xywh(0.0, 0.0, 100.0, 40.0));
        let qs: Vec<_> = list.quads().collect();
        assert_eq!(qs.len(), 1, "no background, bar only");
        assert_eq!(qs[0].rect, Rect::from_xywh(0.0, 0.0, 8.5, 40.0));
        assert_eq!(qs[0].color, Color::hex(0xCACACA));
    }

    #[test]
    fn background_plus_left_border_is_two_quads() {
        let mut list = RenderList::new();
        BoxStyle::new()
            .background(Color::WHITE)
            .border_left(8.0, Color::RED)
            .paint(&mut list, Rect::from_xywh(10.0, 5.0, 100.0, 40.0));
        let qs: Vec<_> = list.quads().collect();
        assert_eq!(qs.len(), 2);
        assert_eq!(qs[0].rect, Rect::from_xywh(10.0, 5.0, 100.0, 40.0));
        assert_eq!(qs[1].rect, Rect::from_xywh(10.0, 5.0, 8.0, 40.0));
    }

    #[test]
    fn top_bottom_fit_between_sides() {
        let mut list = RenderList::new();
        BoxStyle::new()
            .border_left(10.0, Color::RED)
            .border_right(10.0, Color::GREEN)
            .border_top(5.0, Color::BLUE)
            .paint(&mut list, Rect::from_xywh(0.0, 0.0, 100.0, 50.0));
        let qs: Vec<_> = list.quads().collect();
        assert_eq!(qs.len(), 3);

        assert_eq!(qs[2].rect, Rect::from_xywh(10.0, 0.0, 80.0, 5.0));
    }

    #[test]
    fn uniform_detection() {
        let u = BoxStyle::new().border(2.0, Color::WHITE);
        assert_eq!(u.uniform_border(), Some((2.0, Color::WHITE)));
        let mixed = BoxStyle::new()
            .border_left(2.0, Color::WHITE)
            .border_right(3.0, Color::WHITE);
        assert_eq!(mixed.uniform_border(), None);
        let tinted = BoxStyle::new()
            .border_left(2.0, Color::WHITE)
            .border_right(2.0, Color::RED);
        assert_eq!(tinted.uniform_border(), None);
    }
}
