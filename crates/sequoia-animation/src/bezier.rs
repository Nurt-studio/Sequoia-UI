#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CubicBezier {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
}

impl CubicBezier {
    pub const fn new(x1: f32, y1: f32, x2: f32, y2: f32) -> Self {
        Self { x1, y1, x2, y2 }
    }

    pub fn is_valid(&self) -> bool {
        (0.0..=1.0).contains(&self.x1) && (0.0..=1.0).contains(&self.x2)
    }

    pub fn solve(&self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        if t <= 0.0 {
            return 0.0;
        }
        if t >= 1.0 {
            return 1.0;
        }
        let cx = 3.0 * self.x1;
        let bx = 3.0 * (self.x2 - self.x1) - cx;
        let ax = 1.0 - cx - bx;
        let sample_x = |u: f32| ((ax * u + bx) * u + cx) * u;
        let sample_dx = |u: f32| (3.0 * ax * u + 2.0 * bx) * u + cx;

        let mut u = t;
        let mut converged = false;
        for _ in 0..8 {
            let err = sample_x(u) - t;
            if err.abs() <= 1e-6 {
                converged = true;
                break;
            }
            let d = sample_dx(u);
            if d.abs() <= 1e-6 {
                break;
            }
            u -= err / d;
        }
        if !converged || !(0.0..=1.0).contains(&u) {
            let (mut lo, mut hi) = (0.0, 1.0);
            for _ in 0..20 {
                let mid = (lo + hi) * 0.5;
                if sample_x(mid) < t {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            u = (lo + hi) * 0.5;
        }
        let cy = 3.0 * self.y1;
        let by = 3.0 * (self.y2 - self.y1) - cy;
        let ay = 1.0 - cy - by;
        ((ay * u + by) * u + cy) * u
    }
}

impl Default for CubicBezier {
    fn default() -> Self {
        Self::new(0.25, 0.1, 0.25, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoints_are_exact() {
        let c = CubicBezier::new(0.25, 0.1, 0.25, 1.0);
        assert_eq!(c.solve(0.0), 0.0);
        assert_eq!(c.solve(1.0), 1.0);
    }

    #[test]
    fn ease_curve_is_monotonic() {
        let c = CubicBezier::new(0.25, 0.1, 0.25, 1.0);
        let mut prev = 0.0f32;
        for i in 1..=100 {
            let v = c.solve(i as f32 / 100.0);
            assert!(v >= prev, "easing must not decrease");
            prev = v;
        }
        assert!(prev > 0.999);
    }

    #[test]
    fn back_curve_overshoots() {
        let c = CubicBezier::new(0.3, 1.4, 0.4, 1.0);
        let mut peak = 0.0f32;
        for i in 0..=1000 {
            peak = peak.max(c.solve(i as f32 / 1000.0));
        }
        assert!(peak > 1.05, "back curve should overshoot: {peak}");
    }

    #[test]
    fn invalid_x_still_returns_a_value() {
        let c = CubicBezier::new(1.5, 0.0, -0.5, 1.0);
        assert!(!c.is_valid());
        for i in 0..=10 {
            let _ = c.solve(i as f32 / 10.0);
        }
    }
}
