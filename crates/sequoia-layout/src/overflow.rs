#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Overflow {
    #[default]
    Visible,
    Hidden,

    Scroll,

    Auto,
}

impl Overflow {
    pub fn clips(self) -> bool {
        matches!(self, Self::Hidden | Self::Scroll | Self::Auto)
    }

    pub fn is_scrollable(self) -> bool {
        matches!(self, Self::Scroll | Self::Auto)
    }

    pub const fn is_visible(self) -> bool {
        matches!(self, Self::Visible)
    }
}
