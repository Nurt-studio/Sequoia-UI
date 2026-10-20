use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use sequoia_core::{Length, Rect};
use sequoia_input::{Event, Key};
use sequoia_layout::LayoutStyle;
use sequoia_reactive::Signal;

use crate::any::AnyView;
use crate::overlay::{
    Anchor, Dismiss, InputMode, OverlayEntry, OverlayId, OverlayLayer, OverlayOverflow,
};
use crate::view::{EventCtx, PaintCtx, View};
use crate::views::Unit;

thread_local! {
    static PORTAL_ENTRIES: RefCell<HashMap<usize, (OverlayId, Option<crate::runtime::ElementId>)>> = RefCell::new(HashMap::new());
    static POPPER_ENTRIES: RefCell<HashMap<usize, (OverlayId, Option<crate::runtime::ElementId>)>> = RefCell::new(HashMap::new());
}

fn get_portal_entry(key: usize) -> Option<(OverlayId, Option<crate::runtime::ElementId>)> {
    PORTAL_ENTRIES.with(|m| m.borrow().get(&key).copied())
}

fn insert_portal_entry(key: usize, id: OverlayId, anchor: Option<crate::runtime::ElementId>) {
    PORTAL_ENTRIES.with(|m| {
        m.borrow_mut().insert(key, (id, anchor));
    });
}

fn take_portal_entry(key: usize) -> Option<OverlayId> {
    PORTAL_ENTRIES.with(|m| m.borrow_mut().remove(&key).map(|(id, _)| id))
}

fn get_popper_entry(key: usize) -> Option<(OverlayId, Option<crate::runtime::ElementId>)> {
    POPPER_ENTRIES.with(|m| m.borrow().get(&key).copied())
}

fn insert_popper_entry(key: usize, id: OverlayId, anchor: Option<crate::runtime::ElementId>) {
    POPPER_ENTRIES.with(|m| {
        m.borrow_mut().insert(key, (id, anchor));
    });
}

fn take_popper_entry(key: usize) -> Option<OverlayId> {
    POPPER_ENTRIES.with(|m| m.borrow_mut().remove(&key).map(|(id, _)| id))
}

pub struct Portal {
    open: Signal<bool>,
    child: Option<AnyView>,
    layer: OverlayLayer,
    input_mode: InputMode,
    dismiss: Dismiss,
    anchor: Anchor,
    anchor_self: bool,
    placement: crate::overlay::Placement,
    offset: (f32, f32),
    overflow: OverlayOverflow,
    on_dismiss: Option<Rc<dyn Fn()>>,
}

impl Portal {
    pub fn new(open: Signal<bool>) -> Self {
        Self {
            open,
            child: None,
            layer: OverlayLayer::Popup,
            input_mode: InputMode::Interactive,
            dismiss: Dismiss::Manual,
            anchor: Anchor::Viewport,
            anchor_self: false,
            placement: crate::overlay::Placement::Center,
            offset: (0.0, 0.0),
            overflow: OverlayOverflow::FlipAndClamp,
            on_dismiss: None,
        }
    }

    pub fn child<V: View + 'static>(mut self, child: V) -> Self {
        self.child = Some(AnyView::new(child));
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

    pub fn anchor(mut self, anchor: Anchor) -> Self {
        self.anchor = anchor;
        self.anchor_self = false;
        self
    }

    pub fn anchor_self(mut self) -> Self {
        self.anchor_self = true;
        self
    }

    pub fn placement(mut self, placement: crate::overlay::Placement) -> Self {
        self.placement = placement;
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

    pub fn on_dismiss(mut self, f: impl Fn() + 'static) -> Self {
        self.on_dismiss = Some(Rc::new(f));
        self
    }

    pub fn is_open(&self) -> bool {
        self.open.get_untracked()
    }

    pub fn open(&self) {
        self.open.set(true);
    }

    pub fn close(&self) {
        self.open.set(false);
    }

    pub fn toggle(&self) {
        self.open.set(!self.open.get_untracked());
    }

    fn key(&self) -> usize {
        self.open.id()
    }

    fn sync_overlay(&self, ctx: &mut PaintCtx) {
        let key = self.key();
        if !self.open.get_untracked() {
            if let Some(id) = take_portal_entry(key) {
                ctx.hide_overlay(id);
            }
            return;
        }
        let self_anchor = if self.anchor_self {
            Some(ctx.element)
        } else {
            None
        };
        if let Some((id, anchor)) = get_portal_entry(key) {
            if anchor != self_anchor {
                if ctx.has_overlay(id) {
                    ctx.hide_overlay(id);
                }
                take_portal_entry(key);
            } else {
                if !ctx.has_overlay(id) {
                    take_portal_entry(key);
                    self.open.set(false);
                    if let Some(f) = &self.on_dismiss {
                        f();
                    }
                }
                return;
            }
        }
        let Some(child) = self.child.clone() else {
            self.open.set(false);
            return;
        };
        let anchor = if self.anchor_self {
            Anchor::View(ctx.element)
        } else {
            self.anchor
        };
        let id = ctx.show_overlay(
            OverlayEntry::new(PortalBody { child })
                .anchor(anchor)
                .placement(self.placement)
                .layer(self.layer)
                .input_mode(self.input_mode)
                .dismiss(self.dismiss)
                .offset(self.offset.0, self.offset.1)
                .overflow(self.overflow),
        );
        insert_portal_entry(key, id, self_anchor);
    }
}

impl View for Portal {
    fn body(&self) -> impl View + 'static {
        Unit
    }
    fn is_leaf(&self) -> bool {
        true
    }

    fn hit_testable(&self) -> bool {
        false
    }
    fn keyed_children(&self) -> Option<Vec<(u64, AnyView)>> {
        let _ = self.open.get();
        Some(Vec::new())
    }
    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fit,
            height: Length::Fit,
            ..LayoutStyle::default()
        }
    }
    fn on_unmount(&self) {
        take_portal_entry(self.key());
    }
    fn paint_self(&self, ctx: &mut PaintCtx) {
        self.sync_overlay(ctx);
    }
}

struct PortalBody {
    child: AnyView,
}

impl View for PortalBody {
    fn body(&self) -> impl View + 'static {
        Unit
    }
    fn children_override(&self) -> Option<Vec<AnyView>> {
        Some(vec![self.child.clone()])
    }
    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fit,
            height: Length::Fit,
            ..LayoutStyle::default()
        }
    }
}

pub struct Popper {
    open: Signal<bool>,
    child: Option<AnyView>,
    anchor: Anchor,
    anchor_self: bool,
    placement: crate::overlay::Placement,
    offset: (f32, f32),
    overflow: OverlayOverflow,
    layer: OverlayLayer,
    input_mode: InputMode,
    dismiss: Dismiss,
    on_dismiss: Option<Rc<dyn Fn()>>,
}

impl Popper {
    pub fn new(open: Signal<bool>) -> Self {
        Self {
            open,
            child: None,
            anchor: Anchor::Viewport,
            anchor_self: false,
            placement: crate::overlay::Placement::Bottom,
            offset: (0.0, 4.0),
            overflow: OverlayOverflow::FlipAndClamp,
            layer: OverlayLayer::Popup,
            input_mode: InputMode::Interactive,
            dismiss: Dismiss::OutsideClick,
            on_dismiss: None,
        }
    }

    pub fn child<V: View + 'static>(mut self, child: V) -> Self {
        self.child = Some(AnyView::new(child));
        self
    }

    pub fn anchor(mut self, anchor: Anchor) -> Self {
        self.anchor = anchor;
        self.anchor_self = false;
        self
    }

    pub fn anchor_rect(mut self, rect: Rect) -> Self {
        self.anchor = Anchor::Rect(rect);
        self.anchor_self = false;
        self
    }

    pub fn anchor_view(mut self, id: crate::runtime::ElementId) -> Self {
        self.anchor = Anchor::View(id);
        self.anchor_self = false;
        self
    }

    pub fn anchor_self(mut self) -> Self {
        self.anchor_self = true;
        self
    }

    pub fn at_cursor(mut self) -> Self {
        self.anchor = Anchor::Cursor;
        self.anchor_self = false;
        self.placement = crate::overlay::Placement::Cursor;
        self
    }

    pub fn placement(mut self, placement: crate::overlay::Placement) -> Self {
        self.placement = placement;
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

    pub fn on_dismiss(mut self, f: impl Fn() + 'static) -> Self {
        self.on_dismiss = Some(Rc::new(f));
        self
    }

    pub fn is_open(&self) -> bool {
        self.open.get_untracked()
    }

    pub fn open(&self) {
        self.open.set(true);
    }

    pub fn close(&self) {
        self.open.set(false);
    }

    pub fn toggle(&self) {
        self.open.set(!self.open.get_untracked());
    }

    fn key(&self) -> usize {
        self.open.id()
    }

    fn sync_overlay(&self, ctx: &mut PaintCtx) {
        let key = self.key();
        if !self.open.get_untracked() {
            if let Some(id) = take_popper_entry(key) {
                ctx.hide_overlay(id);
            }
            return;
        }
        let self_anchor = if self.anchor_self {
            Some(ctx.element)
        } else {
            None
        };
        if let Some((id, anchor)) = get_popper_entry(key) {
            if anchor != self_anchor {
                if ctx.has_overlay(id) {
                    ctx.hide_overlay(id);
                }
                take_popper_entry(key);
            } else {
                if !ctx.has_overlay(id) {
                    take_popper_entry(key);
                    self.open.set(false);
                    if let Some(f) = &self.on_dismiss {
                        f();
                    }
                }
                return;
            }
        }
        let Some(child) = self.child.clone() else {
            self.open.set(false);
            return;
        };
        let anchor = if self.anchor_self {
            Anchor::View(ctx.element)
        } else {
            self.anchor
        };
        let id = ctx.show_overlay(
            OverlayEntry::new(PortalBody { child })
                .anchor(anchor)
                .placement(self.placement)
                .layer(self.layer)
                .input_mode(self.input_mode)
                .dismiss(self.dismiss)
                .offset(self.offset.0, self.offset.1)
                .overflow(self.overflow),
        );
        insert_popper_entry(key, id, self_anchor);
    }
}

impl View for Popper {
    fn body(&self) -> impl View + 'static {
        Unit
    }
    fn is_leaf(&self) -> bool {
        true
    }

    fn hit_testable(&self) -> bool {
        false
    }
    fn keyed_children(&self) -> Option<Vec<(u64, AnyView)>> {
        let _ = self.open.get();
        Some(Vec::new())
    }
    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fit,
            height: Length::Fit,
            ..LayoutStyle::default()
        }
    }
    fn on_unmount(&self) {
        take_popper_entry(self.key());
    }
    fn paint_self(&self, ctx: &mut PaintCtx) {
        self.sync_overlay(ctx);
    }
}

pub struct DismissableLayer {
    child: AnyView,
    dismiss_on_escape: bool,
    on_dismiss: Option<Rc<dyn Fn()>>,
    on_escape: Option<Rc<dyn Fn()>>,
}

impl DismissableLayer {
    pub fn wrap<V: View + 'static>(child: V) -> Self {
        Self {
            child: AnyView::new(child),
            dismiss_on_escape: true,
            on_dismiss: None,
            on_escape: None,
        }
    }

    pub fn dismiss_on_escape(mut self, on: bool) -> Self {
        self.dismiss_on_escape = on;
        self
    }

    pub fn on_dismiss(mut self, f: impl Fn() + 'static) -> Self {
        self.on_dismiss = Some(Rc::new(f));
        self
    }

    pub fn on_escape(mut self, f: impl Fn() + 'static) -> Self {
        self.on_escape = Some(Rc::new(f));
        self
    }

    pub fn bind(self, open: Signal<bool>) -> Self {
        let prev = self.on_dismiss.clone();
        self.on_dismiss(move || {
            if let Some(f) = &prev {
                f();
            }
            open.set(false);
        })
    }

    fn fire_escape(&self, ctx: &mut EventCtx) {
        if let Some(f) = &self.on_escape {
            f();
        }
        if self.dismiss_on_escape {
            if let Some(f) = &self.on_dismiss {
                f();
            }
            ctx.handled();
        }
    }
}

impl View for DismissableLayer {
    fn body(&self) -> impl View + 'static {
        Unit
    }
    fn children_override(&self) -> Option<Vec<AnyView>> {
        Some(vec![self.child.clone()])
    }
    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fit,
            height: Length::Fit,
            ..LayoutStyle::default()
        }
    }
    fn on_event(&self, event: &Event, ctx: &mut EventCtx) {
        let Event::Key(key) = event else { return };
        if key.key != Key::ESCAPE {
            return;
        }

        if !matches!(
            ctx.phase,
            sequoia_input::EventPhase::Target | sequoia_input::EventPhase::Bubble
        ) {
            return;
        }
        if !ctx.has_focus_within() && !ctx.state.focused {
            return;
        }
        self.fire_escape(ctx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portal_defaults_to_manual_center_popup() {
        let open = Signal::new(false);
        let p = Portal::new(open);
        assert!(matches!(p.dismiss, Dismiss::Manual));
        assert!(matches!(p.layer, OverlayLayer::Popup));
        assert!(matches!(p.anchor, Anchor::Viewport));
        assert!(!p.is_open());
    }

    #[test]
    fn popper_defaults_to_outside_click_bottom_with_gap() {
        let open = Signal::new(true);
        let p = Popper::new(open);
        assert!(matches!(p.dismiss, Dismiss::OutsideClick));
        assert!(matches!(p.placement, crate::overlay::Placement::Bottom));
        assert_eq!(p.offset, (0.0, 4.0));
        assert!(matches!(p.overflow, OverlayOverflow::FlipAndClamp));
        assert!(p.is_open());
    }

    #[test]
    fn popper_at_cursor_sets_cursor_anchor_and_placement() {
        let open = Signal::new(false);
        let p = Popper::new(open).at_cursor();
        assert!(matches!(p.anchor, Anchor::Cursor));
        assert!(matches!(p.placement, crate::overlay::Placement::Cursor));
    }

    #[test]
    fn dismissable_binds_open_signal() {
        let open = Signal::new(true);
        let layer = DismissableLayer::wrap(Unit).bind(open.clone());
        assert!(layer.dismiss_on_escape);

        if let Some(f) = &layer.on_dismiss {
            f();
        }
        assert!(!open.get_untracked());
    }

    #[test]
    fn dismissable_chains_previous_on_dismiss() {
        let open = Signal::new(true);
        let seen = Rc::new(std::cell::Cell::new(false));
        let seen2 = seen.clone();
        let layer = DismissableLayer::wrap(Unit)
            .on_dismiss(move || seen2.set(true))
            .bind(open.clone());
        if let Some(f) = &layer.on_dismiss {
            f();
        }
        assert!(seen.get());
        assert!(!open.get_untracked());
    }
}
