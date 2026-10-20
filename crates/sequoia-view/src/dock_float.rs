use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use sequoia_core::{Color, Constraints, Length, Point, Rect, Size};
use sequoia_dock::{DockModel, DockOp, PanelId};
use sequoia_input::{CursorIcon, Event, PointerButton, PointerEvent};
use sequoia_layout::{LayoutCtx, LayoutStyle};
use sequoia_reactive::Signal;
use sequoia_style::VisualState;

use crate::any::AnyView;
use crate::dock_view::{PanelRenderer, TitleProvider, default_title};
use crate::runtime::{ElementId, Runtime};
use crate::view::{EventCtx, PaintCtx, View};
use crate::views::{Text, Unit};

const HEADER_H: f32 = 26.0;
const BTN_W: f32 = 26.0;

#[derive(Clone)]
pub struct DockFloatLayer {
    model: Signal<DockModel>,
    render: PanelRenderer,
    title: TitleProvider,
    card_bg: Color,
    header_bg: Color,
    layout_width: Length,
    layout_height: Length,

    drags: Rc<RefCell<HashMap<u64, Rc<Cell<Option<CardDrag>>>>>>,
}

impl DockFloatLayer {
    pub fn new(model: Signal<DockModel>, render: impl Fn(PanelId) -> AnyView + 'static) -> Self {
        Self {
            model,
            render: Rc::new(render),
            title: Rc::new(default_title),
            card_bg: Color::rgb8(0x1E, 0x1E, 0x22),
            header_bg: Color::rgb8(0x2C, 0x2C, 0x30),
            layout_width: Length::Fill(1.0),
            layout_height: Length::Fill(1.0),
            drags: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    fn drag_for(&self, panel: PanelId) -> Rc<Cell<Option<CardDrag>>> {
        self.drags
            .borrow_mut()
            .entry(panel.get())
            .or_insert_with(|| Rc::new(Cell::new(None)))
            .clone()
    }

    pub fn title_provider(mut self, f: impl Fn(PanelId) -> String + 'static) -> Self {
        self.title = Rc::new(f);
        self
    }

    pub fn card_background(mut self, color: Color) -> Self {
        self.card_bg = color;
        self
    }

    pub fn header_background(mut self, color: Color) -> Self {
        self.header_bg = color;
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.layout_width = width.into();
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.layout_height = height.into();
        self
    }

    pub fn fill_size(self) -> Self {
        self.width(Length::Fill(1.0)).height(Length::Fill(1.0))
    }
}

impl View for DockFloatLayer {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn children_override(&self) -> Option<Vec<AnyView>> {
        let m = self.model.get();

        {
            let live: Vec<u64> = m
                .floating
                .iter()
                .filter(|f| f.window.is_none())
                .map(|f| f.panel.get())
                .collect();
            self.drags.borrow_mut().retain(|id, _| live.contains(id));
        }
        let cards: Vec<AnyView> = m
            .floating
            .iter()
            .filter(|f| f.window.is_none())
            .map(|f| {
                AnyView::new(
                    DockFloatCard {
                        model: self.model.clone(),
                        panel: f.panel,
                        title: (self.title)(f.panel),
                        content: (self.render)(f.panel),
                        x: f.x,
                        y: f.y,
                        w: f.w,
                        h: f.h,
                        card_bg: self.card_bg,
                        header_bg: self.header_bg,
                        drag: self.drag_for(f.panel),
                    }
                    .absolute_at(f.x, f.y),
                )
            })
            .collect();
        Some(cards)
    }

    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: self.layout_width,
            height: self.layout_height,
            ..LayoutStyle::default()
        }
    }

    fn hit_testable(&self) -> bool {
        false
    }
}

#[derive(Clone, Copy)]
struct CardDrag {
    origin: Point,
    card_x: f32,
    card_y: f32,
}

#[derive(Clone)]
pub(crate) struct DockFloatCard {
    model: Signal<DockModel>,
    panel: PanelId,
    title: String,
    content: AnyView,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    card_bg: Color,
    header_bg: Color,
    drag: Rc<Cell<Option<CardDrag>>>,
}

impl DockFloatCard {
    fn absolute_at(mut self, x: f32, y: f32) -> Self {
        self.x = x;
        self.y = y;
        self
    }

    fn header_rect(&self, card: Rect) -> Rect {
        Rect::from_xywh(
            card.left(),
            card.top(),
            card.size.width,
            HEADER_H.min(card.size.height),
        )
    }

    fn body_rect(&self, card: Rect) -> Rect {
        let hh = HEADER_H.min(card.size.height);
        Rect::from_xywh(
            card.left(),
            card.top() + hh,
            card.size.width,
            (card.size.height - hh).max(0.0),
        )
    }
}

impl View for DockFloatCard {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn children_override(&self) -> Option<Vec<AnyView>> {
        let _ = self.model.get();
        let panel = self.panel;
        let model = self.model.clone();
        let model2 = self.model.clone();
        Some(vec![
            AnyView::new(Text::new(self.title.clone())),
            AnyView::new(DockFloatButton {
                model,
                panel,
                kind: FloatButtonKind::DockBack,
            }),
            AnyView::new(DockFloatButton {
                model: model2,
                panel,
                kind: FloatButtonKind::Close,
            }),
            self.content.clone(),
        ])
    }

    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Px(self.w.max(1.0)),
            height: Length::Px(self.h.max(1.0)),
            ..LayoutStyle::default()
        }
        .position(sequoia_layout::Position::Absolute(
            sequoia_layout::Inset::xy(self.x, self.y),
        ))
    }

    fn captures_pointer(&self) -> bool {
        true
    }

    fn has_visual_state(&self) -> bool {
        true
    }

    fn arrange_override(
        &self,
        rt: &Runtime,
        el: ElementId,
        rect: Rect,
        ctx: &mut LayoutCtx,
    ) -> bool {
        let kids = rt.children(el);
        if kids.len() < 4 {
            return false;
        }
        let head = self.header_rect(rect);
        let body = self.body_rect(rect);

        let title_w = (head.size.width - 2.0 * BTN_W).max(0.0);
        let title_r = Rect::from_xywh(head.left(), head.top(), title_w, head.size.height);
        let dock_r = Rect::from_xywh(head.left() + title_w, head.top(), BTN_W, head.size.height);
        let close_r = Rect::from_xywh(
            head.left() + title_w + BTN_W,
            head.top(),
            BTN_W,
            head.size.height,
        );
        let slots = [
            (kids[0], title_r),
            (kids[1], dock_r),
            (kids[2], close_r),
            (kids[3], body),
        ];
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
    }

    fn on_event(&self, event: &Event, ctx: &mut EventCtx) {
        match event {
            Event::Pointer(PointerEvent::Down {
                button: PointerButton::Left,
                pos,
                ..
            }) => {
                let head = self.header_rect(ctx.rect);
                let title_w = (head.size.width - 2.0 * BTN_W).max(0.0);
                let grab = Rect::from_xywh(head.left(), head.top(), title_w, head.size.height);
                if grab.contains(*pos) {
                    let cur = self.model.get_untracked().floating_of(self.panel);
                    let (cx, cy) = cur.map(|f| (f.x, f.y)).unwrap_or((self.x, self.y));
                    self.drag.set(Some(CardDrag {
                        origin: *pos,
                        card_x: cx,
                        card_y: cy,
                    }));
                    ctx.capture_pointer();
                    ctx.handled();
                }
            }
            Event::Pointer(PointerEvent::Move { pos, .. }) => {
                if let Some(st) = self.drag.get() {
                    let panel = self.panel;
                    let nx = st.card_x + (pos.x - st.origin.x);
                    let ny = st.card_y + (pos.y - st.origin.y);
                    self.model.update(|m| {
                        if let Some(f) = m.floating.iter_mut().find(|f| f.panel == panel) {
                            if nx.is_finite() {
                                f.x = nx;
                            }
                            if ny.is_finite() {
                                f.y = ny;
                            }
                        }
                    });
                    ctx.handled();
                }
            }
            Event::Pointer(PointerEvent::Up {
                button: PointerButton::Left,
                ..
            }) => {
                self.drag.set(None);
            }
            Event::Pointer(PointerEvent::Cancel) => {
                self.drag.set(None);
            }
            _ => {}
        }
    }

    fn paint_self(&self, ctx: &mut PaintCtx) {
        ctx.paint_box(crate::BoxStyle::new().background(self.card_bg).radius(8.0));
        let head = self.header_rect(ctx.rect);
        ctx.paint_box_in(
            crate::BoxStyle::new()
                .background(self.header_bg)
                .radius(8.0),
            head,
        );
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FloatButtonKind {
    DockBack,
    Close,
}

#[derive(Clone)]
struct DockFloatButton {
    model: Signal<DockModel>,
    panel: PanelId,
    kind: FloatButtonKind,
}

impl View for DockFloatButton {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn children_override(&self) -> Option<Vec<AnyView>> {
        let label = match self.kind {
            FloatButtonKind::DockBack => "⧉",
            FloatButtonKind::Close => "×",
        };
        Some(vec![AnyView::new(Text::new(label))])
    }

    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Px(BTN_W),
            height: Length::Px(HEADER_H),
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
                let panel = self.panel;
                match self.kind {
                    FloatButtonKind::DockBack => {
                        self.model.update(|m| {
                            m.dock_first_tab(panel);
                        });
                    }
                    FloatButtonKind::Close => {
                        self.model.update(|m| {
                            m.apply(DockOp::Close { panel });
                        });
                    }
                }
                ctx.handled();
            }
            _ => {}
        }
    }
}
