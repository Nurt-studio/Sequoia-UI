//! Box-model helpers for padding, margins, insets, and aspect ratios.

use sequoia_core::{Edges, Rect, Size};

/// Axes affected when shrinking a box by its padding or margin.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ShrinkAxis {
    /// Shrink both axes.
    Both,
    /// Shrink only the horizontal axis.
    Horizontal,
    /// Shrink only the vertical axis.
    Vertical,
}

/// Returns the content size after subtracting `padding` from both axes.
///
/// The result never drops below zero.
pub fn shrink_by_padding(size: Size, padding: Edges) -> Size {
    Size::new(
        (size.width - padding.horizontal_sum()).max(0.0),
        (size.height - padding.vertical_sum()).max(0.0),
    )
}

/// Returns the content size after subtracting `margin` from both axes.
///
/// The result never drops below zero.
pub fn shrink_by_margin(size: Size, margin: Edges) -> Size {
    Size::new(
        (size.width - margin.horizontal_sum()).max(0.0),
        (size.height - margin.vertical_sum()).max(0.0),
    )
}

/// Insets a rectangle by `padding`, clamping the resulting size to zero.
pub fn inset_rect(rect: Rect, padding: Edges) -> Rect {
    Rect::from_xywh(
        rect.left() + padding.left,
        rect.top() + padding.top,
        (rect.size.width - padding.horizontal_sum()).max(0.0),
        (rect.size.height - padding.vertical_sum()).max(0.0),
    )
}

/// Derives the auto-sized axis from the other axis using `ratio`.
///
/// `w_auto` and `h_auto` indicate which dimensions were left to the layout.
/// When both axes are auto, the ratio is only applied if exactly one of them
/// already has a non-zero size. Invalid ratios leave the size unchanged.
pub fn apply_aspect_ratio(
    w: f32,
    h: f32,
    w_auto: bool,
    h_auto: bool,
    ratio: Option<f32>,
) -> (f32, f32) {
    let Some(r) = ratio else { return (w, h) };
    if !r.is_finite() || r <= 0.0 {
        return (w, h);
    }
    match (w_auto, h_auto) {
        (true, false) => ((h * r).max(0.0), h),
        (false, true) => (w, if r != 0.0 { (w / r).max(0.0) } else { h }),
        (true, true) => {
            if w <= 0.0 && h > 0.0 {
                ((h * r).max(0.0), h)
            } else if h <= 0.0 && w > 0.0 {
                (w, (w / r).max(0.0))
            } else {
                (w, h)
            }
        }
        (false, false) => (w, h),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn padding_shrinks_content_box() {
        let s = shrink_by_padding(Size::new(100.0, 60.0), Edges::xy(10.0, 5.0));
        assert_eq!(s, Size::new(80.0, 50.0));
        let s = shrink_by_padding(Size::new(5.0, 5.0), Edges::all(10.0));
        assert_eq!(s, Size::ZERO);
    }

    #[test]
    fn aspect_derives_auto_axis() {
        assert_eq!(
            apply_aspect_ratio(0.0, 50.0, true, false, Some(2.0)),
            (100.0, 50.0)
        );
        assert_eq!(
            apply_aspect_ratio(100.0, 0.0, false, true, Some(2.0)),
            (100.0, 50.0)
        );
        assert_eq!(
            apply_aspect_ratio(100.0, 50.0, false, false, Some(2.0)),
            (100.0, 50.0)
        );
        assert_eq!(
            apply_aspect_ratio(10.0, 10.0, true, true, Some(2.0)),
            (10.0, 10.0)
        );
    }
}
