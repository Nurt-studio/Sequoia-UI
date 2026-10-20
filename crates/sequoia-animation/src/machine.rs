use std::collections::HashMap;
use std::fmt;

use crate::animation::{Animation, AnimationFrame};
use crate::mixer::Mixer;
use crate::value::Value;

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum MachineState {
    Entry,

    Exit,

    Animation {
        name: String,
        animation: String,
    },

    Single {
        name: String,
        properties: HashMap<String, Value>,
    },
}

impl MachineState {
    pub fn entry() -> Self {
        Self::Entry
    }
    pub fn exit() -> Self {
        Self::Exit
    }
    pub fn animation(name: impl Into<String>, animation: impl Into<String>) -> Self {
        Self::Animation {
            name: name.into(),
            animation: animation.into(),
        }
    }
    pub fn single(name: impl Into<String>, properties: HashMap<String, Value>) -> Self {
        Self::Single {
            name: name.into(),
            properties,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum FromState {
    Entry,

    Any,

    State(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ToState {
    Exit,

    State(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CmpOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

impl CmpOp {
    fn compare(self, a: f32, b: f32) -> bool {
        match self {
            Self::Eq => a == b,
            Self::Ne => a != b,
            Self::Lt => a < b,
            Self::Le => a <= b,
            Self::Gt => a > b,
            Self::Ge => a >= b,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Condition {
    Trigger(String),

    BooleanEquals(String, bool),

    NumberCompare(String, CmpOp, f32),

    AfterSeconds(f64),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Transition {
    pub from: FromState,
    pub to: ToState,
    pub conditions: Vec<Condition>,

    pub mix_seconds: f64,
}

impl Transition {
    pub fn new(from: FromState, to: ToState, conditions: Vec<Condition>) -> Self {
        Self {
            from,
            to,
            conditions,
            mix_seconds: 0.0,
        }
    }
    pub fn with_mix(mut self, secs: f64) -> Self {
        self.mix_seconds = secs;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum MachineError {
    NoStates,

    MissingEntry,

    DuplicateEntryOrExit,

    DuplicateState(String),

    UnknownState(String),

    UnknownAnimation(String),
}

impl fmt::Display for MachineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoStates => write!(f, "state machine has no states"),
            Self::MissingEntry => write!(f, "state machine has no entry state"),
            Self::DuplicateEntryOrExit => {
                write!(f, "state machine has more than one entry or exit state")
            }
            Self::DuplicateState(name) => write!(f, "duplicate state `{name}`"),
            Self::UnknownState(name) => write!(f, "transition references unknown state `{name}`"),
            Self::UnknownAnimation(name) => {
                write!(f, "animation state references unknown animation `{name}`")
            }
        }
    }
}

impl std::error::Error for MachineError {}

#[derive(Clone, Debug, Default)]
pub struct MachineInputs {
    triggers: HashMap<String, bool>,
    pub booleans: HashMap<String, bool>,
    pub numbers: HashMap<String, f32>,
}

impl MachineInputs {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn fire(&mut self, name: impl Into<String>) {
        self.triggers.insert(name.into(), true);
    }

    pub fn set_boolean(&mut self, name: impl Into<String>, value: bool) {
        self.booleans.insert(name.into(), value);
    }

    pub fn set_number(&mut self, name: impl Into<String>, value: f32) {
        self.numbers.insert(name.into(), value);
    }

    fn trigger_fired(&self, name: &str) -> bool {
        self.triggers.get(name).copied().unwrap_or(false)
    }

    fn clear_triggers(&mut self) {
        self.triggers.clear();
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Layer {
    state: usize,

    time: f64,

    state_time: f64,
}

impl Layer {
    fn new(state: usize) -> Self {
        Self {
            state,
            time: 0.0,
            state_time: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Mix {
    from: Layer,
    to: Layer,
    elapsed: f64,
    duration: f64,
}

#[derive(Clone, Debug)]
pub struct StateMachine {
    states: Vec<MachineState>,
    transitions: Vec<Transition>,
    animations: HashMap<String, Animation>,
    current: Option<Layer>,
    mix: Option<Mix>,
    frame: AnimationFrame,
    inputs: MachineInputs,
    state_name: String,
}

impl StateMachine {
    pub fn try_new(
        states: Vec<MachineState>,
        transitions: Vec<Transition>,
        animations: HashMap<String, Animation>,
    ) -> Result<Self, MachineError> {
        if states.is_empty() {
            return Err(MachineError::NoStates);
        }
        let mut named: HashMap<&str, ()> = HashMap::new();
        let mut entries = 0usize;
        let mut exits = 0usize;
        for state in &states {
            match state {
                MachineState::Entry => entries += 1,
                MachineState::Exit => exits += 1,
                MachineState::Animation { name, animation } => {
                    if named.insert(name.as_str(), ()).is_some() {
                        return Err(MachineError::DuplicateState(name.clone()));
                    }
                    if !animations.contains_key(animation) {
                        return Err(MachineError::UnknownAnimation(animation.clone()));
                    }
                }
                MachineState::Single { name, .. } => {
                    if named.insert(name.as_str(), ()).is_some() {
                        return Err(MachineError::DuplicateState(name.clone()));
                    }
                }
            }
        }
        if entries == 0 {
            return Err(MachineError::MissingEntry);
        }
        if entries > 1 || exits > 1 {
            return Err(MachineError::DuplicateEntryOrExit);
        }
        for transition in &transitions {
            match (&transition.from, &transition.to) {
                (FromState::State(name), _) if !named.contains_key(name.as_str()) => {
                    return Err(MachineError::UnknownState(name.clone()));
                }
                (_, ToState::State(name)) if !named.contains_key(name.as_str()) => {
                    return Err(MachineError::UnknownState(name.clone()));
                }
                _ => {}
            }
        }
        Ok(Self {
            states,
            transitions,
            animations,
            current: None,
            mix: None,
            frame: AnimationFrame::new(),
            inputs: MachineInputs::new(),
            state_name: "entry".to_string(),
        })
    }

    pub fn fire(&mut self, trigger: impl Into<String>) {
        self.inputs.fire(trigger);
    }
    pub fn set_boolean(&mut self, name: impl Into<String>, value: bool) {
        self.inputs.set_boolean(name, value);
    }
    pub fn set_number(&mut self, name: impl Into<String>, value: f32) {
        self.inputs.set_number(name, value);
    }
    pub fn inputs(&mut self) -> &mut MachineInputs {
        &mut self.inputs
    }

    pub fn advance(&mut self, dt: f64) -> bool {
        let dt = dt.max(0.0);

        if self.current.is_none()
            && self.mix.is_none()
            && let Some(entry) = self
                .states
                .iter()
                .position(|s| matches!(s, MachineState::Entry))
        {
            self.current = Some(Layer::new(entry));
        }

        if let Some(mix) = self.mix.as_mut() {
            mix.elapsed += dt;
            advance_layer(&self.states, &self.animations, &mut mix.from, dt);
            advance_layer(&self.states, &self.animations, &mut mix.to, dt);
        }
        if let Some(current) = self.current.as_mut() {
            advance_layer(&self.states, &self.animations, current, dt);
        }

        if let Some(mix) = self.mix
            && mix.elapsed >= mix.duration.max(1e-9)
        {
            self.current = Some(mix.to);
            self.mix = None;
        }

        if self.mix.is_none()
            && let Some(idx) = self.evaluate_transition()
        {
            self.apply_transition(idx);
        }
        self.frame = self.blend_output();
        self.state_name = self
            .source_layer()
            .map(|layer| self.state_name_of(Some(layer)))
            .unwrap_or_else(|| "entry".to_string());
        self.is_active()
    }

    pub fn frame(&self) -> &AnimationFrame {
        &self.frame
    }

    pub fn current_state(&self) -> &str {
        &self.state_name
    }

    pub fn is_active(&self) -> bool {
        if self.mix.is_some() {
            return true;
        }
        match self.current {
            None => false,
            Some(layer) => match &self.states[layer.state] {
                MachineState::Entry | MachineState::Exit | MachineState::Single { .. } => false,
                MachineState::Animation { animation, .. } => match self.animations.get(animation) {
                    None => false,
                    Some(a) => match a.loop_mode() {
                        crate::animation::LoopMode::Loop | crate::animation::LoopMode::PingPong => {
                            true
                        }
                        crate::animation::LoopMode::Once => !a.is_finished_at(layer.time),
                    },
                },
            },
        }
    }

    fn source_layer(&self) -> Option<Layer> {
        self.mix.map(|m| m.from).or(self.current)
    }

    fn evaluate_transition(&self) -> Option<usize> {
        let source = self.source_layer()?;
        let in_entry = matches!(&self.states[source.state], MachineState::Entry);
        let in_exit = matches!(&self.states[source.state], MachineState::Exit);
        if in_exit {
            return None;
        }
        if in_entry {
            for (i, t) in self.transitions.iter().enumerate() {
                if matches!(t.from, FromState::Entry) && self.conditions_hold(t, source) {
                    return Some(i);
                }
            }
        }
        for (i, t) in self.transitions.iter().enumerate() {
            if matches!(t.from, FromState::Any) && self.conditions_hold(t, source) {
                return Some(i);
            }
        }
        if let Some(name) = state_name_of(&self.states[source.state]) {
            for (i, t) in self.transitions.iter().enumerate() {
                if matches!(&t.from, FromState::State(n) if n == name)
                    && self.conditions_hold(t, source)
                {
                    return Some(i);
                }
            }
        }
        None
    }

    fn conditions_hold(&self, transition: &Transition, source: Layer) -> bool {
        transition.conditions.iter().all(|c| match c {
            Condition::Trigger(name) => self.inputs.trigger_fired(name),
            Condition::BooleanEquals(name, want) => {
                self.inputs.booleans.get(name).copied() == Some(*want)
            }
            Condition::NumberCompare(name, op, want) => {
                let value = self.inputs.numbers.get(name).copied().unwrap_or(0.0);
                op.compare(value, *want)
            }
            Condition::AfterSeconds(secs) => source.state_time >= *secs,
        })
    }

    fn apply_transition(&mut self, idx: usize) {
        let transition = self.transitions[idx].clone();
        let Some(target) = self.resolve_state(&transition.to) else {
            return;
        };

        self.inputs.clear_triggers();
        let to = Layer::new(target);
        if transition.mix_seconds > 0.0 {
            let from = self.source_layer().unwrap_or(to);
            self.mix = Some(Mix {
                from,
                to,
                elapsed: 0.0,
                duration: transition.mix_seconds,
            });
            self.current = None;
        } else {
            self.current = Some(to);
            self.mix = None;
        }
    }

    fn resolve_state(&self, to: &ToState) -> Option<usize> {
        match to {
            ToState::Exit => self
                .states
                .iter()
                .position(|s| matches!(s, MachineState::Exit)),
            ToState::State(name) => self
                .states
                .iter()
                .position(|s| state_name_of(s) == Some(name.as_str())),
        }
    }

    fn sample_layer(&self, layer: Layer) -> AnimationFrame {
        match &self.states[layer.state] {
            MachineState::Entry | MachineState::Exit => AnimationFrame::new(),
            MachineState::Single { properties, .. } => AnimationFrame {
                properties: properties.clone(),
            },
            MachineState::Animation { animation, .. } => self
                .animations
                .get(animation)
                .map(|a| a.sample(layer.time))
                .unwrap_or_default(),
        }
    }

    fn blend_output(&self) -> AnimationFrame {
        match (self.current, self.mix) {
            (None, None) => AnimationFrame::new(),
            (Some(current), None) => self.sample_layer(current),
            (None, Some(mix)) => {
                let w = (mix.elapsed / mix.duration.max(1e-9)).min(1.0) as f32;
                let from = self.sample_layer(mix.from);
                let to = self.sample_layer(mix.to);
                Mixer::mix_frames([(&from, 1.0 - w), (&to, w)])
            }
            (Some(_), Some(_)) => AnimationFrame::new(),
        }
    }

    fn state_name_of(&self, current: Option<Layer>) -> String {
        match current {
            None => "entry".to_string(),
            Some(layer) => match &self.states[layer.state] {
                MachineState::Entry => "entry".to_string(),
                MachineState::Exit => "exit".to_string(),
                MachineState::Animation { name, .. } | MachineState::Single { name, .. } => {
                    name.clone()
                }
            },
        }
    }
}

fn advance_layer(
    states: &[MachineState],
    animations: &HashMap<String, Animation>,
    layer: &mut Layer,
    dt: f64,
) {
    let speed = match &states[layer.state] {
        MachineState::Animation { animation, .. } => animations
            .get(animation)
            .map(|a| f64::from(a.speed()))
            .unwrap_or(1.0),
        _ => 1.0,
    };
    layer.time += dt * speed;
    layer.state_time += dt;
}

fn state_name_of(state: &MachineState) -> Option<&str> {
    match state {
        MachineState::Animation { name, .. } | MachineState::Single { name, .. } => {
            Some(name.as_str())
        }
        MachineState::Entry | MachineState::Exit => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::{Keyframe, LoopMode, Track};

    fn anim(name: &str, from: f32, to: f32) -> Animation {
        Animation::new(name, 1.0).with_track(
            "x",
            Track::try_new(vec![
                Keyframe::new(0.0, Value::px(from)),
                Keyframe::new(1.0, Value::px(to)),
            ])
            .unwrap(),
        )
    }

    fn registry() -> HashMap<String, Animation> {
        HashMap::from([
            ("idle".to_string(), anim("idle", 0.0, 0.0)),
            ("wave".to_string(), anim("wave", 0.0, 100.0)),
            ("spin".to_string(), anim("spin", 0.0, 360.0)),
        ])
    }

    fn base_machine() -> StateMachine {
        StateMachine::try_new(
            vec![
                MachineState::entry(),
                MachineState::animation("idle", "idle"),
                MachineState::animation("wave", "wave"),
            ],
            vec![
                Transition::new(FromState::Entry, ToState::State("idle".into()), vec![]),
                Transition::new(
                    FromState::State("idle".into()),
                    ToState::State("wave".into()),
                    vec![Condition::Trigger("go".into())],
                )
                .with_mix(0.5),
            ],
            registry(),
        )
        .expect("valid machine")
    }

    #[test]
    fn invalid_machines_are_rejected() {
        assert_eq!(
            StateMachine::try_new(vec![], vec![], HashMap::new()).err(),
            Some(MachineError::NoStates)
        );
        assert_eq!(
            StateMachine::try_new(
                vec![MachineState::animation("a", "idle")],
                vec![],
                registry()
            )
            .err(),
            Some(MachineError::MissingEntry)
        );
        assert_eq!(
            StateMachine::try_new(
                vec![
                    MachineState::entry(),
                    MachineState::animation("idle", "missing"),
                ],
                vec![],
                registry()
            )
            .err(),
            Some(MachineError::UnknownAnimation("missing".into()))
        );
        assert_eq!(
            StateMachine::try_new(
                vec![
                    MachineState::entry(),
                    MachineState::animation("idle", "idle")
                ],
                vec![Transition::new(
                    FromState::Entry,
                    ToState::State("nope".into()),
                    vec![]
                )],
                registry()
            )
            .err(),
            Some(MachineError::UnknownState("nope".into()))
        );
        assert_eq!(
            StateMachine::try_new(
                vec![
                    MachineState::entry(),
                    MachineState::entry(),
                    MachineState::animation("idle", "idle"),
                ],
                vec![],
                registry()
            )
            .err(),
            Some(MachineError::DuplicateEntryOrExit)
        );
    }

    #[test]
    fn entry_transition_starts_playback() {
        let mut sm = base_machine();
        sm.advance(0.0);
        assert_eq!(sm.current_state(), "idle");
        assert!(sm.is_active(), "once animation is not finished");
    }

    #[test]
    fn trigger_fires_transition() {
        let mut sm = base_machine();
        sm.advance(0.0);
        assert_eq!(sm.current_state(), "idle");

        sm.fire("go");
        sm.advance(0.0);
        assert_eq!(sm.current_state(), "idle", "still mixing");
        assert!(sm.is_active(), "crossfade is in flight");

        sm.advance(0.6);
        sm.advance(0.01);
        assert_eq!(sm.current_state(), "wave");
    }

    #[test]
    fn trigger_is_consumed_after_firing() {
        let mut sm = base_machine();
        sm.advance(0.0);
        sm.fire("go");
        sm.advance(0.6);
        sm.advance(0.6);
        assert_eq!(sm.current_state(), "wave");

        sm.advance(0.1);
        assert_eq!(sm.current_state(), "wave");
    }

    #[test]
    fn boolean_and_number_conditions_gate_transitions() {
        let mut sm = StateMachine::try_new(
            vec![
                MachineState::entry(),
                MachineState::animation("idle", "idle"),
                MachineState::animation("wave", "wave"),
                MachineState::animation("spin", "spin"),
            ],
            vec![
                Transition::new(FromState::Entry, ToState::State("idle".into()), vec![]),
                Transition::new(
                    FromState::State("idle".into()),
                    ToState::State("wave".into()),
                    vec![Condition::BooleanEquals("enabled".into(), true)],
                ),
                Transition::new(
                    FromState::State("idle".into()),
                    ToState::State("spin".into()),
                    vec![Condition::NumberCompare("level".into(), CmpOp::Gt, 3.0)],
                ),
            ],
            registry(),
        )
        .expect("valid");

        sm.advance(0.0);
        sm.set_boolean("enabled", false);
        sm.advance(0.1);
        assert_eq!(sm.current_state(), "idle");

        sm.set_boolean("enabled", true);
        sm.advance(0.1);
        assert_eq!(sm.current_state(), "wave");

        let mut sm2 = StateMachine::try_new(sm.states.clone(), sm.transitions.clone(), registry())
            .expect("valid");
        sm2.advance(0.0);
        sm2.set_number("level", 5.0);
        sm2.advance(0.1);
        assert_eq!(sm2.current_state(), "spin");
    }

    #[test]
    fn any_transition_overrides_state_transitions() {
        let mut sm = StateMachine::try_new(
            vec![
                MachineState::entry(),
                MachineState::animation("idle", "idle"),
                MachineState::animation("wave", "wave"),
            ],
            vec![
                Transition::new(FromState::Entry, ToState::State("idle".into()), vec![]),
                Transition::new(
                    FromState::Any,
                    ToState::State("wave".into()),
                    vec![Condition::Trigger("panic".into())],
                ),
            ],
            registry(),
        )
        .expect("valid");
        sm.advance(0.0);
        assert_eq!(sm.current_state(), "idle");
        sm.fire("panic");
        sm.advance(0.1);
        assert_eq!(sm.current_state(), "wave");
    }

    #[test]
    fn after_seconds_auto_fires() {
        let mut sm = StateMachine::try_new(
            vec![
                MachineState::entry(),
                MachineState::animation("idle", "idle"),
                MachineState::animation("wave", "wave"),
            ],
            vec![
                Transition::new(FromState::Entry, ToState::State("idle".into()), vec![]),
                Transition::new(
                    FromState::State("idle".into()),
                    ToState::State("wave".into()),
                    vec![Condition::AfterSeconds(2.0)],
                )
                .with_mix(0.1),
            ],
            registry(),
        )
        .expect("valid");
        sm.advance(0.5);
        sm.advance(0.5);
        assert_eq!(sm.current_state(), "idle", "1s total, gate is 2s");
        sm.advance(1.0);
        assert_eq!(sm.current_state(), "idle", "1.5s in idle, gate is 2s");
        sm.advance(0.5);
        assert_eq!(
            sm.current_state(),
            "idle",
            "2s reached: transition just fired, still mixing from idle"
        );
        sm.advance(0.2);
        assert_eq!(sm.current_state(), "wave");
    }

    #[test]
    fn crossfade_completes_with_small_steps() {
        let mut sm = StateMachine::try_new(
            vec![
                MachineState::entry(),
                MachineState::animation("idle", "idle"),
                MachineState::animation("wave", "wave"),
            ],
            vec![
                Transition::new(FromState::Entry, ToState::State("idle".into()), vec![]),
                Transition::new(
                    FromState::State("idle".into()),
                    ToState::State("wave".into()),
                    vec![Condition::AfterSeconds(1.0)],
                )
                .with_mix(0.5),
            ],
            registry(),
        )
        .expect("valid");
        for _ in 0..75 {
            sm.advance(0.016);
        }
        for _ in 0..64 {
            sm.advance(0.016);
        }
        assert_eq!(sm.current_state(), "wave");
        assert!(
            sm.mix.is_none(),
            "crossfade must complete, not restart every frame"
        );
    }

    #[test]
    fn crossfade_blends_property_values() {
        let mut sm = base_machine();
        sm.advance(0.0);
        assert_eq!(sm.frame().get("x"), Some(&Value::px(0.0)));

        sm.fire("go");
        sm.advance(0.0);
        sm.advance(0.125);
        let x = sm.frame().get("x").copied().unwrap();
        assert!(
            matches!(x, Value::Px(v) if v > 0.0 && v < 100.0),
            "blended value should sit between idle and wave: {x:?}"
        );

        sm.advance(0.5);
        assert_eq!(sm.current_state(), "wave");
        sm.advance(1.0);
        assert_eq!(sm.frame().get("x"), Some(&Value::px(100.0)));
    }

    #[test]
    fn exit_state_stops_playback() {
        let mut sm = StateMachine::try_new(
            vec![
                MachineState::entry(),
                MachineState::animation("idle", "idle"),
                MachineState::exit(),
            ],
            vec![
                Transition::new(FromState::Entry, ToState::State("idle".into()), vec![]),
                Transition::new(
                    FromState::State("idle".into()),
                    ToState::Exit,
                    vec![Condition::Trigger("stop".into())],
                ),
            ],
            registry(),
        )
        .expect("valid");
        sm.advance(0.0);
        assert!(sm.is_active());
        sm.fire("stop");
        sm.advance(0.1);
        assert_eq!(sm.current_state(), "exit");
        assert!(!sm.is_active());
        assert!(sm.frame().is_empty());
    }

    #[test]
    fn single_state_outputs_static_properties() {
        let mut sm = StateMachine::try_new(
            vec![
                MachineState::entry(),
                MachineState::single(
                    "frozen",
                    HashMap::from([("x".to_string(), Value::px(42.0))]),
                ),
            ],
            vec![Transition::new(
                FromState::Entry,
                ToState::State("frozen".into()),
                vec![],
            )],
            registry(),
        )
        .expect("valid");
        sm.advance(0.0);
        assert_eq!(sm.current_state(), "frozen");
        assert_eq!(sm.frame().get("x"), Some(&Value::px(42.0)));
        assert!(!sm.is_active());
    }

    #[test]
    fn looping_animation_stays_active() {
        let looped = anim("idle", 0.0, 0.0).with_loop(LoopMode::Loop);
        let mut sm = StateMachine::try_new(
            vec![
                MachineState::entry(),
                MachineState::animation("idle", "idle"),
            ],
            vec![Transition::new(
                FromState::Entry,
                ToState::State("idle".into()),
                vec![],
            )],
            HashMap::from([("idle".to_string(), looped)]),
        )
        .expect("valid");
        sm.advance(0.0);
        sm.advance(100.0);
        assert_eq!(sm.current_state(), "idle");
        assert!(sm.is_active(), "looping animation never finishes");
    }

    #[test]
    fn once_animation_goes_inactive_when_done() {
        let mut sm = base_machine();
        sm.advance(0.0);
        sm.advance(2.0);
        assert_eq!(sm.current_state(), "idle");
        assert!(!sm.is_active());
    }

    #[test]
    fn speed_scales_playback_time() {
        let fast = anim("idle", 0.0, 0.0).with_speed(2.0);
        let mut sm = StateMachine::try_new(
            vec![
                MachineState::entry(),
                MachineState::animation("idle", "idle"),
            ],
            vec![Transition::new(
                FromState::Entry,
                ToState::State("idle".into()),
                vec![],
            )],
            HashMap::from([("idle".to_string(), fast)]),
        )
        .expect("valid");
        sm.advance(0.0);
        sm.advance(0.5);
        assert!(!sm.is_active());
    }
}
