use std::cell::Cell;

use sequoia_core::{Point, Rect, Size};

use crate::any::AnyView;
use crate::runtime::ElementId;
use crate::view::View;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct OverlayId(pub(crate) u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum OverlayLayer {
    Tooltip,

    Popup,

    Menu,

    DragPreview,

    Modal,

    System,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum InputMode {
    Passthrough,

    #[default]
    Interactive,

    Modal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum Dismiss {
    #[default]
    Manual,

    Escape,

    OutsideClick,

    FocusLost,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum OverlayOverflow {
    #[default]
    FlipAndClamp,

    Clamp,

    Visible,
}

#[derive(Clone, Copy, Debug, Default)]
#[non_exhaustive]
pub enum Anchor {
    View(ElementId),

    Rect(Rect),

    Point(Point),

    Cursor,

    #[default]
    Viewport,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Position {
    #[default]
    Absolute,

    Relative,

    Fixed,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Placement {
    Top,

    TopStart,

    TopEnd,

    #[default]
    Bottom,

    BottomStart,

    BottomEnd,

    Left,

    Right,

    Center,

    Cursor,
}

pub struct OverlayEntry {
    pub(crate) id: OverlayId,
    pub(crate) view: AnyView,
    pub(crate) anchor: Anchor,
    pub(crate) placement: Placement,
    pub(crate) position: Position,
    pub(crate) overflow: OverlayOverflow,
    pub(crate) layer: OverlayLayer,
    pub(crate) input_mode: InputMode,
    pub(crate) dismiss: Dismiss,

    pub(crate) transient: bool,
    pub(crate) offset: (f32, f32),
    pub(crate) element: Cell<Option<ElementId>>,
    pub(crate) rect: Cell<Rect>,
}

impl OverlayEntry {
    pub fn new(view: impl View + 'static) -> Self {
        Self {
            id: OverlayId(0),
            view: AnyView::new(view),
            anchor: Anchor::default(),
            placement: Placement::default(),
            position: Position::default(),
            overflow: OverlayOverflow::default(),
            layer: OverlayLayer::Popup,
            input_mode: InputMode::default(),
            dismiss: Dismiss::default(),
            transient: false,
            offset: (0.0, 0.0),
            element: Cell::new(None),
            rect: Cell::new(Rect::ZERO),
        }
    }

    pub fn anchor(mut self, anchor: Anchor) -> Self {
        self.anchor = anchor;
        self
    }

    pub fn placement(mut self, placement: Placement) -> Self {
        self.placement = placement;
        self
    }

    pub fn position(mut self, position: Position) -> Self {
        self.position = position;
        self
    }

    pub fn at(mut self, x: f32, y: f32) -> Self {
        self.position = Position::Fixed;
        self.offset = (x, y);
        self
    }

    pub fn layer(mut self, layer: OverlayLayer) -> Self {
        self.layer = layer;
        self
    }

    pub fn input_mode(mut self, mode: InputMode) -> Self {
        self.input_mode = mode;
        self
    }

    pub fn dismiss(mut self, dismiss: Dismiss) -> Self {
        self.dismiss = dismiss;
        self
    }

    pub fn transient(mut self, transient: bool) -> Self {
        self.transient = transient;
        self
    }

    pub fn offset(mut self, dx: f32, dy: f32) -> Self {
        self.offset = (dx, dy);
        self
    }

    pub fn overflow(mut self, overflow: OverlayOverflow) -> Self {
        self.overflow = overflow;
        self
    }
}

#[derive(Default)]
pub(crate) struct OverlayStore {
    entries: Vec<OverlayEntry>,
    next_id: u64,
    anchor_index: std::collections::HashMap<crate::runtime::ElementId, Vec<OverlayId>>,
}

impl OverlayStore {
    pub(crate) fn insert(&mut self, mut entry: OverlayEntry) -> OverlayId {
        let id = OverlayId(self.next_id);
        self.next_id += 1;
        entry.id = id;
        if let Anchor::View(a) = entry.anchor {
            self.anchor_index.entry(a).or_default().push(id);
        }
        self.entries.push(entry);
        id
    }

    pub(crate) fn get(&self, id: OverlayId) -> Option<&OverlayEntry> {
        self.entries.iter().find(|e| e.id == id)
    }

    pub(crate) fn remove(&mut self, id: OverlayId) -> Option<OverlayEntry> {
        let pos = self.entries.iter().position(|e| e.id == id)?;
        let entry = self.entries.remove(pos);
        if let Anchor::View(a) = entry.anchor
            && let Some(v) = self.anchor_index.get_mut(&a)
        {
            v.retain(|&x| x != id);
            if v.is_empty() {
                self.anchor_index.remove(&a);
            }
        }
        Some(entry)
    }

    pub(crate) fn ordered_ids(&self) -> Vec<OverlayId> {
        let mut ids: Vec<(OverlayLayer, usize, OverlayId)> = self
            .entries
            .iter()
            .enumerate()
            .map(|(i, e)| (e.layer, i, e.id))
            .collect();
        ids.sort_by_key(|(l, i, _)| (*l, *i));
        ids.into_iter().map(|(_, _, id)| id).collect()
    }

    pub(crate) fn transient_ids(&self) -> Vec<OverlayId> {
        self.entries
            .iter()
            .filter(|e| e.transient)
            .map(|e| e.id)
            .collect()
    }

    pub(crate) fn anchored_to(&self, el: crate::runtime::ElementId) -> Vec<OverlayId> {
        if let Some(v) = self.anchor_index.get(&el) {
            return v.clone();
        }
        self.entries
            .iter()
            .filter(|e| matches!(e.anchor, Anchor::View(a) if a == el))
            .map(|e| e.id)
            .collect()
    }

    pub(crate) fn outside_dismiss_ids(&self, pos: Point) -> Vec<OverlayId> {
        self.entries
            .iter()
            .filter(|e| e.dismiss == Dismiss::OutsideClick && !e.rect.get().contains(pos))
            .map(|e| e.id)
            .collect()
    }

    pub(crate) fn escape_dismiss_id(&self) -> Option<OverlayId> {
        self.ordered_ids()
            .into_iter()
            .rev()
            .find(|id| self.get(*id).is_some_and(|e| e.dismiss == Dismiss::Escape))
    }

    pub(crate) fn focus_lost_ids(
        &self,
        target: Option<ElementId>,
        inside: impl Fn(Option<ElementId>) -> bool,
    ) -> Vec<OverlayId> {
        self.entries
            .iter()
            .filter(|e| {
                if e.dismiss != Dismiss::FocusLost {
                    return false;
                }
                match target {
                    None => true,
                    Some(_) => !inside(e.element.get()),
                }
            })
            .map(|e| e.id)
            .collect()
    }

    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }

    pub(crate) fn set_offset(&mut self, id: OverlayId, offset: (f32, f32)) -> bool {
        match self.entries.iter_mut().find(|e| e.id == id) {
            Some(e) => {
                e.offset = offset;
                true
            }
            None => false,
        }
    }
}

pub(crate) fn compute_position(
    position: Position,
    placement: Placement,
    anchor: Rect,
    size: Size,
    viewport: Rect,
    offset: (f32, f32),
    overflow: OverlayOverflow,
) -> Point {
    match position {
        Position::Absolute => {
            placement_position(placement, anchor, size, viewport, offset, overflow)
        }
        Position::Relative => clamp_to_viewport(
            Point::new(anchor.left() + offset.0, anchor.top() + offset.1),
            size,
            viewport,
        ),
        Position::Fixed => {
            let pos = Point::new(viewport.left() + offset.0, viewport.top() + offset.1);
            match overflow {
                OverlayOverflow::Visible => pos,
                _ => clamp_to_viewport(pos, size, viewport),
            }
        }
    }
}

fn clamp_to_viewport(pos: Point, size: Size, viewport: Rect) -> Point {
    let max_x = (viewport.right() - size.width).max(viewport.left());
    let max_y = (viewport.bottom() - size.height).max(viewport.top());
    Point::new(
        pos.x.clamp(viewport.left(), max_x),
        pos.y.clamp(viewport.top(), max_y),
    )
}

pub(crate) fn placement_position(
    placement: Placement,
    anchor: Rect,
    size: Size,
    viewport: Rect,
    offset: (f32, f32),
    overflow: OverlayOverflow,
) -> Point {
    let placement = match overflow {
        OverlayOverflow::FlipAndClamp => flip_if_needed(placement, anchor, size, viewport),
        _ => placement,
    };
    let (ox, oy) = offset;
    let (mut x, mut y) = match placement {
        Placement::Top => (
            anchor.left() + (anchor.size.width - size.width) * 0.5,
            anchor.top() - size.height,
        ),
        Placement::TopStart => (anchor.left(), anchor.top() - size.height),
        Placement::TopEnd => (anchor.right() - size.width, anchor.top() - size.height),
        Placement::Bottom => (
            anchor.left() + (anchor.size.width - size.width) * 0.5,
            anchor.bottom(),
        ),
        Placement::BottomStart => (anchor.left(), anchor.bottom()),
        Placement::BottomEnd => (anchor.right() - size.width, anchor.bottom()),
        Placement::Left => (
            anchor.left() - size.width,
            anchor.top() + (anchor.size.height - size.height) * 0.5,
        ),
        Placement::Right => (
            anchor.right(),
            anchor.top() + (anchor.size.height - size.height) * 0.5,
        ),
        Placement::Center => (
            anchor.left() + (anchor.size.width - size.width) * 0.5,
            anchor.top() + (anchor.size.height - size.height) * 0.5,
        ),
        Placement::Cursor => (anchor.left(), anchor.top()),
    };
    x += ox;
    y += oy;

    match overflow {
        OverlayOverflow::Visible => Point::new(x, y),
        _ => clamp_to_viewport(Point::new(x, y), size, viewport),
    }
}

fn flip_if_needed(placement: Placement, anchor: Rect, size: Size, viewport: Rect) -> Placement {
    match placement {
        Placement::Bottom | Placement::BottomStart | Placement::BottomEnd => {
            let fits_below = anchor.bottom() + size.height <= viewport.bottom();
            let fits_above =
                anchor.top() - size.height >= viewport.top() && size.height <= viewport.size.height;
            if !fits_below && fits_above {
                match placement {
                    Placement::BottomStart => Placement::TopStart,
                    Placement::BottomEnd => Placement::TopEnd,
                    _ => Placement::Top,
                }
            } else {
                placement
            }
        }
        Placement::Top | Placement::TopStart | Placement::TopEnd => {
            let fits_above = anchor.top() - size.height >= viewport.top();
            let fits_below = anchor.bottom() + size.height <= viewport.bottom()
                && size.height <= viewport.size.height;
            if !fits_above && fits_below {
                match placement {
                    Placement::TopStart => Placement::BottomStart,
                    Placement::TopEnd => Placement::BottomEnd,
                    _ => Placement::Bottom,
                }
            } else {
                placement
            }
        }
        Placement::Right => {
            let fits_right = anchor.right() + size.width <= viewport.right();
            let fits_left =
                anchor.left() - size.width >= viewport.left() && size.width <= viewport.size.width;
            if !fits_right && fits_left {
                Placement::Left
            } else {
                placement
            }
        }
        Placement::Left => {
            let fits_left = anchor.left() - size.width >= viewport.left();
            let fits_right = anchor.right() + size.width <= viewport.right()
                && size.width <= viewport.size.width;
            if !fits_left && fits_right {
                Placement::Right
            } else {
                placement
            }
        }
        Placement::Center | Placement::Cursor => placement,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn viewport() -> Rect {
        Rect::from_xywh(0.0, 0.0, 800.0, 600.0)
    }

    #[test]
    fn bottom_flips_to_top_when_it_does_not_fit() {
        let anchor = Rect::from_xywh(100.0, 500.0, 120.0, 32.0);
        let size = Size::new(200.0, 200.0);
        let p = placement_position(
            Placement::Bottom,
            anchor,
            size,
            viewport(),
            (0.0, 4.0),
            OverlayOverflow::FlipAndClamp,
        );
        assert!(
            p.y + size.height <= anchor.top() + 5.0,
            "flipped above the anchor (plus 4px offset): {p:?}"
        );
    }

    #[test]
    fn clamp_keeps_placement_but_trims_to_viewport() {
        let anchor = Rect::from_xywh(100.0, 500.0, 120.0, 32.0);
        let size = Size::new(200.0, 200.0);
        let p = placement_position(
            Placement::Bottom,
            anchor,
            size,
            viewport(),
            (0.0, 4.0),
            OverlayOverflow::Clamp,
        );

        assert!(
            (p.y - 400.0).abs() < 1e-4,
            "clamped to viewport without flip: {p:?}"
        );
        assert!(
            p.y + size.height <= viewport().bottom() + 1.0,
            "fits vertically: {p:?}"
        );
    }

    #[test]
    fn visible_may_bleed_outside_viewport() {
        let anchor = Rect::from_xywh(100.0, 500.0, 120.0, 32.0);
        let size = Size::new(200.0, 200.0);
        let p = placement_position(
            Placement::Bottom,
            anchor,
            size,
            viewport(),
            (0.0, 4.0),
            OverlayOverflow::Visible,
        );
        assert!(
            (p.y - (anchor.bottom() + 4.0)).abs() < 1e-4,
            "exactly under the anchor, no fitting: {p:?}"
        );
        assert!(p.y + size.height > viewport().bottom(), "sticks out");
    }

    #[test]
    fn fitting_popup_is_unaffected_by_policy() {
        let anchor = Rect::from_xywh(100.0, 100.0, 120.0, 32.0);
        let size = Size::new(200.0, 120.0);
        for policy in [
            OverlayOverflow::FlipAndClamp,
            OverlayOverflow::Clamp,
            OverlayOverflow::Visible,
        ] {
            let p = placement_position(
                Placement::Bottom,
                anchor,
                size,
                viewport(),
                (0.0, 4.0),
                policy,
            );
            assert!(
                (p.y - (anchor.bottom() + 4.0)).abs() < 1e-4,
                "{policy:?}: unchanged when it fits: {p:?}"
            );
        }
    }
}
