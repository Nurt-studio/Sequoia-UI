use std::rc::Rc;

use sequoia_core::{Color, Length, Point, Rect, Size};
use sequoia_layout::LayoutStyle;

use crate::any::AnyView;
use crate::dnd::{DragPayload, DropPosition};
use crate::view::{EventCtx, PaintCtx, View};
use crate::views::{Column, Row, Unit};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EdgeSide {
    Left,
    Right,
    Top,
    Bottom,
}

impl EdgeSide {
    pub const ALL: [EdgeSide; 4] = [
        EdgeSide::Left,
        EdgeSide::Right,
        EdgeSide::Top,
        EdgeSide::Bottom,
    ];

    pub fn is_vertical(self) -> bool {
        matches!(self, EdgeSide::Left | EdgeSide::Right)
    }

    pub fn is_horizontal(self) -> bool {
        !self.is_vertical()
    }
}

pub const EDGE_HINT_PX: f32 = 28.0;

pub const EDGE_GHOST_PX: f32 = 72.0;

pub const EDGE_LINE_PX: f32 = 3.0;

pub const UNDOCK_THRESHOLD_PX: f32 = 6.0;

pub const MIN_VISIBLE_W: f32 = 120.0;
pub const MIN_VISIBLE_H: f32 = 34.0;

pub fn edge_hint_at(pos: Point, window: Size, threshold: f32) -> Option<EdgeSide> {
    let t = if threshold.is_finite() && threshold > 0.0 {
        threshold
    } else {
        EDGE_HINT_PX
    };
    if pos.x < t {
        Some(EdgeSide::Left)
    } else if pos.x > window.width - t {
        Some(EdgeSide::Right)
    } else if pos.y < t {
        Some(EdgeSide::Top)
    } else if pos.y > window.height - t {
        Some(EdgeSide::Bottom)
    } else {
        None
    }
}

pub fn edge_ghost_rect(side: EdgeSide, window: Size, thickness: f32) -> Rect {
    let t = if thickness.is_finite() && thickness > 0.0 {
        thickness
    } else {
        EDGE_GHOST_PX
    };
    match side {
        EdgeSide::Left => Rect::from_xywh(0.0, 0.0, t, window.height),
        EdgeSide::Right => Rect::from_xywh((window.width - t).max(0.0), 0.0, t, window.height),
        EdgeSide::Top => Rect::from_xywh(0.0, 0.0, window.width, t),
        EdgeSide::Bottom => Rect::from_xywh(0.0, (window.height - t).max(0.0), window.width, t),
    }
}

pub fn clamp_card_top_left(global: Point, grab_offset: (f32, f32), window: Size) -> (f32, f32) {
    let nx = global.x - grab_offset.0;
    let ny = global.y - grab_offset.1;
    let max_x = (window.width - MIN_VISIBLE_W).max(0.0);
    let max_y = (window.height - MIN_VISIBLE_H).max(0.0);
    (nx.clamp(0.0, max_x), ny.clamp(0.0, max_y))
}

pub fn exceeded_undock(origin: Point, current: Point) -> bool {
    let dx = current.x - origin.x;
    let dy = current.y - origin.y;
    dx * dx + dy * dy >= UNDOCK_THRESHOLD_PX * UNDOCK_THRESHOLD_PX
}

pub fn main_first_index(side: EdgeSide) -> bool {
    matches!(side, EdgeSide::Right | EdgeSide::Bottom)
}

#[derive(Clone)]
pub struct EdgeGhost {
    side: EdgeSide,
    color: Color,
}

impl EdgeGhost {
    pub fn new(side: EdgeSide) -> Self {
        Self {
            side,

            color: Color::rgb8(0xFF, 0xFF, 0xFF),
        }
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

impl View for EdgeGhost {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn children_override(&self) -> Option<Vec<AnyView>> {
        let line_color = self.color;
        let bar: AnyView = match self.side {
            EdgeSide::Left => AnyView::new(
                Row::new()
                    .fill_size()
                    .child(
                        crate::views::Panel::new()
                            .background(line_color)
                            .width(EDGE_LINE_PX)
                            .fill_height(),
                    )
                    .child_flex(Unit, 1.0),
            ),
            EdgeSide::Right => AnyView::new(
                Row::new().fill_size().child_flex(Unit, 1.0).child(
                    crate::views::Panel::new()
                        .background(line_color)
                        .width(EDGE_LINE_PX)
                        .fill_height(),
                ),
            ),
            EdgeSide::Top => AnyView::new(
                Column::new()
                    .fill_size()
                    .child(
                        crate::views::Panel::new()
                            .background(line_color)
                            .fill_width()
                            .height(EDGE_LINE_PX),
                    )
                    .child_flex(Unit, 1.0),
            ),
            EdgeSide::Bottom => AnyView::new(
                Column::new().fill_size().child_flex(Unit, 1.0).child(
                    crate::views::Panel::new()
                        .background(line_color)
                        .fill_width()
                        .height(EDGE_LINE_PX),
                ),
            ),
        };

        Some(vec![bar])
    }

    fn layout_style(&self) -> LayoutStyle {
        let wide = Length::Px(EDGE_GHOST_PX);
        let full = Length::Fill(1.0);
        let (w, h) = match self.side {
            EdgeSide::Left | EdgeSide::Right => (wide, full),
            EdgeSide::Top | EdgeSide::Bottom => (full, wide),
        };
        LayoutStyle {
            width: w,
            height: h,
            ..LayoutStyle::default()
        }
    }

    fn hit_testable(&self) -> bool {
        false
    }
}

pub struct EdgeSlab {
    side: EdgeSide,
    accepts: Rc<dyn Fn(&DragPayload) -> bool>,
    on_over: Rc<dyn Fn()>,
    on_drop: Rc<dyn Fn(DropPosition, &mut EventCtx)>,
}

impl Clone for EdgeSlab {
    fn clone(&self) -> Self {
        Self {
            side: self.side,
            accepts: self.accepts.clone(),
            on_over: self.on_over.clone(),
            on_drop: self.on_drop.clone(),
        }
    }
}

impl EdgeSlab {
    pub fn new(side: EdgeSide) -> Self {
        Self {
            side,
            accepts: Rc::new(|_| false),
            on_over: Rc::new(|| {}),
            on_drop: Rc::new(|_, _| {}),
        }
    }

    pub fn get_side(&self) -> EdgeSide {
        self.side
    }

    pub fn accepts(mut self, f: impl Fn(&DragPayload) -> bool + 'static) -> Self {
        self.accepts = Rc::new(f);
        self
    }

    pub fn on_drag_over(mut self, f: impl Fn() + 'static) -> Self {
        self.on_over = Rc::new(f);
        self
    }

    pub fn on_drop_with(mut self, f: impl Fn(DropPosition, &mut EventCtx) + 'static) -> Self {
        self.on_drop = Rc::new(f);
        self
    }
}

impl View for EdgeSlab {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn children_override(&self) -> Option<Vec<AnyView>> {
        None
    }

    fn layout_style(&self) -> LayoutStyle {
        let wide = Length::Px(EDGE_GHOST_PX);
        let full = Length::Fill(1.0);
        let (w, h) = match self.side {
            EdgeSide::Left | EdgeSide::Right => (wide, full),
            EdgeSide::Top | EdgeSide::Bottom => (full, wide),
        };
        LayoutStyle {
            width: w,
            height: h,
            ..LayoutStyle::default()
        }
    }

    fn accepts_drop(&self, payload: &DragPayload) -> bool {
        (self.accepts)(payload)
    }

    fn on_drag_over(&self, _pos: DropPosition, _ctx: &mut EventCtx) {
        (self.on_over)();
    }

    fn on_drop(&self, pos: DropPosition, ctx: &mut EventCtx) {
        (self.on_drop)(pos, ctx);
    }

    fn paint_self(&self, _ctx: &mut PaintCtx) {}
}

#[allow(dead_code)]
fn _assert_send_sync() {
    fn is<T: Clone + 'static>() {}
    is::<EdgeSide>();
    is::<EdgeGhost>();
}
