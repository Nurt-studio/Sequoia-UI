use std::rc::Rc;

use sequoia_core::{Color, Edges};
use sequoia_reactive::Signal;
use sequoia_view::{
    AnyView, Column, Dismiss, DismissableLayer, FocusScope, Row, Spacer, Text, Unit, View,
};

use crate::button::Button;
use crate::modal::Modal;
use crate::modal::ModalScrim;
use sequoia_style::{DIALOG_DANGER, DialogStyle};

type ActionFn = Rc<dyn Fn()>;

#[derive(Clone)]
struct DialogAction {
    label: String,
    danger: bool,
    action: ActionFn,
}

#[derive(Clone)]
pub struct Dialog {
    open: Signal<bool>,
    title: String,
    content: AnyView,
    actions: Vec<DialogAction>,
    cancel_label: Option<String>,
    width: f32,
    on_close: Option<ActionFn>,
    style: Option<DialogStyle>,
}

impl Dialog {
    pub fn new(open: Signal<bool>, title: impl Into<String>) -> Self {
        Self {
            open,
            title: title.into(),
            content: AnyView::new(Unit),
            actions: Vec::new(),
            cancel_label: None,
            width: 520.0,
            on_close: None,
            style: None,
        }
    }

    pub fn style(mut self, style: DialogStyle) -> Self {
        self.style = Some(style);
        self
    }

    pub fn content(mut self, content: impl View + 'static) -> Self {
        self.content = AnyView::new(content);
        self
    }

    pub fn action(mut self, label: impl Into<String>, action: impl Fn() + 'static) -> Self {
        self.actions.push(DialogAction {
            label: label.into(),
            danger: false,
            action: Rc::new(action),
        });
        self
    }

    pub fn danger(mut self, label: impl Into<String>, action: impl Fn() + 'static) -> Self {
        self.actions.push(DialogAction {
            label: label.into(),
            danger: true,
            action: Rc::new(action),
        });
        self
    }

    pub fn cancel(mut self, label: impl Into<String>) -> Self {
        self.cancel_label = Some(label.into());
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() && width > 0.0 {
            self.width = width;
        }
        self
    }

    pub fn on_close(mut self, f: impl Fn() + 'static) -> Self {
        self.on_close = Some(Rc::new(f));
        self
    }

    pub fn is_open(&self) -> bool {
        self.open.get_untracked()
    }

    pub fn open_signal(&self) -> Signal<bool> {
        self.open.clone()
    }

    pub fn close(&self) {
        self.open.set(false);
        if let Some(f) = &self.on_close {
            f();
        }
    }

    pub fn scrim(&self) -> ModalScrim {
        Modal::new(self.open.clone()).scrim()
    }

    fn footer(&self) -> AnyView {
        let mut row = Row::new().gap(8.0);
        if let Some(label) = &self.cancel_label {
            let this = self.clone();
            row = row
                .child(Button::new(label.clone()).on_click(move || this.close()))
                .child(Spacer::flex(1.0));
        } else {
            row = row.child(Spacer::flex(1.0));
        }
        for a in &self.actions {
            let action = a.action.clone();
            let btn = Button::new(a.label.clone()).on_click(move || {
                action();
            });
            let btn = if a.danger {
                btn.color(self.danger_color())
            } else {
                btn
            };
            row = row.child(btn);
        }
        AnyView::new(row)
    }

    fn danger_color(&self) -> Color {
        self.style.and_then(|s| s.danger).unwrap_or(DIALOG_DANGER)
    }

    fn title_size(&self) -> f32 {
        self.style.and_then(|s| s.title_size).unwrap_or(16.0)
    }

    fn card(&self) -> AnyView {
        let this = self.clone();
        let content = self.content.clone();
        let footer = self.footer();
        let title = self.title.clone();
        let card = Column::new()
            .gap(12.0)
            .child(Text::new(title).size(this.title_size()).bold())
            .child(content)
            .child(footer);
        let open = self.open.clone();

        let layer = DismissableLayer::wrap(FocusScope::new().trap(true).child(card))
            .bind(open)
            .on_escape(move || {
                this.close();
            });
        AnyView::new(layer)
    }
}

impl View for Dialog {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn accessibility(&self) -> Option<sequoia_view::Accessible> {
        Some(sequoia_view::Accessible::new(
            sequoia_view::AccessibleRole::Dialog,
            self.title.clone(),
        ))
    }

    fn children_override(&self) -> Option<Vec<AnyView>> {
        let modal = Modal::new(self.open.clone())
            .width(self.width)
            .padding(Edges::all(20.0))
            .dismiss(Dismiss::OutsideClick)
            .child(self.card());
        Some(vec![AnyView::new(modal)])
    }

    fn layout_style(&self) -> sequoia_layout::LayoutStyle {
        sequoia_layout::LayoutStyle::default()
    }
}
