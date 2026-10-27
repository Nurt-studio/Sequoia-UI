use std::cell::Cell;
use std::rc::Rc;

use sequoia_core::{Color, Edges, Length};
use sequoia_input::{CursorIcon, Event, EventPhase, Key, PointerButton, PointerEvent};
use sequoia_layout::{
    Align, Axis, ContainerStyle, Inset, Justify, LayoutStyle, Overflow, Position,
};
use sequoia_reactive::{Environment, Signal};
use sequoia_style::{BoxStyle, Theme, VisualState};
use sequoia_view::{AnyView, EventCtx, PaintCtx, Prop, Unit, View};

type EnvAction = Rc<dyn Fn(&Environment)>;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum ClickTrigger {
    #[default]
    Release,

    Press,
}

#[derive(Clone)]
pub struct Container {
    child: Option<AnyView>,
    deco: BoxStyle,
    style: LayoutStyle,
    layout: ContainerStyle,
    clip: bool,
    on_click: Option<Rc<dyn Fn()>>,
    on_click_env: Option<EnvAction>,
    trigger: ClickTrigger,
    disabled: bool,
    hover: Option<Color>,
    pressed: Option<Color>,
    disabled_background: Option<Color>,

    blend_rate: f32,
    hovered: Option<Signal<bool>>,
    pressed_flag: Rc<Cell<bool>>,
}

impl Default for Container {
    fn default() -> Self {
        Self {
            child: None,
            deco: BoxStyle::new(),
            clip: false,
            style: LayoutStyle {
                width: Length::Fit,
                height: Length::Fit,
                padding: Edges::ZERO,
                ..LayoutStyle::default()
            },
            layout: ContainerStyle::default(),
            on_click: None,
            on_click_env: None,
            trigger: ClickTrigger::default(),
            disabled: false,
            hover: None,
            pressed: None,
            disabled_background: None,
            blend_rate: 0.0,
            hovered: None,
            pressed_flag: Rc::new(Cell::new(false)),
        }
    }
}

impl Container {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn fill() -> Self {
        Self {
            child: None,
            deco: BoxStyle::new(),
            clip: false,
            style: LayoutStyle {
                width: Length::Fill(1.0),
                height: Length::Fill(1.0),
                padding: Edges::ZERO,
                ..LayoutStyle::default()
            },
            layout: ContainerStyle::default(),
            on_click: None,
            on_click_env: None,
            trigger: ClickTrigger::default(),
            disabled: false,
            hover: None,
            pressed: None,
            disabled_background: None,
            blend_rate: 0.0,
            hovered: None,
            pressed_flag: Rc::new(Cell::new(false)),
        }
    }

    pub fn hug() -> Self {
        Self::new()
    }

    pub fn child<V: View + 'static>(mut self, child: V) -> Self {
        debug_assert!(
            self.child.is_none(),
            "Container takes a single child - a second `.child()` would overwrite the first. Use Row/Column for many."
        );
        self.child = Some(AnyView::new(child));
        self
    }

    pub fn content<V: View + 'static>(self, child: V) -> Self {
        self.child(child)
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

    pub fn background(mut self, color: Color) -> Self {
        self.deco = self.deco.background(color);
        self
    }

    pub fn border(mut self, width: f32, color: Color) -> Self {
        self.deco = self.deco.border(width, color);
        self
    }

    pub fn border_left(mut self, width: f32, color: Color) -> Self {
        self.deco = self.deco.border_left(width, color);
        self
    }

    pub fn border_right(mut self, width: f32, color: Color) -> Self {
        self.deco = self.deco.border_right(width, color);
        self
    }

    pub fn border_top(mut self, width: f32, color: Color) -> Self {
        self.deco = self.deco.border_top(width, color);
        self
    }

    pub fn border_bottom(mut self, width: f32, color: Color) -> Self {
        self.deco = self.deco.border_bottom(width, color);
        self
    }

    pub fn border_x(mut self, width: f32, color: Color) -> Self {
        self.deco = self.deco.border_x(width, color);
        self
    }

    pub fn border_y(mut self, width: f32, color: Color) -> Self {
        self.deco = self.deco.border_y(width, color);
        self
    }

    pub fn box_style(mut self, style: BoxStyle) -> Self {
        self.deco = style;
        self
    }

    pub fn style(self, style: BoxStyle) -> Self {
        self.box_style(style)
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.deco = self.deco.radius(radius);
        self
    }

    pub fn padding(mut self, padding: impl Into<Edges>) -> Self {
        self.style = self.style.padding(padding);
        self
    }

    pub fn pad(self, padding: impl Into<Edges>) -> Self {
        self.padding(padding)
    }

    pub fn padding_xy(mut self, x: f32, y: f32) -> Self {
        self.style = self.style.padding_xy(x, y);
        self
    }

    pub fn padding_x(mut self, x: f32) -> Self {
        self.style = self.style.padding_x(x);
        self
    }

    pub fn padding_y(mut self, y: f32) -> Self {
        self.style = self.style.padding_y(y);
        self
    }

    pub fn padding_left(mut self, v: f32) -> Self {
        self.style = self.style.padding_left(v);
        self
    }

    pub fn padding_right(mut self, v: f32) -> Self {
        self.style = self.style.padding_right(v);
        self
    }

    pub fn padding_top(mut self, v: f32) -> Self {
        self.style = self.style.padding_top(v);
        self
    }

    pub fn padding_bottom(mut self, v: f32) -> Self {
        self.style = self.style.padding_bottom(v);
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.style = self.style.width(width.into());
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.style = self.style.height(height.into());
        self
    }

    pub fn size(self, side: impl Into<Length>) -> Self {
        self.square(side)
    }

    pub fn square(mut self, side: impl Into<Length>) -> Self {
        let side = side.into();
        self.style = self.style.width(side).height(side);
        self
    }

    pub fn min_width(mut self, v: f32) -> Self {
        self.style = self.style.min_width(v);
        self
    }

    pub fn max_width(mut self, v: f32) -> Self {
        self.style = self.style.max_width(v);
        self
    }

    pub fn min_height(mut self, v: f32) -> Self {
        self.style = self.style.min_height(v);
        self
    }

    pub fn max_height(mut self, v: f32) -> Self {
        self.style = self.style.max_height(v);
        self
    }

    pub fn overflow(mut self, overflow: Overflow) -> Self {
        self.style = self.style.overflow(overflow);
        self
    }

    pub fn margin(mut self, margin: impl Into<Edges>) -> Self {
        self.style = self.style.margin(margin);
        self
    }

    pub fn margin_xy(mut self, x: f32, y: f32) -> Self {
        self.style = self.style.margin_xy(x, y);
        self
    }

    pub fn margin_x(mut self, x: f32) -> Self {
        self.style = self.style.margin_x(x);
        self
    }

    pub fn margin_y(mut self, y: f32) -> Self {
        self.style = self.style.margin_y(y);
        self
    }

    pub fn margin_left(mut self, v: f32) -> Self {
        self.style = self.style.margin_left(v);
        self
    }

    pub fn margin_right(mut self, v: f32) -> Self {
        self.style = self.style.margin_right(v);
        self
    }

    pub fn margin_top(mut self, v: f32) -> Self {
        self.style = self.style.margin_top(v);
        self
    }

    pub fn margin_bottom(mut self, v: f32) -> Self {
        self.style = self.style.margin_bottom(v);
        self
    }

    pub fn flex(mut self, weight: f32) -> Self {
        self.style = self.style.flex(weight);
        self
    }

    pub fn flex_shrink(mut self, shrink: f32) -> Self {
        self.style = self.style.flex_shrink(shrink);
        self
    }

    pub fn flex_basis(mut self, basis: Option<Length>) -> Self {
        self.style = self.style.flex_basis(basis);
        self
    }

    pub fn align_self(mut self, align: Option<Align>) -> Self {
        self.style = self.style.align_self(align);
        self
    }

    pub fn aspect(mut self, ratio: f32) -> Self {
        self.style = self.style.aspect(ratio);
        self
    }

    pub fn position(mut self, position: Position) -> Self {
        self.style = self.style.position(position);
        self
    }

    pub fn absolute(mut self) -> Self {
        self.style = self.style.absolute();
        self
    }

    pub fn absolute_at(mut self, left: f32, top: f32) -> Self {
        self.style = self.style.absolute_at(left, top);
        self
    }

    pub fn absolute_inset(mut self, inset: Inset) -> Self {
        self.style = self.style.absolute_inset(inset);
        self
    }

    pub fn expand(self) -> Self {
        self.flex(1.0)
    }

    pub fn fill_width(mut self) -> Self {
        self.style = self.style.width(Length::Fill(1.0));
        self
    }

    pub fn fill_height(mut self) -> Self {
        self.style = self.style.height(Length::Fill(1.0));
        self
    }

    pub fn fill_size(self) -> Self {
        self.fill_width().fill_height()
    }

    pub fn fit(mut self) -> Self {
        self.style = self.style.fit();
        self
    }

    pub fn container_style(mut self, layout: ContainerStyle) -> Self {
        self.layout = layout;
        self
    }

    pub fn axis(mut self, axis: Axis) -> Self {
        self.layout.axis = axis;
        self
    }

    pub fn justify_content(mut self, justify: Justify) -> Self {
        self.layout.justify = justify;
        self
    }

    pub fn justify(self, justify: Justify) -> Self {
        self.justify_content(justify)
    }

    pub fn align_items(mut self, align: Align) -> Self {
        self.layout.align = align;
        self
    }

    pub fn align(self, align: Align) -> Self {
        self.align_items(align)
    }

    pub fn center(mut self) -> Self {
        self.layout.align = Align::Center;
        self.layout.justify = Justify::Center;
        self
    }

    pub fn hit_testable(mut self, on: bool) -> Self {
        self.style = self.style.hit_testable(on);
        self
    }

    pub fn clip(mut self, clip: bool) -> Self {
        self.clip = clip;
        self
    }

    pub fn clip_children(self) -> Self {
        self.clip(true)
    }

    fn is_interactive(&self) -> bool {
        self.on_click.is_some() || self.on_click_env.is_some()
    }

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

    pub fn hovered(mut self, signal: Signal<bool>) -> Self {
        self.hovered = Some(signal);
        self
    }

    pub fn on_hover(mut self, hovered: Signal<bool>) -> Self {
        self.hovered = Some(hovered);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
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

    pub fn blend_rate(mut self, rate: f32) -> Self {
        self.blend_rate = rate;
        self
    }

    #[cfg(test)]
    pub(crate) fn effective_blend_rate(&self) -> f32 {
        self.blend_rate
    }

    pub fn target_background(&self, theme: &Theme, state: VisualState) -> Color {
        let mut state = state;
        if self.disabled {
            state.disabled = true;
        }
        let idle = self.deco.background;
        if state.disabled {
            self.disabled_background.unwrap_or(theme.disabled)
        } else if state.pressed {
            self.pressed.unwrap_or(idle)
        } else if state.hovered {
            self.hover.unwrap_or(idle)
        } else {
            idle
        }
    }

    fn fire(&self, env: &Environment) {
        if let Some(f) = &self.on_click {
            f();
        }
        if let Some(f) = &self.on_click_env {
            f(env);
        }
    }
}

impl View for Container {
    fn body(&self) -> impl View + 'static {
        Unit
    }
    fn clip_self(&self) -> bool {
        self.clip
    }
    fn clip_radius(&self) -> f32 {
        if self.clip { self.deco.radius } else { 0.0 }
    }
    fn children_override(&self) -> Option<Vec<AnyView>> {
        self.child.clone().map(|c| vec![c])
    }
    fn layout_style(&self) -> LayoutStyle {
        let mut style = self.style;
        let edges = self.deco.border_edges();
        style.padding.left += edges.left;
        style.padding.top += edges.top;
        style.padding.right += edges.right;
        style.padding.bottom += edges.bottom;
        style
    }
    fn container(&self) -> Option<ContainerStyle> {
        (self.layout != ContainerStyle::default()).then_some(self.layout)
    }
    fn paint_self(&self, ctx: &mut PaintCtx) {
        if let Some(sig) = &self.hovered {
            let hovered = ctx.hovered_subtree;
            if sig.get_untracked() != hovered {
                sig.set(hovered);
            }
        }
        if !self.is_interactive() {
            ctx.paint_box(self.deco);
            return;
        }
        let target = self.target_background(&ctx.theme(), ctx.state);
        let bg = if self.blend_rate <= 0.0 {
            target
        } else {
            ctx.live_color_exp(Prop::BACKGROUND, target, self.blend_rate)
        };
        let mut deco = self.deco;
        deco.background = bg;
        ctx.paint_box(deco);
    }

    fn has_visual_state(&self) -> bool {
        self.is_interactive()
    }

    fn focusable(&self) -> bool {
        self.is_interactive() && !self.disabled
    }

    fn cursor_icon(&self, _state: VisualState) -> CursorIcon {
        if self.is_interactive() && !self.disabled {
            CursorIcon::Pointer
        } else {
            CursorIcon::Default
        }
    }

    fn on_event(&self, event: &Event, ctx: &mut EventCtx) {
        if self.disabled || !self.is_interactive() {
            return;
        }
        match event {
            Event::Pointer(pe) => match pe {
                PointerEvent::Down {
                    button: PointerButton::Left,
                    pos,
                    ..
                } => {
                    let inside = ctx.rect.contains(*pos);
                    self.pressed_flag.set(inside);
                    if self.trigger == ClickTrigger::Press
                        && ctx.phase != EventPhase::Capture
                        && inside
                    {
                        self.pressed_flag.set(false);
                        self.fire(ctx.environment);
                        ctx.handled();
                    }
                }
                PointerEvent::Up {
                    button: PointerButton::Left,
                    pos,
                    ..
                } if ctx.phase != EventPhase::Capture
                    && self.pressed_flag.take()
                    && ctx.rect.contains(*pos) =>
                {
                    self.fire(ctx.environment);
                    ctx.handled();
                }
                _ => {}
            },

            Event::Key(key)
                if ctx.phase == EventPhase::Target
                    && ctx.state.focused
                    && (key.key == Key::ENTER || key.key == Key::SPACE) =>
            {
                self.fire(ctx.environment);
                ctx.handled();
            }
            _ => {}
        }
    }
}
