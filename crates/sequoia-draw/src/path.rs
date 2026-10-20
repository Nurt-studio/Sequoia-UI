use sequoia_core::{Point, Rect};

use crate::Transform;
use crate::flatten::{Contour, flatten};
use crate::raster;
use crate::svg_path::parse_svg_path;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum FillRule {
    #[default]
    NonZero,

    EvenOdd,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PathError {
    pub position: usize,

    pub message: &'static str,
}

impl std::fmt::Display for PathError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (position {})", self.message, self.position)
    }
}

impl std::error::Error for PathError {}

#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum PathSeg {
    Move(Point),

    Line(Point),

    Quad(Point, Point),

    Cubic(Point, Point, Point),

    Close,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Radius {
    pub tl: f32,

    pub tr: f32,

    pub br: f32,

    pub bl: f32,
}

impl Radius {
    pub const ZERO: Self = Self {
        tl: 0.0,
        tr: 0.0,
        br: 0.0,
        bl: 0.0,
    };

    pub const fn all(r: f32) -> Self {
        Self {
            tl: r,
            tr: r,
            br: r,
            bl: r,
        }
    }

    pub const fn new(tl: f32, tr: f32, br: f32, bl: f32) -> Self {
        Self { tl, tr, br, bl }
    }

    pub fn uniform(&self) -> Option<f32> {
        if self.tl > 0.0 && self.tl == self.tr && self.tr == self.br && self.br == self.bl {
            Some(self.tl)
        } else {
            None
        }
    }
}

impl From<f32> for Radius {
    fn from(r: f32) -> Self {
        Self::all(r)
    }
}

impl From<[f32; 4]> for Radius {
    fn from([tl, tr, br, bl]: [f32; 4]) -> Self {
        Self { tl, tr, br, bl }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum Hint {
    #[default]
    None,
    Rect(Rect),
    RoundRect(Rect, f32),
    Circle(Point, f32),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Path {
    segs: Vec<PathSeg>,
    current: Point,
    start: Point,
    hint: Hint,
}

impl Path {
    pub fn new() -> Self {
        Self {
            segs: Vec::new(),
            current: Point::ZERO,
            start: Point::ZERO,
            hint: Hint::None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.segs.is_empty()
    }

    pub fn segs(&self) -> &[PathSeg] {
        &self.segs
    }

    fn push(&mut self, seg: PathSeg) -> &mut Self {
        self.hint = Hint::None;
        self.segs.push(seg);
        self
    }

    pub fn move_to(&mut self, p: impl Into<Point>) -> &mut Self {
        let p = p.into();
        self.current = p;
        self.start = p;
        self.push(PathSeg::Move(p))
    }

    pub fn line_to(&mut self, p: impl Into<Point>) -> &mut Self {
        let p = p.into();
        self.current = p;
        self.push(PathSeg::Line(p))
    }

    pub fn quad_to(&mut self, c: impl Into<Point>, p: impl Into<Point>) -> &mut Self {
        let (c, p) = (c.into(), p.into());
        self.current = p;
        self.push(PathSeg::Quad(c, p))
    }

    pub fn cubic_to(
        &mut self,
        c1: impl Into<Point>,
        c2: impl Into<Point>,
        p: impl Into<Point>,
    ) -> &mut Self {
        let (c1, c2, p) = (c1.into(), c2.into(), p.into());
        self.current = p;
        self.push(PathSeg::Cubic(c1, c2, p))
    }

    pub fn close(&mut self) -> &mut Self {
        self.current = self.start;
        self.push(PathSeg::Close)
    }

    pub fn polyline(&mut self, points: &[Point]) -> &mut Self {
        let Some((first, rest)) = points.split_first() else {
            return self;
        };
        self.move_to(*first);
        for p in rest {
            self.line_to(*p);
        }
        self
    }

    pub fn spline(&mut self, points: &[Point], tension: f32) -> &mut Self {
        let n = points.len();
        if n < 2 {
            return self;
        }
        let tension = if tension.is_finite() { tension } else { 1.0 };
        self.move_to(points[0]);
        for i in 0..n - 1 {
            let p0 = if i == 0 { points[0] } else { points[i - 1] };
            let p1 = points[i];
            let p2 = points[i + 1];
            let p3 = if i + 2 < n {
                points[i + 2]
            } else {
                points[n - 1]
            };
            let c1 = Point::new(
                p1.x + (p2.x - p0.x) / 6.0 * tension,
                p1.y + (p2.y - p0.y) / 6.0 * tension,
            );
            let c2 = Point::new(
                p2.x - (p3.x - p1.x) / 6.0 * tension,
                p2.y - (p3.y - p1.y) / 6.0 * tension,
            );
            self.cubic_to(c1, c2, p2);
        }
        self
    }

    pub fn polygon(&mut self, points: &[Point]) -> &mut Self {
        self.polyline(points);
        if !points.is_empty() {
            self.close();
        }
        self
    }

    fn is_first_contour(&self) -> bool {
        !self.segs.iter().any(|seg| matches!(seg, PathSeg::Move(_)))
    }

    pub fn rect(&mut self, rect: Rect) -> &mut Self {
        let single = self.is_first_contour();
        let (x0, y0) = (rect.left(), rect.top());
        let (x1, y1) = (rect.right(), rect.bottom());
        self.move_to((x0, y0));
        self.line_to((x1, y0));
        self.line_to((x1, y1));
        self.line_to((x0, y1));
        self.close();
        if single {
            self.hint = Hint::Rect(rect);
        }
        self
    }

    pub fn rounded_rect(&mut self, rect: Rect, radius: impl Into<Radius>) -> &mut Self {
        let single = self.is_first_contour();
        let radius = radius.into();
        let max = (rect.size.width.min(rect.size.height) * 0.5).max(0.0);
        let r = Radius {
            tl: radius.tl.clamp(0.0, max),
            tr: radius.tr.clamp(0.0, max),
            br: radius.br.clamp(0.0, max),
            bl: radius.bl.clamp(0.0, max),
        };
        let (x0, y0) = (rect.left(), rect.top());
        let (x1, y1) = (rect.right(), rect.bottom());

        self.move_to((x0 + r.tl, y0));
        self.line_to((x1 - r.tr, y0));
        if r.tr > 0.0 {
            self.arc_corner((x1 - r.tr, y0 + r.tr), r.tr, -90.0, 0.0);
        }
        self.line_to((x1, y1 - r.br));
        if r.br > 0.0 {
            self.arc_corner((x1 - r.br, y1 - r.br), r.br, 0.0, 90.0);
        }
        self.line_to((x0 + r.bl, y1));
        if r.bl > 0.0 {
            self.arc_corner((x0 + r.bl, y1 - r.bl), r.bl, 90.0, 180.0);
        }
        self.line_to((x0, y0 + r.tl));
        if r.tl > 0.0 {
            self.arc_corner((x0 + r.tl, y0 + r.tl), r.tl, 180.0, 270.0);
        }
        self.close();
        if let Some(uniform) = r.uniform()
            && single
        {
            self.hint = Hint::RoundRect(rect, uniform);
        }
        self
    }

    pub fn circle(&mut self, center: impl Into<Point>, radius: f32) -> &mut Self {
        let single = self.is_first_contour();
        let center = center.into();
        if radius <= 0.0 {
            return self;
        }
        self.ellipse(center, radius, radius);
        if single {
            self.hint = Hint::Circle(center, radius);
        }
        self
    }

    pub fn ellipse(&mut self, center: impl Into<Point>, rx: f32, ry: f32) -> &mut Self {
        let c = center.into();
        if rx <= 0.0 || ry <= 0.0 {
            return self;
        }
        let k = 0.552_284_8;
        let (kx, ky) = (rx * k, ry * k);
        self.move_to((c.x + rx, c.y));
        self.cubic_to((c.x + rx, c.y + ky), (c.x + kx, c.y + ry), (c.x, c.y + ry));
        self.cubic_to((c.x - kx, c.y + ry), (c.x - rx, c.y + ky), (c.x - rx, c.y));
        self.cubic_to((c.x - rx, c.y - ky), (c.x - kx, c.y - ry), (c.x, c.y - ry));
        self.cubic_to((c.x + kx, c.y - ry), (c.x + rx, c.y - ky), (c.x + rx, c.y));
        self.close();
        self
    }

    pub fn arc(
        &mut self,
        center: impl Into<Point>,
        radius: f32,
        from_deg: f32,
        to_deg: f32,
    ) -> &mut Self {
        let c = center.into();
        if radius <= 0.0 {
            return self;
        }
        let from = from_deg.to_radians();
        let start = Point::new(c.x + from.cos() * radius, c.y + from.sin() * radius);
        if self.segs.is_empty() {
            self.move_to(start);
        } else if self.current != start {
            self.line_to(start);
        }
        self.arc_corner(c, radius, from_deg, to_deg);
        self
    }

    fn arc_corner(
        &mut self,
        center: impl Into<Point>,
        radius: f32,
        from_deg: f32,
        to_deg: f32,
    ) -> &mut Self {
        let center = center.into();
        let from = from_deg.to_radians();
        let to = to_deg.to_radians();
        let span = to - from;
        if span.abs() < 1e-6 {
            return self;
        }
        let steps = (span.abs() / (std::f32::consts::FRAC_PI_2)).ceil().max(1.0) as usize;
        let step = span / steps as f32;
        let k = 4.0 / 3.0 * (step / 4.0).tan();
        let mut a = from;
        let mut p = Point::new(center.x + a.cos() * radius, center.y + a.sin() * radius);
        for _ in 0..steps {
            let b = a + step;
            let q = Point::new(center.x + b.cos() * radius, center.y + b.sin() * radius);
            let c1 = Point::new(p.x - a.sin() * radius * k, p.y + a.cos() * radius * k);
            let c2 = Point::new(q.x + b.sin() * radius * k, q.y - b.cos() * radius * k);
            self.cubic_to(c1, c2, q);
            a = b;
            p = q;
        }
        self
    }

    pub fn from_svg(data: &str) -> Result<Self, PathError> {
        parse_svg_path(data)
    }

    pub fn as_rect(&self) -> Option<Rect> {
        match self.hint {
            Hint::Rect(r) => Some(r),
            _ => None,
        }
    }

    pub fn as_rounded_rect(&self) -> Option<(Rect, f32)> {
        match self.hint {
            Hint::RoundRect(r, radius) => Some((r, radius)),
            _ => None,
        }
    }

    pub fn as_circle(&self) -> Option<(Point, f32)> {
        match self.hint {
            Hint::Circle(c, r) => Some((c, r)),
            _ => None,
        }
    }

    pub fn to_svg(&self) -> String {
        crate::svg_path::segs_to_debug(self)
    }

    pub fn contours(&self, tolerance: f32) -> Vec<Contour> {
        flatten(self, tolerance)
    }

    pub fn bounds(&self) -> Rect {
        bounds_of_contours(&self.contours(crate::TOLERANCE))
    }

    pub fn bounds_with(&self, tolerance: f32) -> Rect {
        bounds_of_contours(&self.contours(tolerance))
    }

    pub fn contains(&self, p: Point, rule: FillRule) -> bool {
        let contours = self.contours(crate::TOLERANCE);
        raster::contains(&contours, p, rule)
    }

    pub fn transformed(&self, t: Transform) -> Self {
        let mut out = Self {
            segs: Vec::with_capacity(self.segs.len()),
            current: t.apply(self.current),
            start: t.apply(self.start),
            hint: Hint::None,
        };
        out.segs = self
            .segs
            .iter()
            .map(|seg| match *seg {
                PathSeg::Move(p) => PathSeg::Move(t.apply(p)),
                PathSeg::Line(p) => PathSeg::Line(t.apply(p)),
                PathSeg::Quad(c, p) => PathSeg::Quad(t.apply(c), t.apply(p)),
                PathSeg::Cubic(c1, c2, p) => PathSeg::Cubic(t.apply(c1), t.apply(c2), t.apply(p)),
                PathSeg::Close => PathSeg::Close,
            })
            .collect();
        out.hint = match self.hint {
            Hint::Rect(r) if t.m[1] == 0.0 && t.m[2] == 0.0 => Hint::Rect(transform_rect(r, t)),
            Hint::Circle(c, r) => match t.similarity_scale() {
                Some(s) => Hint::Circle(t.apply(c), r * s),
                None => Hint::None,
            },
            Hint::RoundRect(r, radius)
                if t.m[1] == 0.0 && t.m[2] == 0.0 && (t.m[0].abs() - t.m[3].abs()).abs() < 1e-6 =>
            {
                Hint::RoundRect(transform_rect(r, t), radius * t.m[0].abs())
            }
            _ => Hint::None,
        };
        out
    }

    pub fn translated(&self, dx: f32, dy: f32) -> Self {
        self.transformed(Transform::translate(dx, dy))
    }

    pub fn scaled(&self, sx: f32, sy: f32) -> Self {
        self.transformed(Transform::scale(sx, sy))
    }

    pub fn rotated(&self, radians: f32, pivot: Point) -> Self {
        self.transformed(Transform::rotate_around(radians, pivot))
    }

    pub(crate) fn fingerprint(&self) -> u64 {
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        hash ^= match self.hint {
            Hint::None => 7,
            Hint::Rect(_) => 8,
            Hint::RoundRect(_, _) => 9,
            Hint::Circle(_, _) => 10,
        };
        for seg in &self.segs {
            match *seg {
                PathSeg::Move(p) => {
                    hash ^= 1;
                    hash_point(&mut hash, p);
                }
                PathSeg::Line(p) => {
                    hash ^= 2;
                    hash_point(&mut hash, p);
                }
                PathSeg::Quad(c, p) => {
                    hash ^= 3;
                    hash_point(&mut hash, c);
                    hash_point(&mut hash, p);
                }
                PathSeg::Cubic(c1, c2, p) => {
                    hash ^= 4;
                    hash_point(&mut hash, c1);
                    hash_point(&mut hash, c2);
                    hash_point(&mut hash, p);
                }
                PathSeg::Close => hash ^= 5,
            }
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        hash
    }
}

impl std::fmt::Display for Path {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_svg())
    }
}

fn hash_point(hash: &mut u64, p: Point) {
    for v in [p.x, p.y] {
        for byte in v.to_bits().to_le_bytes() {
            *hash ^= u64::from(byte);
            *hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
}

fn bounds_of_contours(contours: &[Contour]) -> Rect {
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for c in contours {
        for p in &c.points {
            min_x = min_x.min(p.x);
            min_y = min_y.min(p.y);
            max_x = max_x.max(p.x);
            max_y = max_y.max(p.y);
        }
    }
    if !min_x.is_finite() {
        return Rect::ZERO;
    }
    Rect::from_xywh(min_x, min_y, max_x - min_x, max_y - min_y)
}

fn transform_rect(r: Rect, t: Transform) -> Rect {
    let a = t.apply(Point::new(r.left(), r.top()));
    let b = t.apply(Point::new(r.right(), r.bottom()));
    Rect::from_xywh(
        a.x.min(b.x),
        a.y.min(b.y),
        (b.x - a.x).abs(),
        (b.y - a.y).abs(),
    )
}
