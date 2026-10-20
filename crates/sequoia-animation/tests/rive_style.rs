use std::collections::HashMap;

use sequoia_animation::{
    AnimValue, Animation, AnimationController, Animator, Condition, CubicBezier, Easing, FromState,
    Interpolation, Keyframe, LoopMode, MachineState, Mixer, Spring, SpringMachine, StateDef,
    StateMachine, ToState, Track, Transition, TransitionDef, Trigger, Tween, Value,
};
use sequoia_core::{Color, Point};

fn linear_clip(name: &str, dur: f64, to: f32) -> Animation {
    Animation::new(name, dur).with_track(
        "x",
        Track::try_new(vec![
            Keyframe::new(0.0, Value::px(0.0)),
            Keyframe::new(dur, Value::px(to)),
        ])
        .unwrap(),
    )
}

fn registry() -> HashMap<String, Animation> {
    HashMap::from([
        ("idle".to_string(), linear_clip("idle", 1.0, 0.0)),
        ("wave".to_string(), linear_clip("wave", 1.0, 100.0)),
    ])
}

fn two_state_machine(mix: f64) -> StateMachine {
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
            .with_mix(mix),
        ],
        registry(),
    )
    .unwrap()
}

#[test]
fn clip_samples_linear_midpoint() {
    let a = linear_clip("slide", 1.0, 200.0);
    assert_eq!(a.sample(0.0).get("x"), Some(&Value::px(0.0)));
    assert_eq!(a.sample(0.5).get("x"), Some(&Value::px(100.0)));
    assert_eq!(a.sample(2.0).get("x"), Some(&Value::px(200.0)));
}

#[test]
fn clip_loop_wraps_like_rive() {
    let a = linear_clip("spin", 1.0, 360.0).with_loop(LoopMode::Loop);

    assert_eq!(a.sample(2.25).get("x"), Some(&Value::px(90.0)));
}

#[test]
fn clip_pingpong_mirrors() {
    let a = linear_clip("pp", 1.0, 100.0).with_loop(LoopMode::PingPong);

    assert_eq!(a.sample(0.75).get("x"), Some(&Value::px(75.0)));
    assert_eq!(a.sample(1.25).get("x"), Some(&Value::px(75.0)));
}

#[test]
fn clip_speed_scales_before_sampling() {
    let a = linear_clip("fast", 1.0, 100.0).with_speed(2.0);
    let t = 0.25 * f64::from(a.speed());
    assert_eq!(a.sample(t).get("x"), Some(&Value::px(50.0)));
}

#[test]
fn hold_keyframes_step_like_rive() {
    let t = Track::try_new(vec![
        Keyframe::hold(0.0, Value::f32(1.0)),
        Keyframe::new(1.0, Value::f32(9.0)),
    ])
    .unwrap();
    assert_eq!(t.sample(0.0), Some(Value::f32(1.0)));
    assert_eq!(t.sample(0.999), Some(Value::f32(1.0)));
    assert_eq!(t.sample(1.0), Some(Value::f32(9.0)));
}

#[test]
fn bezier_keyframe_matches_css_ease() {
    let ease = CubicBezier::new(0.25, 0.1, 0.25, 1.0);
    let t = Track::try_new(vec![
        Keyframe::new(0.0, Value::px(0.0)).with_interpolation(Interpolation::Bezier(ease)),
        Keyframe::new(1.0, Value::px(100.0)),
    ])
    .unwrap();
    let v = t.sample(0.5).unwrap().as_px().unwrap();
    assert!(v > 50.0 && v < 100.0, "ease advances line halfway: {v}");
}

#[test]
fn mismatched_keyframe_types_hold() {
    let t = Track::try_new(vec![
        Keyframe::new(0.0, Value::f32(5.0)),
        Keyframe::new(1.0, Value::px(100.0)),
    ])
    .unwrap();
    assert_eq!(t.sample(0.5), Some(Value::f32(5.0)));
}

#[test]
fn mixer_crossfades_two_clips() {
    let idle = linear_clip("idle", 1.0, 0.0);
    let wave = linear_clip("wave", 1.0, 100.0);

    let out = Mixer::mix([(&idle, 0.0, 0.75), (&wave, 0.5, 0.25)]);
    assert_eq!(out.get("x"), Some(&Value::px(12.5)));
}

#[test]
fn mixer_ignores_silent_layers() {
    let a = linear_clip("a", 1.0, 10.0);
    let b = linear_clip("b", 1.0, 80.0);
    let out = Mixer::mix([(&a, 0.0, 0.0), (&b, 1.0, 2.0)]);
    assert_eq!(out.get("x"), Some(&Value::px(80.0)));
}

#[test]
fn machine_full_trigger_flow_with_mix() {
    let mut sm = two_state_machine(0.5);
    sm.advance(0.0);
    assert_eq!(sm.current_state(), "idle");
    sm.fire("go");
    sm.advance(0.0);
    sm.advance(0.25);
    let x = sm.frame().get("x").copied().unwrap().as_px().unwrap();
    assert!(x > 0.0 && x < 100.0, "blend w locie: {x}");
    sm.advance(0.5);
    sm.advance(0.01);
    assert_eq!(sm.current_state(), "wave");

    sm.advance(0.5);
    assert_eq!(sm.current_state(), "wave");
}

#[test]
fn machine_instant_switch_without_mix() {
    let mut sm = two_state_machine(0.0);
    sm.advance(0.0);
    sm.fire("go");
    sm.advance(0.1);
    assert_eq!(sm.current_state(), "wave");
    assert_eq!(sm.frame().get("x"), Some(&Value::px(0.0)));
    sm.advance(0.1);
    assert_eq!(sm.frame().get("x"), Some(&Value::px(10.0)));
}

#[test]
fn machine_boolean_gate() {
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
                vec![Condition::BooleanEquals("on".into(), true)],
            ),
        ],
        registry(),
    )
    .unwrap();
    sm.advance(0.0);
    sm.set_boolean("on", false);
    sm.advance(0.5);
    assert_eq!(sm.current_state(), "idle");
    sm.set_boolean("on", true);
    sm.advance(0.1);
    assert_eq!(sm.current_state(), "wave");
}

#[test]
fn machine_any_state_panic_button() {
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
    .unwrap();
    sm.advance(0.0);
    sm.fire("panic");
    sm.advance(0.05);
    assert_eq!(sm.current_state(), "wave");
}

#[test]
fn machine_exit_stops_output() {
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
    .unwrap();
    sm.advance(0.0);
    assert!(sm.is_active());
    sm.fire("stop");
    sm.advance(0.1);
    assert_eq!(sm.current_state(), "exit");
    assert!(!sm.is_active());
    assert!(sm.frame().is_empty());
}

#[test]
fn spring_response_shape() {
    let soft = Spring::new(0.4, 2.0);
    assert_eq!(soft.value(0.0), 0.0);

    let mut peak = 0.0f32;
    let mut t = 0.0;
    while t < 2.0 {
        peak = peak.max(soft.value(t));
        t += 0.002;
    }
    assert!(peak > 1.02, "peak={peak}");

    let st = soft.settle_time(0.001);
    assert!((soft.value(st) - 1.0).abs() < 0.005);

    let crit = Spring::new(1.0, 4.0);
    let mut cmax = 0.0f32;
    let mut t = 0.0;
    while t < 2.0 {
        cmax = cmax.max(crit.value(t));
        t += 0.002;
    }
    assert!(cmax <= 1.0 + 1e-4, "cmax={cmax}");
}

#[test]
fn spring_machine_flight_and_settle() {
    let mut m = SpringMachine::new(
        vec![
            StateDef::new(
                "idle",
                AnimValue::static_px(200.0),
                AnimValue::static_px(0.0),
            ),
            StateDef::new(
                "drop",
                AnimValue::spring(0.0, 0.6, 2.5),
                AnimValue::static_px(0.0),
            ),
        ],
        vec![TransitionDef::new("idle", "drop", Trigger::Load)],
    );
    assert!(m.advance(0.0));
    assert_eq!(m.offset().x, 200.0);
    for _ in 0..600 {
        if !m.advance(1.0 / 60.0) {
            break;
        }
    }
    assert!(!m.is_flying());
    assert!((m.offset().x).abs() < 0.01);
}

#[test]
fn animator_plays_clip_to_end_and_holds() {
    let mut p = Animator::new().with_clip(linear_clip("s", 0.5, 100.0));
    let mut last = 0.0;
    let mut frames = 0;
    while p.advance(1.0 / 60.0) {
        assert!(p.offset().x >= last, "monotonicznie");
        last = p.offset().x;
        frames += 1;
        assert!(frames < 1000, "must finish");
    }
    assert!(frames >= 20, "lot widoczny przez wiele klatek ({frames})");
    assert_eq!(p.offset().x, 100.0);
}

#[test]
fn animator_combines_spring_and_clip_offsets() {
    let spring = SpringMachine::new(
        vec![StateDef::new(
            "s",
            AnimValue::static_px(5.0),
            AnimValue::static_px(0.0),
        )],
        vec![],
    );
    let mut p = Animator::new()
        .with_clip(linear_clip("s", 1.0, 40.0))
        .with_spring(spring);
    p.advance(0.5);
    assert!(
        (p.offset().x - 25.0).abs() < 1e-4,
        "5 + 20 = {}",
        p.offset().x
    );
}

#[test]
fn controller_advance_by_is_linear_and_exact() {
    let mut c = AnimationController::new(0.5, Easing::Linear);
    c.start();
    assert!(!c.advance_by(0.25));
    assert!((c.value - 0.5).abs() < 1e-6, "half: {}", c.value);
    assert!(c.advance_by(0.25));
    assert_eq!(c.value, 1.0);
    assert!(!c.running);
}

#[test]
fn controller_advance_by_applies_easing() {
    let mut lin = AnimationController::new(1.0, Easing::Linear);
    let mut out = AnimationController::new(1.0, Easing::EaseOut);
    lin.start();
    out.start();
    lin.advance_by(0.5);
    out.advance_by(0.5);
    assert!(out.value > lin.value, "ease-out ahead of the line");
    assert_eq!(lin.value, 0.5);
}

#[test]
fn controller_retarget_midflight() {
    let mut c = AnimationController::new(1.0, Easing::Linear);
    c.start();
    c.advance_by(0.5);
    assert!((c.value - 0.5).abs() < 1e-6);
    c.animate_to(0.0);
    assert!(!c.advance_by(0.25));
    assert!((c.value - 0.375).abs() < 1e-6, "wraca: {}", c.value);
    assert!(c.advance_by(0.75));
    assert_eq!(c.value, 0.0);
}

#[test]
fn tween_advance_by_with_delay_and_color() {
    let mut t = Tween::new(1.0, Color::BLACK, Color::WHITE, Easing::Linear).with_delay(0.5);
    t.start();
    assert!(!t.advance_by(0.25), "delay jeszcze trzyma");
    assert_eq!(t.value, Color::BLACK);
    assert!(!t.advance_by(0.5));
    let (r, _, _, _) = t.value.to_rgba8();
    assert!((r as i16 - 64).abs() <= 1, "quarter of the way: r={r}");
    assert!(t.advance_by(1.0));
    assert_eq!(t.value, Color::WHITE);
    assert!(t.finished);
}

#[test]
fn tween_point_path() {
    let mut t = Tween::new(1.0, Point::ZERO, Point::new(10.0, 20.0), Easing::Linear);
    t.start();
    t.advance_by(0.5);
    assert_eq!(t.value, Point::new(5.0, 10.0));
}

#[test]
fn value_lerp_and_mix_spot_checks() {
    let px = Value::px(0.0).lerp(&Value::px(100.0), 0.3).as_px().unwrap();
    assert!((px - 30.0).abs() < 1e-4, "px={px}");
    let a = Value::px(0.0);
    let b = Value::px(100.0);
    assert_eq!(Value::mix([(&a, 1.0), (&b, 3.0)]), Some(Value::px(75.0)));

    let mid = Value::color(Color::BLACK)
        .lerp(&Value::color(Color::WHITE), 0.5)
        .as_color()
        .unwrap();
    let (r, g, b) = mid.to_rgb8();
    assert_eq!((r, g, b), (128, 128, 128));
}
