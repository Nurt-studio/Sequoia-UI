//! Line wrapping for flow containers.

use crate::container::{Align, Axis};
use crate::dock::sanitize_gap;

/// Wrapping configuration for children along a main axis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WrapStyle {
    /// Main axis along which children are placed.
    pub axis: Axis,
    /// Cross-axis alignment of the items within each line.
    pub align: Align,
    /// Gap between items on the same line.
    pub column_gap: f32,
    /// Gap between wrapped lines.
    pub row_gap: f32,
}

impl Default for WrapStyle {
    fn default() -> Self {
        Self {
            axis: Axis::Horizontal,
            align: Align::Start,
            column_gap: 0.0,
            row_gap: 0.0,
        }
    }
}

impl WrapStyle {
    /// Creates a wrapping style for the given main axis.
    pub fn new(axis: Axis) -> Self {
        Self {
            axis,
            ..Self::default()
        }
    }

    /// Returns the gap used between items along the main axis.
    pub fn main_gap(self) -> f32 {
        match self.axis {
            Axis::Horizontal => self.column_gap,
            Axis::Vertical => self.row_gap,
        }
    }

    /// Returns the gap used between wrapped lines.
    pub fn cross_gap(self) -> f32 {
        match self.axis {
            Axis::Horizontal => self.row_gap,
            Axis::Vertical => self.column_gap,
        }
    }

    /// Sets both the column and row gap.
    pub fn gap(mut self, gap: f32) -> Self {
        self.column_gap = sanitize_gap(gap);
        self.row_gap = sanitize_gap(gap);
        self
    }

    /// Sets the gap between items on the same line.
    pub fn column_gap(mut self, gap: f32) -> Self {
        self.column_gap = sanitize_gap(gap);
        self
    }

    /// Sets the gap between wrapped lines.
    pub fn row_gap(mut self, gap: f32) -> Self {
        self.row_gap = sanitize_gap(gap);
        self
    }

    /// Sets the cross-axis alignment of items within a line.
    pub fn align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }

    /// Sets the main axis and reinterprets the main and cross gaps.
    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }
}

/// Splits main-axis sizes into lines that fit within `avail_main`.
///
/// Returns half-open index ranges `(start, end)` for each line. `gap` is
/// counted between adjacent items. An unbounded axis keeps every item on a
/// single line.
pub fn wrap_lines(main_sizes: &[f32], avail_main: f32, gap: f32) -> Vec<(usize, usize)> {
    let mut lines = Vec::new();
    if main_sizes.is_empty() {
        return lines;
    }
    let gap = sanitize_gap(gap);
    let avail = if avail_main.is_finite() {
        avail_main.max(0.0)
    } else {
        f32::INFINITY
    };

    let mut start = 0usize;
    let mut used = 0.0_f32;
    for (i, &raw) in main_sizes.iter().enumerate() {
        let size = if raw.is_finite() { raw.max(0.0) } else { 0.0 };
        if i == start {
            used = size;
            continue;
        }
        let next = used + gap + size;
        if next > avail {
            lines.push((start, i));
            start = i;
            used = size;
        } else {
            used = next;
        }
    }
    lines.push((start, main_sizes.len()));
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_has_no_lines() {
        assert!(wrap_lines(&[], 100.0, 0.0).is_empty());
    }

    #[test]
    fn children_that_fit_stay_on_one_line() {
        let lines = wrap_lines(&[30.0, 30.0, 30.0], 100.0, 0.0);
        assert_eq!(lines, vec![(0, 3)]);
    }

    #[test]
    fn overflow_wraps_to_the_next_line() {
        let lines = wrap_lines(&[40.0, 40.0, 40.0], 100.0, 0.0);
        assert_eq!(lines, vec![(0, 2), (2, 3)]);
    }

    #[test]
    fn gap_counts_toward_the_line_width() {
        assert_eq!(
            wrap_lines(&[40.0, 40.0, 40.0], 100.0, 10.0),
            vec![(0, 2), (2, 3)]
        );

        assert_eq!(
            wrap_lines(&[40.0, 40.0, 40.0], 80.0, 10.0),
            vec![(0, 1), (1, 2), (2, 3)]
        );
    }

    #[test]
    fn wider_than_the_line_gets_its_own_line() {
        let lines = wrap_lines(&[200.0, 30.0], 100.0, 0.0);
        assert_eq!(lines, vec![(0, 1), (1, 2)]);
    }

    #[test]
    fn unbounded_axis_keeps_everything_on_one_line() {
        let lines = wrap_lines(&[500.0, 500.0, 500.0], f32::INFINITY, 8.0);
        assert_eq!(lines, vec![(0, 3)]);
    }

    #[test]
    fn gaps_sanitize() {
        let s = WrapStyle::new(Axis::Horizontal).gap(f32::NAN);
        assert_eq!(s.column_gap, 0.0);
        assert_eq!(s.row_gap, 0.0);
        let s = s.row_gap(-5.0).column_gap(4.0);
        assert_eq!(s.row_gap, 0.0);
        assert_eq!(s.column_gap, 4.0);
    }

    #[test]
    fn main_and_cross_gaps_follow_the_axis() {
        let s = WrapStyle::new(Axis::Horizontal)
            .column_gap(6.0)
            .row_gap(10.0);
        assert_eq!(s.main_gap(), 6.0);
        assert_eq!(s.cross_gap(), 10.0);
        let s = s.axis(Axis::Vertical);
        assert_eq!(s.main_gap(), 10.0);
        assert_eq!(s.cross_gap(), 6.0);
    }
}
