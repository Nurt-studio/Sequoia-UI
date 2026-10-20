use crate::Lerp;
use sequoia_core::{Color, Point, Size};

#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum Value {
    F32(f32),
    Px(f32),
    Color(Color),
    Point(Point),
    Size(Size),
}

impl Value {
    pub const fn f32(v: f32) -> Self {
        Self::F32(v)
    }
    pub const fn px(v: f32) -> Self {
        Self::Px(v)
    }
    pub const fn color(c: Color) -> Self {
        Self::Color(c)
    }
    pub const fn point(x: f32, y: f32) -> Self {
        Self::Point(Point::new(x, y))
    }
    pub const fn size(w: f32, h: f32) -> Self {
        Self::Size(Size::new(w, h))
    }

    pub fn as_f32(&self) -> Option<f32> {
        match self {
            Self::F32(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_px(&self) -> Option<f32> {
        match self {
            Self::Px(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_pixels(&self) -> Option<f32> {
        self.as_px()
    }
    pub fn as_color(&self) -> Option<Color> {
        match self {
            Self::Color(v) => Some(*v),
            _ => None,
        }
    }
    pub fn as_point(&self) -> Option<Point> {
        match self {
            Self::Point(v) => Some(*v),
            _ => None,
        }
    }
    pub fn as_size(&self) -> Option<Size> {
        match self {
            Self::Size(v) => Some(*v),
            _ => None,
        }
    }

    pub fn lerp(&self, other: &Value, t: f32) -> Value {
        match (self, other) {
            (Self::F32(a), Self::F32(b)) => Self::F32(a.lerp(*b, t)),
            (Self::Px(a), Self::Px(b)) => Self::Px(a.lerp(*b, t)),
            (Self::Color(a), Self::Color(b)) => Self::Color(a.lerp(*b, t)),
            (Self::Point(a), Self::Point(b)) => Self::Point(a.lerp(*b, t)),
            (Self::Size(a), Self::Size(b)) => Self::Size(a.lerp(*b, t)),
            (a, _) => *a,
        }
    }

    pub fn mix<'a>(samples: impl IntoIterator<Item = (&'a Value, f32)>) -> Option<Value> {
        let mut iter = samples.into_iter().peekable();
        let first = iter.peek().map(|(v, _)| *v)?;
        match first {
            Self::F32(_) => {
                let (mut acc, mut wsum) = (0.0f32, 0.0f32);
                for (v, w) in iter {
                    if let Self::F32(x) = v {
                        acc += x * w;
                        wsum += w;
                    }
                }
                (wsum > 0.0).then(|| Self::F32(acc / wsum))
            }
            Self::Px(_) => {
                let (mut acc, mut wsum) = (0.0f32, 0.0f32);
                for (v, w) in iter {
                    if let Self::Px(x) = v {
                        acc += x * w;
                        wsum += w;
                    }
                }
                (wsum > 0.0).then(|| Self::Px(acc / wsum))
            }
            Self::Color(_) => {
                let mut acc = [0.0f32; 4];
                let mut wsum = 0.0f32;
                for (v, w) in iter {
                    if let Self::Color(c) = v {
                        acc[0] += c.r * w;
                        acc[1] += c.g * w;
                        acc[2] += c.b * w;
                        acc[3] += c.a * w;
                        wsum += w;
                    }
                }
                (wsum > 0.0).then(|| {
                    Self::Color(Color::rgba(
                        acc[0] / wsum,
                        acc[1] / wsum,
                        acc[2] / wsum,
                        acc[3] / wsum,
                    ))
                })
            }
            Self::Point(_) => {
                let (mut ax, mut ay, mut wsum) = (0.0f32, 0.0f32, 0.0f32);
                for (v, w) in iter {
                    if let Self::Point(p) = v {
                        ax += p.x * w;
                        ay += p.y * w;
                        wsum += w;
                    }
                }
                (wsum > 0.0).then(|| Self::Point(Point::new(ax / wsum, ay / wsum)))
            }
            Self::Size(_) => {
                let (mut aw, mut ah, mut wsum) = (0.0f32, 0.0f32, 0.0f32);
                for (v, w) in iter {
                    if let Self::Size(s) = v {
                        aw += s.width * w;
                        ah += s.height * w;
                        wsum += w;
                    }
                }
                (wsum > 0.0).then(|| Self::Size(Size::new(aw / wsum, ah / wsum)))
            }
        }
    }
}

impl From<f32> for Value {
    fn from(v: f32) -> Self {
        Self::F32(v)
    }
}
impl From<Color> for Value {
    fn from(v: Color) -> Self {
        Self::Color(v)
    }
}
impl From<Point> for Value {
    fn from(v: Point) -> Self {
        Self::Point(v)
    }
}
impl From<Size> for Value {
    fn from(v: Size) -> Self {
        Self::Size(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lerp_matches_variants() {
        let a = Value::f32(0.0);
        let b = Value::f32(10.0);
        assert_eq!(a.lerp(&b, 0.25), Value::f32(2.5));

        let c = Value::color(Color::rgb8(0, 0, 0));
        let d = Value::color(Color::rgb8(100, 200, 0));
        let mid = c.lerp(&d, 0.5).as_color().unwrap();
        let (r, g, bl) = mid.to_rgb8();
        assert_eq!((r, g, bl), (50, 100, 0));
    }

    #[test]
    fn lerp_holds_on_mismatched_types() {
        let a = Value::f32(5.0);
        let b = Value::px(10.0);
        assert_eq!(a.lerp(&b, 0.5), Value::f32(5.0));
    }

    #[test]
    fn mix_weights_values() {
        let a = Value::f32(0.0);
        let b = Value::f32(100.0);
        assert_eq!(Value::mix([(&a, 0.75), (&b, 0.25)]), Some(Value::f32(25.0)));
    }

    #[test]
    fn mix_skips_mismatched_and_empty_weights() {
        let a = Value::f32(10.0);
        let p = Value::px(99.0);
        assert_eq!(Value::mix([(&a, 1.0), (&p, 1.0)]), Some(Value::f32(10.0)));
        assert_eq!(Value::mix([(&a, 0.0)]), None);
        assert_eq!(Value::mix(std::iter::empty()), None);
    }

    #[test]
    fn mix_blends_colors_per_channel() {
        let black = Value::color(Color::BLACK);
        let white = Value::color(Color::WHITE);
        let out = Value::mix([(&black, 0.5), (&white, 0.5)])
            .unwrap()
            .as_color()
            .unwrap();
        let (r, g, b) = out.to_rgb8();
        assert!((r as i16 - 127).abs() <= 1, "r={r}");
        assert!((g as i16 - 127).abs() <= 1, "g={g}");
        assert!((b as i16 - 127).abs() <= 1, "b={b}");
    }
}
