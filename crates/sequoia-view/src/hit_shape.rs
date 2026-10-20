use sequoia_core::{BrushStroke, Color, Fill, Point, Rect};

use super::painter::Painter;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HitCircle {
    pub center: Point,
    pub radius: f32,
}

impl HitCircle {
    pub fn new(center: Point, radius: f32) -> Self {
        Self { center, radius }
    }

    pub fn paint(&self, g: &mut Painter, fill: Fill) -> Rect {
        g.circle(self.center, self.radius, fill)
    }

    pub fn outline(&self, g: &mut Painter, stroke: BrushStroke, color: Color) -> Rect {
        g.ring(self.center, self.radius, stroke.width.max(0.0), color)
    }

    pub fn contains(&self, p: Point, tol: f32) -> bool {
        if !(self.radius > 0.0) {
            return false;
        }
        self.center.distance(p) <= self.radius + tol.max(0.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HitRect {
    pub rect: Rect,
    pub radius: f32,
}

impl HitRect {
    pub fn new(rect: Rect, radius: f32) -> Self {
        Self { rect, radius }
    }

    pub fn paint(&self, g: &mut Painter, fill: Fill) -> Rect {
        g.rounded(self.rect, self.radius, fill)
    }

    pub fn outline(&self, g: &mut Painter, stroke: BrushStroke, color: Color) -> Rect {
        g.rect_outline(self.rect, self.radius, stroke, color)
    }

    pub fn contains(&self, p: Point, tol: f32) -> bool {
        if self.rect.is_empty() {
            return false;
        }
        let t = tol.max(0.0);
        self.rect.inflate(t, t).contains(p)
    }
}
