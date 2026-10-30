use std::any::{Any, TypeId};
use std::rc::Rc;

use sequoia_core::{Rect, Size, Vec2};
use sequoia_input::{CursorIcon, Event};
use sequoia_style::{Filter, VisualState};

use crate::dnd::{DragEnd, DragPayload, DropPosition};
use crate::runtime::{ElementId, Runtime};
use crate::surface::SurfaceInputBinding;
use crate::view::{EventCtx, PaintCtx, View};
use sequoia_layout::{
    ContainerStyle, DockStyle, GridPlacement, GridStyle, LayoutCtx, LayoutStyle, MasonryStyle,
    WrapStyle,
};

pub(crate) trait ErasedView: 'static {
    fn view_type(&self) -> TypeId;
    fn type_name(&self) -> &'static str;
    fn is_leaf(&self) -> bool;
    fn children_override(&self) -> Option<Vec<AnyView>>;
    fn keyed_children(&self) -> Option<Vec<(u64, AnyView)>>;
    fn call_body(&self) -> AnyView;
    fn layout_style_erased(&self) -> LayoutStyle;
    fn layout_style_ctx_erased(&self, rt: &Runtime, el: ElementId) -> LayoutStyle;
    fn container(&self) -> Option<ContainerStyle>;
    fn grid(&self) -> Option<GridStyle>;
    fn dock(&self) -> Option<DockStyle>;
    fn wrap(&self) -> Option<WrapStyle>;
    fn masonry(&self) -> Option<MasonryStyle>;
    fn grid_placement(&self) -> Option<GridPlacement>;
    fn grid_placements(&self) -> Option<Vec<GridPlacement>>;
    fn flex_weights(&self) -> Option<Vec<f32>>;
    fn intrinsic_size(&self, ctx: &mut LayoutCtx) -> Option<Size>;
    fn baseline_offset_erased(&self, ctx: &mut LayoutCtx, width: f32) -> Option<f32>;
    fn contain_size_erased(&self) -> bool;
    fn arrange_override_erased(
        &self,
        rt: &Runtime,
        el: ElementId,
        rect: Rect,
        ctx: &mut LayoutCtx,
    ) -> bool;
    fn clip_self_erased(&self) -> bool;
    fn clip_radius_erased(&self) -> f32;
    fn paint_opacity_erased(&self) -> Option<f32>;
    fn cache_layer_erased(&self) -> bool;
    fn paint_offset_erased(&self) -> Vec2;
    fn paint_opacity_ctx_erased(&self, rt: &Runtime, el: ElementId) -> Option<f32>;
    fn paint_offset_ctx_erased(&self, rt: &Runtime, el: ElementId) -> Vec2;
    fn paint_scale_erased(&self) -> f32;
    fn paint_scale_ctx_erased(&self, rt: &Runtime, el: ElementId) -> f32;
    fn paint_rotation_erased(&self) -> f32;
    fn paint_rotation_ctx_erased(&self, rt: &Runtime, el: ElementId) -> f32;

    fn foreground_filter_erased(&self) -> Option<Filter>;
    fn surface_target_erased(&self) -> Option<Rc<dyn Any>>;
    fn surface_input_erased(&self) -> Option<SurfaceInputBinding>;
    fn on_event_erased(&self, event: &Event, ctx: &mut EventCtx);
    fn captures_pointer_erased(&self) -> bool;
    fn captures_pointer_ctx_erased(&self, rt: &Runtime, el: ElementId) -> bool;
    fn reuse_state_erased(&self, old: &dyn ErasedView);
    fn as_any(&self) -> &dyn Any;
    fn hit_testable_erased(&self) -> bool;
    fn test_id_erased(&self) -> Option<String>;
    fn focusable_erased(&self) -> bool;
    fn focus_trap_erased(&self) -> bool;
    fn accepts_tab_erased(&self) -> bool;
    fn accepts_text_input_erased(&self) -> bool;
    fn cursor_icon_erased(&self, state: VisualState) -> CursorIcon;
    fn has_visual_state_erased(&self) -> bool;
    fn wants_hover_updates_erased(&self) -> bool;
    fn accessibility_erased(&self) -> Option<crate::a11y::Accessible>;
    fn paint_self_erased(&self, ctx: &mut PaintCtx);
    fn paint_foreground_erased(&self, ctx: &mut PaintCtx);
    fn accepts_drop_erased(&self, payload: &DragPayload) -> bool;
    fn on_drag_start_erased(&self, ctx: &mut EventCtx);
    fn on_drag_over_erased(&self, pos: DropPosition, ctx: &mut EventCtx);
    fn on_drop_erased(&self, pos: DropPosition, ctx: &mut EventCtx);
    fn on_drag_end_erased(&self, end: DragEnd, ctx: &mut EventCtx);
    fn as_any_rc(self: Rc<Self>) -> Rc<dyn Any>;
    fn on_mount(&self);
    fn on_unmount(&self);
}

impl<V: View + 'static> ErasedView for V {
    fn view_type(&self) -> TypeId {
        TypeId::of::<V>()
    }
    fn type_name(&self) -> &'static str {
        std::any::type_name::<V>()
    }
    fn is_leaf(&self) -> bool {
        View::is_leaf(self)
    }
    fn children_override(&self) -> Option<Vec<AnyView>> {
        View::children_override(self)
    }
    fn keyed_children(&self) -> Option<Vec<(u64, AnyView)>> {
        View::keyed_children(self)
    }
    fn call_body(&self) -> AnyView {
        AnyView::new(View::body(self))
    }
    fn layout_style_erased(&self) -> LayoutStyle {
        View::layout_style(self)
    }
    fn layout_style_ctx_erased(&self, rt: &Runtime, el: ElementId) -> LayoutStyle {
        View::layout_style_ctx(self, rt, el)
    }
    fn container(&self) -> Option<ContainerStyle> {
        View::container(self)
    }
    fn grid(&self) -> Option<GridStyle> {
        View::grid(self)
    }
    fn dock(&self) -> Option<DockStyle> {
        View::dock(self)
    }
    fn wrap(&self) -> Option<WrapStyle> {
        View::wrap(self)
    }
    fn masonry(&self) -> Option<MasonryStyle> {
        View::masonry(self)
    }
    fn grid_placement(&self) -> Option<GridPlacement> {
        View::grid_placement(self)
    }
    fn grid_placements(&self) -> Option<Vec<GridPlacement>> {
        View::grid_placements(self)
    }
    fn flex_weights(&self) -> Option<Vec<f32>> {
        View::flex_weights(self).map(|s| s.to_vec())
    }
    fn intrinsic_size(&self, ctx: &mut LayoutCtx) -> Option<Size> {
        View::intrinsic_size(self, ctx)
    }
    fn baseline_offset_erased(&self, ctx: &mut LayoutCtx, width: f32) -> Option<f32> {
        View::baseline_offset(self, ctx, width)
    }
    fn contain_size_erased(&self) -> bool {
        View::contain_size(self)
    }
    fn arrange_override_erased(
        &self,
        rt: &Runtime,
        el: ElementId,
        rect: Rect,
        ctx: &mut LayoutCtx,
    ) -> bool {
        View::arrange_override(self, rt, el, rect, ctx)
    }
    fn clip_self_erased(&self) -> bool {
        View::clip_self(self)
    }
    fn clip_radius_erased(&self) -> f32 {
        View::clip_radius(self)
    }
    fn paint_opacity_erased(&self) -> Option<f32> {
        View::paint_opacity(self)
    }
    fn cache_layer_erased(&self) -> bool {
        View::cache_layer(self)
    }
    fn paint_offset_erased(&self) -> Vec2 {
        View::paint_offset(self)
    }
    fn paint_opacity_ctx_erased(&self, rt: &Runtime, el: ElementId) -> Option<f32> {
        View::paint_opacity_ctx(self, rt, el)
    }
    fn paint_offset_ctx_erased(&self, rt: &Runtime, el: ElementId) -> Vec2 {
        View::paint_offset_ctx(self, rt, el)
    }
    fn paint_scale_erased(&self) -> f32 {
        View::paint_scale(self)
    }
    fn paint_scale_ctx_erased(&self, rt: &Runtime, el: ElementId) -> f32 {
        View::paint_scale_ctx(self, rt, el)
    }
    fn paint_rotation_erased(&self) -> f32 {
        View::paint_rotation(self)
    }
    fn paint_rotation_ctx_erased(&self, rt: &Runtime, el: ElementId) -> f32 {
        View::paint_rotation_ctx(self, rt, el)
    }
    fn foreground_filter_erased(&self) -> Option<Filter> {
        View::foreground_filter(self)
    }
    fn surface_target_erased(&self) -> Option<Rc<dyn Any>> {
        View::surface_target(self)
    }
    fn surface_input_erased(&self) -> Option<SurfaceInputBinding> {
        View::surface_input(self)
    }
    fn on_event_erased(&self, event: &Event, ctx: &mut EventCtx) {
        View::on_event(self, event, ctx);
    }
    fn captures_pointer_erased(&self) -> bool {
        View::captures_pointer(self)
    }
    fn captures_pointer_ctx_erased(&self, rt: &Runtime, el: ElementId) -> bool {
        View::captures_pointer_ctx(self, rt, el)
    }
    fn reuse_state_erased(&self, old: &dyn ErasedView) {
        if let (Some(me), Some(prev)) = (
            self.as_any().downcast_ref::<V>(),
            old.as_any().downcast_ref::<V>(),
        ) {
            View::reuse_state(me, prev);
        }
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn hit_testable_erased(&self) -> bool {
        View::hit_testable(self) && View::layout_style(self).hit_testable
    }
    fn test_id_erased(&self) -> Option<String> {
        View::test_id(self)
    }
    fn focusable_erased(&self) -> bool {
        View::focusable(self)
    }
    fn focus_trap_erased(&self) -> bool {
        View::focus_trap(self)
    }
    fn accepts_tab_erased(&self) -> bool {
        View::accepts_tab(self)
    }
    fn accepts_text_input_erased(&self) -> bool {
        View::accepts_text_input(self)
    }
    fn cursor_icon_erased(&self, state: VisualState) -> CursorIcon {
        View::cursor_icon(self, state)
    }
    fn has_visual_state_erased(&self) -> bool {
        View::has_visual_state(self)
    }
    fn wants_hover_updates_erased(&self) -> bool {
        View::wants_hover_updates(self)
    }
    fn accessibility_erased(&self) -> Option<crate::a11y::Accessible> {
        View::accessibility(self)
    }
    fn paint_self_erased(&self, ctx: &mut PaintCtx) {
        View::paint_self(self, ctx);
    }
    fn paint_foreground_erased(&self, ctx: &mut PaintCtx) {
        View::paint_foreground(self, ctx);
    }
    fn accepts_drop_erased(&self, payload: &DragPayload) -> bool {
        View::accepts_drop(self, payload)
    }
    fn on_drag_start_erased(&self, ctx: &mut EventCtx) {
        View::on_drag_start(self, ctx);
    }
    fn on_drag_over_erased(&self, pos: DropPosition, ctx: &mut EventCtx) {
        View::on_drag_over(self, pos, ctx);
    }
    fn on_drop_erased(&self, pos: DropPosition, ctx: &mut EventCtx) {
        View::on_drop(self, pos, ctx);
    }
    fn on_drag_end_erased(&self, end: DragEnd, ctx: &mut EventCtx) {
        View::on_drag_end(self, end, ctx);
    }
    fn as_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
    fn on_mount(&self) {
        View::on_mount(self)
    }
    fn on_unmount(&self) {
        View::on_unmount(self)
    }
}

pub struct AnyView(Rc<dyn ErasedView>);

impl AnyView {
    pub fn new<V: View + 'static>(view: V) -> Self {
        Self(Rc::new(view))
    }

    pub fn inner_layout_style(&self) -> LayoutStyle {
        self.0.layout_style_erased()
    }
    pub(crate) fn into_inner(self) -> Rc<dyn ErasedView> {
        self.0
    }
    pub(crate) fn clone_inner(&self) -> Rc<dyn ErasedView> {
        Rc::clone(&self.0)
    }
}

impl Clone for AnyView {
    fn clone(&self) -> Self {
        Self(Rc::clone(&self.0))
    }
}

impl View for AnyView {
    fn body(&self) -> impl View + 'static {
        crate::views::Unit
    }
    fn arrange_override(
        &self,
        rt: &Runtime,
        el: ElementId,
        rect: Rect,
        ctx: &mut LayoutCtx,
    ) -> bool {
        self.0.arrange_override_erased(rt, el, rect, ctx)
    }
    fn clip_self(&self) -> bool {
        self.0.clip_self_erased()
    }
    fn clip_radius(&self) -> f32 {
        self.0.clip_radius_erased()
    }
    fn paint_opacity(&self) -> Option<f32> {
        self.0.paint_opacity_erased()
    }
    fn cache_layer(&self) -> bool {
        self.0.cache_layer_erased()
    }
    fn paint_offset(&self) -> Vec2 {
        self.0.paint_offset_erased()
    }
    fn paint_opacity_ctx(&self, rt: &Runtime, el: ElementId) -> Option<f32> {
        self.0.paint_opacity_ctx_erased(rt, el)
    }
    fn paint_offset_ctx(&self, rt: &Runtime, el: ElementId) -> Vec2 {
        self.0.paint_offset_ctx_erased(rt, el)
    }
    fn paint_scale(&self) -> f32 {
        self.0.paint_scale_erased()
    }
    fn paint_scale_ctx(&self, rt: &Runtime, el: ElementId) -> f32 {
        self.0.paint_scale_ctx_erased(rt, el)
    }
    fn paint_rotation(&self) -> f32 {
        self.0.paint_rotation_erased()
    }
    fn paint_rotation_ctx(&self, rt: &Runtime, el: ElementId) -> f32 {
        self.0.paint_rotation_ctx_erased(rt, el)
    }
    fn foreground_filter(&self) -> Option<Filter> {
        self.0.foreground_filter_erased()
    }
    fn surface_target(&self) -> Option<Rc<dyn Any>> {
        self.0.surface_target_erased()
    }
    fn surface_input(&self) -> Option<SurfaceInputBinding> {
        self.0.surface_input_erased()
    }
    fn on_event(&self, event: &Event, ctx: &mut EventCtx) {
        self.0.on_event_erased(event, ctx);
    }
    fn captures_pointer(&self) -> bool {
        self.0.captures_pointer_erased()
    }
    fn captures_pointer_ctx(&self, rt: &Runtime, el: ElementId) -> bool {
        self.0.captures_pointer_ctx_erased(rt, el)
    }
    fn reuse_state(&self, _old: &Self) {}
    fn hit_testable(&self) -> bool {
        self.0.hit_testable_erased()
    }
    fn test_id(&self) -> Option<String> {
        self.0.test_id_erased()
    }
    fn focusable(&self) -> bool {
        self.0.focusable_erased()
    }
    fn cursor_icon(&self, state: VisualState) -> CursorIcon {
        self.0.cursor_icon_erased(state)
    }
    fn has_visual_state(&self) -> bool {
        self.0.has_visual_state_erased()
    }
    fn wants_hover_updates(&self) -> bool {
        self.0.wants_hover_updates_erased()
    }
    fn paint_self(&self, ctx: &mut PaintCtx) {
        self.0.paint_self_erased(ctx)
    }
    fn paint_foreground(&self, ctx: &mut PaintCtx) {
        self.0.paint_foreground_erased(ctx)
    }
}
