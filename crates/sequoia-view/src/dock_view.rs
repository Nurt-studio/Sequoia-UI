use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use sequoia_core::{Color, Constraints, Length, Point, Rect, Size};
use sequoia_dock::{
    DockDrop, DockModel, DockNode, DockOp, PanelId, PanelPayload, drop_from_fraction_or_float,
    preview_rect,
};
use sequoia_input::{CursorIcon, Event, EventPhase, Modifiers, PointerButton, PointerEvent};
use sequoia_layout::{Axis, LayoutCtx, LayoutStyle};
use sequoia_reactive::Signal;
use sequoia_style::VisualState;

use crate::any::AnyView;
use crate::dnd::{DragOptions, DropPosition};
use crate::runtime::{ElementId, Runtime};
use crate::view::{EventCtx, PaintCtx, View};
use crate::views::{Text, Unit};

pub type PanelRenderer = Rc<dyn Fn(PanelId) -> AnyView>;

pub type TitleProvider = Rc<dyn Fn(PanelId) -> String>;

pub(crate) fn default_title(panel: PanelId) -> String {
    format!("{panel}")
}

const HANDLE_W: f32 = 7.0;
const TAB_H: f32 = 28.0;
const PREVIEW_FILL: Color = Color::hex(0x0D99FF);

#[derive(Clone)]
pub struct DockView {
    model: Signal<DockModel>,
    render: PanelRenderer,
    title: TitleProvider,
    edge: f32,
    preview_frac: f32,
    tabstrip_h: f32,
    handle: f32,
    empty_label: String,
    layout: LayoutStyle,

    split_drags: Rc<RefCell<HashMap<Vec<bool>, Rc<Cell<Option<SplitDrag>>>>>>,
}

impl DockView {
    pub fn new(model: Signal<DockModel>, render: impl Fn(PanelId) -> AnyView + 'static) -> Self {
        Self {
            model,
            render: Rc::new(render),
            title: Rc::new(default_title),
            edge: sequoia_dock::EDGE_FRACTION,
            preview_frac: 0.5,
            tabstrip_h: TAB_H,
            handle: HANDLE_W,
            empty_label: String::from("Drag panel here"),
            layout: LayoutStyle {
                width: Length::Fill(1.0),
                height: Length::Fill(1.0),
                ..LayoutStyle::default()
            },
            split_drags: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    pub fn title_provider(mut self, f: impl Fn(PanelId) -> String + 'static) -> Self {
        self.title = Rc::new(f);
        self
    }

    pub fn edge(mut self, edge: f32) -> Self {
        self.edge = if edge.is_finite() {
            edge.clamp(0.05, 0.45)
        } else {
            sequoia_dock::EDGE_FRACTION
        };
        self
    }

    pub fn tabstrip_height(mut self, h: f32) -> Self {
        self.tabstrip_h = if h.is_finite() && h > 0.0 { h } else { TAB_H };
        self
    }

    pub fn handle_width(mut self, w: f32) -> Self {
        self.handle = if w.is_finite() { w.max(3.0) } else { HANDLE_W };
        self
    }

    pub fn empty_label(mut self, label: impl Into<String>) -> Self {
        self.empty_label = label.into();
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.layout = self.layout.width(width.into());
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.layout = self.layout.height(height.into());
        self
    }

    pub fn fill_size(self) -> Self {
        self.width(Length::Fill(1.0)).height(Length::Fill(1.0))
    }

    fn split_drag_for(&self, path: &[bool]) -> Rc<Cell<Option<SplitDrag>>> {
        self.split_drags
            .borrow_mut()
            .entry(path.to_vec())
            .or_insert_with(|| Rc::new(Cell::new(None)))
            .clone()
    }

    fn prune_split_drags(&self, root: &DockNode) {
        let mut live: Vec<Vec<bool>> = Vec::new();
        collect_split_paths(root, Vec::new(), &mut live);
        self.split_drags
            .borrow_mut()
            .retain(|path, _| live.contains(path));
    }

    fn slot(&self, panel: PanelId) -> DockSlot {
        DockSlot {
            model: self.model.clone(),
            target: panel,
            content: (self.render)(panel),
            edge: self.edge,
            preview_frac: self.preview_frac,
        }
    }

    pub(crate) fn node_view(&self, node: &DockNode, path: Vec<bool>) -> AnyView {
        match node {
            DockNode::Leaf(p) => AnyView::new(self.slot(*p)),
            DockNode::Tabs { tabs, active } => AnyView::new(DockTabs {
                model: self.model.clone(),
                render: self.render.clone(),
                title: self.title.clone(),
                tabs: tabs.clone(),
                active: *active,
                edge: self.edge,
                preview_frac: self.preview_frac,
                tabstrip_h: self.tabstrip_h,
            }),
            DockNode::Split {
                axis,
                frac,
                first,
                second,
            } => {
                let mut p_first = path.clone();
                p_first.push(false);
                let mut p_second = path.clone();
                p_second.push(true);
                let first = self.node_view(first, p_first);
                let second = self.node_view(second, p_second);
                let drag = self.split_drag_for(&path);
                AnyView::new(DockSplit {
                    model: self.model.clone(),
                    axis: axis.axis(),
                    frac: *frac,
                    path,
                    handle: self.handle,
                    first,
                    second,
                    drag,
                })
            }

            _ => AnyView::new(Unit),
        }
    }
}

impl View for DockView {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn children_override(&self) -> Option<Vec<AnyView>> {
        let m = self.model.get();
        match &m.root {
            Some(root) => {
                self.prune_split_drags(root);
                Some(vec![self.node_view(root, Vec::new())])
            }
            None => Some(vec![AnyView::new(Text::new(self.empty_label.clone()))]),
        }
    }

    fn layout_style(&self) -> LayoutStyle {
        self.layout
    }
}

#[derive(Clone, Copy)]
struct SplitDrag {
    origin_main: f32,
    frac: f32,
}

fn collect_split_paths(node: &DockNode, prefix: Vec<bool>, out: &mut Vec<Vec<bool>>) {
    if let DockNode::Split { first, second, .. } = node {
        out.push(prefix.clone());
        let mut p_first = prefix.clone();
        p_first.push(false);
        collect_split_paths(first, p_first, out);
        let mut p_second = prefix;
        p_second.push(true);
        collect_split_paths(second, p_second, out);
    }
}

#[derive(Clone)]
pub(crate) struct DockSplit {
    model: Signal<DockModel>,
    axis: Axis,
    frac: f32,
    path: Vec<bool>,
    handle: f32,
    first: AnyView,
    second: AnyView,
    drag: Rc<Cell<Option<SplitDrag>>>,
}

impl DockSplit {
    fn main_pos(&self, p: Point) -> f32 {
        match self.axis {
            Axis::Horizontal => p.x,
            Axis::Vertical => p.y,
        }
    }

    fn handle_range(&self, content: Rect, frac: f32) -> (f32, f32) {
        let total = match self.axis {
            Axis::Horizontal => content.size.width,
            Axis::Vertical => content.size.height,
        };
        let avail = (total - self.handle).max(0.0);
        let off = avail * DockNode::sanitized_frac(frac);
        let start = match self.axis {
            Axis::Horizontal => content.left(),
            Axis::Vertical => content.top(),
        };
        (start + off, start + off + self.handle)
    }

    fn live_frac(&self) -> f32 {
        self.model
            .get_untracked()
            .frac_at(&self.path)
            .unwrap_or(self.frac)
    }
}

impl View for DockSplit {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn children_override(&self) -> Option<Vec<AnyView>> {
        let _ = self.model.get();
        Some(vec![self.first.clone(), self.second.clone()])
    }

    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fill(1.0),
            height: Length::Fill(1.0),
            ..LayoutStyle::default()
        }
    }

    fn captures_pointer(&self) -> bool {
        true
    }

    fn has_visual_state(&self) -> bool {
        true
    }

    fn cursor_icon(&self, state: VisualState) -> CursorIcon {
        if self.drag.get().is_some() || state.pressed {
            match self.axis {
                Axis::Horizontal => CursorIcon::ColResize,
                Axis::Vertical => CursorIcon::RowResize,
            }
        } else {
            CursorIcon::Default
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
        let frac = self.live_frac();
        let avail_main = match self.axis {
            Axis::Horizontal => rect.size.width,
            Axis::Vertical => rect.size.height,
        };
        let avail = (avail_main - self.handle).max(0.0);
        let a = avail * DockNode::sanitized_frac(frac);
        let b = (avail - a).max(0.0);
        let (ra, rb) = match self.axis {
            Axis::Horizontal => (
                Rect::from_xywh(rect.left(), rect.top(), a, rect.size.height),
                Rect::from_xywh(
                    rect.left() + a + self.handle,
                    rect.top(),
                    b,
                    rect.size.height,
                ),
            ),
            Axis::Vertical => (
                Rect::from_xywh(rect.left(), rect.top(), rect.size.width, a),
                Rect::from_xywh(
                    rect.left(),
                    rect.top() + a + self.handle,
                    rect.size.width,
                    b,
                ),
            ),
        };
        for (child, slot) in [(kids[0], ra), (kids[1], rb)] {
            let max = Size::new(slot.size.width.max(0.0), slot.size.height.max(0.0));
            let _ = rt.measure_node(
                child,
                Constraints {
                    min: Size::ZERO,
                    max,
                },
                ctx,
            );
            rt.arrange_node(child, slot, ctx);
        }
        true
    }

    fn on_event(&self, event: &Event, ctx: &mut EventCtx) {
        if ctx.phase != EventPhase::Target {
            return;
        }
        match event {
            Event::Pointer(PointerEvent::Down {
                button: PointerButton::Left,
                pos,
                ..
            }) => {
                let (lo, hi) = self.handle_range(ctx.rect, self.live_frac());
                let p = self.main_pos(*pos);
                if p >= lo && p <= hi {
                    self.drag.set(Some(SplitDrag {
                        origin_main: p,
                        frac: self.live_frac(),
                    }));
                    ctx.capture_pointer();
                    ctx.handled();
                }
            }
            Event::Pointer(PointerEvent::Move { pos, .. }) => {
                if let Some(st) = self.drag.get() {
                    let total = match self.axis {
                        Axis::Horizontal => ctx.rect.size.width,
                        Axis::Vertical => ctx.rect.size.height,
                    };
                    let avail = (total - self.handle).max(0.0);
                    if avail > 0.0 {
                        let p = self.main_pos(*pos);
                        let next = st.frac + (p - st.origin_main) / avail;
                        let path = self.path.clone();
                        self.model.update(|m| {
                            m.set_frac_at(&path, next);
                        });
                    }
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
        let active = self.drag.get().is_some() || ctx.state.pressed;
        let c = if active {
            Color::rgb8(0x0D, 0x99, 0xFF)
        } else {
            Color::rgb8(0x75, 0x75, 0x75).with_alpha(0.55)
        };
        let (lo, _) = self.handle_range(ctx.rect, self.live_frac());
        match self.axis {
            Axis::Horizontal => {
                let cx = lo + self.handle * 0.5;
                ctx.quad(
                    Rect::from_xywh(cx - 0.5, ctx.rect.top(), 1.0, ctx.rect.size.height),
                    c,
                    0.0,
                );
            }
            Axis::Vertical => {
                let cy = lo + self.handle * 0.5;
                ctx.quad(
                    Rect::from_xywh(ctx.rect.left(), cy - 0.5, ctx.rect.size.width, 1.0),
                    c,
                    0.0,
                );
            }
        }
    }
}

#[derive(Clone)]
pub(crate) struct DockTabs {
    model: Signal<DockModel>,
    render: PanelRenderer,
    title: TitleProvider,
    tabs: Vec<PanelId>,
    active: usize,
    edge: f32,
    preview_frac: f32,
    tabstrip_h: f32,
}

impl View for DockTabs {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn children_override(&self) -> Option<Vec<AnyView>> {
        let m = self.model.get();

        let (tabs, active) = match &m.root {
            Some(root) => find_tabs(root, &self.tabs).unwrap_or((self.tabs.clone(), self.active)),
            None => (self.tabs.clone(), self.active),
        };
        let mut strip = crate::views::Row::new();
        for (i, t) in tabs.iter().copied().enumerate() {
            let name = (self.title)(t);
            strip = strip.child(DockTabButton {
                model: self.model.clone(),
                panel: t,
                label: name,
                active: i == active,
            });
        }
        let active_panel = tabs.get(active).or(tabs.first()).copied();
        let content = match active_panel {
            Some(p) => AnyView::new(DockSlot {
                model: self.model.clone(),
                target: p,
                content: (self.render)(p),
                edge: self.edge,
                preview_frac: self.preview_frac,
            }),
            None => AnyView::new(Unit),
        };
        Some(vec![AnyView::new(strip), content])
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
        let strip_h = self.tabstrip_h.min(rect.size.height.max(0.0));
        let strip_rect = Rect::from_xywh(rect.left(), rect.top(), rect.size.width, strip_h);
        let body_rect = Rect::from_xywh(
            rect.left(),
            rect.top() + strip_h,
            rect.size.width,
            (rect.size.height - strip_h).max(0.0),
        );
        let _ = rt.measure_node(
            kids[0],
            Constraints {
                min: Size::ZERO,
                max: Size::new(strip_rect.size.width.max(0.0), strip_h),
            },
            ctx,
        );
        rt.arrange_node(kids[0], strip_rect, ctx);
        let _ = rt.measure_node(
            kids[1],
            Constraints {
                min: Size::ZERO,
                max: Size::new(
                    body_rect.size.width.max(0.0),
                    body_rect.size.height.max(0.0),
                ),
            },
            ctx,
        );
        rt.arrange_node(kids[1], body_rect, ctx);
        true
    }
}

fn find_tabs(root: &DockNode, tabs: &[PanelId]) -> Option<(Vec<PanelId>, usize)> {
    match root {
        DockNode::Leaf(_) => None,
        DockNode::Tabs { tabs: t, active } => {
            if (t.len() == tabs.len() && t.iter().all(|p| tabs.contains(p)))
                || tabs.iter().any(|p| t.contains(p))
            {
                Some((t.clone(), *active))
            } else {
                None
            }
        }
        DockNode::Split { first, second, .. } => {
            find_tabs(first, tabs).or_else(|| find_tabs(second, tabs))
        }

        _ => None,
    }
}

#[derive(Clone)]
pub(crate) struct DockTabButton {
    model: Signal<DockModel>,
    panel: PanelId,
    label: String,
    active: bool,
}

impl View for DockTabButton {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn children_override(&self) -> Option<Vec<AnyView>> {
        Some(vec![AnyView::new(Text::new(self.label.clone()))])
    }

    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fit,
            height: Length::Fit,
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
        CursorIcon::Grab
    }

    fn on_event(&self, event: &Event, ctx: &mut EventCtx) {
        match event {
            Event::Pointer(PointerEvent::Down {
                button: PointerButton::Left,
                pos,
                ..
            }) if ctx.rect.contains(*pos) => {
                ctx.begin_drag(PanelPayload(self.panel), DragOptions::new());
            }
            Event::Pointer(PointerEvent::Up {
                button: PointerButton::Left,
                pos,
                ..
            }) if ctx.rect.contains(*pos) => {
                let panel = self.panel;
                self.model.update(|m| {
                    m.apply(DockOp::Activate { panel });
                });
            }
            _ => {}
        }
    }

    fn paint_self(&self, ctx: &mut PaintCtx) {
        let bg = if self.active {
            Color::rgb8(0x0D, 0x99, 0xFF).with_alpha(0.85)
        } else {
            Color::rgb8(0x3A, 0x3A, 0x3E).with_alpha(0.9)
        };
        ctx.paint_box(crate::BoxStyle::new().background(bg).radius(4.0));
    }
}

#[derive(Clone)]
pub struct DockSlot {
    model: Signal<DockModel>,
    target: PanelId,
    content: AnyView,
    edge: f32,
    preview_frac: f32,
}

impl DockSlot {
    pub fn target(&self) -> PanelId {
        self.target
    }

    pub(crate) fn drop_for(&self, pos: DropPosition) -> DockDrop {
        drop_from_fraction_or_float(
            pos.fraction,
            self.edge,
            pos.modifiers.contains(Modifiers::ALT),
        )
    }
}

impl View for DockSlot {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn children_override(&self) -> Option<Vec<AnyView>> {
        Some(vec![self.content.clone()])
    }

    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fill(1.0),
            height: Length::Fill(1.0),
            ..LayoutStyle::default()
        }
    }

    fn has_visual_state(&self) -> bool {
        true
    }

    fn accepts_drop(&self, payload: &crate::dnd::DragPayload) -> bool {
        payload.is::<PanelPayload>()
    }

    fn on_drag_over(&self, _pos: DropPosition, _ctx: &mut EventCtx) {}

    fn on_drop(&self, pos: DropPosition, ctx: &mut EventCtx) {
        let Some(payload) = ctx
            .drag_session()
            .and_then(|s| s.payload.get::<PanelPayload>().copied())
        else {
            return;
        };
        let panel = payload.0;
        if panel == self.target {
            return;
        }
        let drop = self.drop_for(pos);
        match drop {
            DockDrop::Float => {
                self.model.update(|m| {
                    m.apply(DockOp::Float {
                        panel,
                        x: pos.global.x,
                        y: pos.global.y,
                        w: 320.0,
                        h: 240.0,
                    });
                });
            }
            _ => {
                let target = self.target;
                let frac = self.preview_frac;
                self.model.update(|m| {
                    m.apply(DockOp::Insert {
                        panel,
                        target,
                        drop,
                        frac,
                        moving: true,
                    });
                });
            }
        }
        ctx.handled();
    }

    fn paint_self(&self, ctx: &mut PaintCtx) {
        if !ctx.is_drag_target() {
            return;
        }
        let Some(session) = ctx.drag_session() else {
            return;
        };
        if !session.payload.is::<PanelPayload>() {
            return;
        }
        let pos = DropPosition::in_rect(session.current, ctx.rect, session.modifiers);
        let drop = drop_from_fraction_or_float(
            pos.fraction,
            self.edge,
            pos.modifiers.contains(Modifiers::ALT),
        );
        let rect = preview_rect(ctx.rect, drop, self.preview_frac);
        ctx.quad(rect, PREVIEW_FILL.with_alpha(0.35), 6.0);

        ctx.paint_box_in(
            crate::BoxStyle::new()
                .border(1.5, PREVIEW_FILL.with_alpha(0.9))
                .radius(6.0),
            ctx.rect,
        );
    }
}
