use std::rc::Rc;

use sequoia_core::Color;
use sequoia_view::{Row, View};

use crate::Button;

const BUTTON_SIZE: f32 = 46.0;
const ICON_SIZE: f32 = 10.0;

#[derive(Clone)]
pub struct WindowControls {
    button_size: f32,
    icon_size: f32,
    gap: f32,
    minimize: bool,
    maximize: bool,
    close: bool,
    close_hover: Option<Color>,
    on_minimize: Option<Rc<dyn Fn()>>,
    on_maximize: Option<Rc<dyn Fn()>>,
    on_close: Option<Rc<dyn Fn()>>,
}

impl Default for WindowControls {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowControls {
    pub fn new() -> Self {
        Self {
            button_size: BUTTON_SIZE,
            icon_size: ICON_SIZE,
            gap: 0.0,
            minimize: true,
            maximize: true,
            close: true,
            close_hover: None,
            on_minimize: None,
            on_maximize: None,
            on_close: None,
        }
    }

    pub fn button_size(mut self, px: f32) -> Self {
        self.button_size = px.max(0.0);
        self
    }

    pub fn icon_size(mut self, px: f32) -> Self {
        self.icon_size = px.max(0.0);
        self
    }

    pub fn gap(mut self, px: f32) -> Self {
        self.gap = px.max(0.0);
        self
    }

    pub fn minimize(mut self, show: bool) -> Self {
        self.minimize = show;
        self
    }

    pub fn maximize(mut self, show: bool) -> Self {
        self.maximize = show;
        self
    }

    pub fn close(mut self, show: bool) -> Self {
        self.close = show;
        self
    }

    pub fn close_hover(mut self, color: Color) -> Self {
        self.close_hover = Some(color);
        self
    }

    pub fn on_minimize(mut self, action: impl Fn() + 'static) -> Self {
        self.on_minimize = Some(Rc::new(action));
        self
    }

    pub fn on_maximize(mut self, action: impl Fn() + 'static) -> Self {
        self.on_maximize = Some(Rc::new(action));
        self
    }

    pub fn on_close(mut self, action: impl Fn() + 'static) -> Self {
        self.on_close = Some(Rc::new(action));
        self
    }

    fn control(&self, icon: &str, action: Option<Rc<dyn Fn()>>) -> Button {
        let button = Button::icon(icon)
            .width(self.button_size)
            .height(self.button_size)
            .icon_size(self.icon_size)
            .padding(0.0, 0.0);
        match action {
            Some(action) => button.on_click(move || action()),
            None => button,
        }
    }
}

impl View for WindowControls {
    fn body(&self) -> impl View + 'static {
        let close = self.close_hover.map_or_else(
            || self.control("close", self.on_close.clone()),
            |hover| self.control("close", self.on_close.clone()).hover(hover),
        );

        Row::new()
            .height(self.button_size)
            .gap(self.gap)
            .child_if(self.minimize, || {
                self.control("minimize", self.on_minimize.clone())
            })
            .child_if(self.maximize, || {
                self.control("maximize", self.on_maximize.clone())
            })
            .child_if(self.close, || close)
    }
}
