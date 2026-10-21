use std::rc::Rc;

use sequoia_core::{Color, Edges, Length, Point, Size};
use sequoia_input::{CursorIcon, Event, EventPhase, Key, PointerButton, PointerEvent};
use sequoia_layout::{Align, LayoutCtx, LayoutStyle};
use sequoia_reactive::Signal;
use sequoia_style::RadioStyle;
use sequoia_view::{
    Accessible, AccessibleRole, AnyView, Column, EventCtx, PaintCtx, Row, Text, View,
};

const DIAMETER: f32 = 16.0;
const BORDER_W: f32 = 1.5;
const GAP: f32 = 8.0;
const DOT_RATIO: f32 = 0.52;
const DEFAULT_FONT_SIZE: f32 = 14.0;

pub struct RadioButton {
    selected: Signal<usize>,
    value: usize,
    label: Option<String>,
    style: Option<RadioStyle>,
    background: Option<Color>,
    checked_background: Option<Color>,
    border: Option<Color>,
    checked_border: Option<Color>,
    dot: Option<Color>,
    text_color: Option<Color>,
    font_size: Option<f32>,
    diameter: Option<f32>,
    border_width: Option<f32>,
    disabled: bool,
    on_select: Option<Rc<dyn Fn(usize)>>,
    margin: Edges,
}

impl RadioButton {
    pub fn new(selected: Signal<usize>, value: usize) -> Self {
        Self {
            selected,
            value,
            label: None,
            style: None,
            background: None,
            checked_background: None,
            border: None,
            checked_border: None,
            dot: None,
            text_color: None,
            font_size: None,
            diameter: None,
            border_width: None,
            disabled: false,
            on_select: None,
            margin: Edges::ZERO,
        }
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn style(mut self, style: RadioStyle) -> Self {
        self.style = Some(style);
        self
    }

    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    pub fn checked_background(mut self, color: Color) -> Self {
        self.checked_background = Some(color);
        self
    }

    pub fn border_color(mut self, color: Color) -> Self {
        self.border = Some(color);
        self
    }

    pub fn checked_border(mut self, color: Color) -> Self {
        self.checked_border = Some(color);
        self
    }

    pub fn dot_color(mut self, color: Color) -> Self {
        self.dot = Some(color);
        self
    }

    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color = Some(color);
        self
    }

    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = Some(size);
        self
    }

    pub fn diameter(mut self, size: f32) -> Self {
        self.diameter = Some(size);
        self
    }

    pub fn border_width(mut self, width: f32) -> Self {
        self.border_width = Some(width);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_select(mut self, f: impl Fn(usize) + 'static) -> Self {
        self.on_select = Some(Rc::new(f));
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

    pub fn value(&self) -> usize {
        self.value
    }

    pub fn selected_signal(&self) -> Signal<usize> {
        self.selected.clone()
    }

    pub fn is_selected(&self) -> bool {
        self.selected.get_untracked() == self.value
    }

    fn select(&self) -> bool {
        if self.disabled || self.is_selected() {
            return false;
        }
        self.selected.set(self.value);
        if let Some(f) = &self.on_select {
            f(self.value);
        }
        true
    }

    fn resolved_diameter(&self, style: RadioStyle) -> f32 {
        self.diameter
            .or(style.diameter)
            .unwrap_or(DIAMETER)
            .max(8.0)
    }

    fn resolved_border_width(&self, radius: f32) -> f32 {
        self.border_width.unwrap_or(BORDER_W).max(0.0).min(radius)
    }

    fn resolved_font_size(&self, style: RadioStyle) -> f32 {
        self.font_size
            .or(style.font_size)
            .unwrap_or(DEFAULT_FONT_SIZE)
    }
}

fn sanitize_margin(v: f32) -> f32 {
    if v.is_finite() { v.max(0.0) } else { 0.0 }
}

impl View for RadioButton {
    fn accessibility(&self) -> Option<Accessible> {
        Some(Accessible::new(
            AccessibleRole::Radio,
            self.label
                .clone()
                .unwrap_or_else(|| "przycisk radiowy".to_string()),
        ))
    }

    fn body(&self) -> impl View + 'static {
        sequoia_view::Unit
    }

    fn is_leaf(&self) -> bool {
        true
    }

    fn focusable(&self) -> bool {
        !self.disabled
    }

    fn has_visual_state(&self) -> bool {
        true
    }

    fn cursor_icon(&self, _state: sequoia_style::VisualState) -> CursorIcon {
        if self.disabled {
            CursorIcon::Default
        } else {
            CursorIcon::Pointer
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
        let diameter = self.resolved_diameter(style);
        let font_size = self.resolved_font_size(style);
        let label_w = self
            .label
            .as_deref()
            .map(|label| ctx.text_width(label, font_size))
            .unwrap_or(0.0);
        let gap = if self.label.is_some() { GAP } else { 0.0 };
        let text_h = if self.label.is_some() {
            LayoutCtx::text_line_height(font_size)
        } else {
            0.0
        };
        Some(Size::new(diameter + gap + label_w, diameter.max(text_h)))
    }

    fn on_event(&self, event: &Event, ctx: &mut EventCtx) {
        if self.disabled || ctx.phase != EventPhase::Target {
            return;
        }
        if let Event::Pointer(PointerEvent::Up {
            button: PointerButton::Left,
            ..
        }) = event
        {
            if ctx.state.pressed && ctx.hover_self && self.select() {
                ctx.handled();
            }
            return;
        }
        if let Event::Key(key) = event
            && ctx.state.focused
            && (key.key == Key::ENTER || key.key == Key::SPACE)
        {
            self.select();
            ctx.handled();
        }
    }

    fn paint_self(&self, ctx: &mut PaintCtx) {
        let theme = ctx.theme();
        let style = self.style.unwrap_or_default();
        let selected = self.is_selected();
        let diameter = self.resolved_diameter(style);
        let radius = diameter * 0.5;
        let border_w = self.resolved_border_width(radius);
        let center = Point::new(
            ctx.rect.left() + radius,
            ctx.rect.top() + ctx.rect.size.height * 0.5,
        );

        let background = self
            .background
            .or(style.background)
            .unwrap_or(theme.surface);
        let checked_background = self
            .checked_background
            .or(style.checked)
            .unwrap_or(theme.surface);
        let border = if self.disabled {
            theme.disabled
        } else if selected {
            self.checked_border
                .or(style.checked_border)
                .or(self.border)
                .or(style.border)
                .unwrap_or(theme.accent)
        } else {
            self.border.or(style.border).unwrap_or(theme.border)
        };
        let fill = if selected {
            checked_background
        } else {
            background
        };
        let dot = self.dot.or(style.dot).unwrap_or(theme.accent);

        if border_w > 0.0 {
            ctx.disk(center, radius, border);
            ctx.disk(center, (radius - border_w).max(0.0), fill);
        } else {
            ctx.disk(center, radius, fill);
        }
        let inner = (radius - border_w).max(0.0);
        if selected && inner > 0.5 {
            ctx.disk(center, inner * DOT_RATIO, dot);
        }

        if let Some(label) = &self.label {
            let font_size = self.resolved_font_size(style);
            let color = if self.disabled {
                theme.disabled
            } else {
                self.text_color.or(style.text).unwrap_or(theme.text)
            };
            let x = ctx.rect.left() + diameter + GAP;
            let y = ctx.rect.top() + ctx.rect.size.height * 0.5 + font_size * 0.35;
            ctx.draw_text(label, (x, y), font_size, color);
        }
    }
}

pub struct RadioGroup {
    selected: Signal<usize>,
    options: Vec<String>,
    label: String,
    style: Option<RadioStyle>,
    gap: f32,
    horizontal: bool,
    disabled: bool,
    on_change: Option<Rc<dyn Fn(usize)>>,
    margin: Edges,
}

impl RadioGroup {
    pub fn new(
        selected: Signal<usize>,
        options: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            selected,
            options: options.into_iter().map(Into::into).collect(),
            label: String::new(),
            style: None,
            gap: GAP,
            horizontal: false,
            disabled: false,
            on_change: None,
            margin: Edges::ZERO,
        }
    }

    pub fn option(mut self, label: impl Into<String>) -> Self {
        self.options.push(label.into());
        self
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    pub fn style(mut self, style: RadioStyle) -> Self {
        self.style = Some(style);
        self
    }

    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = if gap.is_finite() { gap.max(0.0) } else { 0.0 };
        self
    }

    pub fn horizontal(mut self) -> Self {
        self.horizontal = true;
        self
    }

    pub fn vertical(mut self) -> Self {
        self.horizontal = false;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_change(mut self, f: impl Fn(usize) + 'static) -> Self {
        self.on_change = Some(Rc::new(f));
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

    pub fn selected_signal(&self) -> Signal<usize> {
        self.selected.clone()
    }

    pub fn selected_index(&self) -> usize {
        self.selected.get_untracked()
    }

    pub fn options(&self) -> &[String] {
        &self.options
    }

    fn select(&self, index: usize) -> bool {
        if self.disabled || index >= self.options.len() || self.selected.get_untracked() == index {
            return false;
        }
        self.selected.set(index);
        if let Some(f) = &self.on_change {
            f(index);
        }
        true
    }

    fn radio(&self, index: usize, label: &str) -> RadioButton {
        let mut radio = RadioButton::new(self.selected.clone(), index).label(label.to_string());
        if let Some(style) = self.style {
            radio = radio.style(style);
        }
        if self.disabled {
            radio = radio.disabled(true);
        }
        if let Some(f) = &self.on_change {
            let f = f.clone();
            radio = radio.on_select(move |index| f(index));
        }
        radio
    }
}

impl View for RadioGroup {
    fn accessibility(&self) -> Option<Accessible> {
        Some(Accessible::new(
            AccessibleRole::RadioGroup,
            if self.label.is_empty() {
                "grupa wyboru".to_string()
            } else {
                self.label.clone()
            },
        ))
    }

    fn body(&self) -> impl View + 'static {
        let inner = if self.horizontal {
            let mut row = Row::new().gap(self.gap).align_items(Align::Center);
            for (index, option) in self.options.iter().enumerate() {
                row = row.child(self.radio(index, option));
            }
            AnyView::new(row)
        } else {
            let mut column = Column::new().gap(self.gap);
            for (index, option) in self.options.iter().enumerate() {
                column = column.child(self.radio(index, option));
            }
            AnyView::new(column)
        };

        if self.label.is_empty() {
            return inner;
        }

        let style = self.style.unwrap_or_default();
        let mut label = Text::new(self.label.clone())
            .size(style.font_size.unwrap_or(DEFAULT_FONT_SIZE))
            .bold();
        if let Some(color) = style.text {
            label = label.color(color);
        }
        AnyView::new(Column::new().gap(6.0).child(label).child(inner))
    }

    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fit,
            height: Length::Fit,
            margin: self.margin,
            ..LayoutStyle::default()
        }
    }

    fn on_event(&self, event: &Event, ctx: &mut EventCtx) {
        if self.disabled || self.options.is_empty() || ctx.phase != EventPhase::Capture {
            return;
        }
        let Event::Key(key) = event else { return };
        let len = self.options.len();
        let selected = self.selected.get_untracked().min(len - 1);
        let next = match key.key {
            Key::ARROW_DOWN | Key::ARROW_RIGHT => (selected + 1) % len,
            Key::ARROW_UP | Key::ARROW_LEFT => (selected + len - 1) % len,
            Key::HOME => 0,
            Key::END => len - 1,
            _ => return,
        };
        self.select(next);
        ctx.handled();
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use super::*;

    #[test]
    fn radio_select_updates_signal_and_fires_once() {
        let selected = Signal::new(1usize);
        let calls = Rc::new(Cell::new(0));
        let counter = calls.clone();
        let radio = RadioButton::new(selected.clone(), 0).on_select(move |index| {
            assert_eq!(index, 0);
            counter.set(counter.get() + 1);
        });

        assert!(radio.select());
        assert_eq!(selected.get_untracked(), 0);
        assert_eq!(calls.get(), 1);
        assert!(!radio.select());
        assert_eq!(calls.get(), 1);
    }

    #[test]
    fn disabled_radio_ignores_select() {
        let selected = Signal::new(0usize);
        let radio = RadioButton::new(selected.clone(), 1).disabled(true);

        assert!(!radio.select());
        assert_eq!(selected.get_untracked(), 0);
    }

    #[test]
    fn radio_reports_fit_layout_and_value() {
        let radio = RadioButton::new(Signal::new(0usize), 2).margin_xy(3.0, 4.0);
        let style = radio.layout_style();
        assert_eq!(style.width, Length::Fit);
        assert_eq!(style.height, Length::Fit);
        assert_eq!(style.margin, Edges::xy(3.0, 4.0));
        assert_eq!(radio.value(), 2);
        assert!(!radio.is_selected());
    }

    #[test]
    fn group_collects_options_and_selects() {
        let selected = Signal::new(0usize);
        let calls = Rc::new(RefCell::new(Vec::new()));
        let log = calls.clone();
        let group = RadioGroup::new(selected.clone(), ["A", "B"])
            .option("C")
            .on_change(move |index| log.borrow_mut().push(index));

        assert_eq!(group.options(), ["A", "B", "C"]);
        assert!(group.select(2));
        assert_eq!(selected.get_untracked(), 2);
        assert_eq!(*calls.borrow(), vec![2]);
        assert!(!group.select(2));
        assert!(!group.select(3));
    }

    #[test]
    fn disabled_group_ignores_select() {
        let selected = Signal::new(0usize);
        let group = RadioGroup::new(selected.clone(), ["A", "B"]).disabled(true);

        assert!(!group.select(1));
        assert_eq!(selected.get_untracked(), 0);
    }

    #[test]
    fn group_direction_and_gap_are_sanitized() {
        let group = RadioGroup::new(Signal::new(0usize), ["A"])
            .horizontal()
            .gap(f32::NAN);
        assert!(group.horizontal);
        assert_eq!(group.gap, 0.0);
        assert!(!group.vertical().horizontal);
    }
}
