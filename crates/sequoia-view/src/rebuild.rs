#![forbid(unsafe_code)]

use std::cell::RefCell;
use std::rc::Rc;

use sequoia_core::{Rect, Size, Vec2};
use sequoia_layout::{
    ContainerStyle, DockStyle, GridPlacement, GridStyle, LayoutCtx, LayoutStyle, MasonryStyle,
    WrapStyle,
};

use crate::any::AnyView;
use crate::runtime::{ElementId, Runtime};
use crate::view::View;

/// A transparent view whose body is rebuilt when a tracked signal changes.
///
/// The closure runs inside the runtime's read-capture scope, so every
/// `Signal::get` / `Memo::get` it performs subscribes the element to that
/// source. Layout properties are delegated to the most recently built child,
/// which keeps a rebuilt subtree behaving exactly like the view it wraps.
#[derive(Clone)]
pub struct Rebuild {
    build: Rc<dyn Fn() -> AnyView>,
    current: RefCell<AnyView>,
}

impl Rebuild {
    pub fn new(initial: AnyView, build: impl Fn() -> AnyView + 'static) -> Self {
        Self {
            build: Rc::new(build),
            current: RefCell::new(initial),
        }
    }

    fn with_current<R>(&self, f: impl FnOnce(&dyn crate::any::ErasedView) -> R) -> R {
        let view = self.current.borrow().clone_inner();
        f(&*view)
    }
}

impl View for Rebuild {
    fn body(&self) -> impl View + 'static {
        let next = (self.build)();
        *self.current.borrow_mut() = next.clone();
        next
    }

    fn layout_style(&self) -> LayoutStyle {
        self.with_current(|v| v.layout_style_erased())
    }

    fn layout_style_ctx(&self, rt: &Runtime, el: ElementId) -> LayoutStyle {
        self.with_current(|v| v.layout_style_ctx_erased(rt, el))
    }

    fn container(&self) -> Option<ContainerStyle> {
        self.with_current(crate::any::ErasedView::container)
    }

    fn grid(&self) -> Option<GridStyle> {
        self.with_current(crate::any::ErasedView::grid)
    }

    fn dock(&self) -> Option<DockStyle> {
        self.with_current(crate::any::ErasedView::dock)
    }

    fn wrap(&self) -> Option<WrapStyle> {
        self.with_current(crate::any::ErasedView::wrap)
    }

    fn masonry(&self) -> Option<MasonryStyle> {
        self.with_current(crate::any::ErasedView::masonry)
    }

    fn grid_placement(&self) -> Option<GridPlacement> {
        self.with_current(crate::any::ErasedView::grid_placement)
    }

    fn grid_placements(&self) -> Option<Vec<GridPlacement>> {
        self.with_current(crate::any::ErasedView::grid_placements)
    }

    fn intrinsic_size(&self, ctx: &mut LayoutCtx) -> Option<Size> {
        self.with_current(|v| v.intrinsic_size(ctx))
    }

    fn baseline_offset(&self, ctx: &mut LayoutCtx, width: f32) -> Option<f32> {
        self.with_current(|v| v.baseline_offset_erased(ctx, width))
    }

    fn contain_size(&self) -> bool {
        self.with_current(crate::any::ErasedView::contain_size_erased)
    }

    fn arrange_override(
        &self,
        rt: &Runtime,
        el: ElementId,
        rect: Rect,
        ctx: &mut LayoutCtx,
    ) -> bool {
        self.with_current(|v| v.arrange_override_erased(rt, el, rect, ctx))
    }

    fn clip_self(&self) -> bool {
        self.with_current(crate::any::ErasedView::clip_self_erased)
    }

    fn clip_radius(&self) -> f32 {
        self.with_current(crate::any::ErasedView::clip_radius_erased)
    }

    fn paint_offset(&self) -> Vec2 {
        self.with_current(crate::any::ErasedView::paint_offset_erased)
    }

    fn hit_testable(&self) -> bool {
        self.with_current(crate::any::ErasedView::hit_testable_erased)
    }

    fn test_id(&self) -> Option<String> {
        self.with_current(crate::any::ErasedView::test_id_erased)
    }
}

pub fn rebuild(initial: AnyView, build: impl Fn() -> AnyView + 'static) -> Rebuild {
    Rebuild::new(initial, build)
}
