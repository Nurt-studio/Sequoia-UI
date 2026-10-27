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
    /// Explicit number of columns, overriding width-based wrapping.
    pub columns: Option<usize>,
    /// Explicit number of rows, overriding width-based wrapping.
    pub rows: Option<usize>,
}

impl Default for WrapStyle {
    fn default() -> Self {
        Self {
            axis: Axis::Horizontal,
            align: Align::Start,
            column_gap: 0.0,
            row_gap: 0.0,
            columns: None,
            rows: None,
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

    /// Forces the layout to use exactly `columns` columns.
    ///
    /// Values below one are clamped to one. For a horizontal axis this is the
    /// number of items per line; for a vertical axis it is the number of
    /// lines.
    pub fn columns(mut self, columns: usize) -> Self {
        self.columns = Some(columns.max(1));
        self
    }

    /// Forces the layout to use exactly `rows` rows.
    ///
    /// Values below one are clamped to one. For a vertical axis this is the
    /// number of items per line; for a horizontal axis it is the number of
    /// lines.
    pub fn rows(mut self, rows: usize) -> Self {
        self.rows = Some(rows.max(1));
        self
    }

    /// Removes the explicit column count and restores width-based wrapping.
    pub fn clear_columns(mut self) -> Self {
        self.columns = None;
        self
    }

    /// Removes the explicit row count and restores width-based wrapping.
    pub fn clear_rows(mut self) -> Self {
        self.rows = None;
        self
    }

    /// Returns how many items fit on one line when an explicit count is set.
    ///
    /// The count matching the main axis wins: `columns` for a horizontal
    /// axis and `rows` for a vertical axis. The other count is treated as the
    /// desired number of lines.
    pub fn items_per_line(self, count: usize) -> Option<usize> {
        if count == 0 {
            return None;
        }
        let columns = self.columns.filter(|&n| n > 0);
        let rows = self.rows.filter(|&n| n > 0);
        match self.axis {
            Axis::Horizontal => columns.or_else(|| rows.map(|rows| count.div_ceil(rows))),
            Axis::Vertical => rows.or_else(|| columns.map(|columns| count.div_ceil(columns))),
        }
    }

    /// Splits `main_sizes` into lines using the explicit counts when present.
    pub fn lines(self, main_sizes: &[f32], avail_main: f32) -> Vec<(usize, usize)> {
        wrap_lines_counted(
            main_sizes,
            avail_main,
            self.main_gap(),
            self.items_per_line(main_sizes.len()),
        )
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

/// Splits items into lines with an optional fixed number of items per line.
///
/// When `per_line` is `None` or zero this falls back to [`wrap_lines`].
pub fn wrap_lines_counted(
    main_sizes: &[f32],
    avail_main: f32,
    gap: f32,
    per_line: Option<usize>,
) -> Vec<(usize, usize)> {
    let Some(per_line) = per_line.filter(|&n| n > 0) else {
        return wrap_lines(main_sizes, avail_main, gap);
    };
    if main_sizes.is_empty() {
        return Vec::new();
    }
    (0..main_sizes.len())
        .step_by(per_line)
        .map(|start| (start, (start + per_line).min(main_sizes.len())))
        .collect()
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

    #[test]
    fn explicit_columns_override_the_available_width() {
        let sizes = [10.0; 7];
        let s = WrapStyle::new(Axis::Horizontal).columns(3);
        assert_eq!(s.lines(&sizes, 1000.0), vec![(0, 3), (3, 6), (6, 7)]);
    }

    #[test]
    fn explicit_rows_limit_the_number_of_lines() {
        let sizes = [10.0; 7];
        let s = WrapStyle::new(Axis::Horizontal).rows(2);
        assert_eq!(s.lines(&sizes, 1000.0), vec![(0, 4), (4, 7)]);
    }

    #[test]
    fn vertical_axis_maps_rows_to_items_per_line() {
        let sizes = [10.0; 7];
        let s = WrapStyle::new(Axis::Vertical).rows(3);
        assert_eq!(s.lines(&sizes, 1000.0), vec![(0, 3), (3, 6), (6, 7)]);
    }

    #[test]
    fn counts_clamp_to_at_least_one() {
        let s = WrapStyle::new(Axis::Horizontal).columns(0).rows(0);
        assert_eq!(s.columns, Some(1));
        assert_eq!(s.rows, Some(1));
        assert_eq!(s.items_per_line(3), Some(1));
    }

    #[test]
    fn clearing_counts_restores_width_based_wrapping() {
        let sizes = [40.0, 40.0, 40.0];
        let s = WrapStyle::new(Axis::Horizontal).columns(1).clear_columns();
        assert_eq!(s.columns, None);
        assert_eq!(s.lines(&sizes, 100.0), vec![(0, 2), (2, 3)]);
    }

    #[test]
    fn counted_wrap_with_no_items_is_empty() {
        assert!(wrap_lines_counted(&[], 100.0, 0.0, Some(3)).is_empty());
    }
}
