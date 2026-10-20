#![forbid(unsafe_code)]

use std::cell::RefCell;
use std::collections::HashMap;

use sequoia_core::{Color, Edges, Length};
use sequoia_layout::LayoutStyle;
use sequoia_reactive::Signal;
use sequoia_style::{Backdrop, DrawerStyle, Theme};
use sequoia_view::{
    Anchor, AnyView, Dismiss, Easing, InputMode, OverlayEntry, OverlayId, OverlayLayer,
    OverlayOverflow, PaintCtx, Placement, Presence, Unit, View,
};

use crate::modal::{DEFAULT_SCRIM, ModalScrim, OverlayPanel};

pub(crate) const DRAWER_ENTER_SECS: f32 = 0.3;
const DRAWER_ENTER_EASE: Easing = Easing::CubicBezier(0.4, 0.0, 0.2, 1.0);

const DRAWER_ENTER_X: f32 = 360.0;
const DRAWER_ENTER_Y: f32 = 480.0;

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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum DrawerSide {
    Top,
    Bottom,
    Left,
    #[default]
    Right,
}

impl DrawerSide {
    fn placement(self) -> Placement {
        match self {
            DrawerSide::Top => Placement::Top,
            DrawerSide::Bottom => Placement::Bottom,
            DrawerSide::Left => Placement::Left,
            DrawerSide::Right => Placement::Right,
        }
    }
}

pub struct Drawer {
    open: Signal<bool>,
    child: Option<AnyView>,
    side: DrawerSide,
    width: Length,
    height: Length,
    padding: Edges,
    style: Option<DrawerStyle>,
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

impl Drawer {
    pub fn new(open: Signal<bool>) -> Self {
        Self {
            open,
            child: None,
            side: DrawerSide::Right,
            width: 330.0.into(),
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

    pub fn side(mut self, side: DrawerSide) -> Self {
        self.side = side;
        self
    }

    pub fn right(self) -> Self {
        self.side(DrawerSide::Right)
    }

    pub fn left(self) -> Self {
        self.side(DrawerSide::Left)
    }

    pub fn top(self) -> Self {
        self.side(DrawerSide::Top)
    }

    pub fn bottom(self) -> Self {
        self.side(DrawerSide::Bottom)
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

    pub fn style(mut self, style: DrawerStyle) -> Self {
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
        ModalScrim::new(self.open.clone())
            .scrim_color(self.scrim.or(style.scrim).unwrap_or(DEFAULT_SCRIM))
            .backdrop(
                self.scrim_backdrop
                    .or(style.backdrop)
                    .unwrap_or(Backdrop::NONE),
            )
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

        let (dx, dy) = match self.side {
            DrawerSide::Left => (-DRAWER_ENTER_X, 0.0),
            DrawerSide::Right => (DRAWER_ENTER_X, 0.0),
            DrawerSide::Top => (0.0, -DRAWER_ENTER_Y),
            DrawerSide::Bottom => (0.0, DRAWER_ENTER_Y),
        };

        let enter = Presence::new(Signal::new(true))
            .from(1.0)
            .enter_duration(DRAWER_ENTER_SECS)
            .enter_easing(DRAWER_ENTER_EASE)
            .slide(dx, dy)
            .child(panel);
        let id = ctx.show_overlay(
            OverlayEntry::new(enter)
                .anchor(Anchor::Viewport)
                .placement(self.side.placement())
                .layer(OverlayLayer::Modal)
                .input_mode(InputMode::Modal)
                .dismiss(self.dismiss)
                .overflow(OverlayOverflow::Clamp),
        );
        insert_entry(key, id);
    }
}

impl View for Drawer {
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
