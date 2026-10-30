use std::rc::Rc;

use sequoia_core::{Color, Edges, Length};
use sequoia_layout::{Align, LayoutStyle};
use sequoia_reactive::Signal;
use sequoia_style::CheckboxStyle;
use sequoia_view::{Icon, Row, Text, Unit, View};

use crate::container::Container;
use crate::primitives::Deco;

const BOX: f32 = 16.0;
const GAP: f32 = 8.0;
const BORDER_W: f32 = 1.5;

pub struct Checkbox {
    checked: Signal<bool>,
    label: Option<String>,
    style: Option<CheckboxStyle>,
    background: Option<Color>,
    checked_background: Option<Color>,
    border: Option<Color>,
    checked_border: Option<Color>,
    check: Option<Color>,
    text_color: Option<Color>,
    font_size: Option<f32>,
    radius: Option<f32>,
    box_size: Option<f32>,
    disabled: bool,
    on_toggle: Option<Rc<dyn Fn(bool)>>,
    margin: Edges,
}

impl Checkbox {
    pub fn new(checked: Signal<bool>) -> Self {
        Self {
            checked,
            label: None,
            style: None,
            background: None,
            checked_background: None,
            border: None,
            checked_border: None,
            check: None,
            text_color: None,
            font_size: None,
            radius: None,
            box_size: None,
            disabled: false,
            on_toggle: None,
            margin: Edges::ZERO,
        }
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn style(mut self, style: CheckboxStyle) -> Self {
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

    pub fn check_color(mut self, color: Color) -> Self {
        self.check = Some(color);
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

    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = Some(radius);
        self
    }

    pub fn box_size(mut self, size: f32) -> Self {
        self.box_size = Some(size);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_toggle(mut self, f: impl Fn(bool) + 'static) -> Self {
        self.on_toggle = Some(Rc::new(f));
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

    fn box_px(&self) -> f32 {
        let style = self.style.unwrap_or_default();
        self.box_size.or(style.box_size).unwrap_or(BOX).max(4.0)
    }
}

fn sanitize_margin(v: f32) -> f32 {
    if v.is_finite() { v.max(0.0) } else { 0.0 }
}

impl View for Checkbox {
    fn accessibility(&self) -> Option<sequoia_view::Accessible> {
        Some(sequoia_view::Accessible::new(
            sequoia_view::AccessibleRole::Checkbox,
            self.label
                .clone()
                .unwrap_or_else(|| "pole wyboru".to_string()),
        ))
    }

    fn body(&self) -> impl View + 'static {
        let selected = self.checked.get();
        let style = self.style.unwrap_or_default();
        let box_px = self.box_px();
        let radius = self.radius.or(style.radius).unwrap_or(3.0);

        let bg = if selected {
            self.checked_background.or(style.checked)
        } else {
            self.background.or(style.background)
        };
        let border = if selected {
            self.checked_border.or(style.checked_border)
        } else {
            self.border.or(style.border)
        };
        let check = self.check.or(style.check);

        let check_view: sequoia_view::AnyView = if selected {
            let icon_px = (box_px - 4.0).max(1.0);
            let inset = ((box_px - icon_px) * 0.5).max(0.0);
            let mut icon = Icon::new("check")
                .size(icon_px)
                .margin_xy(inset, inset)
                .hit_testable(false);

            icon = icon.color(check.unwrap_or(Color::hex(0x191919)));
            sequoia_view::AnyView::new(icon)
        } else {
            sequoia_view::AnyView::new(Unit)
        };
        let checkbox_box = Deco::new(check_view, move |theme| {
            let bg = bg.unwrap_or(if selected { theme.text } else { theme.surface });
            let border = border.unwrap_or(if selected { theme.text } else { theme.border });
            sequoia_style::BoxStyle::new()
                .background(bg)
                .border(BORDER_W, border)
                .radius(radius)
        })
        .square_px(box_px)
        .hit_testable(false);

        let mut row = Row::new()
            .main_gap(GAP)
            .align_items(Align::Center)
            .hit_testable(false)
            .child(checkbox_box);
        if let Some(label) = &self.label {
            let mut t = Text::new(label.clone()).hit_testable(false);
            if let Some(size) = self.font_size.or(style.font_size) {
                t = t.size(size);
            }
            if let Some(c) = self.text_color.or(style.text) {
                t = t.color(c);
            }
            row = row.child(t);
        }

        let checked = self.checked.clone();
        let on_toggle = self.on_toggle.clone();
        let disabled = self.disabled;
        Container::new()
            .child(row)
            .disabled(disabled)
            .on_click(move || {
                if disabled {
                    return;
                }
                let next = !checked.get_untracked();
                checked.set(next);
                if let Some(f) = &on_toggle {
                    f(next);
                }
            })
    }

    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fit,
            height: Length::Fit,
            margin: self.margin,
            ..LayoutStyle::default()
        }
    }

    fn hit_testable(&self) -> bool {
        false
    }
}
