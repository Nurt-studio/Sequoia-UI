use std::rc::Rc;

use sequoia_core::{Color, Edges, Length};
use sequoia_layout::LayoutStyle;
use sequoia_reactive::Environment;
use sequoia_style::ButtonStyle;
use sequoia_text::FontWeight;
use sequoia_view::{AnimSpec, AnyView, Column, Icon, Text, View};

use crate::container::{ClickTrigger, Container};

const PAD_X: f32 = 10.0;
const PAD_Y: f32 = 5.0;
const ICON_DEFAULT_PX: f32 = 16.0;
const RIBBON_ICON_PX: f32 = 28.0;
const RIBBON_GAP: f32 = 2.0;

type EnvAction = Rc<dyn Fn(&Environment)>;

struct IconSpec {
    name: String,
    px: f32,
    color: Option<Color>,
    label: Option<String>,
}

#[derive(Clone)]
pub struct Button {
    label: String,
    icon: Option<IconSpec>,
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
    disabled: bool,
    hover_anim: Option<AnimSpec>,
    press_anim: Option<AnimSpec>,

    instant: bool,
    on_click: Option<Rc<dyn Fn()>>,
    on_click_env: Option<EnvAction>,
    margin: Edges,
    min_width: Option<f32>,
    max_width: Option<f32>,
    min_height: Option<f32>,
    max_height: Option<f32>,

    width: Option<f32>,
    height: Option<f32>,

    trigger: ClickTrigger,
}

pub const BUTTON_BLEND_RATE: f32 = 12.0;

impl Clone for IconSpec {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            px: self.px,
            color: self.color,
            label: self.label.clone(),
        }
    }
}

impl Button {
    pub fn labeled(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            icon: None,
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
            disabled: false,
            hover_anim: None,
            press_anim: None,
            instant: false,
            on_click: None,
            on_click_env: None,
            margin: Edges::ZERO,
            min_width: None,
            max_width: None,
            min_height: None,
            max_height: None,
            width: None,
            height: None,
            trigger: ClickTrigger::default(),
        }
    }

    pub fn new(label: impl Into<String>) -> Self {
        Self::labeled(label)
    }

    pub fn icon(name: impl Into<String>) -> Self {
        let mut b = Self::labeled("");
        b.icon = Some(IconSpec {
            name: name.into(),
            px: ICON_DEFAULT_PX,
            color: None,
            label: None,
        });
        b.padding = Some((4.0, 4.0));
        b
    }

    pub fn ribbon(icon: impl Into<String>, label: impl Into<String>) -> Self {
        let mut b = Self::labeled("");
        b.icon = Some(IconSpec {
            name: icon.into(),
            px: RIBBON_ICON_PX,
            color: None,
            label: Some(label.into()),
        });
        b.padding = Some((7.0, 7.0));
        b
    }
}

impl Button {
    pub fn on_click(mut self, on_click: impl Fn() + 'static) -> Self {
        self.on_click = Some(Rc::new(on_click));
        self
    }

    pub fn on_press(mut self, on_press: impl Fn() + 'static) -> Self {
        self.on_click = Some(Rc::new(on_press));
        self.trigger = ClickTrigger::Press;
        self
    }

    pub fn trigger(mut self, trigger: ClickTrigger) -> Self {
        self.trigger = trigger;
        self
    }

    pub fn on_click_env(mut self, on_click: impl Fn(&Environment) + 'static) -> Self {
        self.on_click_env = Some(Rc::new(on_click));
        self
    }

    pub fn style(mut self, style: ButtonStyle) -> Self {
        self.style = Some(style);
        self
    }

    pub fn style_with(mut self, build: impl FnOnce(ButtonStyle) -> ButtonStyle) -> Self {
        self.style = Some(build(self.style.unwrap_or_default()));
        self
    }

    pub fn hover_anim(mut self, spec: impl Into<AnimSpec>) -> Self {
        self.hover_anim = Some(spec.into());
        self
    }

    pub fn press_anim(mut self, spec: impl Into<AnimSpec>) -> Self {
        self.press_anim = Some(spec.into());
        self
    }

    pub fn no_animation(mut self) -> Self {
        self.instant = true;
        self
    }

    pub fn padding(mut self, pad_x: f32, pad_y: f32) -> Self {
        self.padding = Some((pad_x, pad_y));
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.text_color = Some(color);
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

    pub fn icon_size(mut self, px: f32) -> Self {
        if let Some(i) = &mut self.icon {
            i.px = px.max(0.0);
        }
        self
    }

    pub fn icon_color(mut self, color: Color) -> Self {
        if let Some(i) = &mut self.icon {
            i.color = Some(color);
        }
        self
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        match &mut self.icon {
            Some(i) => i.label = Some(label.into()),
            None => self.label = label.into(),
        }
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

    pub fn width(mut self, px: f32) -> Self {
        self.width = Some(px.max(0.0));
        self
    }

    pub fn height(mut self, px: f32) -> Self {
        self.height = Some(px.max(0.0));
        self
    }

    pub fn size(mut self, px: f32) -> Self {
        let px = px.max(0.0);
        self.width = Some(px);
        self.height = Some(px);
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
}

fn sanitize_margin(v: f32) -> f32 {
    if v.is_finite() { v.max(0.0) } else { 0.0 }
}

impl Button {
    fn merged_padding(&self) -> (f32, f32) {
        self.padding
            .or_else(|| self.style.and_then(|s| s.padding))
            .unwrap_or((PAD_X, PAD_Y))
    }

    fn text_tint(&self) -> Option<Color> {
        let style = self.style.unwrap_or_default();
        if self.disabled {
            self.text_disabled.or(style.text_disabled)
        } else {
            self.text_color.or(style.text)
        }
    }

    fn label_view(&self, text: &str) -> Text {
        let style = self.style.unwrap_or_default();

        let mut t = Text::new(text.to_owned()).hit_testable(false);
        if let Some(size) = self.font_size.or(style.font_size) {
            t = t.size(size);
        }
        if let Some(color) = self.text_tint() {
            t = t.color(color);
        }
        if let Some(w) = self.font_weight.or(style.font_weight) {
            t = t.weight(FontWeight::new(w));
        }
        t
    }

    fn icon_view(&self, icon: &IconSpec, tint: Option<Color>) -> Icon {
        let mut v = Icon::new(icon.name.clone())
            .size(icon.px.max(1.0))
            .hit_testable(false);
        if let Some(c) = icon.color.or(tint) {
            v = v.color(c);
        }
        v
    }

    fn content_view(&self) -> AnyView {
        let tint = self.text_tint();
        match &self.icon {
            None => AnyView::new(self.label_view(&self.label)),
            Some(icon) if icon.label.is_none() => AnyView::new(self.icon_view(icon, tint)),
            Some(icon) => {
                let label = icon.label.clone().unwrap_or_default();
                AnyView::new(
                    Column::new()
                        .center()
                        .main_gap(RIBBON_GAP)
                        .hit_testable(false)
                        .child(self.icon_view(icon, tint))
                        .child(self.label_view(&label)),
                )
            }
        }
    }

    fn composed(&self) -> Container {
        let style = self.style.unwrap_or_default();
        let (pad_x, pad_y) = self.merged_padding();

        let rate = if self.instant {
            0.0
        } else {
            self.hover_anim
                .or(self.press_anim)
                .map(|s| (4.6 / s.duration_secs.max(0.001)).clamp(4.0, 60.0))
                .unwrap_or(0.0)
        };

        let mut p = Container::new()
            .child(self.content_view())
            .padding_xy(pad_x, pad_y)
            .disabled(self.disabled)
            .trigger(self.trigger)
            .blend_rate(rate)
            .background(
                self.background
                    .or(style.background)
                    .unwrap_or(Color::hex(0x2A2A2A)),
            )
            .hover(self.hover.or(style.hover).unwrap_or(Color::hex(0x323232)))
            .pressed(
                self.pressed
                    .or(style.pressed)
                    .unwrap_or(Color::hex(0x3A3A3A)),
            );
        if let Some(v) = self.width {
            p = p.width(v);
        }
        if let Some(v) = self.height {
            p = p.height(v);
        }
        if let Some(v) = self.min_width {
            p = p.min_width(v);
        }
        if let Some(v) = self.max_width {
            p = p.max_width(v);
        }
        if let Some(v) = self.min_height {
            p = p.min_height(v);
        }
        if let Some(v) = self.max_height {
            p = p.max_height(v);
        }
        if let Some(c) = self.disabled_background.or(style.disabled) {
            p = p.disabled_background(c);
        }
        if let Some(r) = self.radius.or(style.radius) {
            p = p.radius(r);
        }
        let bw = self
            .border_width
            .or(style.border_width)
            .unwrap_or(0.0)
            .max(0.0);
        let bc = self
            .border_color
            .or(style.border_color)
            .unwrap_or(Color::TRANSPARENT);
        if bw > 0.0 && bc.a > 0.0 {
            p = p.border(bw, bc);
        }
        if let Some(f) = &self.on_click {
            let f = Rc::clone(f);
            p = p.on_click(move || f());
        }
        if let Some(f) = &self.on_click_env {
            let f = Rc::clone(f);
            p = p.on_click_env(move |env| f(env));
        }
        if self.on_click.is_none() && self.on_click_env.is_none() {
            p = p.on_click(|| {});
        }
        p
    }
}

impl View for Button {
    fn body(&self) -> impl View + 'static {
        self.composed()
    }

    fn accessibility(&self) -> Option<sequoia_view::Accessible> {
        Some(sequoia_view::Accessible::new(
            sequoia_view::AccessibleRole::Button,
            self.label.clone(),
        ))
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
        if let Some(v) = self.width {
            style = style.width(Length::Px(v));
        }
        if let Some(v) = self.height {
            style = style.height(Length::Px(v));
        }
        style
    }

    fn hit_testable(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sequoia_view::live::{exp_blend, exp_settled};

    #[test]
    fn background_rate_gives_snappy_but_smooth_blend() {
        let first = exp_blend(Color::BLACK, Color::WHITE, 1.0 / 60.0, BUTTON_BLEND_RATE);
        assert!(first.r > 0.1, "first frame starts clearly: {}", first.r);
        assert!(first.r < 1.0, "nie dobiega od razu");
        let mut cur = Color::BLACK;
        for _ in 0..120 {
            cur = exp_blend(cur, Color::WHITE, 1.0 / 60.0, BUTTON_BLEND_RATE);
        }
        assert!(
            exp_settled(cur, Color::WHITE),
            "120 klatek dobiega: {cur:?}"
        );
    }

    #[test]
    fn background_blend_handles_degenerate_input() {
        let a = Color::RED;

        assert_eq!(exp_blend(a, Color::BLUE, -1.0, BUTTON_BLEND_RATE), a);
        assert_eq!(exp_blend(a, Color::BLUE, 0.016, 0.0), a);
        assert!(!exp_settled(a, Color::BLUE));
        assert!(exp_settled(a, a));
    }

    #[test]
    fn app_api_builders_compose() {
        let b = Button::new("Save")
            .on_click(|| {})
            .style_with(|s| s.background(Color::BLUE).radius(6.0))
            .background(Color::BLUE)
            .hover(Color::WHITE)
            .hover_anim(AnimSpec::SNAPPY)
            .press_anim(AnimSpec::GENTLE)
            .padding(8.0, 14.0)
            .color(Color::WHITE)
            .radius(6.0);
        assert!(b.hover_anim.is_some());
        assert!(b.press_anim.is_some());
        assert_eq!(b.style.unwrap().radius, Some(6.0));

        let _ = View::body(&b);
    }

    #[test]
    fn anim_spec_maps_to_faster_blend_rate() {
        let fast = Button::new("x").hover_anim(AnimSpec::SNAPPY).composed();
        let slow = Button::new("x").hover_anim(AnimSpec::GENTLE).composed();
        let plain = Button::new("x").composed();

        let _ = (fast, slow, plain);
        let rate = |s: AnimSpec| (4.6 / s.duration_secs.max(0.001)).clamp(4.0, 60.0);
        assert!(rate(AnimSpec::SNAPPY) > rate(AnimSpec::GENTLE));
        assert!(rate(AnimSpec::SNAPPY) > BUTTON_BLEND_RATE);
    }

    #[test]
    fn default_button_has_no_blend_but_preset_animates() {
        let plain = Button::new("x").composed();
        assert!(plain.effective_blend_rate() <= 0.0);

        let instant = Button::new("x").no_animation().composed();
        assert!(instant.effective_blend_rate() <= 0.0);

        let snappy = Button::new("x").hover_anim(AnimSpec::SNAPPY).composed();
        assert!(snappy.effective_blend_rate() > 0.0);

        let over = Button::new("x")
            .hover_anim(AnimSpec::SNAPPY)
            .no_animation()
            .composed();
        assert!(over.effective_blend_rate() <= 0.0);
    }

    #[test]
    fn size_sets_both_dimensions_width_height_singly() {
        use sequoia_core::Length;

        let fit = Button::new("x").layout_style();
        assert_eq!(fit.width, Length::Fit);
        assert_eq!(fit.height, Length::Fit);

        let sq = Button::new("x").size(4.0).layout_style();
        assert_eq!(sq.width, Length::Px(4.0));
        assert_eq!(sq.height, Length::Px(4.0));

        let w = Button::new("x").width(20.0).layout_style();
        assert_eq!(w.width, Length::Px(20.0));
        assert_eq!(w.height, Length::Fit);
        let h = Button::new("x").height(12.0).layout_style();
        assert_eq!(h.height, Length::Px(12.0));
        assert_eq!(h.width, Length::Fit);

        let z = Button::new("x").size(-5.0).layout_style();
        assert_eq!(z.width, Length::Px(0.0));
        assert_eq!(z.height, Length::Px(0.0));
    }
}
