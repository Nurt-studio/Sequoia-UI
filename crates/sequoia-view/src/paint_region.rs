//! Culling while building quads during `paint`.
//!
//! `CullCtx` decides whether an element is visible at all for the current
//! clip (from `ScrollView`, `Container::clip(true)`, or a `Canvas` with
//! `clip_self`). If an element is entirely outside the clip, we skip:
//!
//! - `paint_self` (text shaping, gradient building, pushing to `RenderList`),
//! - recursion into children, **only if** the element clips itself
//!   (`clip_self = true`).
//!
//! This is always safe: the clip is authoritative — pixels outside it never
//! reach the image, so omitting them from the `RenderList` changes nothing.
//!
//! `RenderList` also drops fully off-screen items when they are pushed, but
//! that happens after text shaping and after the view has walked its subtree.
//! Culling at this stage saves that CPU work and the push itself.

use sequoia_core::Rect;

/// Culling configuration for `Runtime::paint_region`.
///
/// `ENABLED` is the default for `paint`, `paint_if_dirty`, and the platform.
/// `DISABLED` forces a full CPU pass; `RenderList` still clips fully
/// off-screen items at push time, so the visible output is unchanged.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CullCtx {
    enabled: bool,
    /// Margin around the element rect -- headroom for AA, shadows, and glow.
    margin: f32,
}

impl CullCtx {
    /// Culling disabled -- walk and paint the full tree.
    pub const DISABLED: Self = Self {
        enabled: false,
        margin: 0.0,
    };

    /// Culling enabled with an 8 px margin (safe for `BoxStyle` shadows).
    pub const ENABLED: Self = Self {
        enabled: true,
        margin: 8.0,
    };

    pub const fn new() -> Self {
        Self::ENABLED
    }

    pub const fn disabled() -> Self {
        Self::DISABLED
    }

    pub fn with_margin(mut self, margin: f32) -> Self {
        self.margin = if margin.is_finite() {
            margin.clamp(0.0, 64.0)
        } else {
            8.0
        };
        self
    }

    pub fn is_enabled(self) -> bool {
        self.enabled
    }

    pub fn margin(self) -> f32 {
        self.margin
    }

    /// Does `rect` (expanded by the margin) intersect `clip`?
    ///
    /// - Culling disabled → `true`.
    /// - `clip = None` (no clip on the path) → `true`.
    /// - Empty `clip` → `false` (nothing is visible).
    pub fn intersects(self, rect: Rect, clip: Option<Rect>) -> bool {
        if !self.enabled {
            return true;
        }
        let Some(clip) = clip else {
            return true;
        };
        if clip.is_empty() {
            return false;
        }
        rect.inflate(self.margin, self.margin).intersects(clip)
    }

    /// Is the whole subtree invisible?
    ///
    /// Unlike `!intersects`, this also requires the element to clip itself
    /// (`clip_self`), so its children are clipped to the parent rect too.
    /// Without `clip_self`, children may overflow (e.g. `overflow: Visible`)
    /// and must not be skipped.
    pub fn fully_clipped(self, rect: Rect, clip: Option<Rect>, clip_self: bool) -> bool {
        if !self.enabled {
            return false;
        }
        if !clip_self {
            return false;
        }
        !self.intersects(rect, clip)
    }
}

impl Default for CullCtx {
    fn default() -> Self {
        Self::ENABLED
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect::from_xywh(x, y, w, h)
    }

    #[test]
    fn disabled_never_culls() {
        let c = CullCtx::DISABLED;
        let clip = Some(r(0.0, 0.0, 100.0, 100.0));
        assert!(c.intersects(r(500.0, 500.0, 10.0, 10.0), clip));
        assert!(!c.fully_clipped(r(500.0, 500.0, 10.0, 10.0), clip, true));
    }

    #[test]
    fn no_clip_never_culls() {
        let c = CullCtx::ENABLED;
        assert!(c.intersects(r(0.0, 0.0, 1.0, 1.0), None));
    }

    #[test]
    fn empty_clip_culls_everything() {
        let c = CullCtx::ENABLED;
        assert!(!c.intersects(r(0.0, 0.0, 1.0, 1.0), Some(Rect::ZERO)));
        assert!(!c.intersects(r(0.0, 0.0, 1.0, 1.0), Some(r(5.0, 5.0, 0.0, 0.0))));
    }

    #[test]
    fn rect_inside_clip_kept() {
        let c = CullCtx::ENABLED.with_margin(0.0);
        let clip = Some(r(0.0, 0.0, 100.0, 100.0));
        assert!(c.intersects(r(10.0, 10.0, 20.0, 20.0), clip));
        assert!(
            c.intersects(r(90.0, 90.0, 20.0, 20.0), clip),
            "partial overlap"
        );
        assert!(
            c.intersects(r(-5.0, -5.0, 20.0, 20.0), clip),
            "partial overlap top-left"
        );
    }

    #[test]
    fn rect_outside_clip_culled() {
        let c = CullCtx::ENABLED.with_margin(0.0);
        let clip = Some(r(0.0, 0.0, 100.0, 100.0));
        assert!(!c.intersects(r(200.0, 200.0, 20.0, 20.0), clip));
        assert!(!c.intersects(r(200.0, 0.0, 20.0, 20.0), clip), "right");
        assert!(!c.intersects(r(0.0, -100.0, 20.0, 20.0), clip), "above");
    }

    #[test]
    fn margin_expands_the_visible_band() {
        let clip = Some(r(0.0, 0.0, 100.0, 100.0));
        // 2 px past the right edge — outside the clip, but close.
        let rect = r(102.0, 10.0, 20.0, 20.0);
        assert!(!CullCtx::ENABLED.with_margin(0.0).intersects(rect, clip));
        assert!(
            CullCtx::ENABLED.with_margin(8.0).intersects(rect, clip),
            "shadow/glow on the border must survive"
        );
    }

    #[test]
    fn fully_clipped_requires_clip_self() {
        let c = CullCtx::ENABLED.with_margin(0.0);
        let clip = Some(r(0.0, 0.0, 100.0, 100.0));
        let far = r(200.0, 200.0, 10.0, 10.0);
        assert!(!c.fully_clipped(far, clip, false), "children may overflow");
        assert!(c.fully_clipped(far, clip, true), "clipping element — safe");
    }

    #[test]
    fn margin_sanitizes() {
        assert_eq!(CullCtx::ENABLED.with_margin(-5.0).margin(), 0.0);
        assert_eq!(CullCtx::ENABLED.with_margin(f32::NAN).margin(), 8.0);
        assert_eq!(CullCtx::ENABLED.with_margin(1e6).margin(), 64.0);
    }
}
