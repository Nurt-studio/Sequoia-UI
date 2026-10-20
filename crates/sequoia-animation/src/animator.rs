use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use sequoia_core::{Color, Point};

use crate::MAX_FRAME_DT;
use crate::animation::{Animation, LoopMode};
use crate::machine::StateMachine;
use crate::spring_machine::SpringMachine;
use crate::value::Value;

#[derive(Debug)]
struct Inner {
    clip: Option<Animation>,
    machine: Option<StateMachine>,
    spring: Option<SpringMachine>,
    autoplay: bool,
    playing: bool,

    elapsed: f64,

    last_frame: Option<Instant>,

    offset: Point,
    width: Option<f32>,
    height: Option<f32>,
    background: Option<Color>,
    opacity: Option<f32>,
}

#[derive(Clone, Debug)]
pub struct Animator {
    inner: Rc<RefCell<Inner>>,
}

impl Animator {
    pub fn new() -> Self {
        Self {
            inner: Rc::new(RefCell::new(Inner {
                clip: None,
                machine: None,
                spring: None,
                autoplay: true,
                playing: true,
                elapsed: 0.0,
                last_frame: None,
                offset: Point::default(),
                width: None,
                height: None,
                background: None,
                opacity: None,
            })),
        }
    }

    pub fn with_clip(self, clip: Animation) -> Self {
        self.inner.borrow_mut().clip = Some(clip);
        self
    }

    pub fn with_machine(self, machine: StateMachine) -> Self {
        self.inner.borrow_mut().machine = Some(machine);
        self
    }

    pub fn with_spring(self, spring: SpringMachine) -> Self {
        self.inner.borrow_mut().spring = Some(spring);
        self
    }

    pub fn with_autoplay(self, on: bool) -> Self {
        {
            let mut inner = self.inner.borrow_mut();
            inner.autoplay = on;
            inner.playing = on;
        }
        self
    }

    pub fn set_clip(&mut self, clip: Animation) {
        let mut inner = self.inner.borrow_mut();
        inner.clip = Some(clip);
        inner.elapsed = 0.0;
    }

    pub fn play(&mut self) {
        let mut inner = self.inner.borrow_mut();
        inner.playing = true;
        inner.elapsed = 0.0;
        inner.last_frame = Some(Instant::now());
    }

    pub fn pause(&mut self) {
        self.inner.borrow_mut().playing = false;
    }

    pub fn toggle_play(&mut self) {
        let playing = self.inner.borrow().playing;
        if playing {
            self.pause();
        } else {
            self.play();
        }
    }

    pub fn fire(&mut self, trigger: impl Into<String>) {
        if let Some(m) = self.inner.borrow_mut().machine.as_mut() {
            m.fire(trigger);
        }
    }

    pub fn set_boolean(&mut self, name: impl Into<String>, value: bool) {
        if let Some(m) = self.inner.borrow_mut().machine.as_mut() {
            m.set_boolean(name, value);
        }
    }

    pub fn set_number(&mut self, name: impl Into<String>, value: f32) {
        if let Some(m) = self.inner.borrow_mut().machine.as_mut() {
            m.set_number(name, value);
        }
    }

    pub fn advance(&mut self, dt: f64) -> bool {
        let dt = dt.max(0.0);

        let clip = self.inner.borrow().clip.clone();
        let mut inner = self.inner.borrow_mut();
        inner.offset = Point::default();
        inner.width = None;
        inner.height = None;
        inner.background = None;
        inner.opacity = None;
        let mut running = false;

        if let Some(spring) = inner.spring.as_mut() {
            running |= spring.advance(dt);
            let p = spring.offset();
            inner.offset.x += p.x;
            inner.offset.y += p.y;
        }

        if let Some(machine) = inner.machine.as_mut() {
            running |= machine.advance(dt);
            let frame = machine.frame().clone();
            apply_frame_additive(&mut inner, &frame);
        }

        let Some(clip) = clip.as_ref() else {
            return running;
        };
        if inner.playing {
            inner.elapsed += dt;
        }
        let t = inner.elapsed * f64::from(clip.speed());
        let frame = clip.sample(t);
        apply_clip_frame(&mut inner, &frame);
        if !inner.playing {
            return running;
        }
        running
            || match clip.loop_mode() {
                LoopMode::Once => !clip.is_finished_at(t),

                LoopMode::Loop | LoopMode::PingPong => true,
            }
    }

    pub fn advance_frame(&mut self) -> bool {
        let now = Instant::now();
        let dt = {
            let mut inner = self.inner.borrow_mut();
            let dt = inner
                .last_frame
                .map(|last| now.duration_since(last).as_secs_f64())
                .unwrap_or(0.0)
                .clamp(0.0, f64::from(MAX_FRAME_DT));
            inner.last_frame = Some(now);
            dt
        };
        self.advance(dt)
    }

    pub fn offset(&self) -> Point {
        self.inner.borrow().offset
    }

    pub fn width(&self) -> Option<f32> {
        self.inner.borrow().width
    }

    pub fn height(&self) -> Option<f32> {
        self.inner.borrow().height
    }

    pub fn background(&self) -> Option<Color> {
        self.inner.borrow().background
    }

    pub fn opacity(&self) -> Option<f32> {
        self.inner.borrow().opacity
    }

    pub fn elapsed(&self) -> f64 {
        self.inner.borrow().elapsed
    }

    pub fn is_playing(&self) -> bool {
        self.inner.borrow().playing
    }

    pub fn state_name(&self) -> String {
        self.inner
            .borrow()
            .machine
            .as_ref()
            .map(|m| m.current_state().to_string())
            .unwrap_or_default()
    }
}

impl Default for Animator {
    fn default() -> Self {
        Self::new()
    }
}

fn apply_clip_frame(inner: &mut Inner, frame: &crate::AnimationFrame) {
    if let Some(v) = frame.get("x").and_then(Value::as_px) {
        inner.offset.x += v;
    }
    if let Some(v) = frame.get("y").and_then(Value::as_px) {
        inner.offset.y += v;
    }
    if let Some(v) = frame.get("width").and_then(Value::as_px) {
        inner.width = Some(v);
    }
    if let Some(v) = frame.get("height").and_then(Value::as_px) {
        inner.height = Some(v);
    }
    if let Some(v) = frame
        .get("background")
        .or_else(|| frame.get("color"))
        .and_then(Value::as_color)
    {
        inner.background = Some(v);
    }
    if let Some(v) = frame.get("opacity").and_then(Value::as_f32) {
        inner.opacity = Some(v.clamp(0.0, 1.0));
    }
}

fn apply_frame_additive(inner: &mut Inner, frame: &crate::AnimationFrame) {
    apply_clip_frame(inner, frame);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Keyframe, Track};

    fn slide(dur: f64, to: f32) -> Animation {
        Animation::new("slide", dur).with_track(
            "x",
            Track::try_new(vec![
                Keyframe::new(0.0, Value::px(0.0)),
                Keyframe::new(dur, Value::px(to)),
            ])
            .unwrap(),
        )
    }

    #[test]
    fn first_frame_starts_at_first_keyframe() {
        let mut a = Animator::new().with_clip(slide(1.0, 100.0));
        assert!(a.advance(0.0), "klip Once leci od t = 0");
        assert_eq!(a.offset(), Point::new(0.0, 0.0));
    }

    #[test]
    fn once_clip_travels_then_holds_last_frame() {
        let mut a = Animator::new().with_clip(slide(0.4, 100.0));
        assert!(a.advance(0.1));
        let x1 = a.offset().x;
        assert!(x1 > 0.0 && x1 < 100.0, "center of flight: {x1}");
        assert!(a.advance(0.1));
        let x2 = a.offset().x;
        assert!(x2 > x1, "leci do przodu ({x1} -> {x2})");
        assert!(a.advance(0.15));
        assert!(
            (a.offset().x - 87.5).abs() < 1e-4,
            "t=0.35: {}",
            a.offset().x
        );
        assert!(!a.advance(0.1));
        assert_eq!(a.offset().x, 100.0, "holds the last frame");
        assert!(!a.advance(0.016), "at the end of Once stands");
    }

    #[test]
    fn loop_clip_wraps_and_never_settles() {
        let clip = Animation::new("pulse", 0.2)
            .with_loop(LoopMode::Loop)
            .with_track(
                "x",
                Track::try_new(vec![
                    Keyframe::new(0.0, Value::px(0.0)),
                    Keyframe::new(0.2, Value::px(50.0)),
                ])
                .unwrap(),
            );
        let mut a = Animator::new().with_clip(clip);
        assert!(a.advance(0.25));
        assert!((a.offset().x - 12.5).abs() < 1e-4, "wrap: {}", a.offset().x);
        assert!(a.advance(10.0), "Loop nigdy nie staje");
    }

    #[test]
    fn pingpong_comes_back() {
        let clip = Animation::new("pp", 1.0)
            .with_loop(LoopMode::PingPong)
            .with_track(
                "x",
                Track::try_new(vec![
                    Keyframe::new(0.0, Value::px(0.0)),
                    Keyframe::new(1.0, Value::px(100.0)),
                ])
                .unwrap(),
            );
        let mut a = Animator::new().with_clip(clip);
        a.advance(0.25);
        assert!((a.offset().x - 25.0).abs() < 1e-4);
        a.advance(1.25);
        assert!(
            (a.offset().x - 50.0).abs() < 1e-4,
            "ping-pong: {}",
            a.offset().x
        );
    }

    #[test]
    fn autoplay_false_freezes_until_play() {
        let mut a = Animator::new()
            .with_clip(slide(0.4, 100.0))
            .with_autoplay(false);
        assert!(!a.is_playing());
        assert!(!a.advance(0.1), "zapauzowany nie gra");
        assert_eq!(a.offset().x, 0.0);
        a.play();
        assert!(a.is_playing());
        assert!(a.advance(0.1));
        assert!(a.offset().x > 0.0, "po play leci");
        a.pause();
        let frozen = a.offset().x;
        assert!(!a.advance(0.1));
        assert_eq!(a.offset().x, frozen, "pause freezes sample");
    }

    #[test]
    fn clones_share_playback_state() {
        let player = Animator::new().with_clip(slide(1.0, 100.0));
        let mut view = player.clone();
        assert!(view.advance(0.25));
        assert!((player.offset().x - 25.0).abs() < 1e-4, "klon dzieli stan");
        let mut other = player.clone();
        other.pause();
        assert!(!player.is_playing(), "pauza widoczna we wszystkich klonach");
    }

    #[test]
    fn width_height_background_opacity_tracks() {
        let clip = Animation::new("grow", 1.0)
            .with_track(
                "width",
                Track::try_new(vec![Keyframe::new(0.0, Value::px(10.0))]).unwrap(),
            )
            .with_track(
                "background",
                Track::try_new(vec![Keyframe::new(0.0, Value::color(Color::GREEN))]).unwrap(),
            )
            .with_track(
                "opacity",
                Track::try_new(vec![
                    Keyframe::new(0.0, Value::f32(1.0)),
                    Keyframe::new(1.0, Value::f32(0.0)),
                ])
                .unwrap(),
            );
        let mut a = Animator::new().with_clip(clip);
        a.advance(0.0);
        assert_eq!(a.width(), Some(10.0));
        assert_eq!(a.background(), Some(Color::GREEN));
        assert_eq!(a.opacity(), Some(1.0));
        a.advance(0.5);
        assert_eq!(a.opacity(), Some(0.5));
    }

    #[test]
    fn opacity_is_clamped() {
        let clip = Animation::new("o", 1.0).with_track(
            "opacity",
            Track::try_new(vec![Keyframe::new(0.0, Value::f32(42.0))]).unwrap(),
        );
        let mut a = Animator::new().with_clip(clip);
        a.advance(0.0);
        assert_eq!(a.opacity(), Some(1.0));
    }

    #[test]
    fn color_alias_track_works() {
        let clip = Animation::new("c", 1.0).with_track(
            "color",
            Track::try_new(vec![Keyframe::new(0.0, Value::color(Color::RED))]).unwrap(),
        );
        let mut a = Animator::new().with_clip(clip);
        a.advance(0.0);
        assert_eq!(a.background(), Some(Color::RED));
    }

    #[test]
    fn spring_offset_adds_to_clip() {
        use crate::{AnimValue, SpringMachine, StateDef};
        let spring = SpringMachine::new(
            vec![
                StateDef::new("a", AnimValue::static_px(10.0), AnimValue::static_px(0.0)),
                StateDef::new("b", AnimValue::static_px(10.0), AnimValue::static_px(0.0)),
            ],
            vec![],
        );
        let mut a = Animator::new()
            .with_clip(slide(1.0, 100.0))
            .with_spring(spring);
        a.advance(0.5);
        assert!((a.offset().x - 60.0).abs() < 1e-4, "suma: {}", a.offset().x);
    }

    #[test]
    fn full_machine_trigger_drives_offset() {
        use crate::{Condition, FromState, MachineState, ToState, Transition};
        use std::collections::HashMap;
        let idle = Animation::new("idle", 1.0).with_track(
            "x",
            Track::try_new(vec![
                Keyframe::new(0.0, Value::px(0.0)),
                Keyframe::new(1.0, Value::px(0.0)),
            ])
            .unwrap(),
        );
        let wave = Animation::new("wave", 1.0).with_track(
            "x",
            Track::try_new(vec![
                Keyframe::new(0.0, Value::px(0.0)),
                Keyframe::new(1.0, Value::px(100.0)),
            ])
            .unwrap(),
        );
        let sm = StateMachine::try_new(
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
            HashMap::from([("idle".into(), idle), ("wave".into(), wave)]),
        )
        .unwrap();
        let mut a = Animator::new().with_machine(sm);
        a.advance(0.0);
        assert_eq!(a.state_name(), "idle");
        a.fire("go");
        a.advance(0.0);
        assert!(a.advance(0.25));
        let x = a.offset().x;
        assert!(x > 0.0 && x < 100.0, "blend w locie: {x}");
    }

    #[test]
    fn empty_animator_is_idle() {
        let mut a = Animator::new();
        assert!(!a.advance(0.016));
        assert_eq!(a.offset(), Point::ZERO);
        assert_eq!(a.width(), None);
        assert_eq!(a.opacity(), None);
    }

    #[test]
    fn negative_dt_is_clamped() {
        let mut a = Animator::new().with_clip(slide(1.0, 100.0));
        assert!(a.advance(-5.0));
        assert_eq!(a.offset().x, 0.0);
    }

    #[test]
    fn set_clip_restarts_time() {
        let mut a = Animator::new().with_clip(slide(1.0, 100.0));
        a.advance(0.75);
        assert!((a.offset().x - 75.0).abs() < 1e-4);
        a.set_clip(slide(1.0, 200.0));
        assert_eq!(a.elapsed(), 0.0);
        a.advance(0.5);
        assert!((a.offset().x - 100.0).abs() < 1e-4);
    }

    #[test]
    fn advance_frame_uses_wall_clock_without_teleport() {
        let mut a = Animator::new().with_clip(slide(60.0, 100.0));
        assert!(a.advance_frame());
        assert!(a.offset().x < 1.0, "the first frame barely moved");
    }
}
