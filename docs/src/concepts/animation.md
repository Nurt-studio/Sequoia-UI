# Animation

Sequoia offers three layers of motion, and they are meant to be mixed:

1. **Motion on views** — `enter`/`exit` transitions and animated property
   wrappers. This is the everyday layer: you attach motion to a view and the
   runtime schedules frames for you.
2. **Sampling primitives** — `AnimSpec`, `Motion`, `Animate`, `Spring`, and
   `Easing`. They turn elapsed time into numbers that you can feed into custom
   painting or physics.
3. **Timelines and clips** — `AnimTrack`, `Animation`, and `Animator`. These
   are for authored sequences with keyframes, looping, and named properties.

All of them produce values in application time; the runtime caps a single
animation step at `MAX_ANIM_DT` (50 ms) so a stalled frame does not make an
animation jump.

## Enter and exit motion

Any view can be wrapped with a motion that plays when it is mounted and,
optionally, one that plays when it leaves the tree:

```rust
# use sequoia_ui::prelude::*;
use std::time::Duration;

let visible = Signal::new(true);
let toast = when(
    visible.clone(),
    Container::new()
        .padding(12.0)
        .radius(10.0)
        .background(Color::hex(0x1F6F43))
        .child(Text::new("Saved").color(Color::WHITE)),
)
.enter(fade().slide_y(16.0).duration(Duration::from_millis(220)))
.exit(fade().duration(Duration::from_millis(120)));

visible.set(false);
assert!(!visible.get());
```

`fade()` starts at zero opacity and ends at one. `slide_y(16.0)` adds an offset
that moves to zero. Both are combined by the same `Motion`, so the toast fades
and rises at once. `duration`, `delay`, `easing`, and `preset` adjust the
timing:

```rust
# use sequoia_ui::prelude::*;
use std::time::Duration;

let open = Signal::new(false);
let dialog = when(open, Container::new())
    .enter(
        fade()
            .slide(0.0, 8.0)
            .easing(Easing::EaseOutCubic)
            .delay(Duration::from_millis(40))
            .duration(Duration::from_millis(180)),
    )
    .exit(fade().duration(Duration::from_millis(100)));
```

`AnimSpec` presets (`FAST`, `SNAPPY`, `SMOOTH`, `GENTLE`, `SLOW`) are the
recommended starting points; use `Motion::preset` to apply one without
overriding its easing. An exit motion runs while the view is still mounted, so
the runtime keeps it visible until the fade completes. Use `on_exit_complete`
when something must happen after the last frame:

```rust
# use sequoia_ui::prelude::*;

let online = Signal::new(false);
let banner = when(online, Text::new("Offline"))
    .enter(fade())
    .exit(fade())
    .on_exit_complete(|| { /* remove the item from the model */ });
```

## Animated values

`enter`/`exit` animate a mount. To animate a property that changes at runtime,
wrap a value in a `Live<_>`: a signal, a memo, or a derived closure that samples
an `AnimSpec` or a spring. The `SignalLiveExt` and `ValueExt` traits add the
conversions:

```rust
# use sequoia_ui::prelude::*;

let reveal = Signal::new(0.0_f32);
let width = Signal::new(260.0_f32);

let drawer = Container::new()
    .width_live(width.clone().animate(AnimSpec::GENTLE))
    .opacity(reveal.clone().animate(AnimSpec::SMOOTH))
    .offset_by(
        reveal
            .clone()
            .map_value(|t| Vec2::new(-24.0 * (1.0 - t), 0.0))
            .spring(SpringSpec::SNAPPY),
    );

reveal.set(1.0);
assert_eq!(reveal.get(), 1.0);
```

The view modifiers that accept a `Live<_>` are `opacity`, `background`,
`offset_by`, `width_live`, `height_live`, `size_live`, `scale`, and `rotate`.
A fixed number converts automatically, so switching a property from static to
animated does not change its call site:

```rust
# use sequoia_ui::prelude::*;
let fixed = Container::new().opacity(0.8);
let animated = Container::new().opacity(Signal::new(0.8_f32).animate(AnimSpec::FAST));
```

`Live` values interpolate with the spec while running and then settle on the
target. They do not restart when the target changes; the animation continues
from the current value, which keeps interrupted transitions smooth.

## Springs

Springs are better than tweens for values the user drags, resizes, or toggles:
they preserve velocity and feel immediate. Use a `SpringSpec` directly:

```rust
# use sequoia_ui::prelude::*;

let spec = SpringSpec::BOUNCY
    .stiffness(260.0)
    .damping(18.0)
    .mass(1.0);

let popover = Container::new()
    .scale(Signal::new(1.0_f32).spring(spec));
```

The four presets are `SNAPPY` (default), `BOUNCY`, `GENTLE`, and `DEFAULT`.
`SpringSpec` is a damped harmonic oscillator: `stiffness` controls how quickly
it pulls toward the target, `damping` controls overshoot, and `mass` controls
inertia. `spring_spec` converts the timeline `Spring` type into a `SpringSpec`
when the same motion is shared between layers.

## Sampling for custom painting

A custom `View` can read time directly. `AnimSpec::sample` maps elapsed time to
a scalar and `is_running` tells you when to request another frame:

```rust
# use sequoia_ui::prelude::*;
use std::time::Duration;

let spec = AnimSpec::SMOOTH.delay_duration(Duration::from_millis(80));
assert!(spec.is_running(0.05));
assert!(!spec.is_running(10.0));
assert_eq!(spec.sample(0.0, 1.0, 10.0), 1.0);

let spring = spring(1.0).stiffness(260.0).damping(18.0);
let value = spring.sample_at(0.2);
assert!(value.is_finite());
```

The `animate()` builder is a convenience for opacity plus vertical translation,
the combination used by almost every entrance animation:

```rust
# use sequoia_ui::prelude::*;
use std::time::Duration;

let intro = animate()
    .opacity(0.0..1.0)
    .translate_y(12.0..0.0)
    .duration(Duration::from_millis(240))
    .ease_out()
    .build();

let frame = intro.sample_at(0.12);
assert!(frame.opacity.unwrap() > 0.0);
assert!(frame.offset.is_some());
```

Use `animate()` when the effect is "fade and slide"; use `AnimSpec` directly
when only one number is involved. `Sequence` plays several `Animate` values at
known offsets, and `stagger(count, delay, make)` generates a cascading set:

```rust
# use sequoia_ui::prelude::*;

let intro = Sequence::new()
    .add(fade_in(), 0.0)
    .add(slide_in(12.0), 0.06);

let rows = stagger(5, 0.04, |_| slide_in(8.0));
assert_eq!(rows.len(), 5);
let frame = intro.sample_at(0.10);
assert!(frame.opacity.is_some());
```

## Timelines and clips

For authored, repeatable sequences, build keyframed tracks. A `Track` holds
time/value pairs and is sampled by an `Animation`. The prelude renames the
generic types to avoid collisions: `AnimTrack` is `sequoia_animation::Track`
and `TrackValue` is `sequoia_animation::Value`.

```rust
# use sequoia_ui::prelude::*;

let bounce = AnimTrack::px(&[(0.0, 0.0), (0.18, 14.0), (0.32, 0.0)])
    .expect("a track needs at least one keyframe");

let clip = Animation::new("bounce-in", 0.32)
    .with_loop(LoopMode::Once)
    .with_track("offset_y", bounce);

let frame = clip.sample(0.18);
assert!(frame.get("offset_y").is_some());
```

`Track::px`, `Track::f32`, and `Track::color` create typed tracks from slices.
For mixed tracks, construct `Keyframe::new(time, value)` values explicitly and
pass them to `AnimTrack::try_new`. `LoopMode` is `Once`, `Loop`, or `PingPong`.
`Animation::with_speed` scales playback and `is_finished_at` reports whether a
non-looping clip has ended.

`Animator` owns a clip and advances it with real or simulated time. It also
hosts state machines and spring machines when an animation needs named states
and triggers:

```rust
# use sequoia_ui::prelude::*;

let clip = Animation::new("fade", 0.25).with_track(
    "opacity",
    AnimTrack::f32(&[(0.0, 0.0), (0.25, 1.0)]).expect("valid track"),
);

let mut animator = Animator::new().with_clip(clip);
animator.play();
animator.advance(1.0 / 60.0);
assert!(animator.is_playing());
```

Use `Animator::advance` from your own frame loop when you drive time manually
(for example in tests), and `advance_frame` when the runtime clock should be
consulted. `set_boolean` and `set_number` feed named parameters into a
`StateMachine`; `fire` sends a trigger.

## Reusable animated views

Three wrappers cover common effects without any state of your own:

```rust
# use sequoia_ui::prelude::*;
use std::time::Duration;

let recording = Pulse::new(Text::new("Recording"))
    .min_opacity(0.35)
    .period_duration(Duration::from_millis(1200));

let invalid = Shake::new(Text::new("Check the value"))
    .amplitude(6.0)
    .total_duration(Duration::from_millis(360));

let faded = Opacity::new(Text::new("Disabled"), 0.5);
let shifted = Translate::new(Text::new("Hint"), 0.0, 8.0);
```

`Pulse` fades between `min_opacity` and full opacity on a fixed period. `Shake`
oscillates horizontally; set `amplitude`, `period_duration`, `total_duration`,
and `delay_duration`. Both request frames only while they are visible. `Opacity`
and `Translate` are static wrappers useful when a value comes from layout rather
than an animation.

## Choosing the right layer

- Use `enter`/`exit` for anything that appears or disappears: dialogs, toasts,
  banners, and rows entering a list.
- Use a `Live` value for hover, focus, selection, and open/closed state that
  remains mounted.
- Use a spring for direct manipulation and for values that change rapidly.
- Use keyframed `Animation` and `Animator` when the motion is authored, loops,
  or must be shared by several views.
- Keep interface motion short (roughly 120–250 ms). Reserve longer durations for
  choreographed sequences, and avoid animating layout on every pointer move.

Because animated views participate in damage tracking, only the affected
region repaints. An application can stay in `RedrawMode::OnDemand` and still
animate; the runtime requests frames while a motion is active. See
[Rendering](rendering.md) for the frame loop and [Styling](styling.md) for the
visual properties being animated.
