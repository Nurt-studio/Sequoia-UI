use std::rc::Rc;

use sequoia_core::{Color, Edges, Length, Rect, Size};
use sequoia_input::{CursorIcon, Event, EventPhase, Key, PointerButton, PointerEvent};
use sequoia_layout::{LayoutCtx, LayoutStyle};
use sequoia_style::{ChipStyle, Theme, VisualState};
use sequoia_view::{AssetRegistry, EventCtx, FontWeight, PaintCtx, Unit, View, icon_key};

const PAD_X: f32 = 10.0;
const PAD_Y: f32 = 4.0;
const RADIUS: f32 = 999.0;
const FONT_SIZE: f32 = 12.0;
const BORDER_W: f32 = 1.0;
const ICON_PX: f32 = 14.0;
const ICON_GAP: f32 = 6.0;
const REMOVE_PX: f32 = 14.0;
const REMOVE_GAP: f32 = 6.0;

pub fn remove_hit_rect(rect: Rect, pad_x: f32, box_px: f32) -> Rect {
    let s = box_px.max(0.0);
    Rect::from_xywh(
        rect.right() - pad_x - s,
        rect.top() + (rect.size.height - s) * 0.5,
        s,
        s,
    )
}

pub struct Chip {
    label: String,
    icon: Option<String>,
    selected: bool,
    disabled: bool,
    removable: bool,
    style: Option<ChipStyle>,
    background: Option<Color>,
    border: Option<Color>,
    text_color: Option<Color>,
    selected_background: Option<Color>,
    selected_border: Option<Color>,
    selected_text: Option<Color>,
    font_size: Option<f32>,
    radius: Option<f32>,
    padding: Option<(f32, f32)>,
    on_press: Option<Rc<dyn Fn()>>,
    on_remove: Option<Rc<dyn Fn()>>,
    margin: Edges,
}

impl Chip {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            selected: false,
            disabled: false,
            removable: false,
            style: None,
            background: None,
            border: None,
            text_color: None,
            selected_background: None,
            selected_border: None,
            selected_text: None,
            font_size: None,
            radius: None,
            padding: None,
            on_press: None,
            on_remove: None,
            margin: Edges::ZERO,
        }
    }

    pub fn icon(mut self, name: impl Into<String>) -> Self {
        self.icon = Some(name.into());
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn removable(mut self, removable: bool) -> Self {
        self.removable = removable;
        self
    }

    pub fn style(mut self, style: ChipStyle) -> Self {
        self.style = Some(style);
        self
    }

    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    pub fn border(mut self, color: Color) -> Self {
        self.border = Some(color);
        self
    }

    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color = Some(color);
        self
    }

    pub fn selected_background(mut self, color: Color) -> Self {
        self.selected_background = Some(color);
        self
    }

    pub fn selected_border(mut self, color: Color) -> Self {
        self.selected_border = Some(color);
        self
    }

    pub fn selected_text(mut self, color: Color) -> Self {
        self.selected_text = Some(color);
        self
    }

    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = Some(size);
        self
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = Some(radius);
        self
    }

    pub fn padding_xy(mut self, pad_x: f32, pad_y: f32) -> Self {
        self.padding = Some((pad_x, pad_y));
        self
    }

    pub fn on_press(mut self, f: impl Fn() + 'static) -> Self {
        self.on_press = Some(Rc::new(f));
        self
    }

    pub fn on_remove(mut self, f: impl Fn() + 'static) -> Self {
        self.on_remove = Some(Rc::new(f));
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

    fn interactive(&self) -> bool {
        !self.disabled && (self.on_press.is_some() || self.on_remove.is_some())
    }

    fn shows_remove(&self) -> bool {
        self.removable && self.on_remove.is_some() && !self.disabled
    }

    fn font_size_resolved(&self, style: ChipStyle) -> f32 {
        self.font_size.or(style.font_size).unwrap_or(FONT_SIZE)
    }

    fn padding_resolved(&self, style: ChipStyle) -> (f32, f32) {
        self.padding.or(style.padding).unwrap_or((PAD_X, PAD_Y))
    }

    fn resolve(&self, theme: &Theme) -> (Color, Color, Color) {
        let style = self.style.unwrap_or_default();
        if self.disabled {
            return (theme.disabled, theme.border, theme.text_secondary);
        }
        if self.selected {
            (
                self.selected_background
                    .or(style.selected_background)
                    .unwrap_or(Color::hex(0x3A3A3A)),
                self.selected_border
                    .or(style.selected_border)
                    .or(self.border)
                    .or(style.border)
                    .unwrap_or(Color::TRANSPARENT),
                self.selected_text
                    .or(style.selected_text)
                    .or(style.text)
                    .unwrap_or(theme.text),
            )
        } else {
            (
                self.background
                    .or(style.background)
                    .unwrap_or(theme.surface),
                self.border.or(style.border).unwrap_or(theme.border),
                self.text_color.or(style.text).unwrap_or(theme.text),
            )
        }
    }

    fn text_width(&self, ctx: &mut LayoutCtx, size: f32) -> f32 {
        ctx.text_width_weighted(&self.label, size, FontWeight::MEDIUM)
    }

    fn content_width(&self, ctx: &mut LayoutCtx, size: f32, pad_x: f32) -> f32 {
        let mut w = self.text_width(ctx, size);
        if self.icon.is_some() {
            w += ICON_PX + ICON_GAP;
        }
        if self.shows_remove() {
            w += REMOVE_GAP + REMOVE_PX;
        }
        w + pad_x * 2.0
    }

    fn press(&self, ctx: &mut EventCtx) {
        if let Some(f) = &self.on_press {
            f();
        }
        ctx.request_focus();
        ctx.handled();
    }

    fn remove(&self, ctx: &mut EventCtx) {
        if let Some(f) = &self.on_remove {
            f();
        }
        ctx.request_focus();
        ctx.handled();
    }
}

fn sanitize_margin(v: f32) -> f32 {
    if v.is_finite() { v.max(0.0) } else { 0.0 }
}

impl View for Chip {
    fn body(&self) -> impl View + 'static {
        Unit
    }
    fn is_leaf(&self) -> bool {
        true
    }
    fn focusable(&self) -> bool {
        self.interactive()
    }
    fn has_visual_state(&self) -> bool {
        self.interactive()
    }
    fn cursor_icon(&self, _state: VisualState) -> CursorIcon {
        if self.interactive() {
            CursorIcon::Pointer
        } else {
            CursorIcon::Default
        }
    }
    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fit,
            height: Length::Fit,
            margin: self.margin,
            ..LayoutStyle::default()
        }
    }
    fn intrinsic_size(&self, ctx: &mut LayoutCtx) -> Option<Size> {
        let style = self.style.unwrap_or_default();
        let size = self.font_size_resolved(style);
        let (pad_x, pad_y) = self.padding_resolved(style);
        Some(Size::new(
            self.content_width(ctx, size, pad_x),
            LayoutCtx::text_line_height(size) + pad_y * 2.0,
        ))
    }
    fn on_event(&self, event: &Event, ctx: &mut EventCtx) {
        if !self.interactive() || ctx.phase != EventPhase::Target {
            return;
        }
        match event {
            Event::Pointer(PointerEvent::Up {
                button: PointerButton::Left,
                pos,
                ..
            }) => {
                if !(ctx.state.pressed && ctx.hover_self) {
                    return;
                }
                if self.shows_remove() {
                    let style = self.style.unwrap_or_default();
                    let (pad_x, _) = self.padding_resolved(style);
                    if remove_hit_rect(ctx.rect, pad_x, REMOVE_PX).contains(*pos) {
                        self.remove(ctx);
                        return;
                    }
                }
                if self.on_press.is_some() {
                    self.press(ctx);
                }
            }
            Event::Pointer(PointerEvent::Down {
                button: PointerButton::Left,
                pos,
                ..
            }) => {
                if self.shows_remove() {
                    let style = self.style.unwrap_or_default();
                    let (pad_x, _) = self.padding_resolved(style);
                    if remove_hit_rect(ctx.rect, pad_x, REMOVE_PX).contains(*pos) {
                        ctx.handled();
                    }
                }
            }
            Event::Key(key) => {
                if !ctx.state.focused {
                    return;
                }
                match key.key {
                    Key::ENTER | Key::SPACE => {
                        if self.on_press.is_some() {
                            self.press(ctx);
                        }
                    }
                    Key::BACKSPACE | Key::DELETE if self.on_remove.is_some() && !self.disabled => {
                        self.remove(ctx);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
    fn paint_self(&self, ctx: &mut PaintCtx) {
        let rect = ctx.rect;
        if rect.size.width <= 0.0 || rect.size.height <= 0.0 || self.label.is_empty() {
            return;
        }
        let theme = ctx.theme();
        let style = self.style.unwrap_or_default();
        let size = self.font_size_resolved(style);
        let (pad_x, _) = self.padding_resolved(style);
        let radius = self.radius.or(style.radius).unwrap_or(RADIUS);
        let (bg, border, text) = self.resolve(&theme);

        let mut bg = bg;
        if ctx.state.hovered && self.interactive() && !self.selected {
            bg = Color::hex(0x4A4A4A);
        }
        ctx.bordered_quad(rect, bg, radius, BORDER_W, border);

        let mut x = rect.left() + pad_x;
        let line_h = LayoutCtx::text_line_height(size);
        if let Some(name) = &self.icon {
            if let Some(assets) = ctx.environment.get::<AssetRegistry>() {
                let raster_px = ((ICON_PX * ctx.scale_factor()).round() as u32).max(1);
                if let Ok(sprite) = assets.rasterize_icon(name, raster_px) {
                    let y = rect.top() + (rect.size.height - ICON_PX) * 0.5;
                    ctx.draw_sprite(
                        icon_key(name, raster_px),
                        sprite.width,
                        sprite.height,
                        &sprite.alpha,
                        Rect::from_xywh(x, y, ICON_PX, ICON_PX),
                        text,
                    );
                }
            }
            x += ICON_PX + ICON_GAP;
        }
        let y = rect.top() + rect.size.height * 0.5 + size * 0.35;
        ctx.draw_text_weighted(&self.label, (x, y), size, FontWeight::MEDIUM, text);
        let _ = line_h;

        if self.shows_remove() {
            let r = remove_hit_rect(rect, pad_x, REMOVE_PX);
            let col = theme.text_secondary;
            let w = ctx.text_width_weighted("×", size, FontWeight::MEDIUM);
            let x = r.left() + (r.size.width - w) * 0.5;
            let y = r.top() + r.size.height * 0.5 + size * 0.35;
            ctx.draw_text_weighted("×", (x, y), size, FontWeight::MEDIUM, col);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sequoia_core::Point;

    #[test]
    fn remove_zone_hugs_right_edge() {
        let rect = Rect::from_xywh(0.0, 0.0, 120.0, 24.0);
        let r = remove_hit_rect(rect, 10.0, 14.0);
        assert_eq!(r, Rect::from_xywh(96.0, 5.0, 14.0, 14.0));
        assert!(r.contains(Point::new(100.0, 12.0)));
        assert!(!r.contains(Point::new(50.0, 12.0)));
    }
}
