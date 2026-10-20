#![forbid(unsafe_code)]

use sequoia_core::{Point, Rect};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Orientation {
    #[default]
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Track1D {
    start: f32,
    len: f32,
    pad: f32,
}

impl Track1D {
    pub fn new(start: f32, len: f32, pad: f32) -> Self {
        Self {
            start,
            len: len.max(0.0),
            pad: pad.max(0.0),
        }
    }

    pub fn horizontal(rect: Rect, pad: f32) -> Self {
        Self::new(rect.left(), rect.size.width, pad)
    }

    pub fn vertical(rect: Rect, pad: f32) -> Self {
        Self::new(rect.top(), rect.size.height, pad)
    }

    fn inner(&self) -> f32 {
        (self.len - self.pad * 2.0).max(1.0)
    }

    pub fn t_of(&self, pos: f32) -> f32 {
        if !pos.is_finite() {
            return 0.0;
        }
        ((pos - (self.start + self.pad)) / self.inner()).clamp(0.0, 1.0)
    }

    pub fn pos_of(&self, t: f32) -> f32 {
        let t = if t.is_finite() {
            t.clamp(0.0, 1.0)
        } else {
            0.0
        };
        self.start + self.pad + t * self.inner()
    }

    pub fn value_of(&self, pos: f32, min: f32, max: f32) -> f32 {
        let (lo, hi) = if max >= min { (min, max) } else { (max, min) };
        lo + self.t_of(pos) * (hi - lo).max(0.0)
    }

    pub fn pos_of_value(&self, v: f32, min: f32, max: f32) -> f32 {
        let span = (max - min).abs().max(f32::MIN_POSITIVE);
        self.pos_of((v - min.min(max)) / span)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pad2D {
    rect: Rect,
    pad: f32,
}

impl Pad2D {
    pub fn new(rect: Rect, pad: f32) -> Self {
        Self {
            rect,
            pad: pad.max(0.0),
        }
    }

    fn track_x(&self) -> Track1D {
        Track1D::horizontal(self.rect, self.pad)
    }

    fn track_y(&self) -> Track1D {
        Track1D::vertical(self.rect, self.pad)
    }

    pub fn sv_of(&self, p: Point) -> (f32, f32) {
        let s = self.track_x().t_of(p.x);
        let v = 1.0 - self.track_y().t_of(p.y);
        (s, v)
    }

    pub fn pos_of(&self, s: f32, v: f32) -> Point {
        Point::new(self.track_x().pos_of(s), self.track_y().pos_of(1.0 - v))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect::from_xywh(x, y, w, h)
    }

    #[test]
    fn track_roundtrips_with_knob_pad() {
        let t = Track1D::horizontal(r(0.0, 0.0, 200.0, 26.0), 7.0);
        assert!((t.pos_of(0.0) - 7.0).abs() < 1e-4);
        assert!((t.pos_of(1.0) - 193.0).abs() < 1e-4);
        assert!((t.t_of(7.0)).abs() < 1e-4);
        assert!((t.t_of(193.0) - 1.0).abs() < 1e-4);

        assert_eq!(t.t_of(0.0), 0.0);
        assert_eq!(t.t_of(200.0), 1.0);
        assert!((t.pos_of(0.5) - 100.0).abs() < 1e-4);
        assert!((t.t_of(100.0) - 0.5).abs() < 1e-4);
    }

    #[test]
    fn track_without_pad_is_full_bleed() {
        let t = Track1D::horizontal(r(10.0, 0.0, 200.0, 26.0), 0.0);
        assert!((t.pos_of(0.0) - 10.0).abs() < 1e-4);
        assert!((t.pos_of(1.0) - 210.0).abs() < 1e-4);
        assert!((t.t_of(110.0) - 0.5).abs() < 1e-4);
    }

    #[test]
    fn track_maps_values_and_survives_garbage() {
        let t = Track1D::horizontal(r(0.0, 0.0, 200.0, 26.0), 7.0);
        assert!((t.value_of(7.0, 0.0, 360.0)).abs() < 1e-3);
        assert!((t.value_of(193.0, 0.0, 360.0) - 360.0).abs() < 1e-3);
        assert!((t.pos_of_value(180.0, 0.0, 360.0) - t.pos_of(0.5)).abs() < 1e-4);
        assert_eq!(t.t_of(f32::NAN), 0.0);
        assert_eq!(t.pos_of(f32::NAN), 7.0);

        let flat = Track1D::horizontal(r(0.0, 0.0, 0.0, 0.0), 7.0);
        assert!(flat.pos_of(0.5).is_finite());
        assert!(flat.t_of(0.0).is_finite());
    }

    #[test]
    fn pad2d_roundtrips_sv() {
        let pad = Pad2D::new(r(0.0, 0.0, 200.0, 200.0), 7.0);
        let c = pad.pos_of(0.5, 0.5);
        assert!((c.x - 100.0).abs() < 1e-4 && (c.y - 100.0).abs() < 1e-4);
        let (s, v) = pad.sv_of(c);
        assert!((s - 0.5).abs() < 1e-4 && (v - 0.5).abs() < 1e-4);

        assert_eq!(pad.sv_of(Point::new(0.0, 0.0)), (0.0, 1.0));
        assert_eq!(pad.sv_of(Point::new(200.0, 200.0)), (1.0, 0.0));
    }
}
