use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use sequoia_animation::{Easing, animate};
use sequoia_core::{Color, Edges, Length, Vec2};
use sequoia_layout::{Indents, LayoutStyle, sanitize_indent};
use sequoia_reactive::Signal;
use sequoia_style::ShadowSpec;

use crate::any::AnyView;
use crate::filter_view::{EffectNode, FilterExt, Filtered};
use crate::live::{AnimSpec, Live, Prop, SpringSpec};
use crate::live_view::{LiveBackground, LiveOffset, LiveOpacity};
use crate::presence::Presence;
use crate::runtime::{ElementId, Runtime};
use crate::view::{PaintCtx, View};
use crate::views::Margined;

#[derive(Clone, Debug)]
pub struct Motion {
    opacity: Option<(f32, f32)>,
    offset: Option<(Vec2, Vec2)>,
    spec: AnimSpec,
}

pub fn fade() -> Motion {
    Motion {
        opacity: Some((0.0, 1.0)),
        offset: None,
        spec: AnimSpec::SMOOTH,
    }
}

pub fn slide_y(px: f32) -> Motion {
    Motion {
        opacity: None,
        offset: Some((Vec2::new(0.0, px), Vec2::ZERO)),
        spec: AnimSpec::SMOOTH,
    }
}

impl Motion {
    pub fn slide_y(mut self, px: f32) -> Self {
        self.offset = Some((Vec2::new(0.0, px), Vec2::ZERO));
        self
    }

    pub fn slide(mut self, dx: f32, dy: f32) -> Self {
        self.offset = Some((Vec2::new(dx, dy), Vec2::ZERO));
        self
    }

    pub fn fade_from(mut self, from: f32) -> Self {
        self.opacity = Some((from.clamp(0.0, 1.0), 1.0));
        self
    }

    pub fn duration(mut self, d: Duration) -> Self {
        self.spec.duration_secs = d.as_secs_f32().max(0.0);
        self
    }

    pub fn easing(mut self, easing: Easing) -> Self {
        self.spec.easing = easing;
        self
    }

    pub fn delay(mut self, d: Duration) -> Self {
        self.spec.delay_secs = d.as_secs_f32().max(0.0);
        self
    }

    pub fn preset(mut self, spec: impl Into<AnimSpec>) -> Self {
        self.spec = spec.into();
        self
    }

    pub fn sample_at(&self, elapsed_secs: f32) -> (Option<f32>, Option<Vec2>) {
        let op = self
            .opacity
            .map(|(a, b)| self.spec.sample(a, b, elapsed_secs));
        let off = self.offset.map(|(a, b)| {
            Vec2::new(
                self.spec.sample(a.x, b.x, elapsed_secs),
                self.spec.sample(a.y, b.y, elapsed_secs),
            )
        });
        (op, off)
    }

    pub fn is_running(&self, elapsed_secs: f32) -> bool {
        (self.opacity.is_some() || self.offset.is_some()) && self.spec.is_running(elapsed_secs)
    }

    pub(crate) fn into_animate(self) -> sequoia_animation::Animate {
        let mut b = animate()
            .duration(Duration::from_secs_f32(self.spec.duration_secs))
            .delay(Duration::from_secs_f32(self.spec.delay_secs))
            .ease(self.spec.easing);
        if let Some((a, c)) = self.opacity {
            b = b.opacity(a..c);
        }
        if let Some((a, c)) = self.offset {
            b = b.translate(a, c);
        }
        b.build()
    }
}

impl From<AnimSpec> for Motion {
    fn from(spec: AnimSpec) -> Self {
        Motion {
            opacity: Some((0.0, 1.0)),
            offset: None,
            spec,
        }
    }
}

impl From<Duration> for Motion {
    fn from(d: Duration) -> Self {
        Motion::from(AnimSpec::from(d))
    }
}

#[derive(Clone)]
pub struct When {
    presence: Presence,
}

pub fn when<V: View + 'static>(present: Signal<bool>, child: V) -> When {
    When {
        presence: Presence::new(present).child(child),
    }
}

impl When {
    pub fn enter(mut self, motion: impl Into<Motion>) -> Self {
        self.presence = self.presence.enter(motion.into().into_animate());
        self
    }

    pub fn exit(mut self, motion: impl Into<Motion>) -> Self {
        self.presence = self.presence.exit(motion.into().into_animate());
        self
    }

    pub fn on_exit_complete(mut self, f: impl Fn() + 'static) -> Self {
        self.presence = self.presence.on_exit_complete(f);
        self
    }

    pub fn is_mounted(&self) -> bool {
        self.presence.is_mounted()
    }
}

impl View for When {
    fn body(&self) -> impl View + 'static {
        crate::views::Unit
    }
    fn reuse_state(&self, old: &Self) {
        self.presence.reuse_state(&old.presence);
    }
    fn keyed_children(&self) -> Option<Vec<(u64, AnyView)>> {
        self.presence.keyed_children()
    }
    fn layout_style(&self) -> LayoutStyle {
        self.presence.layout_style()
    }
    fn paint_opacity(&self) -> Option<f32> {
        self.presence.paint_opacity()
    }
    fn paint_offset(&self) -> Vec2 {
        self.presence.paint_offset()
    }
    fn paint_self(&self, ctx: &mut PaintCtx) {
        self.presence.paint_self(ctx);
    }
}

#[derive(Clone)]
pub struct Clip {
    child: AnyView,
    radius: f32,
}

impl Clip {
    pub fn new<V: View + 'static>(child: V, radius: f32) -> Self {
        Self {
            child: AnyView::new(child),
            radius: radius.max(0.0),
        }
    }
}

impl View for Clip {
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
    fn clip_self(&self) -> bool {
        true
    }
    fn clip_radius(&self) -> f32 {
        self.radius
    }
}

#[derive(Clone)]
pub struct Scaled {
    child: AnyView,
    live: Live<f32>,
}

impl Scaled {
    pub fn new<V: View + 'static>(child: V, live: impl Into<Live<f32>>) -> Self {
        Self {
            child: AnyView::new(child),
            live: live.into(),
        }
    }

    pub fn animate(mut self, spec: impl Into<AnimSpec>) -> Self {
        self.live.animate_mut(spec);
        self
    }

    pub fn spring(mut self, spec: SpringSpec) -> Self {
        self.live.spring_mut(spec);
        self
    }

    pub fn target_untracked(&self) -> f32 {
        self.live.target_untracked()
    }
}

impl View for Scaled {
    fn body(&self) -> impl View + 'static {
        crate::views::Unit
    }
    fn children_override(&self) -> Option<Vec<AnyView>> {
        let _ = self.live.target();
        Some(vec![self.child.clone()])
    }
    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fit,
            height: Length::Fit,
            ..LayoutStyle::default()
        }
    }
    fn paint_scale_ctx(&self, rt: &Runtime, el: ElementId) -> f32 {
        rt.resolve_live_f32(el, Prop::SCALE, &self.live)
    }
    fn paint_self(&self, ctx: &mut PaintCtx) {
        let target = self.live.target_untracked();
        let (_, running) = ctx.runtime.live_f32(
            ctx.element,
            Prop::SCALE,
            target,
            self.live.tween_spec().as_ref(),
            self.live.spring_spec().as_ref(),
        );
        if running {
            ctx.request_frame();
        }
    }
}

#[derive(Clone)]
pub struct Rotated {
    child: AnyView,
    live: Live<f32>,
}

impl Rotated {
    pub fn new<V: View + 'static>(child: V, live: impl Into<Live<f32>>) -> Self {
        Self {
            child: AnyView::new(child),
            live: live.into(),
        }
    }

    pub fn animate(mut self, spec: impl Into<AnimSpec>) -> Self {
        self.live.animate_mut(spec);
        self
    }

    pub fn spring(mut self, spec: SpringSpec) -> Self {
        self.live.spring_mut(spec);
        self
    }

    pub fn target_untracked(&self) -> f32 {
        self.live.target_untracked()
    }
}

impl View for Rotated {
    fn body(&self) -> impl View + 'static {
        crate::views::Unit
    }
    fn children_override(&self) -> Option<Vec<AnyView>> {
        let _ = self.live.target();
        Some(vec![self.child.clone()])
    }
    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fit,
            height: Length::Fit,
            ..LayoutStyle::default()
        }
    }
    fn paint_rotation_ctx(&self, rt: &Runtime, el: ElementId) -> f32 {
        rt.resolve_live_f32(el, Prop::ROTATION, &self.live)
    }
    fn paint_self(&self, ctx: &mut PaintCtx) {
        let target = self.live.target_untracked();
        let (_, running) = ctx.runtime.live_f32(
            ctx.element,
            Prop::ROTATION,
            target,
            self.live.tween_spec().as_ref(),
            self.live.spring_spec().as_ref(),
        );
        if running {
            ctx.request_frame();
        }
    }
}

#[derive(Clone)]
pub struct Enter {
    child: AnyView,
    motion: Motion,
    start: Rc<RefCell<Option<Instant>>>,
}

impl Enter {
    pub fn new<V: View + 'static>(child: V, motion: impl Into<Motion>) -> Self {
        Self {
            child: AnyView::new(child),
            motion: motion.into(),
            start: Rc::new(RefCell::new(None)),
        }
    }

    fn elapsed(&self) -> f32 {
        let now = Instant::now();
        let mut slot = self.start.borrow_mut();
        let t0 = *slot.get_or_insert(now);
        now.duration_since(t0).as_secs_f32().min(600.0)
    }
}

impl View for Enter {
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
        self.motion.sample_at(self.elapsed()).0
    }
    fn paint_offset(&self) -> Vec2 {
        self.motion
            .sample_at(self.elapsed())
            .1
            .unwrap_or(Vec2::ZERO)
    }
    fn paint_self(&self, ctx: &mut PaintCtx) {
        if self.motion.is_running(self.elapsed()) {
            ctx.request_frame();
        }
    }
}

#[derive(Clone, Default)]
pub struct StatePatch {
    background: Option<Live<Color>>,
    opacity: Option<Live<f32>>,
    scale: Option<Live<f32>>,
    spec: Option<AnimSpec>,
    spring: Option<SpringSpec>,
}

impl StatePatch {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn background(mut self, color: impl Into<Live<Color>>) -> Self {
        self.background = Some(color.into());
        self
    }

    pub fn opacity(mut self, opacity: impl Into<Live<f32>>) -> Self {
        self.opacity = Some(opacity.into());
        self
    }

    pub fn scale(mut self, scale: impl Into<Live<f32>>) -> Self {
        self.scale = Some(scale.into());
        self
    }

    pub fn animate(mut self, spec: impl Into<AnimSpec>) -> Self {
        self.spec = Some(spec.into());
        self.spring = None;
        self
    }

    pub fn spring(mut self, spec: SpringSpec) -> Self {
        self.spring = Some(spec);
        self.spec = None;
        self
    }
}

#[derive(Clone)]
pub struct States {
    child: AnyView,
    hover: Option<StatePatch>,
    pressed: Option<StatePatch>,
    focused: Option<StatePatch>,
    selected: Option<StatePatch>,
    disabled: Option<StatePatch>,
    disabled_when: Option<Live<bool>>,
    selected_when: Option<Live<bool>>,
    radius: f32,
}

impl States {
    pub fn new<V: View + 'static>(child: V) -> Self {
        Self {
            child: AnyView::new(child),
            hover: None,
            pressed: None,
            focused: None,
            selected: None,
            disabled: None,
            disabled_when: None,
            selected_when: None,
            radius: 0.0,
        }
    }

    pub fn hover(mut self, build: impl FnOnce(StatePatch) -> StatePatch) -> Self {
        self.hover = Some(build(StatePatch::new()));
        self
    }

    pub fn hover_patch(mut self, patch: StatePatch) -> Self {
        self.hover = Some(patch);
        self
    }

    pub fn pressed(mut self, build: impl FnOnce(StatePatch) -> StatePatch) -> Self {
        self.pressed = Some(build(StatePatch::new()));
        self
    }

    pub fn pressed_patch(mut self, patch: StatePatch) -> Self {
        self.pressed = Some(patch);
        self
    }

    pub fn focused(mut self, build: impl FnOnce(StatePatch) -> StatePatch) -> Self {
        self.focused = Some(build(StatePatch::new()));
        self
    }

    pub fn focused_patch(mut self, patch: StatePatch) -> Self {
        self.focused = Some(patch);
        self
    }

    pub fn selected(mut self, build: impl FnOnce(StatePatch) -> StatePatch) -> Self {
        self.selected = Some(build(StatePatch::new()));
        self
    }

    pub fn selected_patch(mut self, patch: StatePatch) -> Self {
        self.selected = Some(patch);
        self
    }

    pub fn disabled(mut self, build: impl FnOnce(StatePatch) -> StatePatch) -> Self {
        self.disabled = Some(build(StatePatch::new()));
        self
    }

    pub fn disabled_patch(mut self, patch: StatePatch) -> Self {
        self.disabled = Some(patch);
        self
    }

    pub fn disabled_when(mut self, cond: impl Into<Live<bool>>) -> Self {
        self.disabled_when = Some(cond.into());
        self
    }

    pub fn selected_when(mut self, cond: impl Into<Live<bool>>) -> Self {
        self.selected_when = Some(cond.into());
        self
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius.max(0.0);
        self
    }

    fn touch(live: &Live<Color>) {
        let _ = live.target();
    }
    fn touch_f32(live: &Live<f32>) {
        let _ = live.target();
    }
    fn touch_bool(live: &Live<bool>) {
        let _ = live.target();
    }

    fn touch_patch(p: &StatePatch) {
        if let Some(v) = &p.background {
            Self::touch(v);
        }
        if let Some(v) = &p.opacity {
            Self::touch_f32(v);
        }
        if let Some(v) = &p.scale {
            Self::touch_f32(v);
        }
    }

    fn active_patch(&self, hovered: bool, pressed: bool, focused: bool) -> Option<&StatePatch> {
        let disabled = self
            .disabled_when
            .as_ref()
            .map(|c| c.target_untracked())
            .unwrap_or(false);
        if disabled && self.disabled.is_some() {
            return self.disabled.as_ref();
        }
        if pressed && self.pressed.is_some() {
            return self.pressed.as_ref();
        }
        if hovered && self.hover.is_some() {
            return self.hover.as_ref();
        }
        if focused && self.focused.is_some() {
            return self.focused.as_ref();
        }
        let selected = self
            .selected_when
            .as_ref()
            .map(|c| c.target_untracked())
            .unwrap_or(false);
        if selected && self.selected.is_some() {
            return self.selected.as_ref();
        }
        None
    }

    fn eff_f32(patch: &StatePatch, live: &Live<f32>) -> Live<f32> {
        if live.tween_spec().is_some() || live.spring_spec().is_some() {
            return live.clone();
        }
        let mut out = live.clone();
        if let Some(s) = patch.spec {
            out.animate_mut(s);
        } else if let Some(s) = patch.spring {
            out.spring_mut(s);
        }
        out
    }

    fn eff_color(patch: &StatePatch, live: &Live<Color>) -> Live<Color> {
        if live.tween_spec().is_some() {
            return live.clone();
        }
        let mut out = live.clone();
        if let Some(s) = patch.spec {
            out.animate_mut(s);
        }
        out
    }
}

impl View for States {
    fn body(&self) -> impl View + 'static {
        crate::views::Unit
    }
    fn children_override(&self) -> Option<Vec<AnyView>> {
        for p in [
            &self.hover,
            &self.pressed,
            &self.focused,
            &self.selected,
            &self.disabled,
        ]
        .into_iter()
        .flatten()
        {
            Self::touch_patch(p);
        }
        if let Some(c) = &self.disabled_when {
            Self::touch_bool(c);
        }
        if let Some(c) = &self.selected_when {
            Self::touch_bool(c);
        }
        Some(vec![self.child.clone()])
    }
    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fit,
            height: Length::Fit,
            ..LayoutStyle::default()
        }
    }
    fn has_visual_state(&self) -> bool {
        true
    }
    fn paint_opacity_ctx(&self, rt: &Runtime, el: ElementId) -> Option<f32> {
        let st = rt.visual_state(el);
        let patch = self.active_patch(st.hovered, st.pressed, st.focused)?;
        let live = patch.opacity.as_ref()?;
        let live = Self::eff_f32(patch, live);
        Some(
            rt.resolve_live_f32(el, Prop::OPACITY, &live)
                .clamp(0.0, 1.0),
        )
    }
    fn paint_scale_ctx(&self, rt: &Runtime, el: ElementId) -> f32 {
        let st = rt.visual_state(el);
        let patch = match self.active_patch(st.hovered, st.pressed, st.focused) {
            Some(p) => p,
            None => return 1.0,
        };
        let live = match patch.scale.as_ref() {
            Some(v) => v,
            None => return 1.0,
        };
        let live = Self::eff_f32(patch, live);
        rt.resolve_live_f32(el, Prop::SCALE, &live)
    }
    fn paint_self(&self, ctx: &mut PaintCtx) {
        let st = ctx.state;

        let patch = match self.active_patch(st.hovered, st.pressed, st.focused) {
            Some(p) => p,
            None => return,
        };
        let mut running = false;
        if let Some(live) = &patch.background {
            let live = Self::eff_color(patch, live);
            let target = live.target_untracked();
            let (color, r) = ctx.runtime.live_color(
                ctx.element,
                Prop::BACKGROUND,
                target,
                live.tween_spec().as_ref(),
            );
            running |= r;
            if color.a > 0.0 {
                ctx.quad(ctx.rect, color, self.radius);
            }
        }
        if let Some(live) = &patch.opacity {
            let live = Self::eff_f32(patch, live);
            let target = live.target_untracked();
            let (_, r) = ctx.runtime.live_f32(
                ctx.element,
                Prop::OPACITY,
                target,
                live.tween_spec().as_ref(),
                live.spring_spec().as_ref(),
            );
            running |= r;
        }
        if let Some(live) = &patch.scale {
            let live = Self::eff_f32(patch, live);
            let target = live.target_untracked();
            let (_, r) = ctx.runtime.live_f32(
                ctx.element,
                Prop::SCALE,
                target,
                live.tween_spec().as_ref(),
                live.spring_spec().as_ref(),
            );
            running |= r;
        }
        if running {
            ctx.request_frame();
        }
    }
}

pub trait ViewModifiers: View + Sized + 'static {
    fn opacity(self, opacity: impl Into<Live<f32>>) -> LiveOpacity {
        LiveOpacity::new(self, opacity)
    }

    fn offset(self, dx: f32, dy: f32) -> LiveOffset {
        LiveOffset::new(self, Live::fixed(Vec2::new(dx, dy)))
    }

    fn offset_by(self, offset: impl Into<Live<Vec2>>) -> LiveOffset {
        LiveOffset::new(self, offset)
    }

    fn background(self, color: impl Into<Live<Color>>) -> LiveBackground {
        LiveBackground::new(self, color)
    }

    fn bg(self, color: impl Into<Live<Color>>) -> LiveBackground {
        LiveBackground::new(self, color)
    }

    fn blur(self, px: f32) -> Filtered {
        self.blur_own(px)
    }

    fn shadow(self, spec: ShadowSpec) -> EffectNode {
        self.effect(sequoia_style::Effect::Shadow(spec))
    }

    fn clip(self) -> Clip {
        Clip::new(self, 0.0)
    }

    fn clip_rounded(self, radius: f32) -> Clip {
        Clip::new(self, radius)
    }

    fn scale(self, scale: impl Into<Live<f32>>) -> Scaled {
        Scaled::new(self, scale)
    }

    fn rotate(self, degrees: impl Into<Live<f32>>) -> Rotated {
        Rotated::new(self, degrees)
    }

    fn width_live(self, width: impl Into<Live<f32>>) -> crate::layout_live::AnimatedSize {
        crate::layout_live::AnimatedSize::width(self, width)
    }

    fn height_live(self, height: impl Into<Live<f32>>) -> crate::layout_live::AnimatedSize {
        crate::layout_live::AnimatedSize::height(self, height)
    }

    fn size_live(
        self,
        width: impl Into<Live<f32>>,
        height: impl Into<Live<f32>>,
    ) -> crate::layout_live::AnimatedSize {
        crate::layout_live::AnimatedSize::size(self, width, height)
    }

    fn animate(self, spec: impl Into<AnimSpec>) -> Enter {
        Enter::new(self, Motion::from(spec.into()))
    }

    fn enter(self, motion: impl Into<Motion>) -> Enter {
        Enter::new(self, motion)
    }

    fn hover(self, build: impl FnOnce(StatePatch) -> StatePatch) -> States {
        States::new(self).hover_patch(build(StatePatch::new()))
    }

    fn pressed(self, build: impl FnOnce(StatePatch) -> StatePatch) -> States {
        States::new(self).pressed_patch(build(StatePatch::new()))
    }

    fn focused(self, build: impl FnOnce(StatePatch) -> StatePatch) -> States {
        States::new(self).focused_patch(build(StatePatch::new()))
    }

    fn selected(self, build: impl FnOnce(StatePatch) -> StatePatch) -> States {
        States::new(self).selected_patch(build(StatePatch::new()))
    }

    fn disabled(self, build: impl FnOnce(StatePatch) -> StatePatch) -> States {
        States::new(self).disabled_patch(build(StatePatch::new()))
    }

    fn state(self, build: impl FnOnce(States) -> States) -> States {
        build(States::new(self))
    }

    fn margin(self, margin: impl Into<Edges>) -> Margined {
        Margined::new(self, margin)
    }

    fn margin_xy(self, x: f32, y: f32) -> Margined {
        Margined::new(self, Edges::xy(x, y))
    }

    fn margin_x(self, x: f32) -> Margined {
        Margined::new(self, Edges::horizontal(x))
    }

    fn margin_y(self, y: f32) -> Margined {
        Margined::new(self, Edges::vertical(y))
    }

    fn margin_left(self, v: f32) -> Margined {
        Margined::new(self, Edges::left(v))
    }

    fn margin_right(self, v: f32) -> Margined {
        Margined::new(self, Edges::right(v))
    }

    fn margin_top(self, v: f32) -> Margined {
        Margined::new(self, Edges::top(v))
    }

    fn margin_bottom(self, v: f32) -> Margined {
        Margined::new(self, Edges::bottom(v))
    }

    fn indent_top(self, v: f32) -> Margined {
        Margined::with_indents(
            self,
            Indents::only(None, Some(sanitize_indent(v)), None, None),
        )
    }

    fn indent_bottom(self, v: f32) -> Margined {
        Margined::with_indents(
            self,
            Indents::only(None, None, None, Some(sanitize_indent(v))),
        )
    }

    fn indent_left(self, v: f32) -> Margined {
        Margined::with_indents(
            self,
            Indents::only(Some(sanitize_indent(v)), None, None, None),
        )
    }

    fn indent_right(self, v: f32) -> Margined {
        Margined::with_indents(
            self,
            Indents::only(None, None, Some(sanitize_indent(v)), None),
        )
    }

    fn indent(self, indents: Indents) -> Margined {
        Margined::with_indents(self, indents)
    }

    fn indent_all(self, v: f32) -> Margined {
        Margined::with_indents(self, Indents::all(v))
    }

    fn indent_xy(self, x: f32, y: f32) -> Margined {
        Margined::with_indents(self, Indents::xy(x, y))
    }

    fn indent_axes(self, horizontal: f32, vertical: f32) -> Margined {
        Margined::with_indents(self, Indents::axes(horizontal, vertical))
    }
}

impl<T: View + 'static> ViewModifiers for T {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::ValueExt;

    #[test]
    fn motion_fade_and_slide_sample() {
        let m = fade().slide_y(12.0);
        let (op0, off0) = m.sample_at(0.0);
        assert_eq!(op0, Some(0.0));
        assert_eq!(off0, Some(Vec2::new(0.0, 12.0)));
        let (op1, off1) = m.sample_at(10.0);
        assert_eq!(op1, Some(1.0));
        assert_eq!(off1, Some(Vec2::ZERO));
        assert!(m.is_running(0.05));
        assert!(!m.is_running(10.0));
    }

    #[test]
    fn motion_from_spec_and_duration() {
        let a = Motion::from(AnimSpec::SNAPPY);
        assert_eq!(a.sample_at(0.0).0, Some(0.0));
        let b = Motion::from(Duration::from_millis(200));
        assert_eq!(b.sample_at(10.0).0, Some(1.0));
    }

    #[test]
    fn modifiers_chain_static_values() {
        let v = crate::views::Unit
            .opacity(0.5)
            .offset(3.0, -4.0)
            .background(Color::WHITE)
            .blur(2.0)
            .clip_rounded(6.0)
            .scale(1.03);
        assert_eq!(v.paint_opacity(), None);
        assert_eq!(v.target_untracked(), 1.03);
        let r = crate::views::Unit.rotate(15.0);

        assert_eq!(r.paint_rotation(), 0.0);
        assert_eq!(r.target_untracked(), 15.0);
    }

    #[test]
    fn post_hoc_animate_attaches_spec() {
        let v = crate::views::Unit.opacity(0.0).animate(AnimSpec::SNAPPY);
        let _ = v.children_override();
        let v = crate::views::Unit
            .scale(1.0)
            .spring(SpringSpec::BOUNCY)
            .offset(0.0, 8.0)
            .animate(AnimSpec::SMOOTH);
        let _ = v.children_override();
    }

    #[test]
    fn reactive_value_flows_into_modifier() {
        let s = Signal::new(0.8f32);
        let v = crate::views::Unit.opacity(s.clone().animated(AnimSpec::FAST));
        let kids = v.children_override().expect("dziecko");
        assert_eq!(kids.len(), 1);
        s.set(0.2);
        let _ = v.children_override().expect("dziecko");
    }

    #[test]
    fn states_accept_patch_and_builder() {
        let a = crate::views::Unit.state(|s| s.hover_patch(StatePatch::new().opacity(0.5)));
        let b = crate::views::Unit.hover(|h| {
            h.background(Color::WHITE)
                .opacity(0.9)
                .animate(AnimSpec::FAST)
        });
        let c = crate::views::Unit.state(|s| {
            s.hover(|h| h.opacity(1.0))
                .pressed(|p| p.scale(0.97).spring(SpringSpec::SNAPPY))
                .focused(|f| f.opacity(1.0))
                .disabled(|d| d.opacity(0.5))
        });
        assert!(a.hover.is_some());
        assert!(b.hover.as_ref().unwrap().background.is_some());
        assert!(c.pressed.as_ref().unwrap().scale.is_some());

        assert!(c.active_patch(true, true, false).unwrap().scale.is_some());
        assert!(
            c.active_patch(true, false, false)
                .unwrap()
                .opacity
                .is_some()
        );
        assert!(c.active_patch(false, false, false).is_none());
    }

    #[test]
    fn when_builds_with_enter_exit() {
        let open = Signal::new(true);
        let w = when(open, crate::views::Unit)
            .enter(fade().slide_y(-8.0))
            .exit(fade().slide_y(-8.0));
        let _ = w.keyed_children();
    }
}
