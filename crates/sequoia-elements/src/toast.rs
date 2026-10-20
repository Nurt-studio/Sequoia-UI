#![forbid(unsafe_code)]

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::time::{Duration, Instant};

use sequoia_core::{Color, Edges, Length, Vec2};
use sequoia_layout::{Align, LayoutStyle};
use sequoia_reactive::Signal;
use sequoia_style::{Backdrop, Theme, ToastStyle};
use sequoia_view::{
    Anchor, AnyView, BackdropBlur, Column, Dismiss, Easing, InputMode, OverlayEntry, OverlayId,
    OverlayLayer, OverlayOverflow, PaintCtx, Placement, Presence, Row, Text, Unit, View,
};

use crate::Container;

pub(crate) const TOAST_ENTER_SECS: f32 = 0.3;
pub(crate) const TOAST_EXIT_SECS: f32 = 0.25;
const TOAST_ENTER_EASE: Easing = Easing::CubicBezier(0.3, 1.4, 0.4, 1.0);
const TOAST_ENTER_DY: f32 = 14.0;
const TOAST_EXIT_DY: f32 = 8.0;

thread_local! {
    static ENTRIES: RefCell<HashMap<usize, (OverlayId, u64)>> = RefCell::new(HashMap::new());
}

fn get_entry(key: usize) -> Option<(OverlayId, u64)> {
    ENTRIES.with(|m| m.borrow().get(&key).copied())
}

fn insert_entry(key: usize, id: OverlayId, version: u64) {
    ENTRIES.with(|m| {
        m.borrow_mut().insert(key, (id, version));
    });
}

fn take_entry(key: usize) -> Option<(OverlayId, u64)> {
    ENTRIES.with(|m| m.borrow_mut().remove(&key))
}

pub const DEFAULT_TTL: Duration = Duration::from_secs(4);

pub(crate) const BOTTOM_GAP: f32 = 22.0;

pub(crate) const CARD_WIDTH: f32 = 320.0;

pub const DEFAULT_MAX_VISIBLE: usize = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ToastKind {
    Info,
    Success,
    Warn,
    Error,
}

impl ToastKind {
    pub fn fallback_color(self) -> Color {
        match self {
            ToastKind::Info => Color::hex(0x2383E2),
            ToastKind::Success => Color::hex(0x3FB950),
            ToastKind::Warn => Color::hex(0xD29922),
            ToastKind::Error => Color::hex(0xF85149),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Toast {
    pub id: u64,
    pub kind: ToastKind,
    pub title: String,
    pub message: String,
    pub ttl: Duration,
}

#[derive(Clone)]
pub struct ToastQueue {
    items: Signal<Vec<Toast>>,
    deadlines: Rc<RefCell<HashMap<u64, Instant>>>,
    next: Rc<Cell<u64>>,
}

impl ToastQueue {
    pub fn new() -> Self {
        Self {
            items: Signal::new(Vec::new()),
            deadlines: Rc::new(RefCell::new(HashMap::new())),
            next: Rc::new(Cell::new(1)),
        }
    }

    pub fn items(&self) -> Signal<Vec<Toast>> {
        self.items.clone()
    }

    pub fn push(
        &self,
        kind: ToastKind,
        title: impl Into<String>,
        message: impl Into<String>,
        ttl: Duration,
    ) -> u64 {
        let id = self.next.get();
        self.next.set(id.wrapping_add(1).max(1));
        let toast = Toast {
            id,
            kind,
            title: title.into(),
            message: message.into(),
            ttl,
        };
        self.deadlines.borrow_mut().insert(id, Instant::now() + ttl);
        self.items.update(|v| v.push(toast));
        id
    }

    pub fn info(&self, title: impl Into<String>, message: impl Into<String>) -> u64 {
        self.push(ToastKind::Info, title, message, DEFAULT_TTL)
    }

    pub fn success(&self, title: impl Into<String>, message: impl Into<String>) -> u64 {
        self.push(ToastKind::Success, title, message, DEFAULT_TTL)
    }

    pub fn warn(&self, title: impl Into<String>, message: impl Into<String>) -> u64 {
        self.push(ToastKind::Warn, title, message, DEFAULT_TTL)
    }

    pub fn error(&self, title: impl Into<String>, message: impl Into<String>) -> u64 {
        self.push(ToastKind::Error, title, message, DEFAULT_TTL)
    }

    pub fn dismiss(&self, id: u64) -> bool {
        let had = self.items.get_untracked().iter().any(|t| t.id == id);
        self.deadlines.borrow_mut().remove(&id);
        self.items.update(|v| v.retain(|t| t.id != id));
        had
    }

    pub fn clear(&self) {
        self.deadlines.borrow_mut().clear();
        self.items.set(Vec::new());
    }

    pub fn len(&self) -> usize {
        self.items.get_untracked().len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.get_untracked().is_empty()
    }

    pub(crate) fn deadline(&self, id: u64) -> Option<Instant> {
        self.deadlines.borrow().get(&id).copied()
    }
}

impl Default for ToastQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
struct ResolvedRow {
    background: Color,
    border: Color,
    border_width: f32,
    text: Color,
    text_secondary: Color,
    font_size: f32,
    radius: f32,
    pad_x: f32,
    pad_y: f32,
    accent: Color,
    width: f32,
    backdrop: Backdrop,
}

struct ToastRow {
    toast: Toast,
    queue: ToastQueue,
    style: ResolvedRow,
    content: AnyView,

    fade_out: Rc<RefCell<Option<Instant>>>,
}

impl ToastRow {
    fn new(toast: Toast, queue: ToastQueue, style: &ResolvedRow) -> Self {
        let dismiss_queue = queue.clone();
        let dismiss_id = toast.id;
        let text_w = (style.width - style.pad_x * 2.0 - 56.0).max(80.0);
        let mut col = Column::new().gap(2.0).child(
            Text::new(toast.title.clone())
                .size(style.font_size)
                .bold()
                .color(style.text),
        );
        if !toast.message.is_empty() {
            col = col.child(
                Text::new(toast.message.clone())
                    .size((style.font_size - 1.0).max(10.0))
                    .color(style.text_secondary)
                    .wrap_width(text_w),
            );
        }

        let content = AnyView::new(
            Presence::new(Signal::new(true))
                .from(0.0)
                .enter_duration(TOAST_ENTER_SECS)
                .enter_easing(TOAST_ENTER_EASE)
                .slide(0.0, TOAST_ENTER_DY)
                .child(BackdropBlur::new(
                    Container::new()
                        .width(style.width)
                        .background(style.background)
                        .border(style.border_width, style.border)
                        .radius(style.radius)
                        .padding_xy(style.pad_x, style.pad_y)
                        .child(
                            Row::new()
                                .gap(10.0)
                                .align_items(Align::Center)
                                .child(
                                    Container::new()
                                        .square(8.0)
                                        .radius(4.0)
                                        .background(style.accent),
                                )
                                .child(col)
                                .child(
                                    Container::new()
                                        .padding_xy(4.0, 4.0)
                                        .child(
                                            Text::new("×").size(14.0).color(style.text_secondary),
                                        )
                                        .on_click(move || {
                                            dismiss_queue.dismiss(dismiss_id);
                                        }),
                                ),
                        ),
                    Backdrop {
                        radius: style.radius,
                        ..style.backdrop
                    },
                )),
        );
        Self {
            toast,
            queue,
            style: style.clone(),
            content,
            fade_out: Rc::new(RefCell::new(None)),
        }
    }

    fn exit_progress(&self) -> Option<f32> {
        let start = (*self.fade_out.borrow())?;
        let el = Instant::now().duration_since(start).as_secs_f32();
        Some((el / TOAST_EXIT_SECS.max(f32::MIN_POSITIVE)).clamp(0.0, 1.0))
    }
}

impl View for ToastRow {
    fn body(&self) -> impl View + 'static {
        Unit
    }
    fn children_override(&self) -> Option<Vec<AnyView>> {
        Some(vec![self.content.clone()])
    }
    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fit,
            height: Length::Fit,
            ..LayoutStyle::default()
        }
    }
    fn paint_opacity(&self) -> Option<f32> {
        self.exit_progress().map(|p| 1.0 - p)
    }
    fn paint_offset(&self) -> Vec2 {
        match self.exit_progress() {
            Some(p) => Vec2::new(0.0, TOAST_EXIT_DY * p),
            None => Vec2::ZERO,
        }
    }
    fn paint_self(&self, ctx: &mut PaintCtx) {
        let _ = &self.style;
        if let Some(deadline) = self.queue.deadline(self.toast.id)
            && Instant::now() >= deadline
        {
            let now = Instant::now();
            let start = *self.fade_out.borrow_mut().get_or_insert(now);
            if now.duration_since(start).as_secs_f32() >= TOAST_EXIT_SECS {
                self.queue.dismiss(self.toast.id);
                return;
            }
        }

        ctx.request_frame();
    }
}

struct ToastColumn {
    rows: Vec<(u64, AnyView)>,
}

impl View for ToastColumn {
    fn body(&self) -> impl View + 'static {
        Unit
    }
    fn keyed_children(&self) -> Option<Vec<(u64, AnyView)>> {
        Some(self.rows.clone())
    }
    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fit,
            height: Length::Fit,
            ..LayoutStyle::default()
        }
    }
}

pub struct ToastStack {
    queue: ToastQueue,
    style: Option<ToastStyle>,
    background: Option<Color>,
    border: Option<Color>,
    border_width: Option<f32>,
    text: Option<Color>,
    text_secondary: Option<Color>,
    font_size: Option<f32>,
    radius: Option<f32>,
    padding: Option<(f32, f32)>,
    backdrop: Option<Backdrop>,
    max_visible: usize,
    width: f32,
    margin: Edges,
}

impl ToastStack {
    pub fn new(queue: ToastQueue) -> Self {
        Self {
            queue,
            style: None,
            background: None,
            border: None,
            border_width: None,
            text: None,
            text_secondary: None,
            font_size: None,
            radius: None,
            padding: None,
            backdrop: None,
            max_visible: DEFAULT_MAX_VISIBLE,
            width: CARD_WIDTH,
            margin: Edges::ZERO,
        }
    }

    pub fn style(mut self, style: ToastStyle) -> Self {
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

    pub fn text_color(mut self, color: Color) -> Self {
        self.text = Some(color);
        self
    }

    pub fn text_secondary(mut self, color: Color) -> Self {
        self.text_secondary = Some(color);
        self
    }

    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = Some(size.max(1.0));
        self
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = Some(radius.max(0.0));
        self
    }

    pub fn padding_xy(mut self, x: f32, y: f32) -> Self {
        self.padding = Some((x.max(0.0), y.max(0.0)));
        self
    }

    pub fn backdrop(mut self, backdrop: Backdrop) -> Self {
        self.backdrop = Some(backdrop);
        self
    }

    pub fn max_visible(mut self, n: usize) -> Self {
        self.max_visible = n.max(1);
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = width.max(120.0);
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

    pub fn queue(&self) -> ToastQueue {
        self.queue.clone()
    }

    fn key(&self) -> usize {
        self.queue.items.id()
    }

    fn accent(&self, kind: ToastKind, style: ToastStyle) -> Color {
        let slot = match kind {
            ToastKind::Info => style.info,
            ToastKind::Success => style.success,
            ToastKind::Warn => style.warn,
            ToastKind::Error => style.error,
        };
        slot.unwrap_or_else(|| kind.fallback_color())
    }

    fn sync_overlay(&self, ctx: &mut PaintCtx, theme: &Theme) {
        let key = self.key();
        let items = self.queue.items.get_untracked();
        if items.is_empty() {
            if let Some((id, _)) = take_entry(key) {
                ctx.hide_overlay(id);
            }
            return;
        }
        let version = self.queue.items.version();
        if let Some((id, shown)) = get_entry(key) {
            if !ctx.has_overlay(id) {
                take_entry(key);
            } else if shown == version {
                return;
            } else {
                ctx.hide_overlay(id);
                take_entry(key);
            }
        }
        let style = self.style.unwrap_or_default();
        let (pad_x, pad_y) = self.padding.or(style.padding).unwrap_or((12.0, 10.0));
        let font_size = self
            .font_size
            .or(style.font_size)
            .unwrap_or(theme.font_size);
        let backdrop = self.backdrop.or(style.backdrop).unwrap_or(Backdrop::XS);
        let max = self.max_visible;
        let mut rows: Vec<(u64, AnyView)> = Vec::new();
        for toast in items.iter().take(max) {
            let rs = ResolvedRow {
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
                text: self.text.or(style.text).unwrap_or(theme.text),
                text_secondary: self
                    .text_secondary
                    .or(style.text_secondary)
                    .unwrap_or(theme.text_secondary),
                font_size,
                radius: self.radius.or(style.radius).unwrap_or(4.0),
                pad_x,
                pad_y,
                accent: self.accent(toast.kind, style),
                width: self.width,
                backdrop,
            };
            rows.push((
                toast.id,
                AnyView::new(ToastRow::new(toast.clone(), self.queue.clone(), &rs)),
            ));
        }
        if items.len() > max {
            let more = items.len() - max;
            rows.push((
                u64::MAX,
                AnyView::new(
                    Container::new()
                        .background(Color::TRANSPARENT)
                        .padding_xy(4.0, 2.0)
                        .child(
                            Text::new(format!("+{more} more"))
                                .size((font_size - 1.0).max(10.0))
                                .color(theme.text_secondary),
                        ),
                ),
            ));
        }
        let id = ctx.show_overlay(
            OverlayEntry::new(ToastColumn { rows })
                .anchor(Anchor::Viewport)
                .placement(Placement::Bottom)
                .layer(OverlayLayer::System)
                .input_mode(InputMode::Interactive)
                .dismiss(Dismiss::Manual)
                .overflow(OverlayOverflow::Clamp)
                .offset(0.0, -BOTTOM_GAP),
        );
        insert_entry(key, id, version);
    }
}

impl View for ToastStack {
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
        let _ = self.queue.items.get();
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
