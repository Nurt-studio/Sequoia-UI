//! Overflow handling modes for content that exceeds its container.

/// How a container handles content that exceeds its bounds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Overflow {
    /// Content may draw outside the container.
    #[default]
    Visible,
    /// Content outside the container is clipped.
    Hidden,
    /// Content is clipped and can be scrolled by the user.
    Scroll,
    /// Content is clipped and scrollable only when it overflows.
    Auto,
}

impl Overflow {
    /// Returns `true` if content outside the container should be clipped.
    pub fn clips(self) -> bool {
        matches!(self, Self::Hidden | Self::Scroll | Self::Auto)
    }

    /// Returns `true` if the container can be scrolled.
    pub fn is_scrollable(self) -> bool {
        matches!(self, Self::Scroll | Self::Auto)
    }

    /// Returns `true` if overflow is visible.
    pub const fn is_visible(self) -> bool {
        matches!(self, Self::Visible)
    }
}
