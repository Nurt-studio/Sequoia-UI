use std::time::Instant;

use sequoia_core::{Color, Length, Rect};
use sequoia_layout::LayoutStyle;
use sequoia_reactive::Signal;
use sequoia_view::{AnyView, PaintCtx, Panel, Pulse, Unit, View};

#[derive(Clone)]
pub struct ProgressBar {
    value: Signal<f32>,
    height: f32,
    track: Option<Color>,
    fill: Option<Color>,
}

impl ProgressBar {
    pub fn new(value: Signal<f32>) -> Self {
        Self {
            value,
            height: 6.0,
            track: None,
            fill: None,
        }
    }

    pub fn height(mut self, h: f32) -> Self {
        if h.is_finite() && h > 0.0 {
            self.height = h;
        }
        self
    }

    pub fn track(mut self, color: Color) -> Self {
        self.track = Some(color);
        self
    }

    pub fn fill(mut self, color: Color) -> Self {
        self.fill = Some(color);
        self
    }

    fn clamped(&self) -> f32 {
        self.value.get_untracked().clamp(0.0, 1.0)
    }
}

impl View for ProgressBar {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fill(1.0),
            height: Length::Px(self.height),
            ..LayoutStyle::default()
        }
    }

    fn paint_self(&self, ctx: &mut PaintCtx) {
        let _ = self.value.get_untracked();
        let theme = ctx.theme();
        let track = self.track.unwrap_or(theme.surface);
        let fill = self.fill.unwrap_or(theme.accent);
        let r = ctx.rect;
        ctx.quad(r, track, r.size.height * 0.5);
        let w = r.size.width * self.clamped();
        if w > 0.5 {
            ctx.quad(
                Rect::from_xywh(r.left(), r.top(), w, r.size.height),
                fill,
                r.size.height * 0.5,
            );
        }
    }
}

#[derive(Clone)]
pub struct Spinner {
    size: f32,
    color: Option<Color>,
    period_secs: f32,
}

impl Spinner {
    pub fn new() -> Self {
        Self {
            size: 16.0,
            color: None,
            period_secs: 1.0,
        }
    }

    pub fn size(mut self, px: f32) -> Self {
        if px.is_finite() && px > 0.0 {
            self.size = px;
        }
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn period_secs(mut self, secs: f32) -> Self {
        if secs.is_finite() && secs > 0.0 {
            self.period_secs = secs;
        }
        self
    }
}

impl Default for Spinner {
    fn default() -> Self {
        Self::new()
    }
}

impl View for Spinner {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Px(self.size),
            height: Length::Px(self.size),
            ..LayoutStyle::default()
        }
    }

    fn paint_self(&self, ctx: &mut PaintCtx) {
        let theme = ctx.theme();
        let base = self.color.unwrap_or(theme.accent);
        let r = ctx.rect;
        let cx = r.left() + r.size.width * 0.5;
        let cy = r.top() + r.size.height * 0.5;
        let radius = r.size.width.min(r.size.height) * 0.38;
        let dot = (r.size.width.min(r.size.height) * 0.09).max(1.0);
        let elapsed = START.elapsed().as_secs_f32();
        let phase = (elapsed / self.period_secs) % 1.0;
        for i in 0..8 {
            let a = (i as f32 / 8.0) * std::f32::consts::TAU;

            let dist = ((i as f32 / 8.0) - phase).rem_euclid(1.0);
            let alpha = 1.0 - dist * 0.85;
            ctx.quad(
                Rect::from_xywh(
                    cx + radius * a.cos() - dot,
                    cy + radius * a.sin() - dot,
                    dot * 2.0,
                    dot * 2.0,
                ),
                base.with_alpha(alpha),
                dot,
            );
        }
        ctx.request_frame();
    }
}

static START: std::sync::LazyLock<Instant> = std::sync::LazyLock::new(Instant::now);

#[derive(Clone)]
pub struct Skeleton {
    width: Length,
    height: f32,
}

impl Default for Skeleton {
    fn default() -> Self {
        Self::new()
    }
}

impl Skeleton {
    pub fn new() -> Self {
        Self {
            width: Length::Fill(1.0),
            height: 16.0,
        }
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    pub fn height(mut self, h: f32) -> Self {
        if h.is_finite() && h > 0.0 {
            self.height = h;
        }
        self
    }
}

impl View for Skeleton {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn children_override(&self) -> Option<Vec<AnyView>> {
        Some(vec![AnyView::new(Pulse::new(
            Panel::new()
                .width(self.width)
                .height(Length::Px(self.height)),
        ))])
    }

    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: self.width,
            height: Length::Px(self.height),
            ..LayoutStyle::default()
        }
    }
}
