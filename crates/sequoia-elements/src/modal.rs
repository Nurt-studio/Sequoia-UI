#![forbid(unsafe_code)]

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use sequoia_core::{Color, Edges, Length};
use sequoia_layout::LayoutStyle;
use sequoia_reactive::Signal;
use sequoia_style::{Backdrop, BoxStyle, ModalStyle, ShadowSpec, Theme};
use sequoia_view::{
    Anchor, AnyView, Dismiss, Easing, ElementId, InputMode, OverlayEntry, OverlayId, OverlayLayer,
    OverlayOverflow, PaintCtx, Placement, Presence, Runtime, Unit, View,
};

thread_local! {
    static ENTRIES: RefCell<HashMap<usize, OverlayId>> = RefCell::new(HashMap::new());
}

fn get_entry(key: usize) -> Option<OverlayId> {
    ENTRIES.with(|m| m.borrow().get(&key).copied())
}

fn insert_entry(key: usize, id: OverlayId) {
    ENTRIES.with(|m| {
        m.borrow_mut().insert(key, id);
    });
}

fn take_entry(key: usize) -> Option<OverlayId> {
    ENTRIES.with(|m| m.borrow_mut().remove(&key))
}

thread_local! {
    static SCRIM_CLOCKS: RefCell<HashMap<usize, Rc<RefCell<Option<f32>>>>> =
        RefCell::new(HashMap::new());
}

fn scrim_clock(open_id: usize) -> Rc<RefCell<Option<f32>>> {
    SCRIM_CLOCKS.with(|m| m.borrow_mut().entry(open_id).or_default().clone())
}

fn forget_scrim_clock(open_id: usize) {
    SCRIM_CLOCKS.with(|m| {
        m.borrow_mut().remove(&open_id);
    });
}

pub const DEFAULT_SCRIM: Color = Color::rgba(0.0, 0.0, 0.0, 0.45);

pub(crate) const MODAL_ENTER_SECS: f32 = 0.22;
const MODAL_ENTER_EASE: Easing = Easing::CubicBezier(0.3, 1.3, 0.4, 1.0);
const MODAL_ENTER_DY: f32 = -8.0;

pub(crate) const SCRIM_FADE_SECS: f32 = 0.18;

struct ScrimFill {
    scrim: Color,
    backdrop: Backdrop,
    open_id: usize,
    fade: Rc<RefCell<Option<f32>>>,
}

fn fade_in_opacity(clock: &Rc<RefCell<Option<f32>>>, secs: f32, now: f32) -> (f32, bool) {
    let start = *clock.borrow_mut().get_or_insert(now);
    let el = (now - start).max(0.0);
    if el >= secs {
        (1.0, false)
    } else {
        (el / secs.max(f32::MIN_POSITIVE), true)
    }
}

impl View for ScrimFill {
    fn body(&self) -> impl View + 'static {
        Unit
    }
    fn is_leaf(&self) -> bool {
        true
    }
    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fill(1.0),
            height: Length::Fill(1.0),
            ..LayoutStyle::default()
        }
    }
    fn hit_testable(&self) -> bool {
        false
    }
    fn on_unmount(&self) {
        forget_scrim_clock(self.open_id);
    }
    fn paint_opacity_ctx(&self, rt: &Runtime, _el: ElementId) -> Option<f32> {
        Some(fade_in_opacity(&self.fade, SCRIM_FADE_SECS, rt.elapsed_secs()).0)
    }
    fn paint_self(&self, ctx: &mut PaintCtx) {
        ctx.push_backdrop(ctx.rect, self.backdrop);
        if self.scrim.a > 0.0 {
            ctx.quad(ctx.rect, self.scrim, 0.0);
        }
        let now = ctx.runtime().elapsed_secs();
        if fade_in_opacity(&self.fade, SCRIM_FADE_SECS, now).1 {
            ctx.request_frame();
        }
    }
}

fn default_shadow() -> ShadowSpec {
    ShadowSpec::new(Color::BLACK.with_alpha(0.35), 24.0).offset(0.0, 8.0)
}

pub(crate) struct OverlayPanel {
    pub child: AnyView,
    pub width: Length,
    pub height: Length,
    pub padding: Edges,
    pub background: Color,
    pub border: Color,
    pub border_width: f32,
    pub radius: f32,
    pub backdrop: Backdrop,
}

impl View for OverlayPanel {
    fn body(&self) -> impl View + 'static {
        Unit
    }
    fn children_override(&self) -> Option<Vec<AnyView>> {
        Some(vec![self.child.clone()])
    }
    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: self.width,
            height: self.height,
            padding: self.padding,
            ..LayoutStyle::default()
        }
    }
    fn paint_self(&self, ctx: &mut PaintCtx) {
        let radius = self.radius.max(0.0);
        ctx.push_backdrop(
            ctx.rect,
            Backdrop {
                radius,
                ..self.backdrop
            },
        );
        ctx.paint_box(
            BoxStyle::new()
                .background(self.background)
                .border(self.border_width.max(0.0), self.border)
                .radius(radius)
                .shadow(default_shadow()),
        );
    }
}

#[derive(Clone)]
pub struct ModalScrim {
    open: Signal<bool>,
    scrim: Color,
    backdrop: Backdrop,
}

impl ModalScrim {
    pub fn new(open: Signal<bool>) -> Self {
        Self {
            open,
            scrim: DEFAULT_SCRIM,
            backdrop: Backdrop::NONE,
        }
    }

    pub fn scrim_color(mut self, color: Color) -> Self {
        self.scrim = color;
        self
    }

    pub fn backdrop(mut self, backdrop: Backdrop) -> Self {
        self.backdrop = backdrop;
        self
    }

    fn is_open(&self) -> bool {
        self.open.get_untracked()
    }
}

impl View for ModalScrim {
    fn body(&self) -> impl View + 'static {
        Unit
    }
    fn keyed_children(&self) -> Option<Vec<(u64, AnyView)>> {
        let open = self.open.get();
        if !open {
            return Some(Vec::new());
        }
        Some(vec![(
            0,
            AnyView::new(ScrimFill {
                scrim: self.scrim,
                backdrop: Backdrop {
                    radius: 0.0,
                    ..self.backdrop
                },
                open_id: self.open.id(),

                fade: scrim_clock(self.open.id()),
            }),
        )])
    }
    fn layout_style(&self) -> LayoutStyle {
        if self.is_open() {
            LayoutStyle {
                width: Length::Fill(1.0),
                height: Length::Fill(1.0),
                ..LayoutStyle::default()
            }
        } else {
            LayoutStyle {
                width: Length::Fit,
                height: Length::Fit,
                ..LayoutStyle::default()
            }
        }
    }
    fn hit_testable(&self) -> bool {
        false
    }
}

pub struct Modal {
    open: Signal<bool>,
    child: Option<AnyView>,
    width: Length,
    height: Length,
    padding: Edges,
    style: Option<ModalStyle>,
    background: Option<Color>,
    border: Option<Color>,
    border_width: Option<f32>,
    radius: Option<f32>,
    backdrop: Option<Backdrop>,
    scrim: Option<Color>,
    scrim_backdrop: Option<Backdrop>,
    dismiss: Dismiss,
    margin: Edges,
}

impl Modal {
    pub fn new(open: Signal<bool>) -> Self {
        Self {
            open,
            child: None,
            width: Length::Fit,
            height: Length::Fit,
            padding: Edges::ZERO,
            style: None,
            background: None,
            border: None,
            border_width: None,
            radius: None,
            backdrop: None,
            scrim: None,
            scrim_backdrop: None,
            dismiss: Dismiss::OutsideClick,
            margin: Edges::ZERO,
        }
    }

    pub fn child<V: View + 'static>(mut self, child: V) -> Self {
        self.child = Some(AnyView::new(child));
        self
    }

    pub fn child_if<V: View + 'static>(mut self, cond: bool, make: impl FnOnce() -> V) -> Self {
        if cond {
            self.child = Some(AnyView::new(make()));
        }
        self
    }

    pub fn child_opt<V: View + 'static>(mut self, child: Option<V>) -> Self {
        if let Some(v) = child {
            self.child = Some(AnyView::new(v));
        }
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    pub fn size(mut self, width: impl Into<Length>, height: impl Into<Length>) -> Self {
        self.width = width.into();
        self.height = height.into();
        self
    }

    pub fn padding(mut self, padding: impl Into<Edges>) -> Self {
        self.padding = padding.into();
        self
    }

    pub fn pad(self, padding: impl Into<Edges>) -> Self {
        self.padding(padding)
    }

    pub fn padding_xy(mut self, x: f32, y: f32) -> Self {
        self.padding = Edges::xy(x, y);
        self
    }

    pub fn style(mut self, style: ModalStyle) -> Self {
        self.style = Some(style);
        self
    }

    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    pub fn border_color(mut self, color: Color) -> Self {
        self.border = Some(color);
        self
    }

    pub fn border_width(mut self, width: f32) -> Self {
        self.border_width = Some(width.max(0.0));
        self
    }

    pub fn border(mut self, width: f32, color: Color) -> Self {
        self.border_width = Some(width.max(0.0));
        self.border = Some(color);
        self
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = Some(radius.max(0.0));
        self
    }

    pub fn backdrop(mut self, backdrop: Backdrop) -> Self {
        self.backdrop = Some(backdrop);
        self
    }

    pub fn scrim_color(mut self, color: Color) -> Self {
        self.scrim = Some(color);
        self
    }

    pub fn scrim_backdrop(mut self, backdrop: Backdrop) -> Self {
        self.scrim_backdrop = Some(backdrop);
        self
    }

    pub fn dismiss(mut self, dismiss: Dismiss) -> Self {
        self.dismiss = dismiss;
        self
    }

    pub fn margin(mut self, margin: impl Into<Edges>) -> Self {
        let m = margin.into();
        self.margin = Edges::only(
            sanitize_margin(m.left),
            sanitize_margin(m.top),
            sanitize_margin(m.right),
            sanitize_margin(m.bottom),
        );
        self
    }

    pub fn margin_xy(self, x: f32, y: f32) -> Self {
        self.margin(Edges::xy(x, y))
    }

    pub fn margin_x(mut self, x: f32) -> Self {
        let x = sanitize_margin(x);
        self.margin.left = x;
        self.margin.right = x;
        self
    }

    pub fn margin_y(mut self, y: f32) -> Self {
        let y = sanitize_margin(y);
        self.margin.top = y;
        self.margin.bottom = y;
        self
    }

    pub fn margin_left(mut self, v: f32) -> Self {
        self.margin.left = sanitize_margin(v);
        self
    }

    pub fn margin_right(mut self, v: f32) -> Self {
        self.margin.right = sanitize_margin(v);
        self
    }

    pub fn margin_top(mut self, v: f32) -> Self {
        self.margin.top = sanitize_margin(v);
        self
    }

    pub fn margin_bottom(mut self, v: f32) -> Self {
        self.margin.bottom = sanitize_margin(v);
        self
    }

    pub fn scrim(&self) -> ModalScrim {
        let style = self.style.unwrap_or_default();
        ModalScrim {
            open: self.open.clone(),
            scrim: self.scrim.or(style.scrim).unwrap_or(DEFAULT_SCRIM),
            backdrop: self
                .scrim_backdrop
                .or(style.backdrop)
                .unwrap_or(Backdrop::NONE),
        }
    }

    pub fn is_open(&self) -> bool {
        self.open.get_untracked()
    }

    pub fn open(&self) {
        self.open.set(true);
    }

    pub fn close(&self) {
        self.open.set(false);
    }

    pub fn toggle(&self) {
        self.open.set(!self.open.get_untracked());
    }

    fn key(&self) -> usize {
        self.open.id()
    }

    fn sync_overlay(&self, ctx: &mut PaintCtx, theme: &Theme) {
        let key = self.key();
        if !self.open.get_untracked() {
            if let Some(id) = take_entry(key) {
                ctx.hide_overlay(id);
            }
            return;
        }
        if let Some(id) = get_entry(key) {
            if !ctx.has_overlay(id) {
                take_entry(key);
                self.open.set(false);
            }
            return;
        }
        let Some(child) = self.child.clone() else {
            self.open.set(false);
            return;
        };
        let style = self.style.unwrap_or_default();
        let radius = self.radius.or(style.radius).unwrap_or(theme.radius);
        let backdrop = self.backdrop.or(style.backdrop).unwrap_or(Backdrop::MD);
        let panel = OverlayPanel {
            child,
            width: self.width,
            height: self.height,
            padding: self.padding,
            background: self
                .background
                .or(style.background)
                .unwrap_or(theme.surface),
            border: self.border.or(style.border).unwrap_or(theme.border),
            border_width: self
                .border_width
                .or(style.border_width)
                .unwrap_or(1.0)
                .max(0.0),
            radius,
            backdrop,
        };

        let enter = Presence::new(Signal::new(true))
            .from(0.0)
            .enter_duration(MODAL_ENTER_SECS)
            .enter_easing(MODAL_ENTER_EASE)
            .slide(0.0, MODAL_ENTER_DY)
            .child(panel);
        let id = ctx.show_overlay(
            OverlayEntry::new(enter)
                .anchor(Anchor::Viewport)
                .placement(Placement::Center)
                .layer(OverlayLayer::Modal)
                .input_mode(InputMode::Modal)
                .dismiss(self.dismiss)
                .overflow(OverlayOverflow::Clamp),
        );
        insert_entry(key, id);
    }
}

impl View for Modal {
    fn body(&self) -> impl View + 'static {
        Unit
    }
    fn is_leaf(&self) -> bool {
        true
    }

    fn hit_testable(&self) -> bool {
        false
    }
    fn keyed_children(&self) -> Option<Vec<(u64, AnyView)>> {
        let _ = self.open.get();
        Some(Vec::new())
    }
    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fit,
            height: Length::Fit,
            margin: self.margin,
            ..LayoutStyle::default()
        }
    }
    fn on_unmount(&self) {
        take_entry(self.key());
    }
    fn paint_self(&self, ctx: &mut PaintCtx) {
        let theme = ctx.theme();
        self.sync_overlay(ctx, &theme);
    }
}

fn sanitize_margin(v: f32) -> f32 {
    if v.is_finite() { v.max(0.0) } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrim_clock_is_stable_across_rebuilds() {
        let open = Signal::new(true);
        let a = scrim_clock(open.id());
        let b = scrim_clock(open.id());
        assert!(Rc::ptr_eq(&a, &b), "same signal = same clock");

        let (v, _) = fade_in_opacity(&a, SCRIM_FADE_SECS, 1.0);
        let (v2, _) = fade_in_opacity(&b, SCRIM_FADE_SECS, 1.1);
        assert!(v2 > v, "fade does not undo between rebuilds: {v} -> {v2}");
        forget_scrim_clock(open.id());
    }

    #[test]
    fn scrim_clock_resets_after_close() {
        let open = Signal::new(true);
        let a = scrim_clock(open.id());
        let _ = fade_in_opacity(&a, SCRIM_FADE_SECS, 0.0);
        forget_scrim_clock(open.id());
        let b = scrim_clock(open.id());
        assert!(!Rc::ptr_eq(&a, &b), "after closing, the clock is fresh");
        assert!(b.borrow().is_none(), "fresh clock is not anchored");
        forget_scrim_clock(open.id());
    }
}
