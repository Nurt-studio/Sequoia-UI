//! Masonry (waterfall) column layout.
//!
//! A masonry container places equally sized columns side by side and appends
//! every item to the currently shortest column. Unlike a grid, rows do not
//! share a common height, so cards with varying content keep their natural
//! size.

use crate::container::Align;
use crate::dock::sanitize_gap;

/// Layout style for a masonry container.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MasonryStyle {
    /// Number of columns. Values below one are clamped to one.
    pub columns: usize,
    /// Gap between adjacent columns.
    pub column_gap: f32,
    /// Gap between adjacent items in the same column.
    pub row_gap: f32,
    /// Default horizontal alignment of items inside their column.
    pub align: Align,
}

impl Default for MasonryStyle {
    fn default() -> Self {
        Self {
            columns: 2,
            column_gap: 0.0,
            row_gap: 0.0,
            align: Align::Stretch,
        }
    }
}

impl MasonryStyle {
    /// Creates a style with the given number of columns.
    pub fn new(columns: usize) -> Self {
        Self {
            columns: columns.max(1),
            ..Self::default()
        }
    }

    /// Sets the number of columns, clamped to at least one.
    pub fn columns(mut self, columns: usize) -> Self {
        self.columns = columns.max(1);
        self
    }

    /// Sets both the column and row gap.
    pub fn gap(mut self, gap: f32) -> Self {
        let gap = sanitize_gap(gap);
        self.column_gap = gap;
        self.row_gap = gap;
        self
    }

    /// Sets the gap between adjacent columns.
    pub fn column_gap(mut self, gap: f32) -> Self {
        self.column_gap = sanitize_gap(gap);
        self
    }

    /// Sets the gap between adjacent items in the same column.
    pub fn row_gap(mut self, gap: f32) -> Self {
        self.row_gap = sanitize_gap(gap);
        self
    }

    /// Sets the default horizontal alignment of items inside their column.
    pub fn align_items(mut self, align: Align) -> Self {
        self.align = align;
        self
    }
}

/// Placement of a single masonry item.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MasonryPlacement {
    /// Zero-based column index.
    pub column: usize,
    /// Top offset relative to the content box.
    pub top: f32,
}

/// Resolves the width of one column in a masonry container.
///
/// Returns zero when `available` is not finite; callers handling an unbounded
/// axis derive the column width from measured content instead.
pub fn masonry_column_width(available: f32, columns: usize, gap: f32) -> f32 {
    let columns = columns.max(1);
    let gap = sanitize_gap(gap);
    if !available.is_finite() {
        return 0.0;
    }
    let total_gap = gap * columns.saturating_sub(1) as f32;
    ((available.max(0.0) - total_gap) / columns as f32).max(0.0)
}

/// Assigns item heights to columns and returns their placements.
///
/// Items are appended in order to the shortest column, with ties resolved in
/// favour of the lowest column index. The returned height is the bottom edge of
/// the tallest column. Invalid heights and gaps are treated as zero.
pub fn masonry_assign(
    heights: &[f32],
    columns: usize,
    row_gap: f32,
) -> (Vec<MasonryPlacement>, f32) {
    let columns = columns.max(1);
    let row_gap = sanitize_gap(row_gap);
    let mut bottoms = vec![0.0_f32; columns];
    let mut counts = vec![0usize; columns];
    let mut placements = Vec::with_capacity(heights.len());

    for &raw_height in heights {
        let height = if raw_height.is_finite() {
            raw_height.max(0.0)
        } else {
            0.0
        };
        let mut column = 0usize;
        for candidate in 1..columns {
            if bottoms[candidate] < bottoms[column] {
                column = candidate;
            }
        }
        let top = if counts[column] == 0 {
            0.0
        } else {
            bottoms[column] + row_gap
        };
        bottoms[column] = top + height;
        counts[column] += 1;
        placements.push(MasonryPlacement { column, top });
    }

    let height = bottoms.into_iter().fold(0.0_f32, f32::max);
    (placements, height)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.01
    }

    #[test]
    fn column_width_accounts_for_gaps() {
        assert!(approx(masonry_column_width(110.0, 2, 10.0), 50.0));
        assert!(approx(masonry_column_width(100.0, 3, 10.0), 80.0 / 3.0));
        assert_eq!(masonry_column_width(100.0, 0, 10.0), 100.0);
    }

    #[test]
    fn column_width_is_zero_when_unbounded_or_too_small() {
        assert_eq!(masonry_column_width(f32::INFINITY, 2, 10.0), 0.0);
        assert_eq!(masonry_column_width(10.0, 3, 10.0), 0.0);
    }

    #[test]
    fn items_fill_the_shortest_column_in_order() {
        let (placements, height) = masonry_assign(&[100.0, 50.0, 80.0, 120.0, 30.0], 2, 10.0);
        assert_eq!(
            placements,
            vec![
                MasonryPlacement {
                    column: 0,
                    top: 0.0
                },
                MasonryPlacement {
                    column: 1,
                    top: 0.0
                },
                MasonryPlacement {
                    column: 1,
                    top: 60.0
                },
                MasonryPlacement {
                    column: 0,
                    top: 110.0
                },
                MasonryPlacement {
                    column: 1,
                    top: 150.0
                },
            ]
        );
        assert!(approx(height, 230.0));
    }

    #[test]
    fn ties_use_the_lowest_column_index() {
        let (placements, height) = masonry_assign(&[10.0, 10.0, 10.0, 10.0], 3, 0.0);
        assert_eq!(
            placements
                .iter()
                .map(|placement| placement.column)
                .collect::<Vec<_>>(),
            vec![0, 1, 2, 0]
        );
        assert!(approx(height, 20.0));
    }

    #[test]
    fn first_item_in_each_column_has_no_leading_gap() {
        let (placements, height) = masonry_assign(&[20.0, 20.0, 20.0], 2, 6.0);
        assert_eq!(placements[0].top, 0.0);
        assert_eq!(placements[1].top, 0.0);
        assert_eq!(placements[2].top, 26.0);
        assert!(approx(height, 46.0));
    }

    #[test]
    fn invalid_values_are_sanitized() {
        let (placements, height) = masonry_assign(&[f32::NAN, -5.0, 10.0], 0, f32::NAN);
        assert_eq!(placements.len(), 3);
        assert!(placements.iter().all(|placement| placement.column == 0));
        assert_eq!(placements[0].top, 0.0);
        assert_eq!(placements[1].top, 0.0);
        assert_eq!(placements[2].top, 0.0);
        assert!(approx(height, 10.0));
    }

    #[test]
    fn empty_input_has_no_placements_or_height() {
        let (placements, height) = masonry_assign(&[], 3, 8.0);
        assert!(placements.is_empty());
        assert_eq!(height, 0.0);
    }

    #[test]
    fn style_builders_sanitize_values() {
        let style = MasonryStyle::new(0)
            .gap(f32::NAN)
            .column_gap(-1.0)
            .row_gap(4.0)
            .columns(3);
        assert_eq!(style.columns, 3);
        assert_eq!(style.column_gap, 0.0);
        assert_eq!(style.row_gap, 4.0);
    }
}
