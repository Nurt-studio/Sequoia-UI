use sequoia_core::Point;

use crate::Spring;

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum AnimValue {
    Static(f32),

    Spring { target: f32, spring: Spring },
}

impl AnimValue {
    pub fn static_px(v: f32) -> Self {
        Self::Static(v)
    }

    pub fn spring(target: f32, damping_ratio: f32, frequency: f32) -> Self {
        Self::Spring {
            target,
            spring: Spring::new(damping_ratio, frequency),
        }
    }

    pub fn target(&self) -> f32 {
        match self {
            Self::Static(v) | Self::Spring { target: v, .. } => *v,
        }
    }

    pub fn is_spring(&self) -> bool {
        matches!(self, Self::Spring { .. })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StateDef {
    pub name: String,

    pub x: AnimValue,

    pub y: AnimValue,
}

impl StateDef {
    pub fn new(name: &str, x: AnimValue, y: AnimValue) -> Self {
        Self {
            name: name.to_string(),
            x,
            y,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Trigger {
    Load,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TransitionDef {
    pub from: String,

    pub to: String,

    pub trigger: Trigger,
}

impl TransitionDef {
    pub fn new(from: &str, to: &str, trigger: Trigger) -> Self {
        Self {
            from: from.to_string(),
            to: to.to_string(),
            trigger,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Flight {
    from: usize,
    to: usize,

    elapsed: f32,
}

#[derive(Clone, Debug)]
pub struct SpringMachine {
    states: Vec<StateDef>,
    transitions: Vec<TransitionDef>,

    current: usize,

    active: Option<Flight>,

    fired: bool,

    pos: Point,
}

impl SpringMachine {
    pub fn new(states: Vec<StateDef>, transitions: Vec<TransitionDef>) -> Self {
        let pos = states
            .first()
            .map(|s| Point::new(s.x.target(), s.y.target()))
            .unwrap_or_default();
        Self {
            states,
            transitions,
            current: 0,
            active: None,
            fired: false,
            pos,
        }
    }

    pub fn states(&self) -> &[StateDef] {
        &self.states
    }

    pub fn current_name(&self) -> &str {
        self.states
            .get(self.current)
            .map(|s| s.name.as_str())
            .unwrap_or("")
    }

    fn find_state(&self, name: &str) -> Option<usize> {
        self.states.iter().position(|s| s.name == name)
    }

    pub fn advance(&mut self, dt: f64) -> bool {
        let dt = dt.max(0.0) as f32;
        if self.states.is_empty() {
            self.pos = Point::default();
            return false;
        }
        if !self.fired {
            self.fired = true;
            self.active = self.fire_load();
        }
        let running = match self.active.as_mut() {
            None => false,
            Some(f) => {
                f.elapsed += dt;
                if Self::settled(&self.states[f.to], f.elapsed) {
                    self.current = f.to;
                    self.active = None;
                    false
                } else {
                    true
                }
            }
        };
        self.pos = self.offset_at();
        running
    }

    fn fire_load(&mut self) -> Option<Flight> {
        let tr = self
            .transitions
            .iter()
            .find(|t| t.trigger == Trigger::Load)?;
        let (from, to) = (self.find_state(&tr.from)?, self.find_state(&tr.to)?);
        self.current = from;
        let target = &self.states[to];
        if matches!(target.x, AnimValue::Static(_)) && matches!(target.y, AnimValue::Static(_)) {
            self.current = to;
            self.pos = Point::new(target.x.target(), target.y.target());
            return None;
        }
        Some(Flight {
            from,
            to,
            elapsed: 0.0,
        })
    }

    fn settled(state: &StateDef, t: f32) -> bool {
        const EPS: f32 = 0.001;
        let ok_x = match &state.x {
            AnimValue::Static(_) => true,
            AnimValue::Spring { spring, .. } => t >= spring.settle_time(EPS),
        };
        let ok_y = match &state.y {
            AnimValue::Static(_) => true,
            AnimValue::Spring { spring, .. } => t >= spring.settle_time(EPS),
        };
        ok_x && ok_y
    }

    fn offset_at(&self) -> Point {
        let Some(f) = self.active else {
            return self
                .states
                .get(self.current)
                .map(|s| Point::new(s.x.target(), s.y.target()))
                .unwrap_or_default();
        };
        Point::new(
            interpolate_axis(&self.states[f.from].x, &self.states[f.to].x, f.elapsed),
            interpolate_axis(&self.states[f.from].y, &self.states[f.to].y, f.elapsed),
        )
    }

    pub fn offset(&self) -> Point {
        self.pos
    }

    pub fn is_flying(&self) -> bool {
        self.active.is_some()
    }
}

fn interpolate_axis(from: &AnimValue, to: &AnimValue, t: f32) -> f32 {
    let f = from.target();
    let target = to.target();
    match to {
        AnimValue::Static(_) => target,
        AnimValue::Spring { spring, .. } => f + (target - f) * spring.value(t),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn demo() -> SpringMachine {
        SpringMachine::new(
            vec![
                StateDef::new(
                    "idle",
                    AnimValue::static_px(408.0),
                    AnimValue::static_px(8.0),
                ),
                StateDef::new(
                    "bouncing",
                    AnimValue::spring(8.0, 0.5, 8.0),
                    AnimValue::spring(228.0, 0.5, 8.0),
                ),
            ],
            vec![TransitionDef::new("idle", "bouncing", Trigger::Load)],
        )
    }

    #[test]
    fn load_fires_and_starts_at_from() {
        let mut m = demo();
        assert!(m.advance(0.0), "Load leci od pierwszej klatki");
        assert_eq!(m.offset(), Point::new(408.0, 8.0));
        assert_eq!(m.states()[1].x.target(), 8.0);
        assert_eq!(m.states()[1].y.target(), 228.0);
    }

    #[test]
    fn no_transition_stays_put() {
        let mut m = SpringMachine::new(
            vec![StateDef::new(
                "idle",
                AnimValue::static_px(10.0),
                AnimValue::static_px(20.0),
            )],
            vec![],
        );
        assert!(!m.advance(0.016));
        assert_eq!(m.offset(), Point::new(10.0, 20.0));
        assert_eq!(m.current_name(), "idle");
    }

    #[test]
    fn static_target_snaps_immediately() {
        let mut m = SpringMachine::new(
            vec![
                StateDef::new("a", AnimValue::static_px(0.0), AnimValue::static_px(0.0)),
                StateDef::new("b", AnimValue::static_px(100.0), AnimValue::static_px(50.0)),
            ],
            vec![TransitionDef::new("a", "b", Trigger::Load)],
        );
        assert!(!m.advance(0.0));
        assert_eq!(m.offset(), Point::new(100.0, 50.0));
    }

    #[test]
    fn empty_machine_is_inert() {
        let mut m = SpringMachine::new(vec![], vec![]);
        assert!(!m.advance(0.016));
        assert_eq!(m.offset(), Point::ZERO);
        assert_eq!(m.current_name(), "");
    }

    #[test]
    fn unknown_states_never_fly() {
        let mut m = SpringMachine::new(
            vec![StateDef::new(
                "a",
                AnimValue::static_px(0.0),
                AnimValue::static_px(0.0),
            )],
            vec![TransitionDef::new("a", "nope", Trigger::Load)],
        );
        assert!(!m.advance(0.016));
        assert_eq!(m.offset(), Point::new(0.0, 0.0));
    }

    #[test]
    fn negative_dt_is_clamped() {
        let mut m = demo();
        assert!(m.advance(-1.0));
        assert_eq!(m.offset(), Point::new(408.0, 8.0));
    }

    #[test]
    fn flight_is_visible_across_frames_then_settles() {
        let mut m = SpringMachine::new(
            vec![
                StateDef::new(
                    "idle",
                    AnimValue::static_px(408.0),
                    AnimValue::static_px(8.0),
                ),
                StateDef::new(
                    "bouncing",
                    AnimValue::spring(8.0, 0.6, 2.5),
                    AnimValue::spring(228.0, 0.6, 2.5),
                ),
            ],
            vec![TransitionDef::new("idle", "bouncing", Trigger::Load)],
        );
        assert!(m.advance(0.0));
        let mut prev = m.offset().x;
        assert!(prev > 380.0, "start at right edge");
        let mut moved = 0;
        for _ in 0..12 {
            if !m.advance(0.025) {
                break;
            }
            if m.offset().x < prev - 5.0 {
                moved += 1;
            }
            prev = m.offset().x;
        }
        assert!(moved >= 2, "the flight is to be visible for several frames");

        for _ in 0..200 {
            if !m.advance(0.025) {
                break;
            }
        }
        assert!(!m.is_flying());
        let p = m.offset();
        assert!((p.x - 8.0).abs() < 0.01, "x settled: {p:?}");
        assert!((p.y - 228.0).abs() < 0.01, "y settled: {p:?}");
    }

    #[test]
    fn settles_exactly_on_target() {
        let mut m = SpringMachine::new(
            vec![
                StateDef::new("a", AnimValue::static_px(0.0), AnimValue::static_px(0.0)),
                StateDef::new(
                    "b",
                    AnimValue::spring(100.0, 1.0, 100.0),
                    AnimValue::spring(50.0, 1.0, 100.0),
                ),
            ],
            vec![TransitionDef::new("a", "b", Trigger::Load)],
        );
        assert!(m.advance(0.0));
        for _ in 0..100 {
            if !m.advance(0.016) {
                break;
            }
        }
        assert!(!m.is_flying());
        assert_eq!(m.offset(), Point::new(100.0, 50.0));
    }

    #[test]
    fn mixed_static_and_spring_axes() {
        let mut m = SpringMachine::new(
            vec![
                StateDef::new("a", AnimValue::static_px(0.0), AnimValue::static_px(0.0)),
                StateDef::new(
                    "b",
                    AnimValue::static_px(100.0),
                    AnimValue::spring(50.0, 1.0, 4.0),
                ),
            ],
            vec![TransitionDef::new("a", "b", Trigger::Load)],
        );
        assert!(m.advance(0.0));

        assert_eq!(m.offset().x, 100.0);
        for _ in 0..200 {
            if !m.advance(0.016) {
                break;
            }
        }
        assert_eq!(m.offset(), Point::new(100.0, 50.0));
    }

    #[test]
    fn underdamped_axis_overshoots_midflight() {
        let mut m = SpringMachine::new(
            vec![
                StateDef::new("a", AnimValue::static_px(100.0), AnimValue::static_px(0.0)),
                StateDef::new(
                    "b",
                    AnimValue::spring(0.0, 0.3, 3.0),
                    AnimValue::static_px(0.0),
                ),
            ],
            vec![TransitionDef::new("a", "b", Trigger::Load)],
        );
        assert!(m.advance(0.0));
        let mut min = f32::INFINITY;
        for _ in 0..120 {
            m.advance(1.0 / 60.0);
            min = min.min(m.offset().x);
        }
        assert!(min < -1.0, "under-damped X should exceed 0, min={min}");
    }
}
