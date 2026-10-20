use crate::Spring;
use sequoia_core::Point;

const SETTLE_EPS: f32 = 1e-3;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpringValue {
    value: f32,
    target: f32,
    velocity: f32,
    spring: Spring,
}

impl SpringValue {
    pub fn new(value: f32, spring: Spring) -> Self {
        Self {
            value,
            target: value,
            velocity: 0.0,
            spring,
        }
    }

    pub fn with_default_spring(value: f32) -> Self {
        Self::new(value, Spring::new(0.9, 6.0))
    }

    pub fn value(&self) -> f32 {
        self.value
    }

    pub fn target(&self) -> f32 {
        self.target
    }

    pub fn velocity(&self) -> f32 {
        self.velocity
    }

    pub fn spring(&self) -> Spring {
        self.spring
    }

    pub fn is_settled(&self) -> bool {
        (self.value - self.target).abs() <= SETTLE_EPS && self.velocity.abs() <= SETTLE_EPS
    }

    pub fn set_target(&mut self, target: f32) {
        self.target = target;
        if self.is_settled() {
            self.value = target;
            self.velocity = 0.0;
        }
    }

    pub fn jump_to(&mut self, value: f32) {
        self.value = value;
        self.target = value;
        self.velocity = 0.0;
    }

    pub fn reset_value(&mut self, value: f32) {
        self.value = value;
        self.velocity = 0.0;
    }

    pub fn set_spring(&mut self, spring: Spring) {
        self.spring = spring;
    }

    pub fn nudge(&mut self, velocity: f32) {
        self.velocity += velocity;
    }

    pub fn advance(&mut self, dt: f32) -> bool {
        let dt = dt.clamp(0.0, 0.25);
        if dt <= 0.0 {
            return !self.is_settled();
        }
        let w0 = std::f32::consts::TAU * self.spring.frequency.max(0.0001);
        let k = w0 * w0;
        let c = 2.0 * self.spring.damping_ratio.max(0.0) * w0;

        let max_step = 1.0 / (w0 * 4.0).max(60.0);
        let steps = (dt / max_step).ceil().max(1.0) as usize;
        let h = dt / steps as f32;
        for _ in 0..steps {
            let accel = -k * (self.value - self.target) - c * self.velocity;
            self.velocity += accel * h;
            self.value += self.velocity * h;
        }
        if self.is_settled() {
            self.value = self.target;
            self.velocity = 0.0;
            false
        } else {
            true
        }
    }
}

impl Default for SpringValue {
    fn default() -> Self {
        Self::with_default_spring(0.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpringPoint {
    pub x: SpringValue,
    pub y: SpringValue,
}

impl SpringPoint {
    pub fn new(x: f32, y: f32, spring: Spring) -> Self {
        Self {
            x: SpringValue::new(x, spring),
            y: SpringValue::new(y, spring),
        }
    }

    pub fn with_default_spring(x: f32, y: f32) -> Self {
        Self {
            x: SpringValue::with_default_spring(x),
            y: SpringValue::with_default_spring(y),
        }
    }

    pub fn point(&self) -> Point {
        Point::new(self.x.value(), self.y.value())
    }

    pub fn set_target(&mut self, x: f32, y: f32) {
        self.x.set_target(x);
        self.y.set_target(y);
    }

    pub fn jump_to(&mut self, x: f32, y: f32) {
        self.x.jump_to(x);
        self.y.jump_to(y);
    }

    pub fn set_spring(&mut self, spring: Spring) {
        self.x.set_spring(spring);
        self.y.set_spring(spring);
    }

    pub fn is_settled(&self) -> bool {
        self.x.is_settled() && self.y.is_settled()
    }

    pub fn advance(&mut self, dt: f32) -> bool {
        let flying_x = self.x.advance(dt);
        let flying_y = self.y.advance(dt);
        flying_x || flying_y
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chases_target_then_snaps_exactly() {
        let mut v = SpringValue::new(0.0, Spring::new(0.8, 5.0));
        v.set_target(100.0);
        assert_eq!(v.value(), 0.0, "set_target does not teleport");
        assert!(v.advance(1.0 / 60.0), "still flying");
        assert!(v.value() > 0.0 && v.value() < 100.0);
        let mut frames = 1;
        while v.advance(1.0 / 60.0) {
            frames += 1;
            assert!(frames < 10_000, "spring must settle");
        }
        assert_eq!(v.value(), 100.0, "settles exactly on target");
        assert_eq!(v.velocity(), 0.0);
        assert!(v.is_settled());
    }

    #[test]
    fn retarget_keeps_momentum_and_does_not_jump() {
        let mut v = SpringValue::new(0.0, Spring::new(0.9, 3.0));
        v.set_target(100.0);
        for _ in 0..3 {
            v.advance(1.0 / 60.0);
        }
        let mid = v.value();
        let vel = v.velocity();
        assert!(mid > 0.0 && mid < 100.0, "in flight: {mid}");
        assert!(vel > 0.0, "moving forward");

        v.set_target(0.0);
        assert_eq!(v.value(), mid, "no teleport on retarget");
        assert_eq!(v.velocity(), vel, "velocity preserved on retarget");

        for _ in 0..600 {
            v.advance(1.0 / 120.0);
        }
        assert!(v.is_settled());
        assert_eq!(v.value(), 0.0);
    }

    #[test]
    fn overdamped_never_overshoots() {
        let mut v = SpringValue::new(0.0, Spring::new(1.6, 6.0));
        v.set_target(50.0);
        let mut peak = 0.0f32;
        for _ in 0..600 {
            v.advance(1.0 / 120.0);
            peak = peak.max(v.value());
        }
        assert!(peak <= 50.0 + 1e-3, "over-damped overshoot: {peak}");
    }

    #[test]
    fn underdamped_overshoots() {
        let mut v = SpringValue::new(0.0, Spring::new(0.25, 4.0));
        v.set_target(50.0);
        let mut peak = 0.0f32;
        for _ in 0..600 {
            v.advance(1.0 / 120.0);
            peak = peak.max(v.value());
        }
        assert!(peak > 52.0, "under-damped should overshoot: {peak}");
    }

    #[test]
    fn low_frame_rate_stays_stable() {
        let mut v = SpringValue::new(0.0, Spring::new(0.5, 10.0));
        v.set_target(10.0);
        for _ in 0..60 {
            v.advance(0.05);
            assert!(v.value().is_finite());
            assert!(v.value() < 100.0, "stable integration: {}", v.value());
        }
        assert!(v.is_settled() || (v.value() - 10.0).abs() < 1.0);
    }

    #[test]
    fn zero_dt_does_not_move() {
        let mut v = SpringValue::new(0.0, Spring::new(0.7, 6.0));
        v.set_target(10.0);
        v.advance(0.0);
        assert_eq!(v.value(), 0.0);
    }

    #[test]
    fn jump_is_instant_and_no_op_retarget_is_clean() {
        let mut v = SpringValue::new(0.0, Spring::new(0.7, 6.0));
        v.jump_to(3.0);
        assert_eq!(v.value(), 3.0);
        assert!(v.is_settled());
        v.set_target(3.0);
        assert!(v.is_settled());
        assert_eq!(v.value(), 3.0);
    }

    #[test]
    fn nudge_adds_velocity() {
        let mut v = SpringValue::new(0.0, Spring::new(0.7, 6.0));
        v.nudge(120.0);
        v.advance(1.0 / 60.0);
        assert!(v.value() > 0.0, "nudge pushes the value");
    }

    #[test]
    fn spring_point_animates_both_axes() {
        let mut p = SpringPoint::with_default_spring(0.0, 0.0);
        p.set_target(10.0, -20.0);
        for _ in 0..600 {
            p.advance(1.0 / 120.0);
        }
        assert!(p.is_settled());
        assert_eq!(p.point(), Point::new(10.0, -20.0));
    }
}
