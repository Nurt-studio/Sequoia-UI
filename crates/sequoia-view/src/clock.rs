use std::time::{Duration, Instant};

/// Largest wall-clock step advanced in one frame.
///
/// A paused or occluded window can produce arbitrarily large deltas. Tweens
/// catch up from their absolute start time, while springs integrate an
/// accumulator so they keep a stable step size. Capping the accumulator avoids
/// unbounded work after a long stall.
pub const MAX_FRAME_STEP_SECS: f32 = 0.25;

/// Shared frame clock owned by the runtime.
///
/// Every view animation samples this clock instead of calling
/// `Instant::now()` on its own. That gives one coherent timestamp per frame,
/// deterministic tests, global speed control and reduced-motion support.
#[derive(Clone, Copy, Debug)]
pub struct FrameClock {
    origin: Instant,
    last_wall: Instant,
    elapsed: Duration,
    dt: Duration,
    frame: u64,
    speed: f32,
    reduced_motion: bool,
    manual: bool,
}

impl Default for FrameClock {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameClock {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            origin: now,
            last_wall: now,
            elapsed: Duration::ZERO,
            dt: Duration::ZERO,
            frame: 0,
            speed: 1.0,
            reduced_motion: false,
            manual: false,
        }
    }

    /// Current animation time. This is not affected by wall-clock jumps.
    pub fn now(&self) -> Instant {
        self.origin + self.elapsed
    }

    pub fn elapsed(&self) -> Duration {
        self.elapsed
    }

    pub fn elapsed_secs(&self) -> f32 {
        self.elapsed.as_secs_f32()
    }

    pub fn dt(&self) -> Duration {
        self.dt
    }

    pub fn dt_secs(&self) -> f32 {
        self.dt.as_secs_f32()
    }

    pub fn frame(&self) -> u64 {
        self.frame
    }

    pub fn speed(&self) -> f32 {
        self.speed
    }

    pub fn set_speed(&mut self, speed: f32) {
        self.speed = if speed.is_finite() {
            speed.max(0.0)
        } else {
            1.0
        };
    }

    pub fn reduced_motion(&self) -> bool {
        self.reduced_motion
    }

    pub fn set_reduced_motion(&mut self, reduced: bool) {
        self.reduced_motion = reduced;
    }

    pub fn is_manual(&self) -> bool {
        self.manual
    }

    pub fn set_manual(&mut self, manual: bool) {
        self.manual = manual;
        self.last_wall = Instant::now();
        self.dt = Duration::ZERO;
    }

    /// Advance from the wall clock. Manual clocks do not advance implicitly.
    pub fn advance(&mut self) {
        if self.manual {
            self.dt = Duration::ZERO;
            return;
        }
        let now = Instant::now();
        let wall = now.saturating_duration_since(self.last_wall);
        self.last_wall = now;
        let scaled = if self.reduced_motion {
            Duration::ZERO
        } else {
            wall.mul_f32(self.speed)
        };
        let capped = scaled.min(Duration::from_secs_f32(MAX_FRAME_STEP_SECS));
        self.elapsed += capped;
        self.dt = capped;
        self.frame = self.frame.wrapping_add(1);
    }

    /// Advance by an explicit duration. Used by tests and deterministic hosts.
    pub fn advance_by(&mut self, dt: Duration) {
        let scaled = if self.reduced_motion {
            Duration::ZERO
        } else {
            dt.mul_f32(self.speed)
        };
        self.elapsed += scaled;
        self.dt = scaled;
        self.frame = self.frame.wrapping_add(1);
    }

    pub fn reset(&mut self) {
        let now = Instant::now();
        self.origin = now;
        self.last_wall = now;
        self.elapsed = Duration::ZERO;
        self.dt = Duration::ZERO;
        self.frame = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_clock_advances_exactly() {
        let mut c = FrameClock::new();
        c.set_manual(true);
        c.advance_by(Duration::from_millis(16));
        c.advance_by(Duration::from_millis(16));
        assert_eq!(c.frame(), 2);
        assert_eq!(c.elapsed(), Duration::from_millis(32));
        assert_eq!(c.dt(), Duration::from_millis(16));
    }

    #[test]
    fn reduced_motion_stops_time() {
        let mut c = FrameClock::new();
        c.set_manual(true);
        c.set_reduced_motion(true);
        c.advance_by(Duration::from_millis(100));
        assert_eq!(c.elapsed(), Duration::ZERO);
        assert_eq!(c.frame(), 1);
    }

    #[test]
    fn speed_scales_manual_steps() {
        let mut c = FrameClock::new();
        c.set_manual(true);
        c.set_speed(2.0);
        c.advance_by(Duration::from_millis(50));
        assert_eq!(c.elapsed(), Duration::from_millis(100));
    }

    #[test]
    fn automatic_advance_is_disabled_in_manual_mode() {
        let mut c = FrameClock::new();
        c.set_manual(true);
        c.advance();
        assert_eq!(c.dt(), Duration::ZERO);
        assert_eq!(c.frame(), 0);
    }
}
