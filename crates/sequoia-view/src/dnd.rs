use std::any::{Any, TypeId};
use std::fmt;
use std::rc::Rc;

use sequoia_core::{Point, Rect, Vec2};
use sequoia_input::Modifiers;
use sequoia_layout::Axis;

use crate::any::AnyView;
use crate::overlay::OverlayId;
use crate::runtime::ElementId;

pub const DRAG_THRESHOLD: f32 = 4.0;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct DragId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DragEnd {
    pub dropped: bool,

    pub cancelled: bool,
}

#[derive(Clone)]
pub struct DragPayload {
    type_id: TypeId,
    value: Rc<dyn Any>,
    type_name: &'static str,
}

impl DragPayload {
    pub fn new<T: 'static>(value: T) -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            value: Rc::new(value),
            type_name: std::any::type_name::<T>(),
        }
    }

    pub fn get<T: 'static>(&self) -> Option<&T> {
        self.value.downcast_ref::<T>()
    }

    pub fn is<T: 'static>(&self) -> bool {
        self.type_id == TypeId::of::<T>()
    }

    pub fn type_id(&self) -> TypeId {
        self.type_id
    }

    pub fn type_name(&self) -> &'static str {
        self.type_name
    }
}

impl fmt::Debug for DragPayload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DragPayload")
            .field("type", &self.type_name)
            .finish()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DropPosition {
    pub global: Point,

    pub target_rect: Rect,

    pub local: Point,

    pub fraction: Vec2,

    pub modifiers: Modifiers,
}

impl DropPosition {
    pub fn in_rect(global: Point, target_rect: Rect, modifiers: Modifiers) -> Self {
        let local = Point::new(global.x - target_rect.left(), global.y - target_rect.top());
        let fraction = Vec2::new(
            if target_rect.size.width > 0.0 {
                (local.x / target_rect.size.width).clamp(0.0, 1.0)
            } else {
                0.0
            },
            if target_rect.size.height > 0.0 {
                (local.y / target_rect.size.height).clamp(0.0, 1.0)
            } else {
                0.0
            },
        );
        Self {
            global,
            target_rect,
            local: Point::new(local.x, local.y),
            fraction,
            modifiers,
        }
    }

    pub fn insertion_index(&self, count: usize, axis: Axis) -> usize {
        insertion_index(self.global, self.target_rect, count, axis)
    }

    pub fn before_midpoint(&self, axis: Axis) -> bool {
        match axis {
            Axis::Horizontal => self.fraction.x < 0.5,
            Axis::Vertical => self.fraction.y < 0.5,
        }
    }
}

#[derive(Clone, Debug)]
pub struct DragSession {
    pub id: DragId,

    pub payload: DragPayload,

    pub source: Option<ElementId>,

    pub origin: Point,

    pub grab_offset: Vec2,

    pub current: Point,

    pub target: Option<ElementId>,

    pub modifiers: Modifiers,

    pub preview: Option<OverlayId>,
}

impl DragSession {
    pub fn position_in(&self, rect: Rect) -> DropPosition {
        DropPosition::in_rect(self.current, rect, self.modifiers)
    }
}

pub struct DragOptions {
    pub(crate) threshold: f32,
    pub(crate) preview: Option<AnyView>,
}

impl Default for DragOptions {
    fn default() -> Self {
        Self {
            threshold: DRAG_THRESHOLD,
            preview: None,
        }
    }
}

impl DragOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn threshold(mut self, px: f32) -> Self {
        self.threshold = px.max(0.0);
        self
    }

    pub fn preview(mut self, view: impl crate::view::View + 'static) -> Self {
        self.preview = Some(AnyView::new(view));
        self
    }
}

pub(crate) struct PendingDrag {
    pub(crate) source: ElementId,

    pub(crate) origin: Point,

    pub(crate) source_rect: Rect,
    pub(crate) payload: DragPayload,
    pub(crate) preview: Option<AnyView>,
    pub(crate) threshold: f32,
}

pub(crate) struct ActiveDrag {
    pub(crate) id: DragId,
    pub(crate) payload: DragPayload,

    pub(crate) source: Option<ElementId>,
    pub(crate) origin: Point,
    pub(crate) grab_offset: Vec2,
    pub(crate) current: Point,
    pub(crate) modifiers: Modifiers,
    pub(crate) target: Option<ElementId>,
    pub(crate) preview: Option<OverlayId>,
}

impl ActiveDrag {
    pub(crate) fn snapshot(&self) -> DragSession {
        DragSession {
            id: self.id,
            payload: self.payload.clone(),
            source: self.source,
            origin: self.origin,
            grab_offset: self.grab_offset,
            current: self.current,
            target: self.target,
            modifiers: self.modifiers,
            preview: self.preview,
        }
    }
}

pub fn exceeded_threshold(origin: Point, current: Point, threshold: f32) -> bool {
    let d = current - origin;
    if !d.x.is_finite() || !d.y.is_finite() || !threshold.is_finite() {
        return false;
    }
    d.length() >= threshold.max(0.0)
}

pub fn grab_offset(origin: Point, source_rect: Rect) -> Vec2 {
    origin - source_rect.origin
}

pub fn preview_origin(current: Point, grab: Vec2) -> Point {
    current - grab
}

pub fn insertion_index(pos: Point, rect: Rect, count: usize, axis: Axis) -> usize {
    if count == 0 {
        return 0;
    }
    let (start, extent, p) = match axis {
        Axis::Horizontal => (rect.left(), rect.size.width, pos.x),
        Axis::Vertical => (rect.top(), rect.size.height, pos.y),
    };
    if !(extent.is_finite() && extent > 0.0) || !p.is_finite() {
        return 0;
    }
    let t = ((p - start) / extent * count as f32).floor() as i64;
    t.clamp(0, count as i64) as usize
}

pub fn move_item<T>(items: &mut Vec<T>, from: usize, slot: usize) {
    if from >= items.len() {
        return;
    }
    let item = items.remove(from);

    let mut to = if slot > from { slot - 1 } else { slot };
    to = to.min(items.len());
    items.insert(to, item);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect::from_xywh(x, y, w, h)
    }

    #[test]
    fn threshold_is_euclidean() {
        let o = Point::ZERO;
        assert!(exceeded_threshold(o, Point::new(3.0, 4.0), 5.0));
        assert!(!exceeded_threshold(o, Point::new(3.0, 4.0), 5.1));
        assert!(exceeded_threshold(o, Point::new(-4.0, 0.0), 4.0));
        assert!(!exceeded_threshold(o, Point::new(1.0, 1.0), 4.0));
        assert!(!exceeded_threshold(o, Point::new(f32::NAN, 0.0), 0.0));
    }

    #[test]
    fn grab_keeps_preview_under_cursor() {
        let src = rect(100.0, 50.0, 60.0, 20.0);
        let origin = Point::new(115.0, 58.0);
        let grab = grab_offset(origin, src);
        assert_eq!(grab, Vec2::new(15.0, 8.0));

        let cur = Point::new(200.0, 100.0);
        let pv = preview_origin(cur, grab);
        assert_eq!(pv + grab, cur);
    }

    #[test]
    fn drop_position_local_and_fraction() {
        let r = rect(10.0, 20.0, 100.0, 40.0);
        let p = DropPosition::in_rect(Point::new(35.0, 30.0), r, Modifiers::NONE);
        assert_eq!(p.local, Point::new(25.0, 10.0));
        assert!((p.fraction.x - 0.25).abs() < 1e-6);
        assert!((p.fraction.y - 0.25).abs() < 1e-6);

        let outside = DropPosition::in_rect(Point::new(500.0, -50.0), r, Modifiers::NONE);
        assert_eq!(outside.fraction.x, 1.0);
        assert_eq!(outside.fraction.y, 0.0);

        let zero = DropPosition::in_rect(Point::ZERO, Rect::ZERO, Modifiers::NONE);
        assert_eq!(zero.fraction, Vec2::ZERO);
    }

    #[test]
    fn insertion_slots() {
        let r = rect(0.0, 0.0, 100.0, 400.0);
        assert_eq!(
            insertion_index(Point::new(50.0, -10.0), r, 4, Axis::Vertical),
            0
        );
        assert_eq!(
            insertion_index(Point::new(50.0, 50.0), r, 4, Axis::Vertical),
            0
        );
        assert_eq!(
            insertion_index(Point::new(50.0, 150.0), r, 4, Axis::Vertical),
            1
        );
        assert_eq!(
            insertion_index(Point::new(50.0, 350.0), r, 4, Axis::Vertical),
            3
        );
        assert_eq!(
            insertion_index(Point::new(50.0, 500.0), r, 4, Axis::Vertical),
            4
        );
        assert_eq!(insertion_index(Point::ZERO, r, 0, Axis::Vertical), 0);
        let h = rect(0.0, 0.0, 400.0, 20.0);
        assert_eq!(
            insertion_index(Point::new(250.0, 10.0), h, 4, Axis::Horizontal),
            2
        );
    }

    #[test]
    fn move_item_semantics() {
        let mut v = vec!['A', 'B', 'C', 'D'];
        move_item(&mut v, 0, 3);
        assert_eq!(v, vec!['B', 'C', 'A', 'D']);
        let mut v = vec!['A', 'B', 'C', 'D'];
        move_item(&mut v, 3, 1);
        assert_eq!(v, vec!['A', 'D', 'B', 'C']);
        let mut v = vec!['A', 'B', 'C', 'D'];
        move_item(&mut v, 1, 1);
        assert_eq!(v, vec!['A', 'B', 'C', 'D']);
        let mut v = vec!['A', 'B', 'C', 'D'];
        move_item(&mut v, 0, 4);
        assert_eq!(v, vec!['B', 'C', 'D', 'A']);
        let mut v = vec!['A'];
        move_item(&mut v, 5, 0);
        assert_eq!(v, vec!['A']);
    }

    #[test]
    fn payload_typed_access() {
        let p = DragPayload::new(42u32);
        assert!(p.is::<u32>());
        assert!(!p.is::<String>());
        assert_eq!(p.get::<u32>(), Some(&42u32));
        assert_eq!(p.get::<String>(), None);
    }
}
