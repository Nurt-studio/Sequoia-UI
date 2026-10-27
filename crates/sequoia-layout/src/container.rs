//! Container axes, alignment, justification, and style builders.

use crate::dock::sanitize_gap;

/// Main layout axis of a container.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    /// Children are placed left to right.
    Horizontal,
    /// Children are placed top to bottom.
    Vertical,
}

impl Axis {
    /// Returns `true` for [`Axis::Horizontal`].
    pub const fn is_horizontal(self) -> bool {
        matches!(self, Self::Horizontal)
    }

    /// Returns `true` for [`Axis::Vertical`].
    pub const fn is_vertical(self) -> bool {
        matches!(self, Self::Vertical)
    }

    /// Splits `width`/`height` into main and cross sizes for this axis.
    pub fn split_main_cross(self, width: f32, height: f32) -> (f32, f32) {
        match self {
            Self::Horizontal => (width, height),
            Self::Vertical => (height, width),
        }
    }

    /// Combines main and cross sizes back into `width`/`height`.
    pub fn merge_main_cross(self, main: f32, cross: f32) -> (f32, f32) {
        match self {
            Self::Horizontal => (main, cross),
            Self::Vertical => (cross, main),
        }
    }
}

/// Distribution of free space along a container's main axis.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Justify {
    /// Pack items at the start of the line.
    #[default]
    Start,
    /// Center items as a group.
    Center,
    /// Pack items at the end of the line.
    End,
    /// Spread items so the first and last touch the edges.
    SpaceBetween,
    /// Give every item equal space on both sides.
    SpaceAround,
    /// Give every item equal space, including the outer edges.
    SpaceEvenly,
}

impl Justify {
    /// Computes the leading offset and the gap added between items.
    ///
    /// `extra` is the free space to distribute and is clamped to zero for
    /// invalid values.
    pub fn distribute(self, count: usize, _base_gap: f32, extra: f32) -> (f32, f32) {
        let extra = if extra.is_finite() {
            extra.max(0.0)
        } else {
            0.0
        };
        match self {
            Self::Start => (0.0, 0.0),
            Self::Center => (extra * 0.5, 0.0),
            Self::End => (extra, 0.0),
            Self::SpaceBetween => {
                if count > 1 {
                    (0.0, extra / (count - 1) as f32)
                } else if count == 1 {
                    (extra * 0.5, 0.0)
                } else {
                    (0.0, 0.0)
                }
            }
            Self::SpaceAround => {
                if count > 0 {
                    let step = extra / count as f32;
                    (step * 0.5, step)
                } else {
                    (0.0, 0.0)
                }
            }
            Self::SpaceEvenly => {
                if count > 0 {
                    let step = extra / (count + 1) as f32;
                    (step, step)
                } else {
                    (0.0, 0.0)
                }
            }
        }
    }

    /// Returns only the leading offset produced by [`Justify::distribute`].
    pub fn start_offset(self, extra: f32) -> f32 {
        let (start, _) = self.distribute(0, 0.0, extra);
        match self {
            Self::Start | Self::Center | Self::End => start,
            _ => start,
        }
    }
}

/// Alignment of children on a container's cross axis.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Align {
    /// Align children at the start of the cross axis.
    #[default]
    Start,
    /// Center children on the cross axis.
    Center,
    /// Align children at the end of the cross axis.
    End,
    /// Stretch children to fill the cross axis.
    Stretch,
    /// Align children on their first text baseline (horizontal rows only).
    Baseline,
}

impl Align {
    /// Returns `true` for [`Align::Stretch`].
    pub const fn is_stretch(self) -> bool {
        matches!(self, Self::Stretch)
    }

    /// Returns the cross-axis offset of a `child` inside `avail`.
    ///
    /// Stretch and start both produce zero; non-finite space produces zero.
    pub fn offset(self, avail: f32, child: f32) -> f32 {
        if !avail.is_finite() {
            return 0.0;
        }
        let slack = (avail - child).max(0.0);
        match self {
            Self::Start | Self::Stretch | Self::Baseline => 0.0,
            Self::Center => slack * 0.5,
            Self::End => slack,
        }
    }

    /// Uses the child alignment when set, otherwise the container alignment.
    pub fn resolve(child: Option<Align>, container: Align) -> Align {
        child.unwrap_or(container)
    }
}

/// Layout style of a container that arranges its children on an axis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContainerStyle {
    /// Main axis along which children are placed.
    pub axis: Axis,
    /// Gap between children along the main axis.
    pub gap: f32,
    /// Gap between wrapped lines or cross-axis tracks.
    pub cross_gap: f32,
    /// Default cross-axis alignment for children.
    pub align: Align,
    /// Main-axis distribution of free space.
    pub justify: Justify,
    /// Whether children wrap onto multiple lines.
    pub wrap: bool,
    /// Cross-axis alignment of wrapped lines within the container.
    pub align_content: Align,
}

impl Default for ContainerStyle {
    fn default() -> Self {
        Self {
            axis: Axis::Horizontal,
            gap: 0.0,
            cross_gap: 0.0,
            align: Align::Start,
            justify: Justify::Start,
            wrap: false,
            align_content: Align::Start,
        }
    }
}

impl ContainerStyle {
    /// Creates a style for `axis` with equal main and cross gaps.
    pub fn new(axis: Axis, gap: f32) -> Self {
        let gap = sanitize_gap(gap);
        Self {
            axis,
            gap,
            cross_gap: gap,
            ..Self::default()
        }
    }

    /// Sets both the main and cross gaps.
    pub fn gap(mut self, gap: f32) -> Self {
        let gap = sanitize_gap(gap);
        self.gap = gap;
        self.cross_gap = gap;
        self
    }

    /// Sets the main-axis gap.
    pub fn main_gap(mut self, gap: f32) -> Self {
        self.gap = sanitize_gap(gap);
        self
    }

    /// Sets the cross-axis gap.
    pub fn cross_gap(mut self, gap: f32) -> Self {
        self.cross_gap = sanitize_gap(gap);
        self
    }

    /// Sets the main and cross gaps independently.
    pub fn gaps(mut self, main: f32, cross: f32) -> Self {
        self.gap = sanitize_gap(main);
        self.cross_gap = sanitize_gap(cross);
        self
    }

    /// Sets the default cross-axis alignment of children.
    pub fn align_items(mut self, align: Align) -> Self {
        self.align = align;
        self
    }

    /// Sets the main-axis distribution of free space.
    pub fn justify_content(mut self, justify: Justify) -> Self {
        self.justify = justify;
        self
    }

    /// Enables or disables wrapping.
    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }

    /// Enables wrapping.
    pub fn wrapped(mut self) -> Self {
        self.wrap = true;
        self
    }

    /// Sets the cross-axis alignment of wrapped lines.
    pub fn align_content(mut self, align: Align) -> Self {
        self.align_content = align;
        self
    }

    /// Sets the main axis.
    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }

    /// Returns the total main-axis gap for `count` children.
    pub fn main_gaps_total(self, count: usize) -> f32 {
        if count > 1 {
            self.gap * (count - 1) as f32
        } else {
            0.0
        }
    }
}
