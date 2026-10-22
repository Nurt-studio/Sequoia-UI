//! Per-view layout styles and value sanitizers.

use crate::container::{Align, Axis};
use crate::overflow::Overflow;
use sequoia_core::{Edges, Length, Size};

/// Positioning mode for a view.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[non_exhaustive]
pub enum Position {
    /// Participate in normal flow.
    #[default]
    Flow,
    /// Position relative to the container's padding box.
    Absolute(Inset),
}

/// Absolute offsets from each edge of the containing block.
///
/// Each field is optional; `None` means "auto" and lets the layout choose.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct Inset {
    /// Offset from the left edge, or `None` for auto.
    pub left: Option<f32>,
    /// Offset from the top edge, or `None` for auto.
    pub top: Option<f32>,
    /// Offset from the right edge, or `None` for auto.
    pub right: Option<f32>,
    /// Offset from the bottom edge, or `None` for auto.
    pub bottom: Option<f32>,
}

impl Inset {
    /// An inset with every side set to auto.
    pub const AUTO: Self = Self {
        left: None,
        top: None,
        right: None,
        bottom: None,
    };

    /// Creates an inset with explicit left and top offsets.
    pub fn xy(left: f32, top: f32) -> Self {
        Self {
            left: Some(left),
            top: Some(top),
            right: None,
            bottom: None,
        }
    }

    /// Builds an inset from each side, `None` meaning "auto".
    pub const fn only(
        left: Option<f32>,
        top: Option<f32>,
        right: Option<f32>,
        bottom: Option<f32>,
    ) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }
}

/// Optional indents from each edge of a container's content box.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Indents {
    /// Indent from the left edge, or `None` to use the default.
    pub left: Option<f32>,
    /// Indent from the top edge, or `None` to use the default.
    pub top: Option<f32>,
    /// Indent from the right edge, or `None` to use the default.
    pub right: Option<f32>,
    /// Indent from the bottom edge, or `None` to use the default.
    pub bottom: Option<f32>,
}

impl Indents {
    /// Indents that leave every side unset.
    pub const NONE: Self = Self {
        left: None,
        top: None,
        right: None,
        bottom: None,
    };

    /// Sets the same indent on every side.
    pub fn all(v: f32) -> Self {
        let v = sanitize_indent(v);
        Self {
            left: Some(v),
            top: Some(v),
            right: Some(v),
            bottom: Some(v),
        }
    }

    /// Sets horizontal and vertical indents.
    pub fn xy(x: f32, y: f32) -> Self {
        Self {
            left: Some(sanitize_indent(x)),
            top: Some(sanitize_indent(y)),
            right: Some(sanitize_indent(x)),
            bottom: Some(sanitize_indent(y)),
        }
    }

    /// Sets horizontal and vertical indents.
    pub fn axes(horizontal: f32, vertical: f32) -> Self {
        Self::xy(horizontal, vertical)
    }

    /// Sets the left and right indents.
    pub fn horizontal(v: f32) -> Self {
        let v = sanitize_indent(v);
        Self {
            left: Some(v),
            right: Some(v),
            top: None,
            bottom: None,
        }
    }

    /// Sets the top and bottom indents.
    pub fn vertical(v: f32) -> Self {
        let v = sanitize_indent(v);
        Self {
            left: None,
            right: None,
            top: Some(v),
            bottom: Some(v),
        }
    }

    /// Builds indents from each side, `None` meaning unset.
    pub const fn only(
        left: Option<f32>,
        top: Option<f32>,
        right: Option<f32>,
        bottom: Option<f32>,
    ) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    /// Returns `true` if no side has an indent.
    pub fn is_none(self) -> bool {
        self.left.is_none() && self.top.is_none() && self.right.is_none() && self.bottom.is_none()
    }
}

/// Layout properties for a single view.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayoutStyle {
    /// Requested width.
    pub width: Length,
    /// Requested height.
    pub height: Length,
    /// Space between the border and the content.
    pub padding: Edges,
    /// Space around the view outside its border.
    pub margin: Edges,
    /// Lower bound for the resolved width.
    pub min_width: f32,
    /// Upper bound for the resolved width.
    pub max_width: f32,
    /// Lower bound for the resolved height.
    pub min_height: f32,
    /// Upper bound for the resolved height.
    pub max_height: f32,
    /// Grow weight along the parent's main axis.
    pub flex: f32,
    /// Shrink weight along the parent's main axis.
    pub flex_shrink: f32,
    /// Optional basis used before flex distribution.
    pub flex_basis: Option<Length>,
    /// Cross-axis alignment override; `None` inherits from the container.
    pub align_self: Option<Align>,
    /// Preferred width-to-height ratio.
    pub aspect_ratio: Option<f32>,
    /// Extra indent from the top edge of a docked container.
    pub indent_top: Option<f32>,
    /// Extra indent from the bottom edge of a docked container.
    pub indent_bottom: Option<f32>,
    /// Extra indent from the left edge of a docked container.
    pub indent_left: Option<f32>,
    /// Extra indent from the right edge of a docked container.
    pub indent_right: Option<f32>,
    /// Positioning mode.
    pub position: Position,
    /// Overflow handling for content that exceeds the view.
    pub overflow: Overflow,
    /// Whether the view participates in hit testing.
    pub hit_testable: bool,
}

impl Default for LayoutStyle {
    fn default() -> Self {
        Self {
            width: Length::Fit,
            height: Length::Fit,
            padding: Edges::ZERO,
            margin: Edges::ZERO,
            min_width: 0.0,
            max_width: f32::INFINITY,
            min_height: 0.0,
            max_height: f32::INFINITY,
            flex: 0.0,
            flex_shrink: 1.0,
            flex_basis: None,
            align_self: None,
            aspect_ratio: None,
            indent_top: None,
            indent_bottom: None,
            indent_left: None,
            indent_right: None,
            position: Position::Flow,
            overflow: Overflow::Visible,
            hit_testable: true,
        }
    }
}

impl LayoutStyle {
    /// Sets the requested width.
    pub fn width(mut self, width: Length) -> Self {
        self.width = width;
        self
    }

    /// Sets the requested height.
    pub fn height(mut self, height: Length) -> Self {
        self.height = height;
        self
    }

    /// Sets both dimensions to fit their content.
    pub fn fit(mut self) -> Self {
        self.width = Length::Fit;
        self.height = Length::Fit;
        self
    }

    /// Enables or disables hit testing for this view.
    pub fn hit_testable(mut self, on: bool) -> Self {
        self.hit_testable = on;
        self
    }

    /// Sets the padding on every side.
    pub fn padding(mut self, padding: impl Into<Edges>) -> Self {
        self.padding = padding.into();
        self
    }

    /// Alias for [`LayoutStyle::padding`].
    pub fn pad(self, padding: impl Into<Edges>) -> Self {
        self.padding(padding)
    }

    /// Sets the horizontal and vertical padding.
    pub fn padding_xy(self, x: f32, y: f32) -> Self {
        self.padding(Edges::xy(x, y))
    }

    /// Sets the left and right padding.
    pub fn padding_x(mut self, x: f32) -> Self {
        self.padding.left = x;
        self.padding.right = x;
        self
    }

    /// Sets the top and bottom padding.
    pub fn padding_y(mut self, y: f32) -> Self {
        self.padding.top = y;
        self.padding.bottom = y;
        self
    }

    /// Sets the left padding.
    pub fn padding_left(mut self, v: f32) -> Self {
        self.padding.left = v;
        self
    }

    /// Sets the right padding.
    pub fn padding_right(mut self, v: f32) -> Self {
        self.padding.right = v;
        self
    }

    /// Sets the top padding.
    pub fn padding_top(mut self, v: f32) -> Self {
        self.padding.top = v;
        self
    }

    /// Sets the bottom padding.
    pub fn padding_bottom(mut self, v: f32) -> Self {
        self.padding.bottom = v;
        self
    }

    /// Sets the minimum width.
    pub fn min_width(mut self, v: f32) -> Self {
        self.min_width = sanitize_min(v);
        self
    }

    /// Sets the maximum width.
    pub fn max_width(mut self, v: f32) -> Self {
        self.max_width = sanitize_max(v);
        self
    }

    /// Sets the minimum height.
    pub fn min_height(mut self, v: f32) -> Self {
        self.min_height = sanitize_min(v);
        self
    }

    /// Sets the maximum height.
    pub fn max_height(mut self, v: f32) -> Self {
        self.max_height = sanitize_max(v);
        self
    }

    /// Clamps a width to this style's minimum and maximum.
    pub fn clamp_width(self, w: f32) -> f32 {
        clamp_axis(w, self.min_width, self.max_width)
    }

    /// Clamps a height to this style's minimum and maximum.
    pub fn clamp_height(self, h: f32) -> f32 {
        clamp_axis(h, self.min_height, self.max_height)
    }

    /// Clamps a size to this style's minimum and maximum.
    pub fn clamp_size(self, size: sequoia_core::Size) -> sequoia_core::Size {
        sequoia_core::Size::new(self.clamp_width(size.width), self.clamp_height(size.height))
    }

    /// Sets the margin on every side.
    pub fn margin(mut self, margin: impl Into<Edges>) -> Self {
        let m = margin.into();
        self.margin = Edges::only(
            sanitize_edge(m.left),
            sanitize_edge(m.top),
            sanitize_edge(m.right),
            sanitize_edge(m.bottom),
        );
        self
    }

    /// Sets the horizontal and vertical margin.
    pub fn margin_xy(self, x: f32, y: f32) -> Self {
        self.margin(Edges::xy(x, y))
    }

    /// Sets the left and right margin.
    pub fn margin_x(mut self, x: f32) -> Self {
        let x = sanitize_edge(x);
        self.margin.left = x;
        self.margin.right = x;
        self
    }

    /// Sets the top and bottom margin.
    pub fn margin_y(mut self, y: f32) -> Self {
        let y = sanitize_edge(y);
        self.margin.top = y;
        self.margin.bottom = y;
        self
    }

    /// Sets the left margin.
    pub fn margin_left(mut self, v: f32) -> Self {
        self.margin.left = sanitize_edge(v);
        self
    }

    /// Sets the right margin.
    pub fn margin_right(mut self, v: f32) -> Self {
        self.margin.right = sanitize_edge(v);
        self
    }

    /// Sets the top margin.
    pub fn margin_top(mut self, v: f32) -> Self {
        self.margin.top = sanitize_edge(v);
        self
    }

    /// Sets the bottom margin.
    pub fn margin_bottom(mut self, v: f32) -> Self {
        self.margin.bottom = sanitize_edge(v);
        self
    }

    /// Sets the flex grow weight.
    pub fn flex(mut self, weight: f32) -> Self {
        self.flex = sanitize_flex(weight);
        self
    }

    /// Sets the flex grow weight.
    pub fn flex_grow(mut self, weight: f32) -> Self {
        self.flex = sanitize_flex(weight);
        self
    }

    /// Sets the flex shrink weight.
    pub fn flex_shrink(mut self, shrink: f32) -> Self {
        self.flex_shrink = sanitize_shrink(shrink);
        self
    }

    /// Sets the grow weight, shrink weight, and basis together.
    pub fn flex_full(mut self, grow: f32, shrink: f32, basis: Option<Length>) -> Self {
        self.flex = sanitize_flex(grow);
        self.flex_shrink = sanitize_shrink(shrink);
        self.flex_basis = basis;
        self
    }

    /// Sets the flex basis.
    pub fn flex_basis(mut self, basis: Option<Length>) -> Self {
        self.flex_basis = basis;
        self
    }

    /// Clears the flex basis.
    pub fn no_basis(mut self) -> Self {
        self.flex_basis = None;
        self
    }

    /// Sets the flex grow weight to one.
    pub fn expand(self) -> Self {
        self.flex(1.0)
    }

    /// Returns the sanitized flex grow weight.
    pub fn flex_weight(self) -> f32 {
        sanitize_flex(self.flex)
    }

    /// Returns the sanitized flex grow weight.
    pub fn flex_grow_weight(self) -> f32 {
        sanitize_flex(self.flex)
    }

    /// Returns the sanitized flex shrink weight.
    pub fn flex_shrink_weight(self) -> f32 {
        sanitize_shrink(self.flex_shrink)
    }

    /// Sets the cross-axis alignment override.
    pub fn align_self(mut self, align: Option<Align>) -> Self {
        self.align_self = align;
        self
    }

    /// Sets an explicit cross-axis alignment override.
    pub fn align_self_value(mut self, align: Align) -> Self {
        self.align_self = Some(align);
        self
    }

    /// Clears the alignment override so the container's alignment is used.
    pub fn auto_align(mut self) -> Self {
        self.align_self = None;
        self
    }

    /// Resolves the effective alignment against a container alignment.
    pub fn effective_align(self, container: Align) -> Align {
        Align::resolve(self.align_self, container)
    }

    /// Sets the aspect ratio, ignoring invalid values.
    pub fn aspect_ratio(mut self, ratio: Option<f32>) -> Self {
        self.aspect_ratio = sanitize_ratio(ratio);
        self
    }

    /// Sets the aspect ratio to `ratio`.
    pub fn aspect(mut self, ratio: f32) -> Self {
        self.aspect_ratio = sanitize_ratio(Some(ratio));
        self
    }

    /// Clears the aspect ratio.
    pub fn no_aspect(mut self) -> Self {
        self.aspect_ratio = None;
        self
    }

    /// Sets the positioning mode.
    pub fn position(mut self, position: Position) -> Self {
        self.position = position;
        self
    }

    /// Positions the view absolutely with auto insets.
    pub fn absolute(mut self) -> Self {
        self.position = Position::Absolute(Inset::AUTO);
        self
    }

    /// Positions the view absolutely at the given left and top offsets.
    pub fn absolute_at(mut self, left: f32, top: f32) -> Self {
        self.position = Position::Absolute(Inset::xy(left, top));
        self
    }

    /// Positions the view absolutely with the given insets.
    pub fn absolute_inset(mut self, inset: Inset) -> Self {
        self.position = Position::Absolute(inset);
        self
    }

    /// Returns `true` if the view is positioned absolutely.
    pub fn is_absolute(self) -> bool {
        matches!(self.position, Position::Absolute(_))
    }

    /// Returns the absolute insets, or `None` for flow positioning.
    pub fn absolute_inset_of(self) -> Option<Inset> {
        match self.position {
            Position::Absolute(inset) => Some(inset),
            _ => None,
        }
    }

    /// Returns the horizontal or vertical margin for the given axis.
    pub fn margin_main(self, axis: Axis) -> f32 {
        match axis {
            Axis::Horizontal => self.margin.left + self.margin.right,
            Axis::Vertical => self.margin.top + self.margin.bottom,
        }
    }

    /// Returns the cross-axis margin for the given main axis.
    pub fn margin_cross(self, axis: Axis) -> f32 {
        match axis {
            Axis::Horizontal => self.margin.top + self.margin.bottom,
            Axis::Vertical => self.margin.left + self.margin.right,
        }
    }

    /// Applies the aspect ratio to the size, deriving either auto axis.
    pub fn apply_aspect(self, mut w: f32, mut h: f32, w_auto: bool, h_auto: bool) -> (f32, f32) {
        let Some(r) = sanitize_ratio(self.aspect_ratio) else {
            return (w, h);
        };
        if r <= 0.0 {
            return (w, h);
        }
        match (w_auto, h_auto) {
            (true, false) => {
                w = (h * r).max(0.0);
            }
            (false, true) => {
                h = if r != 0.0 { (w / r).max(0.0) } else { h };
            }
            (true, true) => {
                if w <= 0.0 && h > 0.0 {
                    w = (h * r).max(0.0);
                } else if h <= 0.0 && w > 0.0 {
                    h = (w / r).max(0.0);
                }
            }
            (false, false) => {}
        }
        (w, h)
    }

    /// Expands an inner size by this style's margins.
    pub fn outer_size(self, inner: Size) -> Size {
        Size::new(
            inner.width + self.margin.horizontal_sum(),
            inner.height + self.margin.vertical_sum(),
        )
    }

    /// Returns the minimum size along the given axis.
    pub fn min_main(self, axis: Axis) -> f32 {
        sanitize_min(match axis {
            Axis::Horizontal => self.min_width,
            Axis::Vertical => self.min_height,
        })
    }

    /// Returns the maximum size along the given axis.
    pub fn max_main(self, axis: Axis) -> f32 {
        sanitize_max(match axis {
            Axis::Horizontal => self.max_width,
            Axis::Vertical => self.max_height,
        })
    }

    /// Clamps a value to the minimum and maximum along the given axis.
    pub fn clamp_main(self, value: f32, axis: Axis) -> f32 {
        clamp_axis(value, self.min_main(axis), self.max_main(axis))
    }

    /// Sets the overflow behavior.
    pub fn overflow(mut self, overflow: Overflow) -> Self {
        self.overflow = overflow;
        self
    }

    /// Sets the top indent.
    pub fn indent_top(mut self, v: f32) -> Self {
        self.indent_top = Some(sanitize_indent(v));
        self
    }

    /// Sets the bottom indent.
    pub fn indent_bottom(mut self, v: f32) -> Self {
        self.indent_bottom = Some(sanitize_indent(v));
        self
    }

    /// Sets the left indent.
    pub fn indent_left(mut self, v: f32) -> Self {
        self.indent_left = Some(sanitize_indent(v));
        self
    }

    /// Sets the right indent.
    pub fn indent_right(mut self, v: f32) -> Self {
        self.indent_right = Some(sanitize_indent(v));
        self
    }

    /// Clears the top indent.
    pub fn clear_indent_top(mut self) -> Self {
        self.indent_top = None;
        self
    }

    /// Clears the bottom indent.
    pub fn clear_indent_bottom(mut self) -> Self {
        self.indent_bottom = None;
        self
    }

    /// Clears the left indent.
    pub fn clear_indent_left(mut self) -> Self {
        self.indent_left = None;
        self
    }

    /// Clears the right indent.
    pub fn clear_indent_right(mut self) -> Self {
        self.indent_right = None;
        self
    }

    /// Clears every indent.
    pub fn clear_indents(mut self) -> Self {
        self.indent_top = None;
        self.indent_bottom = None;
        self.indent_left = None;
        self.indent_right = None;
        self
    }

    /// Applies the set sides from `indents`.
    pub fn indent(mut self, indents: Indents) -> Self {
        if let Some(v) = indents.top {
            self.indent_top = Some(sanitize_indent(v));
        }
        if let Some(v) = indents.bottom {
            self.indent_bottom = Some(sanitize_indent(v));
        }
        if let Some(v) = indents.left {
            self.indent_left = Some(sanitize_indent(v));
        }
        if let Some(v) = indents.right {
            self.indent_right = Some(sanitize_indent(v));
        }
        self
    }

    /// Sets the same indent on every side.
    pub fn indent_all(mut self, v: f32) -> Self {
        let v = sanitize_indent(v);
        self.indent_top = Some(v);
        self.indent_bottom = Some(v);
        self.indent_left = Some(v);
        self.indent_right = Some(v);
        self
    }

    /// Sets the horizontal and vertical indents.
    pub fn indent_xy(mut self, x: f32, y: f32) -> Self {
        let x = sanitize_indent(x);
        let y = sanitize_indent(y);
        self.indent_left = Some(x);
        self.indent_right = Some(x);
        self.indent_top = Some(y);
        self.indent_bottom = Some(y);
        self
    }

    /// Sets the horizontal and vertical indents.
    pub fn indent_axes(self, horizontal: f32, vertical: f32) -> Self {
        self.indent_xy(horizontal, vertical)
    }

    /// Returns the indent at the main-axis start for the given axis.
    pub fn indent_main_start(self, axis: Axis) -> Option<f32> {
        match axis {
            Axis::Horizontal => self.indent_left,
            Axis::Vertical => self.indent_top,
        }
    }

    /// Returns the indent at the main-axis end for the given axis.
    pub fn indent_main_end(self, axis: Axis) -> Option<f32> {
        match axis {
            Axis::Horizontal => self.indent_right,
            Axis::Vertical => self.indent_bottom,
        }
    }

    /// Returns the indent at the cross-axis start for the given main axis.
    pub fn indent_cross_start(self, axis: Axis) -> Option<f32> {
        match axis {
            Axis::Horizontal => self.indent_top,
            Axis::Vertical => self.indent_left,
        }
    }

    /// Returns the indent at the cross-axis end for the given main axis.
    pub fn indent_cross_end(self, axis: Axis) -> Option<f32> {
        match axis {
            Axis::Horizontal => self.indent_bottom,
            Axis::Vertical => self.indent_right,
        }
    }

    /// Collects the four optional indents.
    pub fn indents(self) -> Indents {
        Indents {
            left: self.indent_left,
            top: self.indent_top,
            right: self.indent_right,
            bottom: self.indent_bottom,
        }
    }
}

/// Clamps a flex grow weight to a finite, non-negative value.
pub fn sanitize_flex(v: f32) -> f32 {
    if v.is_finite() && v > 0.0 { v } else { 0.0 }
}

/// Clamps a flex shrink weight; NaN becomes one and invalid values zero.
pub fn sanitize_shrink(v: f32) -> f32 {
    if v.is_finite() && v >= 0.0 {
        v
    } else if v.is_nan() {
        1.0
    } else {
        0.0
    }
}

/// Keeps an aspect ratio only when it is finite and positive.
pub fn sanitize_ratio(v: Option<f32>) -> Option<f32> {
    match v {
        Some(r) if r.is_finite() && r > 0.0 => Some(r),
        _ => None,
    }
}

fn sanitize_edge(v: f32) -> f32 {
    if v.is_finite() { v.max(0.0) } else { 0.0 }
}

/// Clamps an indent to a finite, non-negative value.
pub fn sanitize_indent(v: f32) -> f32 {
    if v.is_finite() { v.max(0.0) } else { 0.0 }
}

fn sanitize_min(v: f32) -> f32 {
    if !v.is_finite() || v < 0.0 { 0.0 } else { v }
}

fn sanitize_max(v: f32) -> f32 {
    if v.is_nan() {
        f32::INFINITY
    } else if v.is_infinite() {
        if v > 0.0 { f32::INFINITY } else { 0.0 }
    } else {
        v.max(0.0)
    }
}

fn clamp_axis(value: f32, min: f32, max: f32) -> f32 {
    let (min, max) = if min > max { (max, max) } else { (min, max) };
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}
