use std::rc::Rc;

use sequoia_core::{Color, Constraints, Length, Rect, Size};
use sequoia_dock::{PanelId, PanelPayload};
use sequoia_input::{CursorIcon, Event, PointerButton, PointerEvent};
use sequoia_layout::{LayoutCtx, LayoutStyle};
use sequoia_style::VisualState;

use crate::any::AnyView;
use crate::dnd::DragOptions;
use crate::runtime::{ElementId, Runtime};
use crate::view::{EventCtx, PaintCtx, View};
use crate::views::{Text, Unit};

const HEADER_H: f32 = 28.0;

#[derive(Clone)]
pub struct DockHeader {
    panel: PanelId,
    title: String,
    height: f32,
    background: Color,
    on_drag_start: Option<Rc<dyn Fn()>>,
}

impl DockHeader {
    pub fn new(panel: PanelId, title: impl Into<String>) -> Self {
        Self {
            panel,
            title: title.into(),
            height: HEADER_H,
            background: Color::rgb8(0x2C, 0x2C, 0x30),
            on_drag_start: None,
        }
    }

    pub fn height(mut self, h: f32) -> Self {
        self.height = if h.is_finite() && h > 0.0 {
            h
        } else {
            HEADER_H
        };
        self
    }

    pub fn background(mut self, color: Color) -> Self {
        self.background = color;
        self
    }

    pub fn on_drag_start(mut self, f: impl Fn() + 'static) -> Self {
        self.on_drag_start = Some(Rc::new(f));
        self
    }

    pub fn panel(&self) -> PanelId {
        self.panel
    }
}

impl View for DockHeader {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn children_override(&self) -> Option<Vec<AnyView>> {
        Some(vec![AnyView::new(Text::new(self.title.clone()))])
    }

    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fill(1.0),
            height: Length::Px(self.height),
            ..LayoutStyle::default()
        }
    }

    fn has_visual_state(&self) -> bool {
        true
    }

    fn captures_pointer(&self) -> bool {
        true
    }

    fn cursor_icon(&self, state: VisualState) -> CursorIcon {
        if state.pressed {
            CursorIcon::Grabbing
        } else {
            CursorIcon::Grab
        }
    }

    fn on_event(&self, event: &Event, ctx: &mut EventCtx) {
        match event {
            Event::Pointer(PointerEvent::Down {
                button: PointerButton::Left,
                pos,
                ..
            }) if ctx.rect.contains(*pos) => {
                ctx.begin_drag(PanelPayload(self.panel), DragOptions::new());
                if let Some(f) = &self.on_drag_start {
                    f();
                }
            }
            _ => {}
        }
    }

    fn paint_self(&self, ctx: &mut PaintCtx) {
        ctx.paint_box(
            crate::BoxStyle::new()
                .background(self.background)
                .radius(6.0),
        );
    }
}

#[derive(Clone)]
pub struct Dockable {
    panel: PanelId,
    header: DockHeader,
    content: AnyView,
}

impl Dockable {
    pub fn new(panel: PanelId, title: impl Into<String>, content: impl View + 'static) -> Self {
        let title = title.into();
        Self {
            panel,
            header: DockHeader::new(panel, title),
            content: AnyView::new(content),
        }
    }

    pub fn header(mut self, header: DockHeader) -> Self {
        debug_assert_eq!(header.panel(), self.panel, "other panel header");
        self.header = header;
        self
    }

    pub fn header_height(mut self, h: f32) -> Self {
        self.header = self.header.height(h);
        self
    }

    pub fn panel(&self) -> PanelId {
        self.panel
    }
}

impl View for Dockable {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn children_override(&self) -> Option<Vec<AnyView>> {
        Some(vec![
            AnyView::new(self.header.clone()),
            self.content.clone(),
        ])
    }

    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fill(1.0),
            height: Length::Fill(1.0),
            ..LayoutStyle::default()
        }
    }

    fn arrange_override(
        &self,
        rt: &Runtime,
        el: ElementId,
        rect: Rect,
        ctx: &mut LayoutCtx,
    ) -> bool {
        let kids = rt.children(el);
        if kids.len() < 2 {
            return false;
        }
        let hh = self.header.height.min(rect.size.height.max(0.0));
        let head = Rect::from_xywh(rect.left(), rect.top(), rect.size.width, hh);
        let body = Rect::from_xywh(
            rect.left(),
            rect.top() + hh,
            rect.size.width,
            (rect.size.height - hh).max(0.0),
        );
        let _ = rt.measure_node(
            kids[0],
            Constraints {
                min: Size::ZERO,
                max: Size::new(head.size.width.max(0.0), hh),
            },
            ctx,
        );
        rt.arrange_node(kids[0], head, ctx);
        let _ = rt.measure_node(
            kids[1],
            Constraints {
                min: Size::ZERO,
                max: Size::new(body.size.width.max(0.0), body.size.height.max(0.0)),
            },
            ctx,
        );
        rt.arrange_node(kids[1], body, ctx);
        true
    }
}

const PANEL_CLOSE_W: f32 = 26.0;

#[derive(Clone)]
pub struct DockPanel {
    panel: PanelId,
    header: DockHeader,
    content: AnyView,
    background: Color,
    radius: f32,
    show_close: bool,
    on_close: Option<Rc<dyn Fn()>>,
}

impl DockPanel {
    pub fn new(panel: PanelId, title: impl Into<String>, content: impl View + 'static) -> Self {
        Self {
            panel,
            header: DockHeader::new(panel, title),
            content: AnyView::new(content),
            background: Color::rgb8(0x1E, 0x1E, 0x22),
            radius: 6.0,
            show_close: false,
            on_close: None,
        }
    }

    pub fn header(mut self, header: DockHeader) -> Self {
        debug_assert_eq!(header.panel(), self.panel, "other panel header");
        self.header = header;
        self
    }

    pub fn header_height(mut self, h: f32) -> Self {
        self.header = self.header.height(h);
        self
    }

    pub fn header_background(mut self, color: Color) -> Self {
        self.header = self.header.background(color);
        self
    }

    pub fn background(mut self, color: Color) -> Self {
        self.background = color;
        self
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    pub fn closable(mut self, closable: bool) -> Self {
        self.show_close = closable;
        self
    }

    pub fn on_close(mut self, f: impl Fn() + 'static) -> Self {
        self.on_close = Some(Rc::new(f));
        self.show_close = true;
        self
    }

    pub fn panel(&self) -> PanelId {
        self.panel
    }
}

impl View for DockPanel {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn children_override(&self) -> Option<Vec<AnyView>> {
        if self.show_close {
            Some(vec![
                AnyView::new(self.header.clone()),
                AnyView::new(DockPanelClose {
                    on_close: self.on_close.clone(),
                }),
                self.content.clone(),
            ])
        } else {
            Some(vec![
                AnyView::new(self.header.clone()),
                self.content.clone(),
            ])
        }
    }

    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fill(1.0),
            height: Length::Fill(1.0),
            ..LayoutStyle::default()
        }
    }

    fn arrange_override(
        &self,
        rt: &Runtime,
        el: ElementId,
        rect: Rect,
        ctx: &mut LayoutCtx,
    ) -> bool {
        let kids = rt.children(el);
        let hh = self.header.height.min(rect.size.height.max(0.0));
        let head = Rect::from_xywh(rect.left(), rect.top(), rect.size.width, hh);
        let body = Rect::from_xywh(
            rect.left(),
            rect.top() + hh,
            rect.size.width,
            (rect.size.height - hh).max(0.0),
        );
        if self.show_close {
            if kids.len() < 3 {
                return false;
            }
            let close_w = PANEL_CLOSE_W.min(head.size.width.max(0.0));
            let title_r = Rect::from_xywh(
                head.left(),
                head.top(),
                (head.size.width - close_w).max(0.0),
                head.size.height,
            );
            let close_r =
                Rect::from_xywh(head.left() + title_r.size.width, head.top(), close_w, hh);
            let slots = [(kids[0], title_r), (kids[1], close_r), (kids[2], body)];
            for (child, slot) in slots {
                let _ = rt.measure_node(
                    child,
                    Constraints {
                        min: Size::ZERO,
                        max: Size::new(slot.size.width.max(0.0), slot.size.height.max(0.0)),
                    },
                    ctx,
                );
                rt.arrange_node(child, slot, ctx);
            }
            true
        } else {
            if kids.len() < 2 {
                return false;
            }
            let _ = rt.measure_node(
                kids[0],
                Constraints {
                    min: Size::ZERO,
                    max: Size::new(head.size.width.max(0.0), hh),
                },
                ctx,
            );
            rt.arrange_node(kids[0], head, ctx);
            let _ = rt.measure_node(
                kids[1],
                Constraints {
                    min: Size::ZERO,
                    max: Size::new(body.size.width.max(0.0), body.size.height.max(0.0)),
                },
                ctx,
            );
            rt.arrange_node(kids[1], body, ctx);
            true
        }
    }

    fn paint_self(&self, ctx: &mut PaintCtx) {
        ctx.paint_box(
            crate::BoxStyle::new()
                .background(self.background)
                .radius(self.radius),
        );
    }
}

#[derive(Clone)]
struct DockPanelClose {
    on_close: Option<Rc<dyn Fn()>>,
}

impl View for DockPanelClose {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn children_override(&self) -> Option<Vec<AnyView>> {
        Some(vec![AnyView::new(Text::new("×"))])
    }

    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Px(PANEL_CLOSE_W),
            height: Length::Fill(1.0),
            ..LayoutStyle::default()
        }
    }

    fn has_visual_state(&self) -> bool {
        true
    }

    fn captures_pointer(&self) -> bool {
        true
    }

    fn cursor_icon(&self, _state: VisualState) -> CursorIcon {
        CursorIcon::Pointer
    }

    fn on_event(&self, event: &Event, ctx: &mut EventCtx) {
        match event {
            Event::Pointer(PointerEvent::Up {
                button: PointerButton::Left,
                pos,
                ..
            }) if ctx.rect.contains(*pos) => {
                if let Some(f) = &self.on_close {
                    f();
                }
                ctx.handled();
            }
            _ => {}
        }
    }
}
