//! Grid tracks, placements, and automatic flow.

use crate::container::Align;
use crate::dock::sanitize_gap;

/// Size of a single grid column or row.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
#[derive(Default)]
pub enum Track {
    /// Size to the content of the track.
    #[default]
    Fit,
    /// Fixed size in logical pixels.
    Px(f32),
    /// Size as a percentage of the available space.
    Percent(f32),
    /// Share of the free space, proportional to the weight.
    Fill(f32),
    /// Combination of a minimum size and a maximum track definition.
    MinMax {
        /// Lower bound of the track size.
        min: f32,
        /// Definition used for the upper bound.
        max: Box<Track>,
    },
}

impl Track {
    /// Creates a track with a minimum size and a maximum definition.
    ///
    /// Nested [`Track::MinMax`] maximums are flattened to their inner track.
    pub fn minmax(min: f32, max: Track) -> Self {
        let min = if min.is_finite() { min.max(0.0) } else { 0.0 };

        let max = match max {
            Self::MinMax { min: _, max: inner } => *inner,
            other => other,
        };
        Self::MinMax {
            min,
            max: Box::new(max),
        }
    }

    /// Returns `true` if the track claims free space.
    pub fn is_fill_like(&self) -> bool {
        match self {
            Self::Fill(w) => *w > 0.0,
            Self::MinMax { max, .. } => (**max).is_fill_like(),
            _ => false,
        }
    }

    /// Returns the weight used to share free space, or zero.
    pub fn fill_weight(&self) -> f32 {
        match self {
            Self::Fill(w) if w.is_finite() => w.max(0.0),
            Self::MinMax { max, .. } => (**max).fill_weight(),
            _ => 0.0,
        }
    }

    /// Returns the minimum size enforced by this track.
    pub fn min_floor(&self) -> f32 {
        match self {
            Self::MinMax { min, .. } => *min,
            _ => 0.0,
        }
    }
}

/// Placement of a grid item, with optional row and column indices.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GridPlacement {
    /// Explicit column index, or `None` for auto placement.
    pub column: Option<usize>,
    /// Explicit row index, or `None` for auto placement.
    pub row: Option<usize>,
    /// Number of columns the item spans.
    pub col_span: usize,
    /// Number of rows the item spans.
    pub row_span: usize,
}

impl GridPlacement {
    /// Placement with no explicit position and single-cell spans.
    pub const AUTO: Self = Self {
        column: None,
        row: None,
        col_span: 1,
        row_span: 1,
    };

    /// Creates an automatic placement.
    pub fn new() -> Self {
        Self::AUTO
    }

    /// Sets both the column and row index.
    pub fn at(mut self, column: usize, row: usize) -> Self {
        self.column = Some(column);
        self.row = Some(row);
        self
    }

    /// Sets the column index.
    pub fn column(mut self, column: usize) -> Self {
        self.column = Some(column);
        self
    }

    /// Sets the row index.
    pub fn row(mut self, row: usize) -> Self {
        self.row = Some(row);
        self
    }

    /// Sets both spans, clamped to at least one cell.
    pub fn span(mut self, col_span: usize, row_span: usize) -> Self {
        self.col_span = col_span.max(1);
        self.row_span = row_span.max(1);
        self
    }

    /// Sets the column span, clamped to at least one cell.
    pub fn col_span(mut self, span: usize) -> Self {
        self.col_span = span.max(1);
        self
    }

    /// Sets the row span, clamped to at least one cell.
    pub fn row_span(mut self, span: usize) -> Self {
        self.row_span = span.max(1);
        self
    }

    /// Returns `true` if the item has no explicit position or span.
    pub fn is_auto(self) -> bool {
        self.column.is_none() && self.row.is_none() && self.col_span <= 1 && self.row_span <= 1
    }
}

/// Direction in which auto-placed grid items flow.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GridFlow {
    /// Fill rows left to right, then move to the next row.
    #[default]
    Row,
    /// Fill columns top to bottom, then move to the next column.
    Column,
}

/// Layout style for a grid container.
#[derive(Clone, Debug, PartialEq)]
pub struct GridStyle {
    /// Column track definitions.
    pub columns: Vec<Track>,
    /// Row track definitions.
    pub rows: Vec<Track>,
    /// Gap between columns.
    pub column_gap: f32,
    /// Gap between rows.
    pub row_gap: f32,
    /// Auto-placement direction.
    pub auto_flow: GridFlow,
    /// Default cross-axis alignment of items within their cells.
    pub align_items: Align,
    /// Default main-axis alignment of items within their cells.
    pub justify_items: Align,
}

impl Default for GridStyle {
    fn default() -> Self {
        Self {
            columns: Vec::new(),
            rows: Vec::new(),
            column_gap: 0.0,
            row_gap: 0.0,
            auto_flow: GridFlow::Row,
            align_items: Align::Stretch,
            justify_items: Align::Stretch,
        }
    }
}

impl GridStyle {
    /// Creates a style with the given column tracks.
    pub fn new(columns: impl IntoIterator<Item = Track>) -> Self {
        Self {
            columns: columns.into_iter().collect(),
            ..Self::default()
        }
    }

    /// Sets the row tracks.
    pub fn rows(mut self, rows: impl IntoIterator<Item = Track>) -> Self {
        self.rows = rows.into_iter().collect();
        self
    }

    /// Sets both the column and row gaps.
    pub fn gap(mut self, gap: f32) -> Self {
        let gap = sanitize_gap(gap);
        self.column_gap = gap;
        self.row_gap = gap;
        self
    }

    /// Sets the gap between columns.
    pub fn column_gap(mut self, gap: f32) -> Self {
        self.column_gap = sanitize_gap(gap);
        self
    }

    /// Sets the gap between rows.
    pub fn row_gap(mut self, gap: f32) -> Self {
        self.row_gap = sanitize_gap(gap);
        self
    }

    /// Sets the auto-placement direction.
    pub fn auto_flow(mut self, flow: GridFlow) -> Self {
        self.auto_flow = flow;
        self
    }

    /// Uses column-wise auto placement.
    pub fn column_flow(mut self) -> Self {
        self.auto_flow = GridFlow::Column;
        self
    }

    /// Sets the default cross-axis alignment of items.
    pub fn align_items(mut self, align: Align) -> Self {
        self.align_items = align;
        self
    }

    /// Sets the default main-axis alignment of items.
    pub fn justify_items(mut self, align: Align) -> Self {
        self.justify_items = align;
        self
    }

    /// Sets both the align and justify alignment of items.
    pub fn place_items(mut self, align: Align) -> Self {
        self.align_items = align;
        self.justify_items = align;
        self
    }
}

fn resolve_single(track: &Track, content: f32, available: f32) -> (f32, f32) {
    let content = if content.is_finite() {
        content.max(0.0)
    } else {
        0.0
    };
    match track {
        Track::Px(v) => (if v.is_finite() { v.max(0.0) } else { content }, 0.0),
        Track::Percent(p) => {
            if available.is_finite() && p.is_finite() {
                ((available * p / 100.0).max(0.0), 0.0)
            } else {
                (content, 0.0)
            }
        }
        Track::Fit => (content, 0.0),
        Track::Fill(w) => (0.0, if w.is_finite() { w.max(0.0) } else { 0.0 }),
        Track::MinMax { min, max } => {
            let (fixed, fill) = resolve_single(max, content, available);

            if fill > 0.0 {
                (min.max(fixed).max(0.0), fill)
            } else {
                (fixed.max(*min).max(0.0), 0.0)
            }
        }
    }
}

/// Resolves track sizes to concrete lengths.
///
/// `content` provides the intrinsic content size for each track and
/// `available` is the space shared by fill tracks.
pub fn resolve_tracks(tracks: &[Track], content: &[f32], available: f32, gap: f32) -> Vec<f32> {
    resolve_tracks_clamped(tracks, content, available, gap, None, None)
}

/// Resolves track sizes with optional per-track min and max limits.
///
/// Fill tracks share the remaining space; a track that reaches its max stops
/// growing so the rest is handed to its siblings.
pub fn resolve_tracks_clamped(
    tracks: &[Track],
    content: &[f32],
    available: f32,
    gap: f32,
    mins: Option<&[f32]>,
    maxs: Option<&[f32]>,
) -> Vec<f32> {
    let gap = sanitize_gap(gap);
    let gaps = gap * tracks.len().saturating_sub(1) as f32;
    let mut sizes = vec![0.0; tracks.len()];
    let mut fill_weight_sum = 0.0;

    for (i, track) in tracks.iter().enumerate() {
        let c = content.get(i).copied().unwrap_or(0.0);
        let (fixed, fill) = resolve_single(track, c, available);

        let floor = track.min_floor();
        let mut size = fixed.max(floor);
        if let Track::MinMax { min, .. } = track {
            size = size.max(*min).max(0.0);

            let _ = c;
        }
        if let Some(mns) = mins {
            let mn = mns.get(i).copied().unwrap_or(0.0);
            let mn = if mn.is_finite() { mn.max(0.0) } else { 0.0 };
            size = size.max(mn);
        }
        if let Some(mxs) = maxs {
            let mx = mxs.get(i).copied().unwrap_or(f32::INFINITY);
            let mx = if mx.is_nan() {
                f32::INFINITY
            } else if mx.is_infinite() {
                if mx > 0.0 { f32::INFINITY } else { 0.0 }
            } else {
                mx.max(0.0)
            };
            size = size.min(mx);
        }
        sizes[i] = size.max(0.0);
        fill_weight_sum += fill;
    }

    if fill_weight_sum > 0.0 {
        let max_of = |i: usize| -> Option<f32> {
            maxs.and_then(|m| m.get(i).copied()).map(|mx| {
                if mx.is_nan() {
                    f32::INFINITY
                } else if mx.is_infinite() {
                    if mx > 0.0 { f32::INFINITY } else { 0.0 }
                } else {
                    mx.max(0.0)
                }
            })
        };
        let capped = |i: usize, size: f32| max_of(i).is_some_and(|mx| size >= mx - 0.005);

        if available.is_finite() {
            // Fill tracks share the free space; a track that reaches its max
            // stops taking space so the rest is handed to its siblings.
            for _ in 0..tracks.len().max(1) {
                let used: f32 = sizes.iter().sum::<f32>() + gaps;
                let free = available - used;
                if free <= 0.005 {
                    break;
                }
                let active: f32 = tracks
                    .iter()
                    .enumerate()
                    .filter(|(i, t)| t.fill_weight() > 0.0 && !capped(*i, sizes[*i]))
                    .map(|(_, t)| t.fill_weight())
                    .sum();
                if active <= 0.0 {
                    break;
                }
                for (i, track) in tracks.iter().enumerate() {
                    let w = track.fill_weight();
                    if w <= 0.0 || capped(i, sizes[i]) {
                        continue;
                    }
                    let floor = sizes[i];
                    let mut v = floor + free * w / active;
                    if let Some(mx) = max_of(i) {
                        v = v.min(mx.max(floor));
                    }
                    sizes[i] = v.max(floor).max(0.0);
                }
            }
        } else {
            for (i, track) in tracks.iter().enumerate() {
                let w = track.fill_weight();
                if w > 0.0 {
                    let floor = sizes[i];
                    let extra = content.get(i).copied().unwrap_or(0.0).max(0.0).max(floor) - floor;
                    sizes[i] = (floor + extra.max(0.0)).max(floor).max(0.0);
                }
            }
        }
    }

    sizes
}

/// Returns the `(column, row)` cell for each auto-placed item.
///
/// `ncols` is clamped to at least one column.
pub fn auto_cells(count: usize, ncols: usize, flow: GridFlow) -> Vec<(usize, usize)> {
    let ncols = ncols.max(1);
    (0..count)
        .map(|i| match flow {
            GridFlow::Row => (i % ncols, i / ncols),
            GridFlow::Column => {
                let nrows = count.div_ceil(ncols).max(1);
                (i / nrows, i % nrows)
            }
        })
        .collect()
}

/// Returns the starting offset of each track, including gaps.
pub fn track_offsets(sizes: &[f32], gap: f32) -> Vec<f32> {
    let mut offsets = Vec::with_capacity(sizes.len());
    let mut cursor = 0.0;
    for &size in sizes {
        offsets.push(cursor);
        cursor += size + gap;
    }
    offsets
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.01
    }

    #[test]
    fn px_and_auto_tracks_take_fixed_and_content() {
        let sizes = resolve_tracks(
            &[Track::Px(100.0), Track::Fit, Track::Px(50.0)],
            &[0.0, 70.0, 0.0],
            400.0,
            0.0,
        );
        assert_eq!(sizes.len(), 3);
        assert!(approx(sizes[0], 100.0));
        assert!(approx(sizes[1], 70.0));
        assert!(approx(sizes[2], 50.0));
    }

    #[test]
    fn fill_tracks_split_remaining_space() {
        let sizes = resolve_tracks(
            &[Track::Px(100.0), Track::Fill(1.0), Track::Fill(3.0)],
            &[0.0; 3],
            400.0,
            0.0,
        );
        assert!(approx(sizes[0], 100.0));
        assert!(approx(sizes[1], 75.0));
        assert!(approx(sizes[2], 225.0));
    }

    #[test]
    fn fill_respects_gaps_and_overflow_clamps_to_zero() {
        let with_gap = resolve_tracks(
            &[Track::Fill(1.0), Track::Fill(1.0)],
            &[0.0; 2],
            110.0,
            10.0,
        );
        assert!(approx(with_gap[0], 50.0));
        assert!(approx(with_gap[1], 50.0));

        let overflow = resolve_tracks(&[Track::Px(300.0), Track::Fill(2.0)], &[0.0; 2], 100.0, 0.0);
        assert!(approx(overflow[0], 300.0));
        assert!(approx(overflow[1], 0.0));
    }

    #[test]
    fn unbounded_axis_falls_back_to_content() {
        let sizes = resolve_tracks(
            &[Track::Percent(50.0), Track::Fill(1.0), Track::Px(20.0)],
            &[80.0, 40.0, 0.0],
            f32::INFINITY,
            0.0,
        );
        assert!(approx(sizes[0], 80.0));
        assert!(approx(sizes[1], 40.0));
        assert!(approx(sizes[2], 20.0));
    }

    #[test]
    fn track_offsets_place_tracks_after_gaps() {
        let offsets = track_offsets(&[100.0, 50.0, 30.0], 10.0);
        assert!(approx(offsets[0], 0.0));
        assert!(approx(offsets[1], 110.0));
        assert!(approx(offsets[2], 170.0));
    }
}
