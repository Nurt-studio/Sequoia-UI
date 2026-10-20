use std::ops::{Add, AddAssign, Mul, Neg, Sub, SubAssign};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    pub x: f32,

    pub y: f32,
}

impl Point {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };

    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn distance(self, other: Self) -> f32 {
        (other - self).length()
    }
}

impl From<(f32, f32)> for Point {
    fn from((x, y): (f32, f32)) -> Self {
        Self { x, y }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec2 {
    pub x: f32,

    pub y: f32,
}

impl Vec2 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };

    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn length_squared(self) -> f32 {
        self.x * self.x + self.y * self.y
    }

    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    pub fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y
    }

    pub fn normalized(self) -> Self {
        let len = self.length();
        if len > 0.0 {
            Self::new(self.x / len, self.y / len)
        } else {
            Self::ZERO
        }
    }
}

impl From<Point> for Vec2 {
    fn from(p: Point) -> Self {
        Self { x: p.x, y: p.y }
    }
}

impl From<Vec2> for Point {
    fn from(v: Vec2) -> Self {
        Self { x: v.x, y: v.y }
    }
}

impl Add for Vec2 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl Sub for Vec2 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl Neg for Vec2 {
    type Output = Self;
    fn neg(self) -> Self {
        Self::new(-self.x, -self.y)
    }
}

impl Mul<f32> for Vec2 {
    type Output = Self;
    fn mul(self, s: f32) -> Self {
        Self::new(self.x * s, self.y * s)
    }
}

impl AddAssign for Vec2 {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl SubAssign for Vec2 {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Add<Vec2> for Point {
    type Output = Self;
    fn add(self, rhs: Vec2) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl Sub<Vec2> for Point {
    type Output = Self;
    fn sub(self, rhs: Vec2) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl Sub for Point {
    type Output = Vec2;
    fn sub(self, rhs: Self) -> Vec2 {
        Vec2::new(self.x - rhs.x, self.y - rhs.y)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Size {
    pub width: f32,

    pub height: f32,
}

impl Size {
    pub const ZERO: Self = Self {
        width: 0.0,
        height: 0.0,
    };

    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }

    pub fn is_empty(self) -> bool {
        self.width <= 0.0 || self.height <= 0.0
    }
}

impl From<(f32, f32)> for Size {
    fn from((w, h): (f32, f32)) -> Self {
        Self {
            width: w,
            height: h,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub origin: Point,

    pub size: Size,
}

impl Rect {
    pub const ZERO: Self = Self {
        origin: Point::ZERO,
        size: Size::ZERO,
    };

    pub const fn new(origin: Point, size: Size) -> Self {
        Self { origin, size }
    }

    pub const fn from_xywh(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self::new(Point::new(x, y), Size::new(width, height))
    }

    pub fn left(self) -> f32 {
        self.origin.x
    }

    pub fn right(self) -> f32 {
        self.origin.x + self.size.width
    }

    pub fn top(self) -> f32 {
        self.origin.y
    }

    pub fn bottom(self) -> f32 {
        self.origin.y + self.size.height
    }

    pub fn is_empty(self) -> bool {
        self.size.is_empty()
    }

    pub fn contains(self, p: Point) -> bool {
        p.x >= self.left() && p.x < self.right() && p.y >= self.top() && p.y < self.bottom()
    }

    pub fn contains_rect(self, other: Rect) -> bool {
        !other.is_empty()
            && other.left() >= self.left()
            && other.top() >= self.top()
            && other.right() <= self.right()
            && other.bottom() <= self.bottom()
    }

    pub fn intersects(self, other: Rect) -> bool {
        !self.intersect(other).is_empty()
    }

    pub fn intersect(self, other: Rect) -> Rect {
        let x0 = self.left().max(other.left());
        let y0 = self.top().max(other.top());
        let x1 = self.right().min(other.right());
        let y1 = self.bottom().min(other.bottom());
        Rect::from_xywh(x0, y0, (x1 - x0).max(0.0), (y1 - y0).max(0.0))
    }

    pub fn union(self, other: Rect) -> Rect {
        if self.is_empty() {
            return other;
        }
        if other.is_empty() {
            return self;
        }
        let x0 = self.left().min(other.left());
        let y0 = self.top().min(other.top());
        let x1 = self.right().max(other.right());
        let y1 = self.bottom().max(other.bottom());
        Rect::from_xywh(x0, y0, x1 - x0, y1 - y0)
    }

    pub fn inflate(self, dx: f32, dy: f32) -> Rect {
        Rect::from_xywh(
            self.left() - dx,
            self.top() - dy,
            self.size.width + dx * 2.0,
            self.size.height + dy * 2.0,
        )
    }

    pub fn translate(self, v: Vec2) -> Rect {
        Rect::new(self.origin + v, self.size)
    }

    pub fn center(self) -> Point {
        Point::new(
            self.origin.x + self.size.width * 0.5,
            self.origin.y + self.size.height * 0.5,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    pub m: [f32; 6],
}

impl Default for Transform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Transform {
    pub const IDENTITY: Self = Self {
        m: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
    };

    pub const fn translate(x: f32, y: f32) -> Self {
        Self {
            m: [1.0, 0.0, 0.0, 1.0, x, y],
        }
    }

    pub const fn scale(sx: f32, sy: f32) -> Self {
        Self {
            m: [sx, 0.0, 0.0, sy, 0.0, 0.0],
        }
    }

    pub const fn uniform(s: f32) -> Self {
        Self::scale(s, s)
    }

    pub fn rotate(radians: f32) -> Self {
        let (s, c) = radians.sin_cos();
        Self {
            m: [c, s, -s, c, 0.0, 0.0],
        }
    }

    pub fn rotate_degrees(degrees: f32) -> Self {
        Self::rotate(degrees.to_radians())
    }

    pub fn rotate_around(radians: f32, pivot: Point) -> Self {
        Self::translate(-pivot.x, -pivot.y)
            .then(Self::rotate(radians))
            .then(Self::translate(pivot.x, pivot.y))
    }

    pub fn rotate_around_degrees(degrees: f32, pivot: Point) -> Self {
        Self::rotate_around(degrees.to_radians(), pivot)
    }

    pub fn then(self, next: Self) -> Self {
        let [a1, b1, c1, d1, e1, f1] = self.m;
        let [a2, b2, c2, d2, e2, f2] = next.m;
        Self {
            m: [
                a1 * a2 + b1 * c2,
                a1 * b2 + b1 * d2,
                c1 * a2 + d1 * c2,
                c1 * b2 + d1 * d2,
                e1 * a2 + f1 * c2 + e2,
                e1 * b2 + f1 * d2 + f2,
            ],
        }
    }

    pub fn apply(self, p: Point) -> Point {
        let [a, b, c, d, e, f] = self.m;
        Point::new(a * p.x + c * p.y + e, b * p.x + d * p.y + f)
    }

    pub fn apply_vec(self, v: Vec2) -> Vec2 {
        let [a, b, c, d, ..] = self.m;
        Vec2::new(a * v.x + c * v.y, b * v.x + d * v.y)
    }

    pub fn apply_rect_aabb(self, r: Rect) -> Rect {
        if self.is_identity() {
            return r;
        }
        let p0 = self.apply(r.origin);
        let p1 = self.apply(Point::new(r.right(), r.top()));
        let p2 = self.apply(Point::new(r.right(), r.bottom()));
        let p3 = self.apply(Point::new(r.left(), r.bottom()));
        let x0 = p0.x.min(p1.x).min(p2.x).min(p3.x);
        let y0 = p0.y.min(p1.y).min(p2.y).min(p3.y);
        let x1 = p0.x.max(p1.x).max(p2.x).max(p3.x);
        let y1 = p0.y.max(p1.y).max(p2.y).max(p3.y);
        Rect::from_xywh(x0, y0, (x1 - x0).max(0.0), (y1 - y0).max(0.0))
    }

    pub fn is_identity(self) -> bool {
        let [a, b, c, d, e, f] = self.m;
        a == 1.0 && b == 0.0 && c == 0.0 && d == 1.0 && e == 0.0 && f == 0.0
    }

    pub fn determinant(self) -> f32 {
        let [a, b, c, d, ..] = self.m;
        a * d - b * c
    }

    pub fn max_scale(self) -> f32 {
        let [a, b, c, d, ..] = self.m;
        let sx = (a * a + b * b).sqrt();
        let sy = (c * c + d * d).sqrt();
        sx.max(sy).max(f32::MIN_POSITIVE)
    }

    pub fn similarity_scale(self) -> Option<f32> {
        let [a, b, c, d, ..] = self.m;
        let col1 = a * a + b * b;
        let col2 = c * c + d * d;
        let dot = a * c + b * d;
        let scale_hint = (col1 + col2).sqrt();
        if col1 <= 0.0 || col2 <= 0.0 {
            return None;
        }
        if dot.abs() > 1e-4 * scale_hint || (col1 - col2).abs() > 1e-4 * scale_hint {
            return None;
        }
        Some(col1.sqrt())
    }

    pub fn inverse(self) -> Option<Self> {
        let det = self.determinant();
        if det.abs() < f32::EPSILON {
            return None;
        }
        let [a, b, c, d, e, f] = self.m;
        let inv = 1.0 / det;
        Some(Self {
            m: [
                d * inv,
                -b * inv,
                -c * inv,
                a * inv,
                (c * f - d * e) * inv,
                (b * e - a * f) * inv,
            ],
        })
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct Edges {
    pub left: f32,

    pub top: f32,

    pub right: f32,

    pub bottom: f32,
}

impl Edges {
    pub const ZERO: Self = Self {
        left: 0.0,
        top: 0.0,
        right: 0.0,
        bottom: 0.0,
    };

    pub const fn uniform(v: f32) -> Self {
        Self {
            left: v,
            top: v,
            right: v,
            bottom: v,
        }
    }

    pub const fn horizontal(v: f32) -> Self {
        Self {
            left: v,
            top: 0.0,
            right: v,
            bottom: 0.0,
        }
    }

    pub const fn vertical(v: f32) -> Self {
        Self {
            left: 0.0,
            top: v,
            right: 0.0,
            bottom: v,
        }
    }

    pub const fn all(v: f32) -> Self {
        Self::uniform(v)
    }

    pub const fn symmetric(horizontal: f32, vertical: f32) -> Self {
        Self {
            left: horizontal,
            top: vertical,
            right: horizontal,
            bottom: vertical,
        }
    }

    pub const fn xy(x: f32, y: f32) -> Self {
        Self::symmetric(x, y)
    }

    pub const fn only(left: f32, top: f32, right: f32, bottom: f32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    pub const fn left(v: f32) -> Self {
        Self {
            left: v,
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
        }
    }

    pub const fn top(v: f32) -> Self {
        Self {
            left: 0.0,
            top: v,
            right: 0.0,
            bottom: 0.0,
        }
    }

    pub const fn right(v: f32) -> Self {
        Self {
            left: 0.0,
            top: 0.0,
            right: v,
            bottom: 0.0,
        }
    }

    pub const fn bottom(v: f32) -> Self {
        Self {
            left: 0.0,
            top: 0.0,
            right: 0.0,
            bottom: v,
        }
    }

    pub fn with_left(mut self, v: f32) -> Self {
        self.left = v;
        self
    }

    pub fn with_top(mut self, v: f32) -> Self {
        self.top = v;
        self
    }

    pub fn with_right(mut self, v: f32) -> Self {
        self.right = v;
        self
    }

    pub fn with_bottom(mut self, v: f32) -> Self {
        self.bottom = v;
        self
    }

    pub fn with_horizontal(mut self, v: f32) -> Self {
        self.left = v;
        self.right = v;
        self
    }

    pub fn with_vertical(mut self, v: f32) -> Self {
        self.top = v;
        self.bottom = v;
        self
    }

    pub fn is_zero(self) -> bool {
        self.left == 0.0 && self.top == 0.0 && self.right == 0.0 && self.bottom == 0.0
    }

    pub fn horizontal_sum(self) -> f32 {
        self.left + self.right
    }

    pub fn vertical_sum(self) -> f32 {
        self.top + self.bottom
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Length {
    Px(f32),

    Percent(f32),

    Fill(f32),

    Fit,
}

impl Length {
    pub const FIT: Self = Self::Fit;

    pub fn px(v: f32) -> Self {
        Self::Px(v.max(0.0))
    }

    pub fn percent(p: f32) -> Self {
        Self::Percent(p)
    }

    pub fn fill(weight: f32) -> Self {
        Self::Fill(weight)
    }

    pub fn fit() -> Self {
        Self::Fit
    }

    pub fn resolve(self, available: f32) -> Option<f32> {
        match self {
            Self::Px(v) => Some(v),
            Self::Percent(p) => Some(available * p / 100.0),
            Self::Fill(_) | Self::Fit => None,
        }
    }

    pub fn is_fill(self) -> bool {
        matches!(self, Self::Fill(_))
    }

    pub fn is_fit(self) -> bool {
        matches!(self, Self::Fit)
    }
}

impl From<f32> for Length {
    fn from(v: f32) -> Self {
        Self::px(v)
    }
}

pub trait LengthExt {
    fn px(self) -> Length;

    fn percent(self) -> Length;

    fn fill(self) -> Length;
}

macro_rules! impl_length_ext {
    ($($t:ty),* $(,)?) => {
        $(
            impl LengthExt for $t {
                fn px(self) -> Length { Length::px(self as f32) }
                fn percent(self) -> Length { Length::percent(self as f32) }
                fn fill(self) -> Length { Length::fill(self as f32) }
            }
        )*
    };
}

impl_length_ext!(f32, i32, u16, u32);

impl From<f32> for Edges {
    fn from(v: f32) -> Self {
        Self::uniform(v)
    }
}

macro_rules! impl_edges_from_int {
    ($($t:ty),* $(,)?) => {
        $(
            impl From<$t> for Edges {
                fn from(v: $t) -> Self {
                    Self::uniform(v as f32)
                }
            }
        )*
    };
}

impl_edges_from_int!(i32, u32, u16, i16, u8, i8);

impl From<(f32, f32)> for Edges {
    fn from((x, y): (f32, f32)) -> Self {
        Self::xy(x, y)
    }
}

impl From<[f32; 2]> for Edges {
    fn from([x, y]: [f32; 2]) -> Self {
        Self::xy(x, y)
    }
}

impl From<[f32; 4]> for Edges {
    fn from([left, top, right, bottom]: [f32; 4]) -> Self {
        Self::only(left, top, right, bottom)
    }
}

impl From<(f32, f32, f32, f32)> for Edges {
    fn from((left, top, right, bottom): (f32, f32, f32, f32)) -> Self {
        Self::only(left, top, right, bottom)
    }
}

impl Add for Edges {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self {
            left: self.left + rhs.left,
            top: self.top + rhs.top,
            right: self.right + rhs.right,
            bottom: self.bottom + rhs.bottom,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Constraints {
    pub min: Size,

    pub max: Size,
}

impl Constraints {
    pub const UNBOUNDED: Self = Self {
        min: Size::ZERO,
        max: Size {
            width: f32::INFINITY,
            height: f32::INFINITY,
        },
    };

    pub const fn tight(size: Size) -> Self {
        Self {
            min: size,
            max: size,
        }
    }

    pub const fn loose(max: Size) -> Self {
        Self {
            min: Size::ZERO,
            max,
        }
    }

    pub fn loosen(self) -> Self {
        Self {
            min: Size::ZERO,
            max: self.max,
        }
    }

    pub fn constrain(self, size: Size) -> Size {
        Size::new(
            clamp_axis(size.width, self.min.width, self.max.width),
            clamp_axis(size.height, self.min.height, self.max.height),
        )
    }

    pub fn is_tight(self) -> bool {
        self.min == self.max
    }

    pub fn is_bounded(self) -> bool {
        self.max.width.is_finite() && self.max.height.is_finite()
    }

    pub fn with_width(self, min: f32, max: f32) -> Self {
        Self {
            min: Size::new(min, self.min.height),
            max: Size::new(max, self.max.height),
        }
    }

    pub fn with_height(self, min: f32, max: f32) -> Self {
        Self {
            min: Size::new(self.min.width, min),
            max: Size::new(self.max.width, max),
        }
    }
}

fn clamp_axis(value: f32, min: f32, max: f32) -> f32 {
    let (min, max) = if min > max { (max, max) } else { (min, max) };
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect::from_xywh(x, y, w, h)
    }

    #[test]
    fn point_arithmetic() {
        let a = Point::new(2.0, 3.0);
        let b = Point::new(5.0, 7.0);
        assert_eq!(b - a, Vec2::new(3.0, 4.0));
        assert_eq!(a + Vec2::new(1.0, 1.0), Point::new(3.0, 4.0));
        assert_eq!(a.distance(Point::ZERO), (13.0f32).sqrt());
    }

    #[test]
    fn vec_math() {
        let v = Vec2::new(3.0, 4.0);
        assert_eq!(v.length(), 5.0);
        assert_eq!(v.dot(Vec2::new(1.0, 0.0)), 3.0);
        assert_eq!(Vec2::ZERO.normalized(), Vec2::ZERO);
        assert_eq!(-v, Vec2::new(-3.0, -4.0));
    }

    #[test]
    fn rect_ltrb_and_contains() {
        let r = rect(10.0, 20.0, 30.0, 40.0);
        assert_eq!(r.left(), 10.0);
        assert_eq!(r.right(), 40.0);
        assert_eq!(r.top(), 20.0);
        assert_eq!(r.bottom(), 60.0);

        assert!(r.contains(Point::new(39.9, 59.9)));
        assert!(!r.contains(Point::new(40.0, 59.9)));
        assert!(!r.contains(Point::new(39.9, 60.0)));
        assert!(!r.contains(Point::new(9.9, 20.0)));
    }

    #[test]
    fn rect_intersect() {
        let a = rect(0.0, 0.0, 10.0, 10.0);
        let touching = rect(10.0, 0.0, 10.0, 10.0);
        assert!(!a.intersects(touching));

        let overlap = rect(5.0, 5.0, 10.0, 10.0);
        assert_eq!(a.intersect(overlap), rect(5.0, 5.0, 5.0, 5.0));

        let disjoint = rect(100.0, 100.0, 1.0, 1.0);
        assert!(a.intersect(disjoint).is_empty());
        assert!(!a.intersects(disjoint));
    }

    #[test]
    fn rect_union() {
        let a = rect(0.0, 0.0, 10.0, 10.0);
        let b = rect(20.0, 20.0, 10.0, 10.0);
        assert_eq!(a.union(b), rect(0.0, 0.0, 30.0, 30.0));

        let empty = rect(5.0, 5.0, 0.0, 0.0);
        assert_eq!(empty.union(a), a);
        assert_eq!(a.union(empty), a);
    }

    #[test]
    fn rect_contains_rect_and_inflate() {
        let outer = rect(0.0, 0.0, 10.0, 10.0);
        assert!(outer.contains_rect(rect(2.0, 2.0, 3.0, 3.0)));
        assert!(!outer.contains_rect(rect(9.0, 9.0, 5.0, 5.0)));

        let inflated = outer.inflate(1.0, 2.0);
        assert_eq!(inflated.origin, Point::new(-1.0, -2.0));
        assert_eq!(inflated.size, Size::new(12.0, 14.0));

        assert_eq!(
            outer.translate(Vec2::new(5.0, -5.0)),
            rect(5.0, -5.0, 10.0, 10.0)
        );
    }

    #[test]
    fn edges_sums() {
        let e = Edges::uniform(2.0);
        assert_eq!(e.horizontal_sum(), 4.0);
        assert_eq!(e.vertical_sum(), 4.0);
        assert_eq!(Edges::horizontal(3.0).vertical_sum(), 0.0);
    }

    #[test]
    fn edges_ergonomics() {
        assert_eq!(Edges::all(4.0), Edges::uniform(4.0));
        assert_eq!(Edges::xy(10.0, 6.0), Edges::symmetric(10.0, 6.0));
        assert_eq!(Edges::xy(10.0, 6.0), Edges::only(10.0, 6.0, 10.0, 6.0));
        assert_eq!(Edges::from((10.0f32, 6.0f32)), Edges::xy(10.0, 6.0));
        assert_eq!(Edges::from([10.0f32, 6.0f32]), Edges::xy(10.0, 6.0));
        assert_eq!(
            Edges::from([1.0f32, 2.0, 3.0, 4.0]),
            Edges::only(1.0, 2.0, 3.0, 4.0)
        );
        assert_eq!(
            Edges::from((1.0f32, 2.0, 3.0, 4.0)),
            Edges::only(1.0, 2.0, 3.0, 4.0)
        );
        assert_eq!(Edges::from(8), Edges::all(8.0));
        assert_eq!(
            Edges::left(5.0).with_top(3.0),
            Edges::only(5.0, 3.0, 0.0, 0.0)
        );
        assert!(Edges::ZERO.is_zero());
        assert!(!Edges::all(1.0).is_zero());
        assert_eq!(
            Edges::all(2.0) + Edges::xy(1.0, 3.0),
            Edges::only(3.0, 5.0, 3.0, 5.0)
        );
    }

    #[test]
    fn length_resolve() {
        assert_eq!(Length::from(12.5), Length::Px(12.5));
        assert_eq!(Length::Px(100.0).resolve(1000.0), Some(100.0));
        assert_eq!(Length::Percent(50.0).resolve(200.0), Some(100.0));
        assert_eq!(Length::Fill(1.0).resolve(1000.0), None);
        assert_eq!(Length::FIT.resolve(1000.0), None);
        assert_eq!(Length::px(-5.0), Length::Px(0.0));
    }

    #[test]
    fn length_ext_sugar() {
        assert_eq!(100.px(), Length::Px(100.0));
        assert_eq!(50.percent(), Length::Percent(50.0));
        assert_eq!(1.0.fill(), Length::Fill(1.0));
    }

    #[test]
    fn constraints_constrain() {
        assert_eq!(
            Constraints::UNBOUNDED.constrain(Size::new(1e9, 1e9)),
            Size::new(1e9, 1e9)
        );
        assert_eq!(
            Constraints::tight(Size::new(50.0, 50.0)).constrain(Size::new(1.0, 999.0)),
            Size::new(50.0, 50.0)
        );
        assert_eq!(
            Constraints::loose(Size::new(100.0, 100.0)).constrain(Size::new(150.0, 50.0)),
            Size::new(100.0, 50.0)
        );

        let c = Constraints {
            min: Size::new(50.0, 50.0),
            max: Size::new(100.0, 100.0),
        };
        assert_eq!(c.constrain(Size::new(10.0, 500.0)), Size::new(50.0, 100.0));

        let inverted = Constraints {
            min: Size::new(100.0, 100.0),
            max: Size::new(50.0, 50.0),
        };
        assert_eq!(
            inverted.constrain(Size::new(75.0, 75.0)),
            Size::new(50.0, 50.0)
        );
    }

    #[test]
    fn constraints_builders() {
        let c = Constraints::UNBOUNDED.with_width(10.0, 20.0);
        assert_eq!(c.constrain(Size::new(100.0, 100.0)), Size::new(20.0, 100.0));
        assert!(!c.is_tight());
        assert!(!c.is_bounded());
        assert!(c.loosen().min == Size::ZERO);
        assert!(Constraints::tight(Size::new(4.0, 4.0)).is_tight());
    }

    #[test]
    fn transform_identity_and_compose() {
        let p = Point::new(3.0, -2.0);
        assert_eq!(Transform::IDENTITY.apply(p), p);
        assert!(Transform::IDENTITY.is_identity());
        let t = Transform::translate(5.0, -1.0).then(Transform::translate(-5.0, 1.0));
        assert!(t.is_identity(), "offsets cancel: {t:?}");
        let r = Transform::rotate_around_degrees(90.0, Point::new(5.0, 5.0));
        let q = r.apply(Point::new(10.0, 5.0));
        assert!((q.x - 5.0).abs() < 1e-4, "x={}", q.x);
        assert!((q.y - 10.0).abs() < 1e-4, "y={}", q.y);
    }

    #[test]
    fn transform_aabb_of_rotated_rect() {
        let r = Rect::from_xywh(0.0, 0.0, 10.0, 10.0);
        let t = Transform::rotate_around_degrees(45.0, r.center());
        let aabb = t.apply_rect_aabb(r);
        let expected = 10.0 * std::f32::consts::SQRT_2;
        assert!((aabb.size.width - expected).abs() < 0.05, "{aabb:?}");
        assert!((aabb.size.height - expected).abs() < 0.05, "{aabb:?}");
    }
}
