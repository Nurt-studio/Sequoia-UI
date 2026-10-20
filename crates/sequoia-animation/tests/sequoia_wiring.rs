use sequoia_animation::{Animation, Animator, Easing, Keyframe, Track, Value};
use sequoia_core::{Color, Point};
use sequoia_reactive::Signal;

#[derive(Clone)]
struct ProgressApp {
    progress: Signal<f32>,
    color: Signal<Color>,
}

#[test]
fn signals_carry_animation_output() {
    let app = ProgressApp {
        progress: Signal::new(0.0),
        color: Signal::new(Color::BLACK),
    };

    let mut ctrl = sequoia_animation::AnimationController::new(1.0, Easing::Linear);
    ctrl.start();
    let mut frames = 0;
    while {
        let done = ctrl.advance_by(1.0 / 60.0);
        app.progress.set(ctrl.value);
        app.color.set(Color::rgba(ctrl.value, 0.0, 0.0, 1.0));
        frames += 1;
        !done
    } {}
    assert!((50..=70).contains(&frames), "60 fps × 1 s: {frames}");
    assert_eq!(app.progress.get_untracked(), 1.0);

    assert_eq!(app.progress.get(), 1.0);
}

#[test]
fn animator_handle_built_once_cloned_per_frame() {
    let clip = Animation::new("slide", 1.0).with_track(
        "x",
        Track::try_new(vec![
            Keyframe::new(0.0, Value::px(0.0)),
            Keyframe::new(1.0, Value::px(300.0)),
        ])
        .unwrap(),
    );

    let player = Animator::new().with_clip(clip);

    let mut positions = Vec::new();
    for _ in 0..60 {
        let mut frame_view = player.clone();
        frame_view.advance(1.0 / 60.0);
        positions.push(frame_view.offset().x);
    }
    assert!(positions.first().copied().unwrap_or(-1.0) >= 0.0);
    assert_eq!(*positions.last().unwrap(), 300.0);

    for w in positions.windows(2) {
        assert!(w[1] >= w[0], "lot nie cofa: {w:?}");
    }
}

#[test]
fn frame_loop_pattern_with_fixed_dt() {
    let clip = Animation::new("fade", 0.5).with_track(
        "opacity",
        Track::try_new(vec![
            Keyframe::new(0.0, Value::f32(0.0)),
            Keyframe::new(0.5, Value::f32(1.0)),
        ])
        .unwrap(),
    );
    let mut player = Animator::new().with_clip(clip);
    let mut request_frames = 0;
    let mut last_opacity = 0.0;
    while player.advance(1.0 / 60.0) {
        request_frames += 1;
        let o = player.opacity().unwrap();
        assert!(o >= last_opacity, "fade-in increases");
        last_opacity = o;
    }
    assert_eq!(player.opacity(), Some(1.0));
    assert!(
        (25..=35).contains(&request_frames),
        "0.5 s × 60 fps: {request_frames}"
    );
}

#[test]
fn frame_counters_stay_outside_signals() {
    use std::cell::RefCell;
    use std::rc::Rc;
    let frames: Rc<RefCell<u32>> = Rc::new(RefCell::new(0));
    let mut player = Animator::new().with_clip(
        Animation::new("s", 0.2).with_track(
            "x",
            Track::try_new(vec![
                Keyframe::new(0.0, Value::px(0.0)),
                Keyframe::new(0.2, Value::px(10.0)),
            ])
            .unwrap(),
        ),
    );
    let shown = Signal::new(0.0f32);
    while player.advance(1.0 / 60.0) {
        *frames.borrow_mut() += 1;
        shown.set(player.offset().x);
    }
    assert!(*frames.borrow() >= 8, "klatki policzone");
    assert_eq!(shown.get_untracked(), 10.0);
}

#[test]
fn easings_start_and_end_exactly_for_snappy_ui() {
    let all = [
        Easing::Linear,
        Easing::EaseIn,
        Easing::EaseOut,
        Easing::EaseInOut,
        Easing::EaseInQuad,
        Easing::EaseOutQuad,
        Easing::EaseInOutQuad,
        Easing::EaseInCubic,
        Easing::EaseOutCubic,
        Easing::EaseInOutCubic,
        Easing::EaseInQuart,
        Easing::EaseOutQuart,
        Easing::EaseInOutQuart,
        Easing::EaseInQuint,
        Easing::EaseOutQuint,
        Easing::EaseInOutQuint,
        Easing::EaseInExpo,
        Easing::EaseOutExpo,
        Easing::EaseInOutExpo,
        Easing::EaseInCirc,
        Easing::EaseOutCirc,
        Easing::EaseInOutCirc,
        Easing::EaseInBack,
        Easing::EaseOutBack,
        Easing::EaseInOutBack,
        Easing::BounceOut,
        Easing::BounceIn,
        Easing::ElasticOut,
    ];
    for e in all {
        assert_eq!(e.apply(0.0), 0.0, "{e:?} start");
        assert!((e.apply(1.0) - 1.0).abs() < 1e-5, "{e:?} koniec");

        for i in 0..=20 {
            let v = e.apply(i as f32 / 20.0);
            assert!(v.is_finite(), "{e:?} @ {i}/20 = {v}");
        }
    }
}

#[test]
fn hover_easings_feel_fast_then_settle() {
    let v = Easing::EaseOut.apply(0.25);
    assert!(v > 0.4, "ease-out szybko reaguje: {v}");
    let v_in = Easing::EaseIn.apply(0.25);
    assert!(v_in < 0.25, "ease-in startuje ospale: {v_in}");
}

#[test]
fn same_dt_sequence_same_result() {
    let run = || {
        let mut p = Animator::new().with_clip(
            Animation::new("s", 1.0).with_track(
                "x",
                Track::try_new(vec![
                    Keyframe::new(0.0, Value::px(0.0)),
                    Keyframe::new(1.0, Value::px(100.0)),
                ])
                .unwrap(),
            ),
        );
        let mut out = Vec::new();
        for _ in 0..10 {
            p.advance(0.033);
            out.push(p.offset());
        }
        out
    };
    assert_eq!(run(), run(), "frame-to-frame repeatability");
    assert_eq!(run()[0], Point::new(3.3, 0.0));
}
