use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, Instant};

use sequoia_animation::{Animator, Easing, Spring};
use sequoia_core::{Length, Vec2};
use sequoia_layout::LayoutStyle;

use crate::any::AnyView;
use crate::runtime::{ElementId, Runtime};
use crate::view::{PaintCtx, View};

pub const MAX_ANIM_DT: f32 = 1.0 / 20.0;

pub fn fade_sample(from: f32, delay: f32, duration: f32, easing: Easing, elapsed: f32) -> f32 {
    let from = from.clamp(0.0, 1.0);
    let delay = delay.max(0.0);
    if elapsed < delay {
        return from;
    }
    if duration <= 0.0 {
        return 1.0;
    }
    let t = (elapsed - delay) / duration;
    if t <= 0.0 {
        return from;
    }
    if t >= 1.0 {
        return 1.0;
    }
    from + (1.0 - from) * easing.apply(t)
}

pub fn slide_progress(delay: f32, duration: f32, easing: Easing, elapsed: f32) -> f32 {
    let delay = delay.max(0.0);
    if elapsed < delay {
        return 0.0;
    }
    if duration <= 0.0 {
        return 1.0;
    }
    let t = (elapsed - delay) / duration;
    if t <= 0.0 {
        return 0.0;
    }
    if t >= 1.0 {
        return 1.0;
    }
    easing.apply(t)
}

fn wall_elapsed(clock: &Rc<RefCell<Option<Instant>>>) -> f32 {
    let now = Instant::now();
    let mut slot = clock.borrow_mut();
    let start = *slot.get_or_insert(now);
    now.duration_since(start).as_secs_f32().min(600.0)
}

#[derive(Clone)]
pub struct Opacity {
    child: AnyView,
    opacity: f32,
}

impl Opacity {
    pub fn new<V: View + 'static>(child: V, opacity: f32) -> Self {
        Self {
            child: AnyView::new(child),
            opacity,
        }
    }

    pub fn opacity(&self) -> f32 {
        self.opacity.clamp(0.0, 1.0)
    }
}

impl View for Opacity {
    fn body(&self) -> impl View + 'static {
        crate::views::Unit
    }
    fn children_override(&self) -> Option<Vec<AnyView>> {
        Some(vec![self.child.clone()])
    }
    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fit,
            height: Length::Fit,
            ..LayoutStyle::default()
        }
    }
    fn paint_opacity(&self) -> Option<f32> {
        Some(self.opacity())
    }
}

#[derive(Clone)]
pub struct Translate {
    child: AnyView,
    offset: Vec2,
}

impl Translate {
    pub fn new<V: View + 'static>(child: V, dx: f32, dy: f32) -> Self {
        Self {
            child: AnyView::new(child),
            offset: Vec2::new(dx, dy),
        }
    }

    pub fn offset(&self) -> Vec2 {
        self.offset
    }
}

impl View for Translate {
    fn body(&self) -> impl View + 'static {
        crate::views::Unit
    }
    fn children_override(&self) -> Option<Vec<AnyView>> {
        Some(vec![self.child.clone()])
    }
    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fit,
            height: Length::Fit,
            ..LayoutStyle::default()
        }
    }
    fn paint_offset(&self) -> Vec2 {
        self.offset
    }
}

pub fn pop_sample(
    from: Vec2,
    fade_from: f32,
    spring: Spring,
    delay: f32,
    elapsed: f32,
) -> (f32, Vec2) {
    let fade_from = fade_from.clamp(0.0, 1.0);
    let delay = delay.max(0.0);
    if elapsed <= delay {
        return (fade_from, from);
    }
    let p = spring.value(elapsed - delay);
    let opacity = (fade_from + (1.0 - fade_from) * p).clamp(0.0, 1.0);
    let offset = Vec2::new(from.x * (1.0 - p), from.y * (1.0 - p));
    (opacity, offset)
}

pub fn pulse_sample(min_opacity: f32, period: f32, elapsed: f32) -> f32 {
    let min = min_opacity.clamp(0.0, 1.0);
    let period = period.max(1e-3);
    let phase = (elapsed.max(0.0) / period) % 1.0;

    let wave = 0.5 - 0.5 * (phase * std::f32::consts::TAU).cos();
    min + (1.0 - min) * wave
}

#[derive(Clone)]
pub struct Pulse {
    child: AnyView,
    min_opacity: f32,
    period: f32,
    clock: Rc<RefCell<Option<Instant>>>,
}

impl Pulse {
    pub fn new<V: View + 'static>(child: V) -> Self {
        Self {
            child: AnyView::new(child),
            min_opacity: 0.35,
            period: 1.2,
            clock: Rc::new(RefCell::new(None)),
        }
    }

    pub fn min_opacity(mut self, opacity: f32) -> Self {
        self.min_opacity = opacity.clamp(0.0, 1.0);
        self
    }

    pub fn period_duration(mut self, d: Duration) -> Self {
        self.period = d.as_secs_f32().max(1e-3);
        self
    }

    pub fn sample_at(&self, elapsed_secs: f32) -> f32 {
        pulse_sample(self.min_opacity, self.period, elapsed_secs)
    }

    fn current(&self) -> f32 {
        self.sample_at(wall_elapsed(&self.clock))
    }

    fn current_ctx(&self, rt: &Runtime) -> f32 {
        if rt.reduced_motion() {
            1.0
        } else {
            self.sample_at(rt.elapsed_secs())
        }
    }
}

impl View for Pulse {
    fn body(&self) -> impl View + 'static {
        crate::views::Unit
    }
    fn children_override(&self) -> Option<Vec<AnyView>> {
        Some(vec![self.child.clone()])
    }
    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fit,
            height: Length::Fit,
            ..LayoutStyle::default()
        }
    }
    fn paint_opacity(&self) -> Option<f32> {
        Some(self.current())
    }
    fn paint_opacity_ctx(&self, rt: &Runtime, _el: ElementId) -> Option<f32> {
        Some(self.current_ctx(rt))
    }
    fn paint_self(&self, ctx: &mut PaintCtx) {
        if !ctx.runtime().reduced_motion() {
            ctx.request_frame();
        }
    }
}

pub fn shake_sample(amplitude: f32, period: f32, duration: f32, delay: f32, elapsed: f32) -> Vec2 {
    let delay = delay.max(0.0);
    if duration <= 0.0 || elapsed <= delay {
        return Vec2::ZERO;
    }
    let t = elapsed - delay;
    if t >= duration {
        return Vec2::ZERO;
    }
    let decay = 1.0 - t / duration;
    let period = period.max(1e-3);
    let x = (t / period * std::f32::consts::TAU).sin() * amplitude * decay;
    Vec2::new(x, 0.0)
}

#[derive(Clone)]
pub struct Shake {
    child: AnyView,
    amplitude: f32,
    period: f32,
    duration: f32,
    delay: f32,
    clock: Rc<RefCell<Option<Instant>>>,
}

impl Shake {
    pub fn new<V: View + 'static>(child: V) -> Self {
        Self {
            child: AnyView::new(child),
            amplitude: 8.0,
            period: 0.08,
            duration: 0.36,
            delay: 0.0,
            clock: Rc::new(RefCell::new(None)),
        }
    }

    pub fn amplitude(mut self, px: f32) -> Self {
        self.amplitude = px;
        self
    }

    pub fn period_duration(mut self, d: Duration) -> Self {
        self.period = d.as_secs_f32().max(1e-3);
        self
    }

    pub fn total_duration(mut self, d: Duration) -> Self {
        self.duration = d.as_secs_f32().max(0.0);
        self
    }

    pub fn delay_duration(mut self, d: Duration) -> Self {
        self.delay = d.as_secs_f32().max(0.0);
        self
    }

    pub fn sample_at(&self, elapsed_secs: f32) -> Vec2 {
        shake_sample(
            self.amplitude,
            self.period,
            self.duration,
            self.delay,
            elapsed_secs,
        )
    }

    fn current(&self) -> (Vec2, bool) {
        let elapsed = wall_elapsed(&self.clock);
        let offset = self.sample_at(elapsed);
        let running = elapsed < self.delay + self.duration;
        (offset, running)
    }

    fn current_ctx(&self, rt: &Runtime) -> (Vec2, bool) {
        if rt.reduced_motion() {
            return (Vec2::ZERO, false);
        }
        let elapsed = rt.elapsed_secs();
        (
            self.sample_at(elapsed),
            elapsed < self.delay + self.duration,
        )
    }
}

impl View for Shake {
    fn body(&self) -> impl View + 'static {
        crate::views::Unit
    }
    fn children_override(&self) -> Option<Vec<AnyView>> {
        Some(vec![self.child.clone()])
    }
    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fit,
            height: Length::Fit,
            ..LayoutStyle::default()
        }
    }
    fn paint_offset(&self) -> Vec2 {
        self.current().0
    }
    fn paint_offset_ctx(&self, rt: &Runtime, _el: ElementId) -> Vec2 {
        self.current_ctx(rt).0
    }
    fn paint_self(&self, ctx: &mut PaintCtx) {
        if self.current_ctx(ctx.runtime()).1 {
            ctx.request_frame();
        }
    }
}

const TICK_MEMO_WINDOW: Duration = Duration::from_millis(1);

#[derive(Clone)]
pub struct Animated {
    child: AnyView,
    animator: Animator,
    clock: Rc<RefCell<Option<Instant>>>,
    memo: Rc<RefCell<(Option<Instant>, bool)>>,
    ctx_frame: Rc<Cell<Option<u64>>>,
    ctx_started: Rc<Cell<bool>>,
    ctx_running: Rc<Cell<bool>>,
}

impl Animated {
    pub fn new<V: View + 'static>(child: V, animator: Animator) -> Self {
        Self {
            child: AnyView::new(child),
            animator,
            clock: Rc::new(RefCell::new(None)),
            memo: Rc::new(RefCell::new((None, false))),
            ctx_frame: Rc::new(Cell::new(None)),
            ctx_started: Rc::new(Cell::new(false)),
            ctx_running: Rc::new(Cell::new(false)),
        }
    }

    pub fn animator(&self) -> Animator {
        self.animator.clone()
    }

    fn tick(&self) -> bool {
        let now = Instant::now();
        {
            let memo = self.memo.borrow();
            if let (Some(last), running) = (memo.0, memo.1)
                && now.saturating_duration_since(last) < TICK_MEMO_WINDOW
            {
                return running;
            }
        }
        let dt = {
            let mut slot = self.clock.borrow_mut();
            let dt = slot
                .map(|prev| (now - prev).as_secs_f32())
                .unwrap_or(0.0)
                .min(MAX_ANIM_DT);
            *slot = Some(now);
            dt
        };
        let running = self.animator.clone().advance(f64::from(dt));
        *self.memo.borrow_mut() = (Some(now), running);
        running
    }

    fn tick_ctx(&self, rt: &Runtime) -> bool {
        let frame = rt.clock().frame();
        if self.ctx_frame.get() == Some(frame) {
            return self.ctx_running.get();
        }
        let dt = if self.ctx_started.replace(true) {
            rt.frame_dt_secs()
        } else {
            0.0
        };
        let running = if rt.reduced_motion() {
            self.animator.clone().finish()
        } else {
            self.animator.clone().advance(f64::from(dt))
        };
        self.ctx_frame.set(Some(frame));
        self.ctx_running.set(running);
        running
    }
}

impl View for Animated {
    fn body(&self) -> impl View + 'static {
        crate::views::Unit
    }
    fn children_override(&self) -> Option<Vec<AnyView>> {
        Some(vec![self.child.clone()])
    }
    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fit,
            height: Length::Fit,
            ..LayoutStyle::default()
        }
    }
    fn paint_opacity(&self) -> Option<f32> {
        self.tick();
        self.animator.opacity()
    }
    fn paint_offset(&self) -> Vec2 {
        self.tick();
        let p = self.animator.offset();
        Vec2::new(p.x, p.y)
    }
    fn paint_opacity_ctx(&self, rt: &Runtime, _el: ElementId) -> Option<f32> {
        self.tick_ctx(rt);
        self.animator.opacity()
    }
    fn paint_offset_ctx(&self, rt: &Runtime, _el: ElementId) -> Vec2 {
        self.tick_ctx(rt);
        let p = self.animator.offset();
        Vec2::new(p.x, p.y)
    }
    fn paint_self(&self, ctx: &mut PaintCtx) {
        if self.tick_ctx(ctx.runtime()) {
            ctx.request_frame();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_fade_at(elapsed: f32) -> f32 {
        fade_sample(0.0, 0.0, 0.25, Easing::EaseOutCubic, elapsed)
    }

    #[test]
    fn fade_sample_walks_from_to_one() {
        assert_eq!(default_fade_at(-1.0), 0.0);
        assert_eq!(default_fade_at(0.0), 0.0);
        let mid = default_fade_at(0.125);
        assert!(mid > 0.0 && mid < 1.0, "entrance center: {mid}");
        assert_eq!(default_fade_at(10.0), 1.0);
    }

    #[test]
    fn fade_sample_honors_from_delay_and_zero_duration() {
        let at = |elapsed: f32| fade_sample(0.4, 0.5, 0.0, Easing::EaseOutCubic, elapsed);
        assert_eq!(at(0.0), 0.4, "przed delayem trzyma `from`");
        assert_eq!(at(10.0), 1.0, "zero time = immediately full");
    }

    fn default_slide_at(elapsed: f32) -> Vec2 {
        let from = Vec2::new(0.0, 24.0);
        let p = slide_progress(0.0, 0.3, Easing::EaseOutCubic, elapsed);
        Vec2::new(from.x * (1.0 - p), from.y * (1.0 - p))
    }

    #[test]
    fn slide_sample_returns_to_zero() {
        let start = default_slide_at(0.0);
        assert!((start.y - 24.0).abs() < 1e-4, "start: {start:?}");
        let mid = default_slide_at(0.15);
        assert!(mid.y > 0.0 && mid.y < 24.0, "middle: {mid:?}");
        assert_eq!(default_slide_at(10.0), Vec2::ZERO);
    }

    #[test]
    fn wrappers_report_paint_hooks() {
        let o = Opacity::new(crate::views::Unit, 0.5);
        assert_eq!(o.paint_opacity(), Some(0.5));
        let o = Opacity::new(crate::views::Unit, 42.0);
        assert_eq!(o.paint_opacity(), Some(1.0), "clamp do 1");

        let t = Translate::new(crate::views::Unit, 3.0, -4.0);
        assert_eq!(t.paint_offset(), Vec2::new(3.0, -4.0));
    }

    #[test]
    fn html_easing_curves_drive_fade_and_slide() {
        for ease in [
            Easing::CubicBezier(0.25, 0.1, 0.25, 1.0),
            Easing::CubicBezier(0.3, 0.7, 0.3, 1.0),
            Easing::CubicBezier(0.3, 1.3, 0.4, 1.0),
            Easing::CubicBezier(0.3, 1.4, 0.4, 1.0),
            Easing::CubicBezier(0.3, 1.2, 0.4, 1.0),
            Easing::CubicBezier(0.4, 0.0, 0.2, 1.0),
        ] {
            let fade_at = |elapsed: f32| fade_sample(0.0, 0.0, 0.4, ease, elapsed);
            assert_eq!(fade_at(0.0), 0.0);
            assert_eq!(fade_at(10.0), 1.0);
            let slide_at = |elapsed: f32| {
                let p = slide_progress(0.0, 0.4, ease, elapsed);
                Vec2::new(0.0, 10.0 * (1.0 - p))
            };
            assert!((slide_at(0.0).y - 10.0).abs() < 1e-4);
            assert_eq!(slide_at(10.0), Vec2::ZERO);
        }
    }

    #[test]
    fn pop_sample_starts_at_from_and_overshoots_zero() {
        let spring = Spring::new(0.4, 4.0);
        let (op0, off0) = pop_sample(Vec2::new(0.0, 24.0), 0.0, spring, 0.0, 0.0);
        assert_eq!(op0, 0.0);
        assert_eq!(off0, Vec2::new(0.0, 24.0));

        let mut min = f32::INFINITY;
        let mut t = 0.0;
        while t < 2.0 {
            min = min.min(pop_sample(Vec2::new(0.0, 24.0), 0.0, spring, 0.0, t).1.y);
            t += 0.005;
        }
        assert!(min < -0.5, "under-damped pop should overshoot: min={min}");
    }

    #[test]
    fn pop_sample_reports_opacity_and_offset() {
        let at = |elapsed: f32| {
            pop_sample(
                Vec2::new(0.0, 20.0),
                0.2,
                Spring::new(0.6, 5.0),
                0.0,
                elapsed,
            )
        };
        assert_eq!(at(0.0).0, 0.2);
        assert_eq!(at(0.0).1, Vec2::new(0.0, 20.0));

        assert_eq!(at(10.0).0, 1.0);
        assert_eq!(at(10.0).1, Vec2::ZERO);
    }

    #[test]
    fn pulse_breathes_between_min_and_one() {
        assert_eq!(pulse_sample(0.3, 1.0, 0.0), 0.3, "starts at min");
        assert_eq!(pulse_sample(0.3, 1.0, 0.5), 1.0, "peak at half period");
        assert_eq!(pulse_sample(0.3, 1.0, 1.0), 0.3, "wraps");
        let p = Pulse::new(crate::views::Unit)
            .min_opacity(0.2)
            .period_duration(Duration::from_secs_f32(0.8));
        assert!((p.sample_at(0.0) - 0.2).abs() < 1e-6);
        let mut max = 0.0f32;
        for i in 0..=100 {
            max = max.max(p.sample_at(i as f32 * 0.008));
        }
        assert!((max - 1.0).abs() < 1e-3, "peak reaches 1.0: {max}");
        assert_eq!(p.paint_opacity(), Some(0.2), "deterministic first sample");
    }

    #[test]
    fn shake_decays_to_zero() {
        assert_eq!(shake_sample(8.0, 0.1, 0.4, 0.0, 0.0), Vec2::ZERO);
        let mut peak = 0.0f32;
        let mut t = 0.0;
        while t < 0.4 {
            peak = peak.max(shake_sample(8.0, 0.1, 0.4, 0.0, t).x.abs());
            t += 0.002;
        }
        assert!(peak > 3.0 && peak <= 8.0, "limits the amplitude: {peak}");
        assert_eq!(shake_sample(8.0, 0.1, 0.4, 0.0, 0.4), Vec2::ZERO, "stops");
        assert_eq!(
            shake_sample(8.0, 0.1, 0.4, 0.0, 10.0),
            Vec2::ZERO,
            "stays put"
        );
        let s = Shake::new(crate::views::Unit).amplitude(4.0);
        assert_eq!(s.sample_at(0.0), Vec2::ZERO);
    }

    #[test]
    fn animated_applies_a_hosted_animation() {
        use sequoia_animation::{Animation, Animator, Keyframe, Track, Value};
        let clip = Animation::new("enter", 0.4)
            .with_track(
                "x",
                Track::try_new(vec![
                    Keyframe::new(0.0, Value::px(10.0)),
                    Keyframe::new(0.4, Value::px(50.0)),
                ])
                .unwrap(),
            )
            .with_track(
                "opacity",
                Track::try_new(vec![
                    Keyframe::new(0.0, Value::f32(0.5)),
                    Keyframe::new(0.4, Value::f32(1.0)),
                ])
                .unwrap(),
            );
        let player = Animator::new().with_clip(clip);
        let view = Animated::new(crate::views::Unit, player);

        let opacity = view.paint_opacity().expect("clip has an opacity track");
        assert!((0.5..0.55).contains(&opacity), "opacity: {opacity}");
        let offset = view.paint_offset();
        assert!((10.0..12.0).contains(&offset.x), "offset: {offset:?}");
        assert_eq!(offset.y, 0.0);
        assert!(view.animator().is_playing());
    }

    #[test]
    fn animated_coalesces_same_frame_ticks() {
        use sequoia_animation::{Animation, Animator, Keyframe, Track, Value};
        let clip = Animation::new("enter", 0.4).with_track(
            "opacity",
            Track::try_new(vec![
                Keyframe::new(0.0, Value::f32(0.5)),
                Keyframe::new(0.4, Value::f32(1.0)),
            ])
            .unwrap(),
        );
        let player = Animator::new().with_clip(clip);
        let view = Animated::new(crate::views::Unit, player);
        let _ = view.paint_opacity();
        let e1 = view.animator().elapsed();

        let _ = view.paint_offset();
        let e2 = view.animator().elapsed();
        assert_eq!(e1, e2, "tick w tej samej klatce nie kroczy: {e1} vs {e2}");
    }
}
