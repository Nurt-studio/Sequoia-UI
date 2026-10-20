#![forbid(unsafe_code)]

use crate::paint::Color;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GradientStop {
    pub at: f32,
    pub color: Color,
}

impl GradientStop {
    pub fn new(at: f32, color: Color) -> Self {
        Self { at, color }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct GradientStops {
    stops: Vec<GradientStop>,
}

pub const MAX_GRADIENT_STOPS: usize = 4;

impl GradientStops {
    pub fn new(mut stops: Vec<GradientStop>) -> Self {
        for s in &mut stops {
            s.at = if s.at.is_finite() {
                s.at.clamp(0.0, 1.0)
            } else {
                0.0
            };
        }
        stops.sort_by(|a, b| a.at.partial_cmp(&b.at).unwrap_or(std::cmp::Ordering::Equal));

        let mut out: Vec<GradientStop> = Vec::with_capacity(stops.len());
        for s in stops {
            if let Some(last) = out.last_mut()
                && (last.at - s.at).abs() < f32::EPSILON
            {
                *last = s;
                continue;
            }
            out.push(s);
        }
        Self { stops: out }
    }

    pub fn solid(color: Color) -> Self {
        Self {
            stops: vec![GradientStop::new(0.0, color)],
        }
    }

    pub fn two(from: Color, to: Color) -> Self {
        Self::new(vec![
            GradientStop::new(0.0, from),
            GradientStop::new(1.0, to),
        ])
    }

    pub fn is_empty(&self) -> bool {
        self.stops.is_empty()
    }

    pub fn len(&self) -> usize {
        self.stops.len()
    }

    pub fn stops(&self) -> &[GradientStop] {
        &self.stops
    }

    pub fn for_gpu(&self) -> Option<([[f32; 4]; MAX_GRADIENT_STOPS], [f32; 4], u32)> {
        if self.stops.is_empty() || self.stops.len() > MAX_GRADIENT_STOPS {
            return None;
        }
        let mut colors = [[0.0; 4]; MAX_GRADIENT_STOPS];
        let mut pos = [0.0; 4];
        for (i, s) in self.stops.iter().enumerate() {
            let c = s.color.to_linear();
            colors[i] = [c.r, c.g, c.b, c.a];
            pos[i] = s.at;
        }

        if self.stops.len() == 1 {
            for i in 1..MAX_GRADIENT_STOPS {
                colors[i] = colors[0];
                pos[i] = 1.0;
            }
        }
        Some((colors, pos, self.stops.len() as u32))
    }

    pub fn cache_key(&self) -> u64 {
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        let mut mix = |b: u8| {
            hash ^= u64::from(b);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        };
        for s in &self.stops {
            for b in s.at.to_bits().to_le_bytes() {
                mix(b);
            }
            let (r, g, b, a) = s.color.to_rgba8();
            mix(r);
            mix(g);
            mix(b);
            mix(a);
        }
        hash ^= 0x9e37_79b9_7f4a_7c15;
        hash
    }

    pub fn sample_linear(&self, t: f32) -> [f32; 4] {
        let Some(first) = self.stops.first() else {
            return [0.0, 0.0, 0.0, 0.0];
        };
        if self.stops.len() == 1 || !t.is_finite() {
            let c = first.color.to_linear();
            return [c.r, c.g, c.b, c.a];
        }
        let t = t.clamp(0.0, 1.0);
        if t <= first.at {
            let c = first.color.to_linear();
            return [c.r, c.g, c.b, c.a];
        }
        let last = self.stops[self.stops.len() - 1];
        if t >= last.at {
            let c = last.color.to_linear();
            return [c.r, c.g, c.b, c.a];
        }
        for pair in self.stops.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            if t <= b.at {
                let span = b.at - a.at;
                let k = if span <= f32::EPSILON {
                    0.0
                } else {
                    ((t - a.at) / span).clamp(0.0, 1.0)
                };
                let ca = a.color.to_linear();
                let cb = b.color.to_linear();

                let mix = |x: f32, y: f32| x + (y - x) * k;
                return [
                    mix(ca.r, cb.r),
                    mix(ca.g, cb.g),
                    mix(ca.b, cb.b),
                    mix(ca.a, cb.a),
                ];
            }
        }
        let c = last.color.to_linear();
        [c.r, c.g, c.b, c.a]
    }

    pub fn bake_lut256(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(256 * 4);
        for i in 0..256 {
            let t = i as f32 / 255.0;
            let [r, g, b, a] = self.sample_linear(t);
            let q = |c: f32| (c.clamp(0.0, 1.0) * 255.0).round() as u8;
            out.push(q(r));
            out.push(q(g));
            out.push(q(b));
            out.push(q(a));
        }
        out
    }
}

impl From<Vec<GradientStop>> for GradientStops {
    fn from(v: Vec<GradientStop>) -> Self {
        Self::new(v)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GradientKind {
    Linear { angle_deg: f32 },
    Radial { radius: f32 },
}

impl GradientKind {
    pub fn horizontal() -> Self {
        Self::Linear { angle_deg: 0.0 }
    }

    pub fn vertical() -> Self {
        Self::Linear { angle_deg: 90.0 }
    }

    pub fn cover() -> Self {
        Self::Radial { radius: 0.0 }
    }

    pub fn direction(self) -> (f32, f32) {
        match self {
            Self::Linear { angle_deg } => {
                let a = if angle_deg.is_finite() {
                    angle_deg.to_radians()
                } else {
                    0.0
                };
                (a.cos(), a.sin())
            }
            Self::Radial { .. } => (0.0, 0.0),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Fill {
    Solid(Color),
    Gradient {
        kind: GradientKind,
        stops: GradientStops,
    },
}

impl Fill {
    pub fn solid(color: Color) -> Self {
        Self::Solid(color)
    }

    pub fn linear(stops: impl Into<GradientStops>, angle_deg: f32) -> Self {
        Self::Gradient {
            kind: GradientKind::Linear { angle_deg },
            stops: stops.into(),
        }
    }

    pub fn horizontal(stops: impl Into<GradientStops>) -> Self {
        Self::Gradient {
            kind: GradientKind::horizontal(),
            stops: stops.into(),
        }
    }

    pub fn vertical(stops: impl Into<GradientStops>) -> Self {
        Self::Gradient {
            kind: GradientKind::vertical(),
            stops: stops.into(),
        }
    }

    pub fn radial(stops: impl Into<GradientStops>) -> Self {
        Self::Gradient {
            kind: GradientKind::cover(),
            stops: stops.into(),
        }
    }

    pub fn radial_radius(stops: impl Into<GradientStops>, radius: f32) -> Self {
        let radius = if radius.is_finite() {
            radius.max(0.0)
        } else {
            0.0
        };
        Self::Gradient {
            kind: GradientKind::Radial { radius },
            stops: stops.into(),
        }
    }

    pub fn is_solid(&self) -> bool {
        matches!(self, Self::Solid(_))
    }

    pub fn is_visible(&self) -> bool {
        match self {
            Self::Solid(c) => c.a > 0.0,
            Self::Gradient { stops, .. } => !stops.is_empty(),
        }
    }
}

impl From<Color> for Fill {
    fn from(c: Color) -> Self {
        Self::Solid(c)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stroke {
    pub width: f32,
    pub rounded: bool,
}

impl Stroke {
    pub fn new(width: f32) -> Self {
        Self {
            width: if width.is_finite() {
                width.max(0.0)
            } else {
                0.0
            },
            rounded: false,
        }
    }

    pub fn round(width: f32) -> Self {
        Self {
            width: if width.is_finite() {
                width.max(0.0)
            } else {
                0.0
            },
            rounded: true,
        }
    }

    pub fn is_visible(self) -> bool {
        self.width > 0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stops_sort_and_clamp() {
        let s = GradientStops::new(vec![
            GradientStop::new(1.5, Color::BLUE),
            GradientStop::new(-1.0, Color::RED),
            GradientStop::new(0.5, Color::GREEN),
        ]);
        let ats: Vec<f32> = s.stops().iter().map(|x| x.at).collect();
        assert_eq!(ats, vec![0.0, 0.5, 1.0]);
    }

    #[test]
    fn empty_is_transparent() {
        let s = GradientStops::new(vec![]);
        assert_eq!(s.sample_linear(0.5), [0.0, 0.0, 0.0, 0.0]);
        assert_eq!(s.bake_lut256().len(), 256 * 4);
    }

    #[test]
    fn linear_mid_is_not_dark() {
        let s = GradientStops::two(Color::RED, Color::BLUE);
        let mid = s.sample_linear(0.5);

        assert!((mid[0] - 0.5).abs() < 1e-5, "{mid:?}");
        assert!((mid[2] - 0.5).abs() < 1e-5, "{mid:?}");

        let gray = GradientStops::two(Color::WHITE, Color::BLACK).sample_linear(0.5);
        assert!((gray[0] - 0.5).abs() < 1e-5, "{gray:?}");
    }

    #[test]
    fn lut_roundtrip_ends() {
        let s = GradientStops::two(Color::RED, Color::BLUE);
        let lut = s.bake_lut256();
        assert_eq!(lut.len(), 1024);
        assert_eq!(&lut[0..4], &[255, 0, 0, 255]);
        assert_eq!(&lut[1020..1024], &[0, 0, 255, 255]);
    }

    #[test]
    fn cache_key_stable() {
        let a = GradientStops::two(Color::RED, Color::BLUE).cache_key();
        let b = GradientStops::two(Color::RED, Color::BLUE).cache_key();
        let c = GradientStops::two(Color::BLUE, Color::RED).cache_key();
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
