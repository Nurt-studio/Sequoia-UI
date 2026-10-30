use std::any::Any;
use std::cell::Cell;
use std::rc::Rc;

use sequoia_core::{Color, ImageSource, Point, Rect, RenderList, Size, Vec2, WindowId};
use sequoia_draw::ShapeCache;
use sequoia_input::{CursorIcon, Event, EventPhase, Modifiers, PenTool};
use sequoia_layout::{
    ContainerStyle, DockStyle, GridPlacement, GridStyle, LayoutCtx, LayoutStyle, MasonryStyle,
    WrapStyle,
};
use sequoia_reactive::Environment;
use sequoia_style::{BoxStyle, Theme, VisualState};
use sequoia_text::{
    ElideMode, FontStyle, FontWeight, LetterSpacing, LineHeight, RichAlign, RichParagraph,
    ShapedText, TextSpan, TextSystem, WrapMode,
};

use crate::any::AnyView;
use crate::dnd::{DragEnd, DragOptions, DragPayload, DragSession, DropPosition};
use crate::live::Prop;
use crate::overlay::{OverlayEntry, OverlayId};
use crate::runtime::{ElementId, Runtime};
use crate::surface::SurfaceInputBinding;

pub trait View {
    fn body(&self) -> impl View + 'static;

    #[doc(hidden)]
    fn is_leaf(&self) -> bool {
        false
    }

    #[doc(hidden)]
    fn children_override(&self) -> Option<Vec<AnyView>> {
        None
    }

    #[doc(hidden)]
    fn keyed_children(&self) -> Option<Vec<(u64, AnyView)>> {
        None
    }

    #[doc(hidden)]
    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle::default()
    }

    fn layout_style_ctx(&self, _rt: &Runtime, _el: ElementId) -> LayoutStyle {
        self.layout_style()
    }

    #[doc(hidden)]
    fn has_visual_state(&self) -> bool {
        false
    }

    #[doc(hidden)]
    fn wants_hover_updates(&self) -> bool {
        false
    }

    fn accessibility(&self) -> Option<crate::a11y::Accessible> {
        None
    }

    #[doc(hidden)]
    fn container(&self) -> Option<ContainerStyle> {
        None
    }

    #[doc(hidden)]
    fn grid(&self) -> Option<GridStyle> {
        None
    }

    #[doc(hidden)]
    fn dock(&self) -> Option<DockStyle> {
        None
    }

    #[doc(hidden)]
    fn wrap(&self) -> Option<WrapStyle> {
        None
    }

    #[doc(hidden)]
    fn masonry(&self) -> Option<MasonryStyle> {
        None
    }

    #[doc(hidden)]
    fn flex_weights(&self) -> Option<&[f32]> {
        None
    }

    #[doc(hidden)]
    fn grid_placement(&self) -> Option<GridPlacement> {
        None
    }

    #[doc(hidden)]
    fn grid_placements(&self) -> Option<Vec<GridPlacement>> {
        None
    }

    #[doc(hidden)]
    fn intrinsic_size(&self, _ctx: &mut LayoutCtx) -> Option<Size> {
        None
    }

    /// Returns the first text baseline relative to the view's top edge.
    ///
    /// Used by [`sequoia_layout::Align::Baseline`] in horizontal containers.
    #[doc(hidden)]
    fn baseline_offset(&self, _ctx: &mut LayoutCtx, _width: f32) -> Option<f32> {
        None
    }

    #[doc(hidden)]
    fn contain_size(&self) -> bool {
        false
    }

    fn arrange_override(
        &self,
        _rt: &Runtime,
        _el: ElementId,
        _rect: Rect,
        _ctx: &mut LayoutCtx,
    ) -> bool {
        false
    }

    fn clip_self(&self) -> bool {
        false
    }

    fn clip_radius(&self) -> f32 {
        0.0
    }

    fn paint_opacity(&self) -> Option<f32> {
        None
    }

    /// Allows the runtime to replay this element's painted subtree from cache
    /// while its element revision and absolute paint context stay unchanged.
    ///
    /// Cache hits skip `paint_self` and `paint_foreground`, so their side
    /// effects must not be required for later frames.
    fn cache_layer(&self) -> bool {
        false
    }

    fn paint_opacity_ctx(&self, _rt: &Runtime, _el: ElementId) -> Option<f32> {
        self.paint_opacity()
    }

    fn paint_offset(&self) -> Vec2 {
        Vec2::ZERO
    }

    fn paint_offset_ctx(&self, _rt: &Runtime, _el: ElementId) -> Vec2 {
        self.paint_offset()
    }

    fn paint_scale(&self) -> f32 {
        1.0
    }

    fn paint_scale_ctx(&self, _rt: &Runtime, _el: ElementId) -> f32 {
        self.paint_scale()
    }

    fn paint_rotation(&self) -> f32 {
        0.0
    }

    fn paint_rotation_ctx(&self, _rt: &Runtime, _el: ElementId) -> f32 {
        self.paint_rotation()
    }

    fn foreground_filter(&self) -> Option<sequoia_style::Filter> {
        None
    }

    #[doc(hidden)]
    fn surface_target(&self) -> Option<Rc<dyn Any>> {
        None
    }

    #[doc(hidden)]
    fn surface_input(&self) -> Option<SurfaceInputBinding> {
        None
    }

    #[doc(hidden)]
    fn on_event(&self, _event: &Event, _ctx: &mut EventCtx) {}

    #[doc(hidden)]
    fn captures_pointer(&self) -> bool {
        false
    }

    fn captures_pointer_ctx(&self, _rt: &Runtime, _el: ElementId) -> bool {
        self.captures_pointer()
    }

    fn reuse_state(&self, _old: &Self) {}

    fn hit_testable(&self) -> bool {
        true
    }

    fn test_id(&self) -> Option<String> {
        None
    }

    #[doc(hidden)]
    fn focusable(&self) -> bool {
        false
    }

    fn focus_trap(&self) -> bool {
        false
    }

    fn accepts_tab(&self) -> bool {
        false
    }

    /// Whether this view edits text when focused.
    ///
    /// Hosts use this to keep single-key accelerators (Delete, Backspace,
    /// letter keys) from stealing input while a text field or text area has
    /// focus. Command shortcuts such as Ctrl+S still fire.
    fn accepts_text_input(&self) -> bool {
        false
    }

    #[doc(hidden)]
    fn cursor_icon(&self, _state: VisualState) -> CursorIcon {
        CursorIcon::Default
    }

    #[doc(hidden)]
    fn paint_self(&self, _ctx: &mut PaintCtx) {}

    fn paint_foreground(&self, _ctx: &mut PaintCtx) {}

    fn accepts_drop(&self, _payload: &DragPayload) -> bool {
        false
    }

    fn on_drag_over(&self, _pos: DropPosition, _ctx: &mut EventCtx) {}

    fn on_drop(&self, _pos: DropPosition, _ctx: &mut EventCtx) {}

    fn on_drag_start(&self, _ctx: &mut EventCtx) {}

    fn on_drag_end(&self, _end: DragEnd, _ctx: &mut EventCtx) {}

    #[doc(hidden)]
    fn on_mount(&self) {}

    #[doc(hidden)]
    fn on_unmount(&self) {}
}

pub struct EventCtx<'a> {
    pub phase: EventPhase,
    pub element: ElementId,
    pub state: VisualState,
    pub hover_self: bool,
    pub pos: Point,
    pub rect: Rect,
    pub environment: &'a Environment,
    pub window: WindowId,
    pub(crate) stopped: &'a Cell<bool>,
    pub(crate) handled: &'a Cell<bool>,
    pub(crate) capture_req: &'a Cell<Option<ElementId>>,
    pub(crate) release_req: &'a Cell<bool>,
    pub(crate) focus_req: &'a Cell<bool>,
    pub(crate) text: &'a mut TextSystem,
    pub(crate) runtime: &'a Runtime,
}

impl EventCtx<'_> {
    pub fn local_pos(&self) -> Point {
        Point::new(self.pos.x - self.rect.left(), self.pos.y - self.rect.top())
    }

    pub fn stop_propagation(&mut self) {
        self.stopped.set(true);
    }

    pub fn handled(&mut self) {
        self.handled.set(true);
        self.stopped.set(true);
    }

    pub fn capture_pointer(&mut self) {
        self.capture_req.set(Some(self.element));
        self.release_req.set(false);
    }

    pub fn release_pointer(&mut self) {
        self.release_req.set(true);
        self.capture_req.set(None);
    }

    pub fn request_focus(&mut self) {
        self.focus_req.set(true);
    }

    pub fn begin_drag<T: 'static>(&self, payload: T, options: DragOptions) {
        self.runtime.begin_pending(
            self.element,
            DragPayload::new(payload),
            self.pos,
            self.rect,
            options,
        );
    }

    pub fn drag_session(&self) -> Option<DragSession> {
        self.runtime.current_drag()
    }

    pub fn drop_position(&self, modifiers: Modifiers) -> DropPosition {
        DropPosition::in_rect(self.pos, self.rect, modifiers)
    }

    pub fn cancel_drag(&mut self) {
        self.runtime.cancel_drag(&mut *self.text);
    }

    pub fn pressure(&self) -> f32 {
        self.runtime.pointer_pressure()
    }

    pub fn tool(&self) -> PenTool {
        self.runtime.pointer_tool()
    }

    pub fn text_width(&mut self, text: &str, size: f32) -> f32 {
        self.text.text_width(text, size)
    }

    pub fn text_width_weighted(&mut self, text: &str, size: f32, weight: FontWeight) -> f32 {
        self.text.text_width_weighted(text, size, weight)
    }

    pub fn text_width_weighted_spaced(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
        spacing: LetterSpacing,
    ) -> f32 {
        self.text
            .text_width_weighted_spaced(text, size, weight, spacing)
    }

    pub fn caret_positions(&mut self, text: &str, size: f32, weight: FontWeight) -> Vec<f32> {
        self.text.caret_positions(text, size, weight)
    }

    pub fn caret_positions_spaced(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
        spacing: LetterSpacing,
    ) -> Vec<f32> {
        self.text
            .caret_positions_spaced(text, size, weight, spacing)
    }

    pub fn char_index_from_x(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
        x: f32,
    ) -> usize {
        self.text.char_index_from_x(text, size, weight, x)
    }

    pub fn char_index_from_x_spaced(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
        x: f32,
        spacing: LetterSpacing,
    ) -> usize {
        self.text
            .char_index_from_x_spaced(text, size, weight, x, spacing)
    }

    pub fn shape_rich_full(
        &mut self,
        spans: &[TextSpan],
        size: f32,
        width: f32,
        mode: WrapMode,
        spacing: LetterSpacing,
        align: RichAlign,
    ) -> std::rc::Rc<RichParagraph> {
        self.text
            .shape_rich_full(spans, size, width, mode, spacing, align)
    }

    pub fn request_frame(&self) {
        self.runtime.request_frame();
    }

    pub fn hide_overlay(&self, id: OverlayId) -> bool {
        self.runtime.overlay_hide(id)
    }

    pub fn has_overlay(&self, id: OverlayId) -> bool {
        self.runtime.overlay_exists(id)
    }

    pub fn overlay_rect(&self, id: OverlayId) -> Option<Rect> {
        self.runtime.overlay_rect(id)
    }

    pub fn overlay_set_position(&self, id: OverlayId, x: f32, y: f32) -> bool {
        self.runtime.overlay_set_position(id, x, y)
    }

    pub fn has_focus_within(&self) -> bool {
        self.runtime.focus_in_subtree(self.element)
    }

    pub fn window_rect(&self) -> Rect {
        self.runtime.window_rect()
    }

    pub fn window_size(&self) -> Size {
        self.runtime.window_size_value()
    }

    pub fn scale(&self) -> f32 {
        self.runtime.scale()
    }

    pub fn safe_insets(&self) -> sequoia_core::Edges {
        self.runtime.safe_insets()
    }

    pub fn runtime(&self) -> &Runtime {
        self.runtime
    }

    pub fn element(&self) -> ElementId {
        self.element
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TextMetrics {
    /// Width of the measured text. Zero for [`PaintCtx::text_metrics`], which
    /// measures the font rather than a specific string.
    pub width: f32,

    /// Height of the measured text block, or the line-box height for
    /// [`PaintCtx::text_metrics`].
    pub height: f32,

    /// Distance from the top of the line box to the baseline.
    pub ascent: f32,

    /// Distance from the baseline to the bottom of the line box.
    pub descent: f32,

    /// Extra leading outside ascent and descent. Sequoia line boxes are tight,
    /// so this is currently always `0.0`.
    pub line_gap: f32,
}

fn metrics_from_shaped(shaped: &ShapedText) -> TextMetrics {
    let line_height = shaped.line_height();
    let ascent = shaped.first_baseline();
    TextMetrics {
        width: shaped.width(),
        height: shaped.height(),
        ascent,
        descent: (line_height - ascent).max(0.0),
        line_gap: 0.0,
    }
}

pub struct PaintCtx<'a> {
    pub(crate) list: &'a mut RenderList,
    pub(crate) text: &'a mut TextSystem,

    pub environment: &'a Environment,

    pub rect: Rect,

    pub element: ElementId,

    pub state: VisualState,

    pub window: WindowId,

    pub hovered_subtree: bool,
    pub(crate) runtime: &'a Runtime,

    pub(crate) shapes: &'a ShapeCache,

    pub(crate) draw_offset: Vec2,

    pub(crate) offset_stack: Vec<Vec2>,
}

impl PaintCtx<'_> {
    pub fn theme(&self) -> Theme {
        self.environment.get::<Theme>().unwrap_or_else(Theme::dark)
    }

    pub fn runtime(&self) -> &Runtime {
        self.runtime
    }

    pub fn window_rect(&self) -> Rect {
        self.runtime.window_rect()
    }

    pub fn window_size(&self) -> Size {
        self.runtime.window_size_value()
    }

    pub fn scale(&self) -> f32 {
        self.runtime.scale()
    }

    pub fn safe_insets(&self) -> sequoia_core::Edges {
        self.runtime.safe_insets()
    }

    pub fn element(&self) -> ElementId {
        self.element
    }

    /// Measures a specific string.
    pub fn measure_text(&mut self, text: &str, size: f32) -> TextMetrics {
        if text.is_empty() || !(size > 0.0) {
            return TextMetrics::default();
        }
        let shaped = self.text.shape(text, size);
        metrics_from_shaped(&shaped)
    }

    /// Returns vertical metrics for the default font at `size`.
    ///
    /// Use this instead of guessing a baseline offset: center a line box with
    /// `rect.top() + (rect.height() - metrics.height) * 0.5`, then draw it at
    /// that top edge with [`Self::draw_text_tl`].
    pub fn text_metrics(&mut self, size: f32) -> TextMetrics {
        if !(size > 0.0) {
            return TextMetrics::default();
        }
        let shaped = self.text.shape("Mg", size);
        TextMetrics {
            width: 0.0,
            ..metrics_from_shaped(&shaped)
        }
    }

    /// Baseline placed in the vertical center of a line box whose top is
    /// `rect_top` and whose height is `rect_h`.
    ///
    /// Prefer [`Self::text_metrics`] when the real font ascent is needed.
    pub fn baseline_centered(rect_top: f32, rect_h: f32, size: f32) -> f32 {
        rect_top + rect_h * 0.5 + size * 0.35
    }

    /// Approximate baseline for a line box whose top is `rect_top`.
    ///
    /// Prefer [`Self::text_metrics`] when the real font ascent is needed.
    pub fn baseline_top(rect_top: f32, size: f32) -> f32 {
        rect_top + size * 1.35 * 0.5 + size * 0.35
    }

    pub fn quad(&mut self, rect: Rect, color: Color, radius: f32) {
        let rect = if rect.is_empty() {
            Rect::from_xywh(rect.origin.x, rect.origin.y, 1.0, 1.0)
        } else {
            rect
        };
        self.list.push_quad(rect, color, radius);
    }

    pub fn bordered_quad(
        &mut self,
        rect: Rect,
        background: Color,
        radius: f32,
        border_width: f32,
        border_color: Color,
    ) {
        self.list
            .push_bordered_quad(rect, background, radius, border_width, border_color);
    }

    pub fn paint_box(&mut self, style: BoxStyle) {
        let rect = self.rect;
        self.paint_box_in(style, rect);
    }

    pub fn paint_box_in(&mut self, style: BoxStyle, rect: Rect) {
        style.paint(self.list, rect);
    }

    pub fn push_clip(&mut self, rect: Rect) {
        self.list.push_clip(rect);
    }
    pub fn pop_clip(&mut self) {
        self.list.pop_clip();
    }
    pub fn push_rounded_clip(&mut self, rect: Rect, radius: f32) {
        self.list.push_rounded_clip(rect, radius);
    }

    pub fn list_clip_depth(&self) -> usize {
        self.list.clip_depth()
    }

    pub fn push_backdrop(&mut self, rect: Rect, backdrop: sequoia_style::Backdrop) {
        if backdrop.is_empty() {
            return;
        }
        self.list.push_backdrop(
            rect,
            backdrop.radius,
            backdrop.blur_px(),
            backdrop.dim,
            backdrop.noise,
            backdrop.span_mask(),
        );
    }

    /// Draws a single line with `origin` at the **text baseline**.
    ///
    /// The y coordinate is not a top edge: glyphs extend above it by the font
    /// ascent. Use [`Self::draw_text_tl`] when positioning from the top-left of
    /// a layout rectangle, or [`Self::text_metrics`] to center a line box.
    pub fn draw_text(&mut self, text: &str, origin: (f32, f32), size: f32, color: Color) {
        self.draw_text_weighted(text, origin, size, FontWeight::NORMAL, color);
    }

    /// Draws a single line with `top_left` at the top-left of its line box.
    ///
    /// This is the counterpart of [`Self::draw_text`], whose origin is the
    /// baseline, and matches [`crate::Painter::text_tl`].
    pub fn draw_text_tl(&mut self, text: &str, top_left: (f32, f32), size: f32, color: Color) {
        if text.is_empty() || !(size > 0.0) {
            return;
        }
        let ascent = self.text_metrics(size).ascent;
        self.draw_text(text, (top_left.0, top_left.1 + ascent), size, color);
    }

    pub fn draw_text_weighted(
        &mut self,
        text: &str,
        origin: (f32, f32),
        size: f32,
        weight: FontWeight,
        color: Color,
    ) {
        let shaped = self.text.shape_weighted(text, size, weight);
        self.text.draw(&shaped, origin, color, self.list);
    }

    pub fn draw_text_wrapped(
        &mut self,
        text: &str,
        origin: (f32, f32),
        size: f32,
        weight: FontWeight,
        width: f32,
        mode: WrapMode,
        color: Color,
    ) {
        self.draw_text_wrapped_spaced(text, origin, size, weight, width, mode, 0.0, color);
    }

    pub fn draw_text_wrapped_spaced(
        &mut self,
        text: &str,
        origin: (f32, f32),
        size: f32,
        weight: FontWeight,
        width: f32,
        mode: WrapMode,
        spacing: LetterSpacing,
        color: Color,
    ) {
        let shaped = self
            .text
            .shape_wrapped_spaced(text, size, weight, width, mode, spacing);
        self.text.draw(&shaped, origin, color, self.list);
    }

    pub fn draw_text_spaced(
        &mut self,
        text: &str,
        origin: (f32, f32),
        size: f32,
        spacing: LetterSpacing,
        color: Color,
    ) {
        self.draw_text_weighted_spaced(text, origin, size, FontWeight::NORMAL, spacing, color);
    }

    pub fn draw_text_weighted_spaced(
        &mut self,
        text: &str,
        origin: (f32, f32),
        size: f32,
        weight: FontWeight,
        spacing: LetterSpacing,
        color: Color,
    ) {
        let shaped = self.text.shape_spaced(text, size, weight, spacing);
        self.text.draw(&shaped, origin, color, self.list);
    }

    pub fn draw_text_styled_spaced(
        &mut self,
        text: &str,
        origin: (f32, f32),
        size: f32,
        weight: FontWeight,
        spacing: LetterSpacing,
        family: Option<&str>,
        style: FontStyle,
        color: Color,
    ) {
        let shaped = self
            .text
            .shape_spaced_styled(text, size, weight, spacing, family, style);
        self.text.draw(&shaped, origin, color, self.list);
    }

    pub fn draw_text_wrapped_styled_spaced(
        &mut self,
        text: &str,
        origin: (f32, f32),
        size: f32,
        weight: FontWeight,
        width: f32,
        mode: WrapMode,
        spacing: LetterSpacing,
        family: Option<&str>,
        style: FontStyle,
        color: Color,
    ) {
        let shaped = self
            .text
            .shape_wrapped_styled_spaced(text, size, weight, width, mode, spacing, family, style);
        self.text.draw(&shaped, origin, color, self.list);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn shape_wrapped_styled_spaced_lh(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
        width: f32,
        mode: WrapMode,
        spacing: LetterSpacing,
        family: Option<&str>,
        style: FontStyle,
        line_height: LineHeight,
    ) -> Rc<ShapedText> {
        self.text.shape_wrapped_styled_spaced_lh(
            text,
            size,
            weight,
            width,
            mode,
            spacing,
            family,
            style,
            line_height,
        )
    }

    pub fn draw_shaped(&mut self, shaped: &ShapedText, origin: (f32, f32), color: Color) {
        self.text.draw(shaped, origin, color, self.list);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn shape_rich_full_lh(
        &mut self,
        spans: &[TextSpan],
        size: f32,
        width: f32,
        mode: WrapMode,
        spacing: LetterSpacing,
        align: RichAlign,
        line_height: LineHeight,
    ) -> Rc<RichParagraph> {
        self.text
            .shape_rich_full_lh(spans, size, width, mode, spacing, align, line_height)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn elide_lines_styled_spaced(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
        width: f32,
        mode: WrapMode,
        spacing: LetterSpacing,
        family: Option<&str>,
        style: FontStyle,
        line_height: LineHeight,
        max_lines: usize,
    ) -> String {
        self.text.elide_lines_styled_spaced(
            text,
            size,
            weight,
            width,
            mode,
            spacing,
            family,
            style,
            line_height,
            max_lines,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn elide_rich_lines(
        &mut self,
        spans: &[TextSpan],
        size: f32,
        width: f32,
        mode: WrapMode,
        spacing: LetterSpacing,
        align: RichAlign,
        line_height: LineHeight,
        max_lines: usize,
    ) -> Vec<TextSpan> {
        self.text.elide_rich_lines(
            spans,
            size,
            width,
            mode,
            spacing,
            align,
            line_height,
            max_lines,
        )
    }

    pub fn draw_rich(&mut self, rich: &RichParagraph, origin: (f32, f32), default_color: Color) {
        self.text.draw_rich(rich, origin, default_color, self.list);
    }

    pub fn draw_rich_spans(
        &mut self,
        spans: &[TextSpan],
        origin: (f32, f32),
        size: f32,
        width: f32,
        mode: WrapMode,
        spacing: LetterSpacing,
        align: RichAlign,
        default_color: Color,
    ) {
        let rich = self
            .text
            .shape_rich_full(spans, size, width, mode, spacing, align);
        self.text.draw_rich(&rich, origin, default_color, self.list);
    }

    pub fn text_width(&mut self, text: &str, size: f32) -> f32 {
        self.text.shape(text, size).width()
    }

    pub fn text_width_weighted(&mut self, text: &str, size: f32, weight: FontWeight) -> f32 {
        self.text.shape_weighted(text, size, weight).width()
    }

    pub fn text_width_weighted_spaced(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
        spacing: LetterSpacing,
    ) -> f32 {
        self.text.shape_spaced(text, size, weight, spacing).width()
    }

    pub fn text_width_styled_spaced(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
        spacing: LetterSpacing,
        family: Option<&str>,
        style: FontStyle,
    ) -> f32 {
        self.text
            .text_width_styled_spaced(text, size, weight, spacing, family, style)
    }

    pub fn elide_spaced(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
        max_width: f32,
        mode: ElideMode,
        spacing: LetterSpacing,
    ) -> String {
        self.text
            .elide_spaced(text, size, weight, max_width, mode, spacing)
    }

    pub fn elide_styled_spaced(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
        max_width: f32,
        mode: ElideMode,
        spacing: LetterSpacing,
        family: Option<&str>,
        style: FontStyle,
    ) -> String {
        self.text
            .elide_styled_spaced(text, size, weight, max_width, mode, spacing, family, style)
    }

    pub fn caret_positions(&mut self, text: &str, size: f32, weight: FontWeight) -> Vec<f32> {
        self.text.caret_positions(text, size, weight)
    }

    pub fn caret_positions_spaced(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
        spacing: LetterSpacing,
    ) -> Vec<f32> {
        self.text
            .caret_positions_spaced(text, size, weight, spacing)
    }

    pub fn shape_rich_full(
        &mut self,
        spans: &[TextSpan],
        size: f32,
        width: f32,
        mode: WrapMode,
        spacing: LetterSpacing,
        align: RichAlign,
    ) -> std::rc::Rc<RichParagraph> {
        self.text
            .shape_rich_full(spans, size, width, mode, spacing, align)
    }

    pub fn scale_factor(&self) -> f32 {
        self.text.scale_factor()
    }

    pub fn request_frame(&self) {
        self.runtime.request_frame();
    }

    pub fn live_color_exp(&self, prop: Prop, target: Color, rate: f32) -> Color {
        self.runtime
            .live_color_exp(self.element, prop, target, rate)
            .0
    }

    pub fn draw_sprite(
        &mut self,
        key: u64,
        width: u32,
        height: u32,
        alpha: &[u8],
        rect: Rect,
        color: Color,
    ) {
        self.text
            .draw_sprite(key, width, height, alpha, rect, color, self.list);
    }

    pub fn image(&mut self, rect: Rect, uv: [f32; 4], source: ImageSource) {
        self.list.push_image(rect, uv, source);
    }

    pub fn show_overlay(&self, entry: OverlayEntry) -> OverlayId {
        self.runtime.overlay_show(entry)
    }

    pub fn hide_overlay(&self, id: OverlayId) -> bool {
        self.runtime.overlay_hide(id)
    }

    pub fn has_overlay(&self, id: OverlayId) -> bool {
        self.runtime.overlay_exists(id)
    }

    pub fn overlay_rect(&self, id: OverlayId) -> Option<Rect> {
        self.runtime.overlay_rect(id)
    }

    pub fn drag_session(&self) -> Option<DragSession> {
        self.runtime.current_drag()
    }

    pub fn is_drag_target(&self) -> bool {
        self.runtime.drag_target_contains(self.element)
    }

    pub fn has_focus_within(&self) -> bool {
        self.runtime.focus_in_subtree(self.element)
    }
}

pub trait CustomLayout {
    fn arrange(&self, rt: &Runtime, el: ElementId, rect: Rect, ctx: &mut LayoutCtx) -> bool;
}
