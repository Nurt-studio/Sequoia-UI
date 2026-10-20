use std::cell::Cell;
use std::rc::Rc;

use sequoia_assets::{AssetRegistry, icon_key};
use sequoia_core::{Color, Edges, Length, Rect, Size};
use sequoia_input::{CursorIcon, Event, EventPhase, Key, PointerButton, PointerEvent};
use sequoia_layout::{Align, Axis, ContainerStyle, LayoutCtx, LayoutStyle};
use sequoia_reactive::Signal;
use sequoia_style::{ButtonStyle, Theme, VisualState};
use sequoia_text::FontWeight;
use sequoia_view::{
    Anchor, AnyView, Dismiss, EventCtx, OverlayEntry, OverlayId, OverlayLayer, PaintCtx, Placement,
    Unit, View,
};

const PAD_X: f32 = 10.0;

const PAD_Y: f32 = 5.0;

const CHEV_GAP: f32 = 8.0;

const CHEV_W: f32 = 10.0;

const CHEV_H: f32 = 6.0;

const ITEM_H: f32 = 28.0;

const ITEM_PAD_X: f32 = 12.0;

const MENU_RADIUS: f32 = 4.0;

pub struct DropdownButton {
    selected: Signal<usize>,
    options: Vec<String>,
    placeholder: String,
    style: Option<ButtonStyle>,
    background: Option<Color>,
    hover: Option<Color>,
    pressed: Option<Color>,
    disabled_background: Option<Color>,
    text_color: Option<Color>,
    text_disabled: Option<Color>,
    font_size: Option<f32>,
    font_weight: Option<u16>,
    radius: Option<f32>,
    padding: Option<(f32, f32)>,
    border_width: Option<f32>,
    border_color: Option<Color>,
    menu_background: Option<Color>,
    menu_border: Option<Color>,
    item_height: f32,
    disabled: bool,
    on_select: Option<Rc<dyn Fn(usize)>>,

    chevron_icon: Option<String>,

    chevron_open_icon: Option<String>,

    chevron_size: Option<f32>,

    chevron_color: Option<Color>,

    open: Signal<bool>,
    highlight: Signal<usize>,
    overlay: Rc<Cell<Option<OverlayId>>>,
    margin: Edges,
    min_width: Option<f32>,
    max_width: Option<f32>,
    min_height: Option<f32>,
    max_height: Option<f32>,
}

impl DropdownButton {
    pub fn new(selected: Signal<usize>, options: Vec<impl Into<String>>) -> Self {
        Self {
            selected,
            options: options.into_iter().map(Into::into).collect(),
            placeholder: String::new(),
            style: None,
            background: None,
            hover: None,
            pressed: None,
            disabled_background: None,
            text_color: None,
            text_disabled: None,
            font_size: None,
            font_weight: None,
            radius: None,
            padding: None,
            border_width: None,
            border_color: None,
            menu_background: None,
            menu_border: None,
            item_height: ITEM_H,
            disabled: false,
            on_select: None,
            chevron_icon: None,
            chevron_open_icon: None,
            chevron_size: None,
            chevron_color: None,
            open: Signal::new(false),
            highlight: Signal::new(0),
            overlay: Rc::new(Cell::new(None)),
            margin: Edges::ZERO,
            min_width: None,
            max_width: None,
            min_height: None,
            max_height: None,
        }
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn style(mut self, style: ButtonStyle) -> Self {
        self.style = Some(style);
        self
    }

    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    pub fn hover(mut self, color: Color) -> Self {
        self.hover = Some(color);
        self
    }

    pub fn pressed(mut self, color: Color) -> Self {
        self.pressed = Some(color);
        self
    }

    pub fn disabled_background(mut self, color: Color) -> Self {
        self.disabled_background = Some(color);
        self
    }

    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color = Some(color);
        self
    }

    pub fn text_disabled(mut self, color: Color) -> Self {
        self.text_disabled = Some(color);
        self
    }

    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = Some(size);
        self
    }

    pub fn font_weight(mut self, weight: u16) -> Self {
        self.font_weight = Some(weight);
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

    pub fn border_width(mut self, width: f32) -> Self {
        self.border_width = Some(width);
        self
    }

    pub fn border_color(mut self, color: Color) -> Self {
        self.border_color = Some(color);
        self
    }

    pub fn border(mut self, width: f32, color: Color) -> Self {
        self.border_width = Some(width);
        self.border_color = Some(color);
        self
    }

    pub fn menu_background(mut self, color: Color) -> Self {
        self.menu_background = Some(color);
        self
    }

    pub fn menu_border(mut self, color: Color) -> Self {
        self.menu_border = Some(color);
        self
    }

    pub fn item_height(mut self, height: f32) -> Self {
        self.item_height = height.max(8.0);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn min_width(mut self, v: f32) -> Self {
        self.min_width = Some(v.max(0.0));
        self
    }

    pub fn max_width(mut self, v: f32) -> Self {
        self.max_width = Some(v.max(0.0));
        self
    }

    pub fn min_height(mut self, v: f32) -> Self {
        self.min_height = Some(v.max(0.0));
        self
    }

    pub fn max_height(mut self, v: f32) -> Self {
        self.max_height = Some(v.max(0.0));
        self
    }

    pub fn on_select(mut self, f: impl Fn(usize) + 'static) -> Self {
        self.on_select = Some(Rc::new(f));
        self
    }

    pub fn chevron_icon(mut self, name: impl Into<String>) -> Self {
        self.chevron_icon = Some(name.into());
        self
    }

    pub fn chevron_open_icon(mut self, name: impl Into<String>) -> Self {
        self.chevron_open_icon = Some(name.into());
        self
    }

    pub fn disclosure_icons(
        mut self,
        collapsed: impl Into<String>,
        expanded: impl Into<String>,
    ) -> Self {
        self.chevron_icon = Some(collapsed.into());
        self.chevron_open_icon = Some(expanded.into());
        self
    }

    pub fn chevron_size(mut self, px: f32) -> Self {
        self.chevron_size = Some(px.max(1.0));
        self
    }

    pub fn chevron_color(mut self, color: Color) -> Self {
        self.chevron_color = Some(color);
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

    fn open_state(&self) -> Signal<bool> {
        self.open.clone()
    }

    fn highlight_state(&self) -> Signal<usize> {
        self.highlight.clone()
    }

    fn take_overlay(&self) -> Option<OverlayId> {
        self.overlay.take()
    }

    fn display(&self) -> String {
        self.options
            .get(self.selected.get_untracked())
            .cloned()
            .unwrap_or_else(|| self.placeholder.clone())
    }

    fn showing_placeholder(&self) -> bool {
        self.options.get(self.selected.get_untracked()).is_none()
    }

    fn open(&self) {
        if self.disabled || self.options.is_empty() {
            return;
        }
        let sel = self.selected.get_untracked().min(self.options.len() - 1);
        self.highlight_state().set(sel);
        self.open_state().set(true);
    }

    fn close_with(&self, ctx: &mut EventCtx) {
        self.open_state().set(false);
        if let Some(id) = self.take_overlay() {
            ctx.hide_overlay(id);
        }
    }

    fn select_with(&self, idx: usize, ctx: &mut EventCtx) {
        if idx >= self.options.len() {
            return;
        }
        self.selected.set(idx);
        self.highlight_state().set(idx);
        if let Some(f) = &self.on_select {
            f(idx);
        }
        self.close_with(ctx);
        ctx.request_focus();
    }

    fn resolved_font_size(&self) -> f32 {
        self.font_size
            .or_else(|| self.style.and_then(|s| s.font_size))
            .unwrap_or(14.0)
    }

    fn resolved_weight(&self) -> FontWeight {
        self.font_weight
            .or_else(|| self.style.and_then(|s| s.font_weight))
            .map(FontWeight::new)
            .unwrap_or_default()
    }

    fn resolved_padding(&self) -> (f32, f32) {
        self.padding
            .or_else(|| self.style.and_then(|s| s.padding))
            .unwrap_or((PAD_X, PAD_Y))
    }
}

struct ResolvedButton {
    bg: Color,
    text: Color,
    font_size: f32,
    weight: FontWeight,
    radius: f32,
    border_width: f32,
    border_color: Color,
}

impl DropdownButton {
    fn resolve(&self, theme: &Theme, state: VisualState) -> ResolvedButton {
        let style = self.style.unwrap_or_default();

        let bg = if self.disabled || state.disabled {
            self.disabled_background
                .or(style.disabled)
                .unwrap_or(theme.disabled)
        } else if state.pressed {
            self.pressed
                .or(style.pressed)
                .unwrap_or(Color::hex(0x3A3A3A))
        } else if state.hovered {
            self.hover.or(style.hover).unwrap_or(Color::hex(0x323232))
        } else {
            self.background
                .or(style.background)
                .unwrap_or(Color::hex(0x2A2A2A))
        };
        let placeholder = self.showing_placeholder();
        let text = if self.disabled || state.disabled {
            self.text_disabled
                .or(style.text_disabled)
                .unwrap_or(theme.text_secondary)
        } else if placeholder {
            self.text_color
                .or(style.text)
                .unwrap_or(theme.text_secondary)
        } else {
            self.text_color.or(style.text).unwrap_or(theme.text)
        };
        ResolvedButton {
            bg,
            text,
            font_size: self
                .font_size
                .or(style.font_size)
                .unwrap_or(theme.font_size),
            weight: self
                .font_weight
                .or(style.font_weight)
                .map(FontWeight::new)
                .unwrap_or_default(),
            radius: self.radius.or(style.radius).unwrap_or(theme.radius),
            border_width: self
                .border_width
                .or(style.border_width)
                .unwrap_or(1.0)
                .max(0.0),
            border_color: self
                .border_color
                .or(style.border_color)
                .unwrap_or(theme.border),
        }
    }

    fn sync_overlay(&self, ctx: &mut PaintCtx, theme: &Theme) {
        let open = self.open_state();
        if self.disabled {
            if open.get_untracked() {
                open.set(false);
            }
            if let Some(id) = self.take_overlay() {
                ctx.hide_overlay(id);
            }
            return;
        }
        if !open.get_untracked() {
            if let Some(id) = self.take_overlay() {
                ctx.hide_overlay(id);
            }
            return;
        }
        if self.options.is_empty() {
            open.set(false);
            return;
        }
        if let Some(id) = self.overlay.get() {
            if !ctx.has_overlay(id) {
                self.take_overlay();
                open.set(false);
            }
            return;
        }
        let menu = DropdownMenu {
            overlay: Rc::clone(&self.overlay),
            options: self.options.clone(),
            selected: self.selected.clone(),
            open: open.clone(),
            highlight: self.highlight_state(),
            font_size: self.resolved_font_size(),
            weight: self.resolved_weight(),
            item_height: self.item_height,
            background: self.menu_background.unwrap_or(theme.surface),
            border: self.menu_border.unwrap_or(theme.border),
            radius: MENU_RADIUS,

            menu_min_width: ctx.rect.size.width.max(0.0),
            on_select: self.on_select.clone(),
        };

        let id = ctx.show_overlay(
            OverlayEntry::new(menu)
                .anchor(Anchor::View(ctx.element))
                .placement(Placement::Bottom)
                .layer(OverlayLayer::Popup)
                .dismiss(Dismiss::FocusLost)
                .offset(0.0, 4.0),
        );
        self.overlay.set(Some(id));
    }
}

impl View for DropdownButton {
    fn reuse_state(&self, old: &Self) {
        self.open.set(old.open.get_untracked());
        self.highlight.set(old.highlight.get_untracked());
        self.overlay.set(old.overlay.get());
    }

    fn body(&self) -> impl View + 'static {
        Unit
    }
    fn is_leaf(&self) -> bool {
        true
    }
    fn focusable(&self) -> bool {
        !self.disabled
    }
    fn cursor_icon(&self, _state: VisualState) -> CursorIcon {
        if self.disabled {
            CursorIcon::Default
        } else {
            CursorIcon::Pointer
        }
    }
    fn has_visual_state(&self) -> bool {
        true
    }
    fn keyed_children(&self) -> Option<Vec<(u64, AnyView)>> {
        let _ = self.selected.get();
        let _ = self.open_state().get();
        let _ = self.highlight_state().get();
        Some(Vec::new())
    }
    fn layout_style(&self) -> LayoutStyle {
        let mut style = LayoutStyle {
            width: Length::Fit,
            height: Length::Fit,
            margin: self.margin,
            ..LayoutStyle::default()
        };
        if let Some(v) = self.min_width {
            style = style.min_width(v);
        }
        if let Some(v) = self.max_width {
            style = style.max_width(v);
        }
        if let Some(v) = self.min_height {
            style = style.min_height(v);
        }
        if let Some(v) = self.max_height {
            style = style.max_height(v);
        }
        style
    }
    fn intrinsic_size(&self, ctx: &mut LayoutCtx) -> Option<Size> {
        let size = self.resolved_font_size();
        let weight = self.resolved_weight();
        let (pad_x, pad_y) = self.resolved_padding();

        let mut w = ctx.text_width_weighted(&self.placeholder, size, weight);
        for opt in &self.options {
            w = w.max(ctx.text_width_weighted(opt, size, weight));
        }
        Some(Size::new(
            w + CHEV_GAP + CHEV_W + pad_x * 2.0,
            LayoutCtx::text_line_height(size) + pad_y * 2.0,
        ))
    }
    fn on_event(&self, event: &Event, ctx: &mut EventCtx) {
        if self.disabled {
            return;
        }
        let open = self.open_state();
        let is_open = open.get_untracked();
        match event {
            Event::Pointer(PointerEvent::Up {
                button: PointerButton::Left,
                ..
            }) => {
                if ctx.phase != EventPhase::Target {
                    return;
                }
                if ctx.state.pressed && ctx.hover_self {
                    if is_open {
                        open.set(false);
                        if let Some(id) = self.take_overlay() {
                            ctx.hide_overlay(id);
                        }
                    } else {
                        self.open();
                    }
                    ctx.request_focus();
                    ctx.handled();
                }
            }
            Event::Key(key) => {
                if ctx.phase != EventPhase::Target || !ctx.state.focused {
                    return;
                }
                if self.options.is_empty() {
                    return;
                }
                let len = self.options.len();
                let hl = self.highlight_state().get_untracked().min(len - 1);
                match key.key {
                    Key::ESCAPE => {
                        if is_open {
                            open.set(false);
                            if let Some(id) = self.take_overlay() {
                                ctx.hide_overlay(id);
                            }
                            ctx.handled();
                        }
                    }
                    Key::ENTER | Key::SPACE => {
                        if is_open {
                            self.select_with(hl, ctx);
                        } else {
                            self.open();
                        }
                        ctx.handled();
                    }
                    Key::ARROW_DOWN => {
                        if is_open {
                            self.highlight_state().set((hl + 1).min(len - 1));
                        } else {
                            self.open();
                        }
                        ctx.handled();
                    }
                    Key::ARROW_UP => {
                        if is_open {
                            self.highlight_state().set(hl.saturating_sub(1));
                        } else {
                            self.open();
                        }
                        ctx.handled();
                    }
                    Key::HOME => {
                        if is_open {
                            self.highlight_state().set(0);
                            ctx.handled();
                        }
                    }
                    Key::END if is_open => {
                        self.highlight_state().set(len - 1);
                        ctx.handled();
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
    fn on_unmount(&self) {
        self.overlay.set(None);
    }
    fn paint_self(&self, ctx: &mut PaintCtx) {
        let theme = ctx.theme();
        let mut vs = ctx.state;
        vs.disabled = self.disabled;

        if self.open_state().get_untracked() {
            vs.pressed = true;
        }
        let r = self.resolve(&theme, vs);
        if r.border_width > 0.0 && r.border_color.a > 0.0 {
            ctx.bordered_quad(ctx.rect, r.bg, r.radius, r.border_width, r.border_color);
        } else {
            ctx.quad(ctx.rect, r.bg, r.radius);
        }

        let label = self.display();
        let (pad_x, _) = self.resolved_padding();
        let x = ctx.rect.left() + pad_x;
        let y = ctx.rect.top() + ctx.rect.size.height * 0.5 + r.font_size * 0.35;
        ctx.draw_text_weighted(&label, (x, y), r.font_size, r.weight, r.text);

        let cx = ctx.rect.right() - pad_x - CHEV_W;
        let cy = ctx.rect.top() + ctx.rect.size.height * 0.5;
        let hw = CHEV_W * 0.5;
        let hh = CHEV_H * 0.5;

        let is_open = self.open_state().get_untracked();
        let icon_name: Option<&str> = if is_open {
            self.chevron_open_icon
                .as_deref()
                .or(self.chevron_icon.as_deref())
        } else {
            self.chevron_icon.as_deref()
        };
        let mut painted_icon = false;
        if let Some(name) = icon_name {
            let px = self.chevron_size.unwrap_or(10.0);
            let color = self.chevron_color.unwrap_or(r.text);
            if let Some(assets) = ctx.environment.get::<AssetRegistry>() {
                let raster_px = ((px * ctx.scale()).round() as u32).max(1);
                if let Ok(sprite) = assets.rasterize_icon(name, raster_px) {
                    let dst = Rect::from_xywh(cx + hw - px * 0.5, cy - px * 0.5, px, px);
                    ctx.draw_sprite(
                        icon_key(name, raster_px),
                        sprite.width,
                        sprite.height,
                        &sprite.alpha,
                        dst,
                        color,
                    );
                    painted_icon = true;
                }
            }
        }
        if !painted_icon {
            ctx.polygon(
                &[
                    sequoia_core::Point::new(cx, cy - hh),
                    sequoia_core::Point::new(cx + CHEV_W, cy - hh),
                    sequoia_core::Point::new(cx + hw, cy + hh),
                ],
                r.text,
            );
        }

        self.sync_overlay(ctx, &theme);
    }
}

struct DropdownMenu {
    overlay: Rc<Cell<Option<OverlayId>>>,
    options: Vec<String>,
    selected: Signal<usize>,
    open: Signal<bool>,
    highlight: Signal<usize>,
    font_size: f32,
    weight: FontWeight,
    item_height: f32,
    background: Color,
    border: Color,
    radius: f32,
    menu_min_width: f32,
    on_select: Option<Rc<dyn Fn(usize)>>,
}

impl View for DropdownMenu {
    fn body(&self) -> impl View + 'static {
        Unit
    }
    fn children_override(&self) -> Option<Vec<AnyView>> {
        Some(
            self.options
                .iter()
                .enumerate()
                .map(|(i, label)| {
                    AnyView::new(DropdownItem {
                        overlay: Rc::clone(&self.overlay),
                        index: i,
                        label: label.clone(),
                        selected: self.selected.clone(),
                        open: self.open.clone(),
                        highlight: self.highlight.clone(),
                        font_size: self.font_size,
                        weight: self.weight,
                        item_height: self.item_height,
                        on_select: self.on_select.clone(),
                    })
                })
                .collect(),
        )
    }
    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fit,
            height: Length::Fit,
            ..LayoutStyle::default().min_width(self.menu_min_width.max(0.0))
        }
    }
    fn container(&self) -> Option<ContainerStyle> {
        Some(ContainerStyle {
            axis: Axis::Vertical,
            gap: 0.0,
            cross_gap: 0.0,
            align: Align::Stretch,
            justify: sequoia_layout::Justify::Start,
            wrap: false,
            align_content: Align::Start,
        })
    }
    fn paint_self(&self, ctx: &mut PaintCtx) {
        let rect = ctx.rect;
        if rect.size.width <= 0.0 || rect.size.height <= 0.0 {
            return;
        }
        ctx.shadow(
            rect,
            self.radius,
            16.0,
            sequoia_core::Point::new(0.0, 4.0),
            Color::rgba(0.0, 0.0, 0.0, 0.3),
        );
        ctx.bordered_quad(rect, self.background, self.radius, 1.0, self.border);
    }
}

struct DropdownItem {
    overlay: Rc<Cell<Option<OverlayId>>>,
    index: usize,
    label: String,
    selected: Signal<usize>,
    open: Signal<bool>,
    highlight: Signal<usize>,
    font_size: f32,
    weight: FontWeight,
    item_height: f32,
    on_select: Option<Rc<dyn Fn(usize)>>,
}

impl DropdownItem {
    fn select(&self, ctx: &mut EventCtx) {
        self.selected.set(self.index);
        self.highlight.set(self.index);
        if let Some(f) = &self.on_select {
            f(self.index);
        }
        self.open.set(false);
        if let Some(id) = self.overlay.take() {
            ctx.hide_overlay(id);
        }
        ctx.handled();
    }
}

impl View for DropdownItem {
    fn body(&self) -> impl View + 'static {
        Unit
    }
    fn is_leaf(&self) -> bool {
        true
    }
    fn has_visual_state(&self) -> bool {
        true
    }
    fn captures_pointer(&self) -> bool {
        true
    }
    fn keyed_children(&self) -> Option<Vec<(u64, AnyView)>> {
        let _ = self.selected.get();
        let _ = self.highlight.get();
        Some(Vec::new())
    }
    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fit,
            height: Length::Px(self.item_height),
            ..LayoutStyle::default()
        }
    }
    fn intrinsic_size(&self, ctx: &mut LayoutCtx) -> Option<Size> {
        let w = ctx.text_width_weighted(&self.label, self.font_size, self.weight);
        Some(Size::new(w + ITEM_PAD_X * 2.0, self.item_height))
    }
    fn on_event(&self, event: &Event, ctx: &mut EventCtx) {
        if ctx.phase != EventPhase::Target {
            return;
        }
        match event {
            Event::Pointer(PointerEvent::Up {
                button: PointerButton::Left,
                ..
            }) => {
                if ctx.state.pressed && ctx.hover_self {
                    self.select(ctx);
                }
            }
            Event::Pointer(PointerEvent::Move { .. }) => {
                if ctx.hover_self {
                    let cur = self.highlight.get_untracked();
                    if cur != self.index {
                        self.highlight.set(self.index);
                    }
                }
            }
            Event::Key(key) if key.key == Key::ESCAPE => {
                self.open.set(false);
                if let Some(id) = self.overlay.take() {
                    ctx.hide_overlay(id);
                }
                ctx.handled();
            }
            _ => {}
        }
    }
    fn paint_self(&self, ctx: &mut PaintCtx) {
        let theme = ctx.theme();
        let rect = ctx.rect;
        if rect.size.width <= 0.0 || rect.size.height <= 0.0 {
            return;
        }
        let is_selected = self.selected.get_untracked() == self.index;
        let hl = self.highlight.get_untracked() == self.index;

        if ctx.state.hovered || hl {
            ctx.quad(
                Rect::from_xywh(
                    rect.left() + 2.0,
                    rect.top(),
                    (rect.size.width - 4.0).max(0.0),
                    rect.size.height,
                ),
                Color::hex(0x4A4A4A),
                4.0,
            );
        } else if is_selected {
            ctx.quad(
                Rect::from_xywh(
                    rect.left() + 2.0,
                    rect.top(),
                    (rect.size.width - 4.0).max(0.0),
                    rect.size.height,
                ),
                Color::hex(0x3A3A3A),
                4.0,
            );
        }
        let weight = if is_selected {
            FontWeight::SEMIBOLD
        } else {
            self.weight
        };
        let x = rect.left() + ITEM_PAD_X;
        let y = rect.top() + rect.size.height * 0.5 + self.font_size * 0.35;
        ctx.draw_text_weighted(&self.label, (x, y), self.font_size, weight, theme.text);
    }
}

fn sanitize_margin(v: f32) -> f32 {
    if v.is_finite() { v.max(0.0) } else { 0.0 }
}
