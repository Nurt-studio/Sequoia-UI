use std::cell::Cell;
use std::rc::Rc;

use sequoia_core::{Color, Edges, Length, Point};
use sequoia_input::{CursorIcon, Event, EventPhase, Key, PointerButton, PointerEvent};
use sequoia_layout::LayoutStyle;
use sequoia_reactive::{Environment, Signal};
use sequoia_style::{ContextMenuStyle, VisualState};
use sequoia_view::{
    Align, Anchor, AnyView, Column, Dismiss, EventCtx, OverlayEntry, OverlayLayer, PaintCtx,
    Placement, Row, Text, Unit, View,
};

use crate::container::Container;
use crate::popup::PopupState;

#[derive(Clone)]
enum MenuAction {
    Plain(Rc<dyn Fn()>),
    Env(Rc<dyn Fn(&Environment)>),
}

#[derive(Clone)]
struct MenuRow {
    label: String,
    action: MenuAction,
}

pub struct ContextMenu {
    child: AnyView,
    items: Vec<MenuRow>,
    popup: PopupState,

    open_at: Rc<Cell<Option<Point>>>,
    style: LayoutStyle,
    appearance: Option<ContextMenuStyle>,
    menu_background: Option<Color>,
    menu_border: Option<Color>,
    menu_radius: Option<f32>,
    menu_text_color: Option<Color>,
    menu_font_size: Option<f32>,
}

impl ContextMenu {
    pub fn new(child: impl View + 'static) -> Self {
        Self {
            child: AnyView::new(child),
            items: Vec::new(),
            popup: PopupState::new(Signal::new(false)),
            open_at: Rc::new(Cell::new(None)),
            style: LayoutStyle::default(),
            appearance: None,
            menu_background: None,
            menu_border: None,
            menu_radius: None,
            menu_text_color: None,
            menu_font_size: None,
        }
    }

    /// Opens the menu at a screen-space point, without waiting for a click.
    ///
    /// Right-clicking the wrapped child also opens the menu; this method is for
    /// keyboard shortcuts, toolbar buttons, and tests.
    pub fn open_at(&self, pos: Point) {
        self.open_at.set(Some(pos));
        self.popup.open();
    }

    /// Closes the menu if it is open.
    pub fn close(&self) {
        self.popup.close();
    }

    /// Whether the menu is currently open.
    pub fn is_open(&self) -> bool {
        self.popup.is_open()
    }

    pub fn item(mut self, label: impl Into<String>, action: impl Fn() + 'static) -> Self {
        self.items.push(MenuRow {
            label: label.into(),
            action: MenuAction::Plain(Rc::new(action)),
        });
        self
    }

    pub fn item_env(
        mut self,
        label: impl Into<String>,
        action: impl Fn(&Environment) + 'static,
    ) -> Self {
        self.items.push(MenuRow {
            label: label.into(),
            action: MenuAction::Env(Rc::new(action)),
        });
        self
    }

    pub fn appearance(mut self, style: ContextMenuStyle) -> Self {
        self.appearance = Some(style);
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

    pub fn menu_radius(mut self, radius: f32) -> Self {
        self.menu_radius = Some(radius);
        self
    }

    pub fn menu_text_color(mut self, color: Color) -> Self {
        self.menu_text_color = Some(color);
        self
    }

    pub fn menu_font_size(mut self, size: f32) -> Self {
        self.menu_font_size = Some(size);
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

    pub fn flex(mut self, weight: f32) -> Self {
        self.style = self.style.flex(weight);
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

    fn menu_view(&self) -> AnyView {
        let popup = self.popup.clone();
        let appearance = self.appearance.unwrap_or_default();
        let bg = self
            .menu_background
            .or(appearance.background)
            .unwrap_or(Color::hex(0x2B2B2B));
        let border = self
            .menu_border
            .or(appearance.border)
            .unwrap_or(Color::hex(0x454545));
        let radius = self.menu_radius.or(appearance.radius).unwrap_or(4.0);
        let font_size = self.menu_font_size.or(appearance.font_size).unwrap_or(12.0);
        let text_color = self.menu_text_color.or(appearance.text);
        let (pad_x, pad_y) = appearance.padding.unwrap_or((6.0, 6.0));

        let mut col = Column::new().fill_width().gap(2.0);
        for row in &self.items {
            let (action, popup) = (row.action.clone(), popup.clone());
            let mut label = Text::new(row.label.clone()).size(font_size);
            if let Some(color) = text_color {
                label = label.color(color);
            }
            let cell = Container::new()
                .fill_width()
                .background(Color::TRANSPARENT)
                .radius(3.0)
                .padding_xy(10.0, 7.0)
                .child(
                    Row::new()
                        .fill_width()
                        .align_items(Align::Center)
                        .child(label),
                );
            let cell = match action {
                MenuAction::Plain(f) => cell.on_click(move || {
                    f();
                    popup.close();
                }),
                MenuAction::Env(f) => cell.on_click_env(move |env| {
                    f(env);
                    popup.close();
                }),
            };
            col = col.child(cell);
        }
        AnyView::new(
            Container::new()
                .width(240.0)
                .background(bg)
                .border(1.0, border)
                .radius(radius)
                .padding_xy(pad_x, pad_y)
                .child(col),
        )
    }
}

impl View for ContextMenu {
    fn body(&self) -> impl View + 'static {
        self.popup.subscribe();
        Unit
    }

    fn children_override(&self) -> Option<Vec<AnyView>> {
        Some(vec![self.child.clone()])
    }

    fn layout_style(&self) -> LayoutStyle {
        self.style
    }

    fn focusable(&self) -> bool {
        true
    }

    fn has_visual_state(&self) -> bool {
        true
    }

    fn cursor_icon(&self, _state: VisualState) -> CursorIcon {
        CursorIcon::Default
    }

    fn on_event(&self, event: &Event, ctx: &mut EventCtx) {
        match event {
            Event::Pointer(PointerEvent::Down {
                button: PointerButton::Right,
                pos,
                ..
            }) => {
                if ctx.phase == EventPhase::Capture || self.items.is_empty() {
                    return;
                }

                self.open_at.set(Some(*pos));
                self.popup.open();
                ctx.request_focus();
                ctx.handled();
            }
            Event::Key(key) => {
                if ctx.phase != EventPhase::Target || !ctx.state.focused {
                    return;
                }
                if key.key == Key::ESCAPE {
                    self.popup.close_in(ctx);
                }
            }
            _ => {}
        }
    }

    fn on_unmount(&self) {
        self.popup.forget();
    }

    fn paint_self(&self, ctx: &mut PaintCtx) {
        if self.items.is_empty() {
            return;
        }

        if !self.popup.is_open() {
            self.open_at.set(None);
        }
        let menu = self.menu_view();
        let open_at = self.open_at.clone();
        self.popup.sync_show(ctx, move || {
            let anchor = open_at.get().map(Anchor::Point).unwrap_or(Anchor::Cursor);
            OverlayEntry::new(menu)
                .anchor(anchor)
                .placement(Placement::Cursor)
                .layer(OverlayLayer::Menu)
                .dismiss(Dismiss::OutsideClick)
        });
    }
}
