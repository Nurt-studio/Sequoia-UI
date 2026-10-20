use sequoia_core::{Color, Point, Vec2};

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

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Gradient {
    Linear {
        from: Point,

        to: Point,

        stops: Vec<GradientStop>,
    },

    Radial {
        center: Point,

        radius: f32,

        stops: Vec<GradientStop>,
    },
}

impl Gradient {
    pub fn linear(
        from: impl Into<Point>,
        to: impl Into<Point>,
        stops: impl Into<Vec<GradientStop>>,
    ) -> Self {
        Self::Linear {
            from: from.into(),
            to: to.into(),
            stops: sorted(stops.into()),
        }
    }

    pub fn radial(
        center: impl Into<Point>,
        radius: f32,
        stops: impl Into<Vec<GradientStop>>,
    ) -> Self {
        Self::Radial {
            center: center.into(),
            radius,
            stops: sorted(stops.into()),
        }
    }

    pub fn stops(&self) -> &[GradientStop] {
        match self {
            Self::Linear { stops, .. } | Self::Radial { stops, .. } => stops,
        }
    }

    pub fn is_vertical(&self) -> bool {
        match self {
            Self::Linear { from, to, .. } => {
                let d = Vec2::new(to.x - from.x, to.y - from.y);
                d.y.abs() > d.x.abs()
            }
            Self::Radial { .. } => false,
        }
    }

    pub fn direction(&self) -> Vec2 {
        match self {
            Self::Linear { from, to, .. } => Vec2::new(to.x - from.x, to.y - from.y),
            Self::Radial { .. } => Vec2::ZERO,
        }
    }

    pub fn color_at(&self, p: Point) -> Color {
        let t = match self {
            Self::Linear { from, to, .. } => {
                let d = Vec2::new(to.x - from.x, to.y - from.y);
                let len2 = d.x * d.x + d.y * d.y;
                if len2 <= f32::MIN_POSITIVE {
                    0.0
                } else {
                    ((p.x - from.x) * d.x + (p.y - from.y) * d.y) / len2
                }
            }
            Self::Radial { center, radius, .. } => {
                if !(*radius > 0.0) || !radius.is_finite() {
                    0.0
                } else {
                    let d = Vec2::new(p.x - center.x, p.y - center.y);
                    (d.x * d.x + d.y * d.y).sqrt() / *radius
                }
            }
        };
        sample_stops(t, self.stops())
    }
}

fn sorted(mut stops: Vec<GradientStop>) -> Vec<GradientStop> {
    stops.sort_by(|a, b| a.at.partial_cmp(&b.at).unwrap_or(std::cmp::Ordering::Equal));
    stops
}

fn sample_stops(t: f32, stops: &[GradientStop]) -> Color {
    let Some(first) = stops.first() else {
        return Color::TRANSPARENT;
    };
    if t <= first.at || stops.len() == 1 {
        return first.color;
    }
    let last = stops[stops.len() - 1];
    if t >= last.at {
        return last.color;
    }
    for pair in stops.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        if t <= b.at {
            let span = b.at - a.at;
            let k = if span <= f32::MIN_POSITIVE {
                0.0
            } else {
                (t - a.at) / span
            };
            return lerp_color(a.color, b.color, k);
        }
    }
    last.color
}

fn lerp_color(a: Color, b: Color, k: f32) -> Color {
    let k = k.clamp(0.0, 1.0);
    let mix = |x: f32, y: f32| x + (y - x) * k;
    Color {
        r: mix(a.r, b.r),
        g: mix(a.g, b.g),
        b: mix(a.b, b.b),
        a: mix(a.a, b.a),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn red() -> Color {
        Color::rgb8(255, 0, 0)
    }

    fn blue() -> Color {
        Color::rgb8(0, 0, 255)
    }

    #[test]
    fn linear_gradient_runs_along_the_axis() {
        let g = Gradient::linear(
            (0.0, 0.0),
            (100.0, 0.0),
            vec![
                GradientStop::new(0.0, red()),
                GradientStop::new(1.0, blue()),
            ],
        );
        assert_eq!(g.color_at(Point::new(0.0, 50.0)), red());
        assert_eq!(g.color_at(Point::new(100.0, 50.0)), blue());
        let mid = g.color_at(Point::new(50.0, 0.0));
        assert!(
            (mid.r - 0.5).abs() < 1e-6 && (mid.b - 0.5).abs() < 1e-6,
            "{mid:?}"
        );

        assert_eq!(g.color_at(Point::new(-40.0, 0.0)), red());
        assert_eq!(g.color_at(Point::new(400.0, 0.0)), blue());
        assert!(!g.is_vertical());
    }

    #[test]
    fn vertical_linear_gradient_is_detected() {
        let g = Gradient::linear(
            (0.0, 0.0),
            (0.0, 100.0),
            vec![
                GradientStop::new(0.0, red()),
                GradientStop::new(1.0, blue()),
            ],
        );
        assert!(g.is_vertical());
        let quarter = g.color_at(Point::new(80.0, 25.0));
        assert!((quarter.b - 0.25).abs() < 1e-6, "{quarter:?}");
    }

    #[test]
    fn radial_gradient_uses_distance() {
        let g = Gradient::radial(
            (50.0, 50.0),
            40.0,
            vec![
                GradientStop::new(0.0, red()),
                GradientStop::new(1.0, blue()),
            ],
        );
        assert_eq!(g.color_at(Point::new(50.0, 50.0)), red());
        assert_eq!(g.color_at(Point::new(90.0, 50.0)), blue());
        assert!(!g.is_vertical());
        let mid = g.color_at(Point::new(50.0, 70.0));
        assert!((mid.b - 0.5).abs() < 1e-6, "{mid:?}");

        let dot = Gradient::radial((0.0, 0.0), 0.0, vec![GradientStop::new(0.0, red())]);
        assert_eq!(dot.color_at(Point::new(10.0, 10.0)), red());
    }

    #[test]
    fn stops_are_sorted_and_three_stops_interpolate() {
        let g = Gradient::linear(
            (0.0, 0.0),
            (100.0, 0.0),
            vec![
                GradientStop::new(1.0, blue()),
                GradientStop::new(0.0, red()),
                GradientStop::new(0.5, Color::rgb8(0, 255, 0)),
            ],
        );
        let at = g.stops().iter().map(|s| s.at).collect::<Vec<_>>();
        assert_eq!(at, vec![0.0, 0.5, 1.0], "input order does not matter");
        let mid = g.color_at(Point::new(25.0, 0.0));
        assert!(
            (mid.g - 0.5).abs() < 1e-6 && (mid.r - 0.5).abs() < 1e-6,
            "{mid:?}"
        );
    }

    #[test]
    fn empty_gradient_is_transparent() {
        let g = Gradient::linear((0.0, 0.0), (10.0, 0.0), Vec::new());
        assert_eq!(g.color_at(Point::new(5.0, 0.0)), Color::TRANSPARENT);
    }

    #[test]
    fn alpha_interpolates_too() {
        let g = Gradient::linear(
            (0.0, 0.0),
            (10.0, 0.0),
            vec![
                GradientStop::new(0.0, Color { a: 0.0, ..red() }),
                GradientStop::new(1.0, Color { a: 1.0, ..red() }),
            ],
        );
        assert!((g.color_at(Point::new(5.0, 0.0)).a - 0.5).abs() < 1e-6);
    }
}
