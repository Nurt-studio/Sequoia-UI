use bitflags::bitflags;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Dirty: u16 {
        const NONE      = 0;

        const PAINT     = 1 << 0;

        const LAYOUT    = 1 << 1;

        const CHILDREN  = 1 << 2;

        const TRANSFORM = 1 << 3;

        const TEXT      = 1 << 4;

        const HIT_TEST  = 1 << 5;

        const DATA      = 1 << 6;

        const CANVAS    = 1 << 7;

        const OVERLAY   = 1 << 8;

        const SURFACE   = 1 << 9;
    }
}

impl Dirty {
    pub fn needs_layout(self) -> bool {
        self.intersects(Self::LAYOUT | Self::CHILDREN | Self::TEXT)
    }

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
