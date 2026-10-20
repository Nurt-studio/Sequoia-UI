use std::cell::RefCell;
use std::collections::HashMap;

use sequoia_core::{Color, Edges, Length};
use sequoia_layout::LayoutStyle;
use sequoia_reactive::Signal;
use sequoia_style::{SheetStyle, Theme};
use sequoia_view::{
    Anchor, AnyView, Dismiss, InputMode, OverlayEntry, OverlayId, OverlayLayer, OverlayOverflow,
    PaintCtx, Placement, Unit, View,
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum SheetSide {
    #[default]
    Bottom,
    Top,
    Left,
    Right,
}

impl SheetSide {
    fn placement(self) -> Placement {
        match self {
            SheetSide::Bottom => Placement::Bottom,
            SheetSide::Top => Placement::Top,
            SheetSide::Left => Placement::Left,
            SheetSide::Right => Placement::Right,
        }
    }
}

struct SheetPanel {
    child: AnyView,
    width: Length,
    height: Length,
    padding: Edges,
    background: Color,
    border: Color,
    border_width: f32,
    radius: f32,
}

impl View for SheetPanel {
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
        if self.border_width > 0.0 && self.border.a > 0.0 {
            ctx.bordered_quad(
                ctx.rect,
                self.background,
                self.radius,
                self.border_width,
                self.border,
            );
        } else {
            ctx.quad(ctx.rect, self.background, self.radius);
        }
    }
}

pub struct Sheet {
    open: Signal<bool>,
    child: Option<AnyView>,
    side: SheetSide,
    width: Length,
    height: Length,
    padding: Edges,
    style: Option<SheetStyle>,
    background: Option<Color>,
    border: Option<Color>,
    border_width: Option<f32>,
    radius: Option<f32>,
    dismiss: Dismiss,
    margin: Edges,
}

impl Sheet {
    pub fn new(open: Signal<bool>) -> Self {
        Self {
            open,
            child: None,
            side: SheetSide::Bottom,
            width: Length::Fit,
            height: Length::Fit,
            padding: Edges::ZERO,
            style: None,
            background: None,
            border: None,
            border_width: None,
            radius: None,
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

    pub fn side(mut self, side: SheetSide) -> Self {
        self.side = side;
        self
    }

    pub fn bottom(self) -> Self {
        self.side(SheetSide::Bottom)
    }

    pub fn top(self) -> Self {
        self.side(SheetSide::Top)
    }

    pub fn left(self) -> Self {
        self.side(SheetSide::Left)
    }

    pub fn right(self) -> Self {
        self.side(SheetSide::Right)
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

    pub fn style(mut self, style: SheetStyle) -> Self {
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
        let panel = SheetPanel {
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
                .unwrap_or(0.0)
                .max(0.0),
            radius: self.radius.or(style.radius).unwrap_or(theme.radius),
        };
        let id = ctx.show_overlay(
            OverlayEntry::new(panel)
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

impl View for Sheet {
    fn body(&self) -> impl View + 'static {
        Unit
    }
    fn is_leaf(&self) -> bool {
        true
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
