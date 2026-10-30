use bitflags::bitflags;

bitflags! {
    /// What changed for one element.
    ///
    /// Layout flags (`LAYOUT`, `CHILDREN`, `TEXT`) require a rebuild and a
    /// full repaint. Paint flags (`PAINT`, `TRANSFORM`, `CANVAS`, `OVERLAY`,
    /// `SURFACE`) only require painting; `PAINT` alone uses the element bounds
    /// for a partial repaint. `HIT_TEST` and `DATA` are bookkeeping flags and
    /// never request a frame by themselves.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Dirty: u16 {
        const NONE      = 0;

        /// Pixel-only change (hover, focus, press, direct repaint request).
        const PAINT     = 1 << 0;

        /// Measure/arrange may change; implies rebuild and full repaint.
        const LAYOUT    = 1 << 1;

        /// Child set may change; implies rebuild and full repaint.
        const CHILDREN  = 1 << 2;

        /// Transform or clip changed; repaint without rebuilding.
        const TRANSFORM = 1 << 3;

        /// Text content or shaping changed; implies rebuild and full repaint.
        const TEXT      = 1 << 4;

        /// Pointer hit-test results may change; no repaint by itself.
        const HIT_TEST  = 1 << 5;

        /// Application data changed; add `PAINT` if pixels depend on it.
        const DATA      = 1 << 6;

        /// Canvas display list changed; repaint without rebuilding.
        const CANVAS    = 1 << 7;

        /// Overlay stack changed; repaint without rebuilding.
        const OVERLAY   = 1 << 8;

        /// Embedded surface content changed; repaint without rebuilding.
        const SURFACE   = 1 << 9;
    }
}

impl Dirty {
    /// The element must run `body`/children reconciliation before layout.
    pub fn needs_layout(self) -> bool {
        self.intersects(Self::LAYOUT | Self::CHILDREN | Self::TEXT)
    }

    /// A frame must paint pixels for this change.
    ///
    /// Layout changes are included because they can move or remove children.
    pub fn needs_repaint(self) -> bool {
        self.intersects(
            Self::PAINT
                | Self::LAYOUT
                | Self::CHILDREN
                | Self::TRANSFORM
                | Self::TEXT
                | Self::CANVAS
                | Self::OVERLAY
                | Self::SURFACE,
        )
    }

    /// Pointer hit testing must be recomputed (it is evaluated on demand).
    pub fn needs_hit_test(self) -> bool {
        self.intersects(Self::HIT_TEST | Self::CHILDREN | Self::LAYOUT | Self::TRANSFORM)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_is_clean() {
        let d = Dirty::empty();
        assert!(!d.needs_layout());
        assert!(!d.needs_repaint());
        assert!(!d.needs_hit_test());
    }

    #[test]
    fn body_change_needs_layout() {
        let d = Dirty::LAYOUT | Dirty::CHILDREN;
        assert!(d.needs_layout());
        assert!(d.needs_repaint());
    }

    #[test]
    fn hover_needs_only_repaint() {
        let d = Dirty::PAINT;
        assert!(!d.needs_layout());
        assert!(d.needs_repaint());
        assert!(!d.needs_hit_test());
    }

    #[test]
    fn flags_accumulate() {
        let mut d = Dirty::empty();
        d |= Dirty::PAINT;
        d |= Dirty::TEXT;
        assert!(d.needs_layout());
        assert!(d.needs_repaint());
    }
}
