use crate::container::{Align, Axis};
use crate::overflow::Overflow;
use sequoia_core::{Edges, Length, Size};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[non_exhaustive]
pub enum Position {
    #[default]
    Flow,

    Absolute(Inset),
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct Inset {
    pub left: Option<f32>,
    pub top: Option<f32>,
    pub right: Option<f32>,
    pub bottom: Option<f32>,
}

impl Inset {
    pub const AUTO: Self = Self {
        left: None,
        top: None,
        right: None,
        bottom: None,
    };

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

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Indents {
    pub left: Option<f32>,
    pub top: Option<f32>,
    pub right: Option<f32>,
    pub bottom: Option<f32>,
}

impl Indents {
    pub const NONE: Self = Self {
        left: None,
        top: None,
        right: None,
        bottom: None,
    };

    pub fn all(v: f32) -> Self {
        let v = sanitize_indent(v);
        Self {
            left: Some(v),
            top: Some(v),
            right: Some(v),
            bottom: Some(v),
        }
    }

    pub fn xy(x: f32, y: f32) -> Self {
        Self {
            left: Some(sanitize_indent(x)),
            top: Some(sanitize_indent(y)),
            right: Some(sanitize_indent(x)),
            bottom: Some(sanitize_indent(y)),
        }
    }

    pub fn axes(horizontal: f32, vertical: f32) -> Self {
        Self::xy(horizontal, vertical)
    }

    pub fn horizontal(v: f32) -> Self {
        let v = sanitize_indent(v);
        Self {
            left: Some(v),
            right: Some(v),
            top: None,
            bottom: None,
        }
    }

    pub fn vertical(v: f32) -> Self {
        let v = sanitize_indent(v);
        Self {
            left: None,
            right: None,
            top: Some(v),
            bottom: Some(v),
        }
    }

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

    pub fn is_none(self) -> bool {
        self.left.is_none() && self.top.is_none() && self.right.is_none() && self.bottom.is_none()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayoutStyle {
    pub width: Length,
    pub height: Length,
    pub padding: Edges,

    pub margin: Edges,
    pub min_width: f32,
    pub max_width: f32,
    pub min_height: f32,
    pub max_height: f32,

    pub flex: f32,

    pub flex_shrink: f32,

    pub flex_basis: Option<Length>,

    pub align_self: Option<Align>,

    pub aspect_ratio: Option<f32>,

    pub indent_top: Option<f32>,
    pub indent_bottom: Option<f32>,
    pub indent_left: Option<f32>,
    pub indent_right: Option<f32>,

    pub position: Position,
    pub overflow: Overflow,

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
    pub fn width(mut self, width: Length) -> Self {
        self.width = width;
        self
    }

    pub fn height(mut self, height: Length) -> Self {
        self.height = height;
        self
    }

    pub fn fit(mut self) -> Self {
        self.width = Length::Fit;
        self.height = Length::Fit;
        self
    }

    pub fn hit_testable(mut self, on: bool) -> Self {
        self.hit_testable = on;
        self
    }

    pub fn padding(mut self, padding: impl Into<Edges>) -> Self {
        self.padding = padding.into();
        self
    }

    pub fn pad(self, padding: impl Into<Edges>) -> Self {
        self.padding(padding)
    }

    pub fn padding_xy(self, x: f32, y: f32) -> Self {
        self.padding(Edges::xy(x, y))
    }

    pub fn padding_x(mut self, x: f32) -> Self {
        self.padding.left = x;
        self.padding.right = x;
        self
    }

    pub fn padding_y(mut self, y: f32) -> Self {
        self.padding.top = y;
        self.padding.bottom = y;
        self
    }

    pub fn padding_left(mut self, v: f32) -> Self {
        self.padding.left = v;
        self
    }

    pub fn padding_right(mut self, v: f32) -> Self {
        self.padding.right = v;
        self
    }

    pub fn padding_top(mut self, v: f32) -> Self {
        self.padding.top = v;
        self
    }

    pub fn padding_bottom(mut self, v: f32) -> Self {
        self.padding.bottom = v;
        self
    }

    pub fn min_width(mut self, v: f32) -> Self {
        self.min_width = sanitize_min(v);
        self
    }

    pub fn max_width(mut self, v: f32) -> Self {
        self.max_width = sanitize_max(v);
        self
    }

    pub fn min_height(mut self, v: f32) -> Self {
        self.min_height = sanitize_min(v);
        self
    }

    pub fn max_height(mut self, v: f32) -> Self {
        self.max_height = sanitize_max(v);
        self
    }

    pub fn clamp_width(self, w: f32) -> f32 {
        clamp_axis(w, self.min_width, self.max_width)
    }

    pub fn clamp_height(self, h: f32) -> f32 {
        clamp_axis(h, self.min_height, self.max_height)
    }

    pub fn clamp_size(self, size: sequoia_core::Size) -> sequoia_core::Size {
        sequoia_core::Size::new(self.clamp_width(size.width), self.clamp_height(size.height))
    }

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

    pub fn margin_xy(self, x: f32, y: f32) -> Self {
        self.margin(Edges::xy(x, y))
    }

    pub fn margin_x(mut self, x: f32) -> Self {
        let x = sanitize_edge(x);
        self.margin.left = x;
        self.margin.right = x;
        self
    }

    pub fn margin_y(mut self, y: f32) -> Self {
        let y = sanitize_edge(y);
        self.margin.top = y;
        self.margin.bottom = y;
        self
    }

    pub fn margin_left(mut self, v: f32) -> Self {
        self.margin.left = sanitize_edge(v);
        self
    }

    pub fn margin_right(mut self, v: f32) -> Self {
        self.margin.right = sanitize_edge(v);
        self
    }

    pub fn margin_top(mut self, v: f32) -> Self {
        self.margin.top = sanitize_edge(v);
        self
    }

    pub fn margin_bottom(mut self, v: f32) -> Self {
        self.margin.bottom = sanitize_edge(v);
        self
    }

    pub fn flex(mut self, weight: f32) -> Self {
        self.flex = sanitize_flex(weight);
        self
    }

    pub fn flex_grow(mut self, weight: f32) -> Self {
        self.flex = sanitize_flex(weight);
        self
    }

    pub fn flex_shrink(mut self, shrink: f32) -> Self {
        self.flex_shrink = sanitize_shrink(shrink);
        self
    }

    pub fn flex_full(mut self, grow: f32, shrink: f32, basis: Option<Length>) -> Self {
        self.flex = sanitize_flex(grow);
        self.flex_shrink = sanitize_shrink(shrink);
        self.flex_basis = basis;
        self
    }

    pub fn flex_basis(mut self, basis: Option<Length>) -> Self {
        self.flex_basis = basis;
        self
    }

    pub fn no_basis(mut self) -> Self {
        self.flex_basis = None;
        self
    }

    pub fn expand(self) -> Self {
        self.flex(1.0)
    }

    pub fn flex_weight(self) -> f32 {
        sanitize_flex(self.flex)
    }

    pub fn flex_grow_weight(self) -> f32 {
        sanitize_flex(self.flex)
    }

    pub fn flex_shrink_weight(self) -> f32 {
        sanitize_shrink(self.flex_shrink)
    }

    pub fn align_self(mut self, align: Option<Align>) -> Self {
        self.align_self = align;
        self
    }

    pub fn align_self_value(mut self, align: Align) -> Self {
        self.align_self = Some(align);
        self
    }

    pub fn auto_align(mut self) -> Self {
        self.align_self = None;
        self
    }

    pub fn effective_align(self, container: Align) -> Align {
        Align::resolve(self.align_self, container)
    }

    pub fn aspect_ratio(mut self, ratio: Option<f32>) -> Self {
        self.aspect_ratio = sanitize_ratio(ratio);
        self
    }

    pub fn aspect(mut self, ratio: f32) -> Self {
        self.aspect_ratio = sanitize_ratio(Some(ratio));
        self
    }

    pub fn no_aspect(mut self) -> Self {
        self.aspect_ratio = None;
        self
    }

    pub fn position(mut self, position: Position) -> Self {
        self.position = position;
        self
    }

    pub fn absolute(mut self) -> Self {
        self.position = Position::Absolute(Inset::AUTO);
        self
    }

    pub fn absolute_at(mut self, left: f32, top: f32) -> Self {
        self.position = Position::Absolute(Inset::xy(left, top));
        self
    }

    pub fn absolute_inset(mut self, inset: Inset) -> Self {
        self.position = Position::Absolute(inset);
        self
    }

    pub fn is_absolute(self) -> bool {
        matches!(self.position, Position::Absolute(_))
    }

    pub fn absolute_inset_of(self) -> Option<Inset> {
        match self.position {
            Position::Absolute(inset) => Some(inset),
            _ => None,
        }
    }

    pub fn margin_main(self, axis: Axis) -> f32 {
        match axis {
            Axis::Horizontal => self.margin.left + self.margin.right,
            Axis::Vertical => self.margin.top + self.margin.bottom,
        }
    }

    pub fn margin_cross(self, axis: Axis) -> f32 {
        match axis {
            Axis::Horizontal => self.margin.top + self.margin.bottom,
            Axis::Vertical => self.margin.left + self.margin.right,
        }
    }

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

    pub fn outer_size(self, inner: Size) -> Size {
        Size::new(
            inner.width + self.margin.horizontal_sum(),
            inner.height + self.margin.vertical_sum(),
        )
    }

    pub fn min_main(self, axis: Axis) -> f32 {
        sanitize_min(match axis {
            Axis::Horizontal => self.min_width,
            Axis::Vertical => self.min_height,
        })
    }

    pub fn max_main(self, axis: Axis) -> f32 {
        sanitize_max(match axis {
            Axis::Horizontal => self.max_width,
            Axis::Vertical => self.max_height,
        })
    }

    pub fn clamp_main(self, value: f32, axis: Axis) -> f32 {
        clamp_axis(value, self.min_main(axis), self.max_main(axis))
    }

    pub fn overflow(mut self, overflow: Overflow) -> Self {
        self.overflow = overflow;
        self
    }

    pub fn indent_top(mut self, v: f32) -> Self {
        self.indent_top = Some(sanitize_indent(v));
        self
    }

    pub fn indent_bottom(mut self, v: f32) -> Self {
        self.indent_bottom = Some(sanitize_indent(v));
        self
    }

    pub fn indent_left(mut self, v: f32) -> Self {
        self.indent_left = Some(sanitize_indent(v));
        self
    }

    pub fn indent_right(mut self, v: f32) -> Self {
        self.indent_right = Some(sanitize_indent(v));
        self
    }

    pub fn clear_indent_top(mut self) -> Self {
        self.indent_top = None;
        self
    }

    pub fn clear_indent_bottom(mut self) -> Self {
        self.indent_bottom = None;
        self
    }

    pub fn clear_indent_left(mut self) -> Self {
        self.indent_left = None;
        self
    }

    pub fn clear_indent_right(mut self) -> Self {
        self.indent_right = None;
        self
    }

    pub fn clear_indents(mut self) -> Self {
        self.indent_top = None;
        self.indent_bottom = None;
        self.indent_left = None;
        self.indent_right = None;
        self
    }

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

    pub fn indent_all(mut self, v: f32) -> Self {
        let v = sanitize_indent(v);
        self.indent_top = Some(v);
        self.indent_bottom = Some(v);
        self.indent_left = Some(v);
        self.indent_right = Some(v);
        self
    }

    pub fn indent_xy(mut self, x: f32, y: f32) -> Self {
        let x = sanitize_indent(x);
        let y = sanitize_indent(y);
        self.indent_left = Some(x);
        self.indent_right = Some(x);
        self.indent_top = Some(y);
        self.indent_bottom = Some(y);
        self
    }

    pub fn indent_axes(self, horizontal: f32, vertical: f32) -> Self {
        self.indent_xy(horizontal, vertical)
    }

    pub fn indent_main_start(self, axis: Axis) -> Option<f32> {
        match axis {
            Axis::Horizontal => self.indent_left,
            Axis::Vertical => self.indent_top,
        }
    }

    pub fn indent_main_end(self, axis: Axis) -> Option<f32> {
        match axis {
            Axis::Horizontal => self.indent_right,
            Axis::Vertical => self.indent_bottom,
        }
    }

    pub fn indent_cross_start(self, axis: Axis) -> Option<f32> {
        match axis {
            Axis::Horizontal => self.indent_top,
            Axis::Vertical => self.indent_left,
        }
    }

    pub fn indent_cross_end(self, axis: Axis) -> Option<f32> {
        match axis {
            Axis::Horizontal => self.indent_bottom,
            Axis::Vertical => self.indent_right,
        }
    }

    pub fn indents(self) -> Indents {
        Indents {
            left: self.indent_left,
            top: self.indent_top,
            right: self.indent_right,
            bottom: self.indent_bottom,
        }
    }
}

pub fn sanitize_flex(v: f32) -> f32 {
    if v.is_finite() && v > 0.0 { v } else { 0.0 }
}

pub fn sanitize_shrink(v: f32) -> f32 {
    if v.is_finite() && v >= 0.0 {
        v
    } else if v.is_nan() {
        1.0
    } else {
        0.0
    }
}

pub fn sanitize_ratio(v: Option<f32>) -> Option<f32> {
    match v {
        Some(r) if r.is_finite() && r > 0.0 => Some(r),
        _ => None,
    }
}

fn sanitize_edge(v: f32) -> f32 {
    if v.is_finite() { v.max(0.0) } else { 0.0 }
}

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
