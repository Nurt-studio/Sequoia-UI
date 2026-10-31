//! Cross-frame invalidation model.
//!
//! `DamageTracker` accumulates marks from a single frame so the runtime can:
//! - **skip the whole frame** when there is no paint or animation work,
//! - **limit repaint** to the damage bounding box (Phase 2 — scissor),
//! - **not skip the frame** while an animation is running (`request_frame`).
//!
//! # What triggers a frame
//!
//! - `Runtime::request_rebuild` (signals, effects, view state, mount/unmount)
//!   → `Dirty::LAYOUT | Dirty::CHILDREN`: rebuild in `settle`, then full paint.
//! - `Runtime::mark_damage(.., Dirty::PAINT)` (hover/press/focus)
//!   → `PAINT` + element bounding box: paint without rebuild.
//! - `PaintCtx::request_frame` (animation) → motion: paint the next frame.
//! - `Runtime::mark_full_damage` (resize, DPI, theme, new root)
//!   → full viewport and layer cache invalidation.
//! - `Runtime::layout` with a different viewport → full paint + `LAYOUT`.
//!
//! `HIT_TEST` and `DATA` are bookkeeping markers only: they require no pixels
//! and do not create a frame on their own.
//!
//! # Frame phases
//!
//! 1. `tick()` → `settle()` coalesces dirty state per element and rebuilds only
//!    elements with `needs_layout()`. Paint-only marks do not trigger a rebuild.
//! 2. `layout()` resolves geometry; a viewport change forces a full paint.
//! 3. `paint_if_dirty()`:
//!    - takes `take_motion()` (consumes the request from the previous frame),
//!    - if there is no motion and no `needs_repaint()` → `record_skipped`,
//!      return `Skipped`,
//!    - otherwise: paint (partial bbox or full), `after_paint()`, return `Painted`.
//!
//! # Aggregation
//!
//! - repeated marks of the same element: OR the flag, the element enters the queue once,
//! - many elements: one `settle`, one paint and one bbox (union) per frame,
//! - marks outside the viewport still contribute to the bbox until there is `full`/layout.

use sequoia_core::{Dirty, Rect};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameOutcome {
    Painted,
    Skipped,
}

/// Upper bound on disjoint damage regions kept for one frame.
///
/// Extra marks collapse into a single bounding box so the renderer never
/// records an unbounded number of scissored passes.
pub(crate) const MAX_DAMAGE_REGIONS: usize = 8;

#[derive(Clone, Debug)]
pub struct DamageTracker {
    // — current frame state —
    flags: Dirty,
    regions: Vec<Rect>,
    full: bool,

    // — "motion" (request_frame from the previous frame) —
    motion_pending: bool,

    // — statistics —
    frames_painted: u64,
    frames_skipped: u64,
}

impl Default for DamageTracker {
    fn default() -> Self {
        Self {
            flags: Dirty::empty(),
            regions: Vec::new(),
            full: false,
            motion_pending: false,
            frames_painted: 0,
            frames_skipped: 0,
        }
    }
}

impl DamageTracker {
    pub fn new() -> Self {
        Self::default()
    }

    // ──────────────────────────────────────────────────────────
    // Markers (called during a frame, before paint)
    // ──────────────────────────────────────────────────────────

    /// Zaznacz konkretny obszar (window coords) jako brudny.
    pub fn mark_rect(&mut self, rect: Rect) {
        if rect.is_empty() {
            return;
        }
        self.flags |= Dirty::PAINT;
        if !self.full {
            self.add_region(rect);
        }
    }

    fn add_region(&mut self, rect: Rect) {
        let mut merged = rect;
        let mut i = 0;
        while i < self.regions.len() {
            if self.regions[i].intersects(merged) {
                merged = merged.union(self.regions.remove(i));
            } else {
                i += 1;
            }
        }
        self.regions.push(merged);
        if self.regions.len() > MAX_DAMAGE_REGIONS {
            let all = self.regions.drain(..).fold(Rect::ZERO, Rect::union);
            self.regions.push(all);
        }
    }

    /// Whole viewport needs repainting (resize, DPI, theme, root mount/unmount).
    pub fn mark_full(&mut self) {
        self.full = true;
        self.flags |= Dirty::PAINT | Dirty::SURFACE;
    }

    /// The change affects measurement/layout (not just pixels).
    pub fn mark_layout(&mut self) {
        self.flags |= Dirty::LAYOUT;
    }

    /// Surowy mark (np. `Dirty::OVERLAY`, `Dirty::CANVAS`).
    pub fn mark(&mut self, flags: Dirty) {
        self.flags |= flags;
    }

    // ──────────────────────────────────────────────────────────
    // Motion (animations, spinner, scroll velocity)
    // ──────────────────────────────────────────────────────────

    /// Called by `PaintCtx::request_frame()` — "paint me in the next frame".
    pub fn request_frame(&mut self) {
        self.motion_pending = true;
    }

    /// Consumes the request from the previous frame. Called once per frame, BEFORE paint.
    pub fn take_motion(&mut self) -> bool {
        let was = self.motion_pending;
        self.motion_pending = false;
        was
    }

    // ──────────────────────────────────────────────────────────
    // Queries
    // ──────────────────────────────────────────────────────────

    /// No pixels or animation work are pending.
    ///
    /// Bookkeeping-only flags (`HIT_TEST`, `DATA`) do not count as a frame.
    pub fn is_empty(&self) -> bool {
        !self.motion_pending && !self.flags.needs_repaint()
    }

    pub fn needs_frame(&self) -> bool {
        !self.is_empty()
    }

    pub fn needs_layout(&self) -> bool {
        self.flags.needs_layout()
    }

    pub fn needs_paint(&self) -> bool {
        self.flags.needs_repaint()
    }

    pub fn flags(&self) -> Dirty {
        self.flags
    }

    /// `Some(rect)` = repaint only this area. `None` = full repaint.
    ///
    /// Layout changes can move or remove children, so they always require a
    /// full repaint; a screen-space rect alone cannot describe that damage.
    /// The 2 px margin covers anti-aliasing; the renderer adds more headroom
    /// for shadows before using the region as a scissor.
    ///
    /// This is the bounding box of [`paint_regions`](Self::paint_regions); use
    /// the individual regions when the backend supports multiple scissors.
    pub fn paint_region(&self) -> Option<Rect> {
        self.paint_regions()
            .map(|regions| regions.into_iter().fold(Rect::ZERO, Rect::union))
    }

    /// `Some(regions)` = repaint only these disjoint areas. `None` = full
    /// repaint.
    ///
    /// Each region is inflated by 2 px for anti-aliasing. Regions that overlap
    /// are merged while marks accumulate; if more than the region cap
    /// remain, they collapse into one bounding box.
    pub fn paint_regions(&self) -> Option<Vec<Rect>> {
        if self.full || self.flags.needs_layout() || self.regions.is_empty() {
            None
        } else {
            Some(
                self.regions
                    .iter()
                    .map(|rect| rect.inflate(2.0, 2.0))
                    .collect(),
            )
        }
    }

    // ──────────────────────────────────────────────────────────
    // End of frame
    // ──────────────────────────────────────────────────────────

    /// Called after paint — resets the current frame state.
    /// **Does NOT** touch `motion_pending`, because paint may have queued a new request.
    pub fn after_paint(&mut self) {
        self.flags = Dirty::empty();
        self.regions.clear();
        self.full = false;
        self.frames_painted += 1;
    }

    /// Called when a frame is skipped.
    ///
    /// Also clears bookkeeping markers (`HIT_TEST`, `DATA`) that require no
    /// pixels so they do not keep `needs_frame()` alive forever.
    pub fn record_skipped(&mut self) {
        self.flags = Dirty::empty();
        self.regions.clear();
        self.full = false;
        self.frames_skipped += 1;
    }

    /// Full reset (new root, new window).
    pub fn clear_all(&mut self) {
        self.flags = Dirty::empty();
        self.regions.clear();
        self.full = false;
        self.motion_pending = false;
    }

    // ──────────────────────────────────────────────────────────
    // Statistics
    // ──────────────────────────────────────────────────────────

    pub fn frames_painted(&self) -> u64 {
        self.frames_painted
    }

    pub fn frames_skipped(&self) -> u64 {
        self.frames_skipped
    }

    /// Fraction of skipped frames (0.0 – 1.0).
    pub fn skip_rate(&self) -> f32 {
        let total = self.frames_painted + self.frames_skipped;
        if total == 0 {
            0.0
        } else {
            self.frames_skipped as f32 / total as f32
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sequoia_core::Point;

    fn r(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect::from_xywh(x, y, w, h)
    }

    #[test]
    fn fresh_tracker_is_empty() {
        let d = DamageTracker::new();
        assert!(d.is_empty());
        assert!(!d.needs_frame());
        assert!(!d.needs_layout());
        assert!(!d.needs_paint());
        assert_eq!(d.paint_region(), None);
    }

    #[test]
    fn single_rect_marks_paint_only() {
        let mut d = DamageTracker::new();
        d.mark_rect(r(10.0, 10.0, 4.0, 4.0));
        assert!(!d.is_empty());
        assert!(!d.needs_layout(), "sam mark_rect nie zmienia layoutu");
        assert!(d.needs_paint());
        let region = d.paint_region().unwrap();
        assert!(region.contains(Point::new(12.0, 12.0)));
        assert!(region.size.width >= 4.0);
    }

    #[test]
    fn disjoint_rects_stay_separate_regions() {
        let mut d = DamageTracker::new();
        d.mark_rect(r(0.0, 0.0, 10.0, 10.0));
        d.mark_rect(r(100.0, 50.0, 4.0, 4.0));
        let regions = d.paint_regions().unwrap();
        assert_eq!(regions.len(), 2);
        assert!(regions[0].right() < regions[1].left());
        assert!(regions[0].bottom() < regions[1].top());

        let bbox = d.paint_region().unwrap();
        assert!(bbox.left() <= 0.0);
        assert!(bbox.right() >= 104.0);
        assert!(bbox.top() <= 0.0);
        assert!(bbox.bottom() >= 54.0);
    }

    #[test]
    fn overlapping_rects_merge_into_one_region() {
        let mut d = DamageTracker::new();
        d.mark_rect(r(0.0, 0.0, 10.0, 10.0));
        d.mark_rect(r(5.0, 5.0, 10.0, 10.0));
        let regions = d.paint_regions().unwrap();
        assert_eq!(regions.len(), 1);
        assert!(regions[0].contains(Point::new(12.0, 12.0)));
    }

    #[test]
    fn too_many_regions_collapse_to_bbox() {
        let mut d = DamageTracker::new();
        for i in 0..=MAX_DAMAGE_REGIONS {
            d.mark_rect(r(i as f32 * 10.0, 0.0, 1.0, 1.0));
        }
        let regions = d.paint_regions().unwrap();
        assert_eq!(regions.len(), 1, "bounded number of scissor passes");
        assert!(regions[0].right() >= MAX_DAMAGE_REGIONS as f32 * 10.0);
    }

    #[test]
    fn empty_rects_are_ignored() {
        let mut d = DamageTracker::new();
        d.mark_rect(Rect::ZERO);
        d.mark_rect(r(5.0, 5.0, 0.0, 0.0));
        assert!(d.is_empty(), "puste recty nie liczą się");
    }

    #[test]
    fn full_wins_over_rects() {
        let mut d = DamageTracker::new();
        d.mark_rect(r(0.0, 0.0, 10.0, 10.0));
        d.mark_full();
        assert_eq!(d.paint_region(), None, "full = brak partial clip");
        assert!(d.flags().contains(Dirty::SURFACE));
    }

    #[test]
    fn layout_flag_propagates() {
        let mut d = DamageTracker::new();
        d.mark_layout();
        assert!(d.needs_layout());
        assert!(d.needs_paint(), "layout implies paint");
        assert_eq!(d.paint_region(), None, "no rect = full paint");
    }

    #[test]
    fn layout_with_rect_still_requires_full_paint() {
        let mut d = DamageTracker::new();
        d.mark_rect(r(10.0, 10.0, 20.0, 20.0));
        d.mark_layout();
        assert_eq!(
            d.paint_region(),
            None,
            "layout changes can move or remove children"
        );
    }

    #[test]
    fn request_frame_forces_next_paint() {
        let mut d = DamageTracker::new();
        assert!(!d.needs_frame());
        d.request_frame();
        assert!(d.needs_frame(), "motion = ramka do namalowania");

        // take_motion consumes the request
        assert!(d.take_motion());
        assert!(!d.take_motion(), "drugie take_motion = false");
    }

    #[test]
    fn bookkeeping_flags_do_not_request_a_frame() {
        let mut d = DamageTracker::new();
        d.mark(Dirty::HIT_TEST | Dirty::DATA);
        assert!(!d.needs_paint());
        assert!(!d.needs_frame(), "hit-test/data alone do not paint pixels");

        d.record_skipped();
        assert_eq!(d.flags(), Dirty::empty());
        assert!(!d.needs_frame());
    }

    #[test]
    fn paint_without_rect_requests_full_repaint() {
        let mut d = DamageTracker::new();
        d.mark(Dirty::PAINT);
        assert!(d.needs_frame());
        assert_eq!(
            d.paint_region(),
            None,
            "unknown pixel bounds require a full repaint"
        );
    }

    #[test]
    fn after_paint_clears_rects_but_not_motion() {
        let mut d = DamageTracker::new();
        d.mark_rect(r(0.0, 0.0, 10.0, 10.0));
        d.after_paint();
        assert!(d.is_empty(), "po klatce puste");

        // paint may have set motion for the next frame
        d.request_frame();
        d.after_paint();
        assert!(d.needs_frame(), "motion z paint nie ginie");
    }

    #[test]
    fn clear_all_resets_everything() {
        let mut d = DamageTracker::new();
        d.mark_rect(r(0.0, 0.0, 10.0, 10.0));
        d.mark_layout();
        d.request_frame();
        d.clear_all();
        assert!(d.is_empty());
        assert!(!d.needs_frame());
        assert_eq!(d.paint_region(), None);
    }

    #[test]
    fn stats_track_painted_and_skipped() {
        let mut d = DamageTracker::new();
        d.mark_rect(r(0.0, 0.0, 1.0, 1.0));
        d.after_paint();
        d.record_skipped();
        d.record_skipped();
        d.mark_rect(r(0.0, 0.0, 1.0, 1.0));
        d.after_paint();
        assert_eq!(d.frames_painted(), 2);
        assert_eq!(d.frames_skipped(), 2);
        assert!((d.skip_rate() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn many_marks_coalesce_into_one_frame() {
        let mut d = DamageTracker::new();
        for i in 0..100 {
            d.mark_rect(r(i as f32, 0.0, 1.0, 1.0));
        }
        // 100 marks = one frame
        d.after_paint();
        assert_eq!(d.frames_painted(), 1);
        assert!(d.is_empty());
    }
}
