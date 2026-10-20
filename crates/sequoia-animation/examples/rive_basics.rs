use std::collections::HashMap;

use sequoia_animation::{
    AnimValue, Animation, Animator, Condition, FromState, Keyframe, LoopMode, MachineState, Mixer,
    Spring, SpringMachine, StateDef, StateMachine, ToState, Track, Transition, TransitionDef,
    Trigger, Value,
};

fn main() {
    let clip = Animation::new("slide", 0.4)
        .with_track(
            "x",
            Track::try_new(vec![
                Keyframe::new(0.0, Value::px(0.0)),
                Keyframe::new(0.4, Value::px(120.0)),
            ])
            .unwrap(),
        )
        .with_track(
            "opacity",
            Track::try_new(vec![
                Keyframe::new(0.0, Value::f32(0.0)),
                Keyframe::new(0.4, Value::f32(1.0)),
            ])
            .unwrap(),
        );
    println!("--- klip Once (dt = 1/60) ---");
    let mut player = Animator::new().with_clip(clip);
    let mut n = 0;
    while player.advance(1.0 / 60.0) {
        n += 1;
        if n % 6 == 0 {
            println!(
                "t={:.2}s x={:.1} opacity={:.2}",
                player.elapsed(),
                player.offset().x,
                player.opacity().unwrap()
            );
        }
    }
    println!("koniec po {n} klatkach, x={}", player.offset().x);

    let pulse = Animation::new("pulse", 0.5)
        .with_loop(LoopMode::Loop)
        .with_track(
            "x",
            Track::try_new(vec![
                Keyframe::new(0.0, Value::px(0.0)),
                Keyframe::new(0.5, Value::px(50.0)),
            ])
            .unwrap(),
        );
    println!(
        "--- klip Loop: t=1.3 → {:.1} (wrap 0.3) ---",
        pulse.sample(1.3).get("x").unwrap().as_px().unwrap()
    );

    let idle = Animation::new("idle", 1.0).with_track(
        "x",
        Track::try_new(vec![Keyframe::new(0.0, Value::px(0.0))]).unwrap(),
    );
    let wave = Animation::new("wave", 1.0).with_track(
        "x",
        Track::try_new(vec![
            Keyframe::new(0.0, Value::px(0.0)),
            Keyframe::new(1.0, Value::px(100.0)),
        ])
        .unwrap(),
    );
    let blended = Mixer::mix([(&idle, 0.0, 0.5), (&wave, 0.5, 0.5)]);
    println!(
        "--- mixer 50/50 @ wave(0.5): x={:.1} ---",
        blended.get("x").unwrap().as_px().unwrap()
    );

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
                vec![Condition::Trigger("go".into())],
            )
            .with_mix(0.5),
        ],
        HashMap::from([("idle".into(), idle), ("wave".into(), wave)]),
    )
    .unwrap();
    sm.advance(0.0);
    println!("--- maszyna: start w '{}' ---", sm.current_state());
    sm.fire("go");
    sm.advance(0.0);
    sm.advance(0.25);
    println!(
        "--- maszyna: w locie x={:.1}, gra={} ---",
        sm.frame().get("x").unwrap().as_px().unwrap(),
        sm.is_active()
    );

    let mut spring = SpringMachine::new(
        vec![
            StateDef::new(
                "idle",
                AnimValue::static_px(200.0),
                AnimValue::static_px(0.0),
            ),
            StateDef::new(
                "drop",
                AnimValue::spring(0.0, 0.4, 3.0),
                AnimValue::static_px(0.0),
            ),
        ],
        vec![TransitionDef::new("idle", "drop", Trigger::Load)],
    );
    spring.advance(0.0);
    let mut min = f32::INFINITY;
    for _ in 0..120 {
        spring.advance(1.0 / 60.0);
        min = min.min(spring.offset().x);
    }
    println!("--- spring: start 200 → target 0, min in flight {min:.1} (overshoot) ---");

    let s = Spring::new(0.5, 4.0);
    println!(
        "--- spring(0.5, 4Hz): t=0 → {:.2}, settle ≈ {:.2}s ---",
        s.value(0.0),
        s.settle_time(0.001)
    );
}
