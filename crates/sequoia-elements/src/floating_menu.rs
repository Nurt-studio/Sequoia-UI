#![forbid(unsafe_code)]

use std::cell::Cell;
use std::rc::Rc;

use sequoia_core::{Color, Edges, Length, Point};
use sequoia_input::{CursorIcon, Event, EventPhase, Key, PointerButton, PointerEvent};
use sequoia_layout::LayoutStyle;
use sequoia_reactive::{Environment, Signal};
use sequoia_style::{Backdrop, FloatingMenuStyle, Theme, VisualState};
use sequoia_view::{
    Align, Anchor, AnyView, Column, Dismiss, Easing, EventCtx, Icon, OverlayEntry, OverlayLayer,
    PaintCtx, Placement, Presence, Row, Spacer, Text, Unit, View,
};

use crate::container::Container;
use crate::kbd::Kbd;
use crate::modal::OverlayPanel;
use crate::popup::PopupState;

pub const MENU_WIDTH: f32 = 220.0;

pub(crate) const MENU_ENTER_SECS: f32 = 0.16;
const MENU_ENTER_EASE: Easing = Easing::CubicBezier(0.3, 1.2, 0.4, 1.0);
const MENU_ENTER_DY: f32 = 6.0;

#[derive(Clone)]
enum MenuAction {
    Plain(Rc<dyn Fn()>),
    Env(Rc<dyn Fn(&Environment)>),
}

enum MenuEntry {
    Item {
        icon: Option<String>,
        label: String,
        shortcut: Option<String>,
        danger: bool,
        disabled: bool,
        action: MenuAction,
    },
    Custom {
        content: AnyView,
        disabled: bool,
        action: MenuAction,
    },
    Separator,
}

pub struct FloatingMenu {
    child: AnyView,
    rows: Vec<MenuEntry>,
    open: Signal<bool>,
    popup: PopupState,

    open_at: Rc<Cell<Option<Point>>>,
    style: Option<FloatingMenuStyle>,
    width: Length,
    background: Option<Color>,
    border: Option<Color>,
    text_color: Option<Color>,
    radius: Option<f32>,
    padding: Option<(f32, f32)>,
    font_size: Option<f32>,
    backdrop: Option<Backdrop>,
    below_invoker: bool,
    click_opens: bool,
    margin: Edges,
}

#[must_use]
pub struct MenuItem {
    icon: Option<String>,
    label: String,
    shortcut: Option<String>,
    danger: bool,
    disabled: bool,
    menu: FloatingMenu,
}

impl MenuItem {
    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    pub fn danger(mut self, danger: bool) -> Self {
        self.danger = danger;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_click(self, action: impl Fn() + 'static) -> FloatingMenu {
        let mut menu = self.menu;
        menu.push_item(
            self.icon,
            self.label,
            self.shortcut,
            self.danger,
            self.disabled,
            MenuAction::Plain(Rc::new(action)),
        );
        menu
    }

    pub fn on_click_env(self, action: impl Fn(&Environment) + 'static) -> FloatingMenu {
        let mut menu = self.menu;
        menu.push_item(
            self.icon,
            self.label,
            self.shortcut,
            self.danger,
            self.disabled,
            MenuAction::Env(Rc::new(action)),
        );
        menu
    }
}

#[must_use]
pub struct CustomMenuItem {
    content: AnyView,
    disabled: bool,
    menu: FloatingMenu,
}

impl CustomMenuItem {
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_click(self, action: impl Fn() + 'static) -> FloatingMenu {
        let mut menu = self.menu;
        menu.rows.push(MenuEntry::Custom {
            content: self.content,
            disabled: self.disabled,
            action: MenuAction::Plain(Rc::new(action)),
        });
        menu
    }

    pub fn on_click_env(self, action: impl Fn(&Environment) + 'static) -> FloatingMenu {
        let mut menu = self.menu;
        menu.rows.push(MenuEntry::Custom {
            content: self.content,
            disabled: self.disabled,
            action: MenuAction::Env(Rc::new(action)),
        });
        menu
    }
}

impl FloatingMenu {
    pub fn new(child: impl View + 'static) -> Self {
        Self::wrap(child)
    }

    pub fn wrap(child: impl View + 'static) -> Self {
        let open = Signal::new(false);
        let popup = PopupState::new(open.clone());
        Self {
            child: AnyView::new(child),
            rows: Vec::new(),
            open,
            popup,
            open_at: Rc::new(Cell::new(None)),
            style: None,
            width: MENU_WIDTH.into(),
            background: None,
            border: None,
            text_color: None,
            radius: None,
            padding: None,
            font_size: None,
            backdrop: None,
            below_invoker: false,
            click_opens: false,
            margin: Edges::ZERO,
        }
    }

    pub fn open_signal(&self) -> Signal<bool> {
        self.open.clone()
    }

    pub fn is_open(&self) -> bool {
        self.open.get_untracked()
    }

    pub fn row(
        mut self,
        icon: impl Into<String>,
        label: impl Into<String>,
        action: impl Fn() + 'static,
    ) -> Self {
        self.rows.push(MenuEntry::Item {
            icon: Some(icon.into()),
            label: label.into(),
            shortcut: None,
            danger: false,
            disabled: false,
            action: MenuAction::Plain(Rc::new(action)),
        });
        self
    }

    pub fn row_env(
        mut self,
        icon: impl Into<String>,
        label: impl Into<String>,
        action: impl Fn(&Environment) + 'static,
    ) -> Self {
        self.rows.push(MenuEntry::Item {
            icon: Some(icon.into()),
            label: label.into(),
            shortcut: None,
            danger: false,
            disabled: false,
            action: MenuAction::Env(Rc::new(action)),
        });
        self
    }

    pub fn shortcut_row(
        mut self,
        icon: impl Into<String>,
        label: impl Into<String>,
        shortcut: impl Into<String>,
        action: impl Fn() + 'static,
    ) -> Self {
        self.rows.push(MenuEntry::Item {
            icon: Some(icon.into()),
            label: label.into(),
            shortcut: Some(shortcut.into()),
            danger: false,
            disabled: false,
            action: MenuAction::Plain(Rc::new(action)),
        });
        self
    }

    pub fn text_row(mut self, label: impl Into<String>, action: impl Fn() + 'static) -> Self {
        self.rows.push(MenuEntry::Item {
            icon: None,
            label: label.into(),
            shortcut: None,
            danger: false,
            disabled: false,
            action: MenuAction::Plain(Rc::new(action)),
        });
        self
    }

    pub fn text_row_env(
        mut self,
        label: impl Into<String>,
        action: impl Fn(&Environment) + 'static,
    ) -> Self {
        self.rows.push(MenuEntry::Item {
            icon: None,
            label: label.into(),
            shortcut: None,
            danger: false,
            disabled: false,
            action: MenuAction::Env(Rc::new(action)),
        });
        self
    }

    pub fn danger_row(
        mut self,
        icon: impl Into<String>,
        label: impl Into<String>,
        action: impl Fn() + 'static,
    ) -> Self {
        self.rows.push(MenuEntry::Item {
            icon: Some(icon.into()),
            label: label.into(),
            shortcut: None,
            danger: true,
            disabled: false,
            action: MenuAction::Plain(Rc::new(action)),
        });
        self
    }

    pub fn item_raw(
        mut self,
        icon: Option<String>,
        label: impl Into<String>,
        shortcut: Option<String>,
        danger: bool,
        disabled: bool,
        action: impl Fn() + 'static,
    ) -> Self {
        self.push_item(
            icon,
            label.into(),
            shortcut,
            danger,
            disabled,
            MenuAction::Plain(Rc::new(action)),
        );
        self
    }

    pub fn item(self, label: impl Into<String>) -> MenuItem {
        MenuItem {
            icon: None,
            label: label.into(),
            shortcut: None,
            danger: false,
            disabled: false,
            menu: self,
        }
    }

    pub fn custom_item(self, content: impl View + 'static) -> CustomMenuItem {
        CustomMenuItem {
            content: AnyView::new(content),
            disabled: false,
            menu: self,
        }
    }

    pub fn custom_row(mut self, content: impl View + 'static, action: impl Fn() + 'static) -> Self {
        self.rows.push(MenuEntry::Custom {
            content: AnyView::new(content),
            disabled: false,
            action: MenuAction::Plain(Rc::new(action)),
        });
        self
    }

    pub fn custom_row_env(
        mut self,
        content: impl View + 'static,
        action: impl Fn(&Environment) + 'static,
    ) -> Self {
        self.rows.push(MenuEntry::Custom {
            content: AnyView::new(content),
            disabled: false,
            action: MenuAction::Env(Rc::new(action)),
        });
        self
    }

    fn push_item(
        &mut self,
        icon: Option<String>,
        label: String,
        shortcut: Option<String>,
        danger: bool,
        disabled: bool,
        action: MenuAction,
    ) {
        self.rows.push(MenuEntry::Item {
            icon,
            label,
            shortcut,
            danger,
            disabled,
            action,
        });
    }

    pub fn separator(mut self) -> Self {
        self.rows.push(MenuEntry::Separator);
        self
    }

    pub fn style(mut self, style: FloatingMenuStyle) -> Self {
        self.style = Some(style);
        self
    }

    pub fn popup_width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    #[deprecated(
        since = "0.1.0",
        note = "misleading name - use `popup_width()` (popup width, not invoker width)"
    )]
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
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

    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = Some(radius.max(0.0));
        self
    }

    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = Some(size);
        self
    }

    pub fn padding_xy(mut self, pad_x: f32, pad_y: f32) -> Self {
        self.padding = Some((pad_x, pad_y));
        self
    }

    pub fn backdrop(mut self, backdrop: Backdrop) -> Self {
        self.backdrop = Some(backdrop);
        self
    }

    pub fn below_invoker(mut self) -> Self {
        self.below_invoker = true;
        self
    }

    pub fn open_on_click(mut self) -> Self {
        self.click_opens = true;
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

    fn menu_view(&self, theme: &Theme) -> AnyView {
        let style = self.style.unwrap_or_default();
        let font_size = self.font_size.or(style.font_size).unwrap_or(13.5);
        let text = self.text_color.or(style.text).unwrap_or(theme.text);
        let danger = Color::hex(0xC2452D);
        let (pad_x, pad_y) = self.padding.or(style.padding).unwrap_or((6.0, 6.0));

        let mut col = Column::new().fill_width().gap(2.0);
        for row in &self.rows {
            match row {
                MenuEntry::Separator => {
                    col = col.child(
                        Container::new()
                            .fill_width()
                            .height(1.0)
                            .background(theme.border),
                    );
                }
                MenuEntry::Item {
                    icon,
                    label,
                    shortcut,
                    danger: is_danger,
                    disabled: is_disabled,
                    action,
                } => {
                    let color = if *is_disabled {
                        theme.text_secondary
                    } else if *is_danger {
                        danger
                    } else {
                        text
                    };
                    let popup = self.popup.clone();
                    let action = action.clone();
                    let mut line = Row::new().fill_width().gap(10.0).align_items(Align::Center);
                    if let Some(name) = icon {
                        line = line.child(Icon::new(name.clone()).size(16.0).color(color));
                    }
                    line = line
                        .child(Text::new(label.clone()).size(font_size).color(color))
                        .child(Spacer::flex(1.0));
                    if let Some(kb) = shortcut {
                        line = line.child(Kbd::new(kb.clone()));
                    }
                    let cell = Container::new()
                        .fill_width()
                        .radius(8.0)
                        .padding_xy(10.0, 7.0)
                        .child(line);

                    if *is_disabled {
                        col = col.child(cell);
                        continue;
                    }
                    col = col.child({
                        match action {
                            MenuAction::Plain(f) => cell.on_click(move || {
                                f();
                                popup.close();
                            }),
                            MenuAction::Env(f) => cell.on_click_env(move |env| {
                                f(env);
                                popup.close();
                            }),
                        }
                    });
                }
                MenuEntry::Custom {
                    content,
                    disabled: is_disabled,
                    action,
                } => {
                    let popup = self.popup.clone();
                    let action = action.clone();
                    let cell = Container::new()
                        .fill_width()
                        .radius(8.0)
                        .padding_xy(10.0, 7.0)
                        .child(content.clone());

                    if *is_disabled {
                        col = col.child(cell);
                        continue;
                    }
                    col = col.child({
                        match action {
                            MenuAction::Plain(f) => cell.on_click(move || {
                                f();
                                popup.close();
                            }),
                            MenuAction::Env(f) => cell.on_click_env(move |env| {
                                f(env);
                                popup.close();
                            }),
                        }
                    });
                }
            }
        }
        AnyView::new(
            Container::new()
                .fill_width()
                .padding_xy(pad_x, pad_y)
                .child(col),
        )
    }
}

impl View for FloatingMenu {
    fn body(&self) -> impl View + 'static {
        self.popup.subscribe();
        Unit
    }

    fn children_override(&self) -> Option<Vec<AnyView>> {
        Some(vec![self.child.clone()])
    }

    fn layout_style(&self) -> LayoutStyle {
        let mut s = self.child.inner_layout_style();
        if self.margin != Edges::ZERO {
            s.margin = self.margin;
        }
        s
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
            Event::Pointer(PointerEvent::Down { button, pos, .. }) => {
                if self.rows.is_empty() {
                    return;
                }
                match button {
                    PointerButton::Right => {
                        if ctx.phase == EventPhase::Capture {
                            return;
                        }

                        self.open_at.set(Some(*pos));

                        self.popup.open();
                        ctx.handled();
                    }

                    PointerButton::Left if self.click_opens => {
                        if ctx.phase != EventPhase::Capture {
                            return;
                        }
                        self.popup.open();
                    }
                    _ => {}
                }
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
        if self.rows.is_empty() {
            return;
        }
        let theme = ctx.theme();
        let style = self.style.unwrap_or_default();
        let radius = self.radius.or(style.radius).unwrap_or(4.0);
        let panel = OverlayPanel {
            child: self.menu_view(&theme),
            width: self.width,
            height: Length::Fit,
            padding: Edges::ZERO,
            background: self
                .background
                .or(style.background)
                .unwrap_or(theme.surface),
            border: self.border.or(style.border).unwrap_or(theme.border),
            border_width: 1.0,
            radius,
            backdrop: self.backdrop.unwrap_or(Backdrop::XS),
        };

        let enter = Presence::new(Signal::new(true))
            .from(0.0)
            .enter_duration(MENU_ENTER_SECS)
            .enter_easing(MENU_ENTER_EASE)
            .slide(0.0, MENU_ENTER_DY)
            .child(panel);
        let below = self.below_invoker;
        let el = ctx.element;

        if !self.popup.is_open() {
            self.open_at.set(None);
        }
        let open_at = self.open_at.clone();
        self.popup.sync_show(ctx, move || {
            let entry = OverlayEntry::new(enter)
                .layer(OverlayLayer::Menu)
                .dismiss(Dismiss::OutsideClick);
            if below {
                entry
                    .anchor(Anchor::View(el))
                    .placement(Placement::BottomStart)
                    .offset(0.0, 4.0)
            } else {
                let anchor = open_at.get().map(Anchor::Point).unwrap_or(Anchor::Cursor);
                entry.anchor(anchor).placement(Placement::Cursor)
            }
        });
    }
}

fn sanitize_margin(v: f32) -> f32 {
    if v.is_finite() { v.max(0.0) } else { 0.0 }
}
