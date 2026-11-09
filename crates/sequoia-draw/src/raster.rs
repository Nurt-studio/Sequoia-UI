use sequoia_core::{Point, Rect};

use crate::flatten::Contour;
use crate::path::{FillRule, Path};
use crate::{DEFAULT_OPTIONS, Span, union_rect};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RasterOptions {
    pub tolerance: f32,

    pub row_height: f32,

    pub merge_tol: f32,

    pub aa_samples: u32,

    pub analytic: bool,

    /// Split large rasters across worker threads. Small shapes always stay on
    /// the calling thread, so this only matters for big fills and strokes.
    pub parallel: bool,
}

impl Default for RasterOptions {
    fn default() -> Self {
        DEFAULT_OPTIONS
    }
}

impl RasterOptions {
    pub const SMOOTH: Self = Self {
        tolerance: crate::TOLERANCE,
        row_height: 1.0,
        merge_tol: 0.35,
        aa_samples: 4,
        analytic: true,
        parallel: true,
    };

    pub const CRISP: Self = Self {
        tolerance: crate::TOLERANCE,
        row_height: 1.0,
        merge_tol: 0.35,
        aa_samples: 1,
        analytic: true,
        parallel: true,
    };

    pub const BANDED: Self = Self {
        tolerance: crate::TOLERANCE,
        row_height: 1.0,
        merge_tol: 0.35,
        aa_samples: 4,
        analytic: false,
        parallel: true,
    };

    pub fn tolerance(mut self, tolerance: f32) -> Self {
        self.tolerance = tolerance;
        self
    }

    pub fn row_height(mut self, row_height: f32) -> Self {
        self.row_height = row_height;
        self
    }

    pub fn merge_tol(mut self, merge_tol: f32) -> Self {
        self.merge_tol = merge_tol;
        self
    }

    pub fn aa_samples(mut self, samples: u32) -> Self {
        self.aa_samples = samples.clamp(1, 64);
        self
    }

    pub fn analytic(mut self, analytic: bool) -> Self {
        self.analytic = analytic;
        self
    }

    /// Enables or disables band-parallel tessellation of large rasters.
    ///
    /// Useful on single-core hosts and when a caller needs repeatable span
    /// order.
    pub fn parallel(mut self, parallel: bool) -> Self {
        self.parallel = parallel;
        self
    }

    pub fn for_scale(mut self, scale: f32) -> Self {
        let s = if scale.is_finite() && scale > 0.0 {
            scale
        } else {
            1.0
        };
        self.tolerance = (crate::TOLERANCE / s).max(0.05);
        self.row_height = (1.0 / s).max(1.0 / 8.0);
        self.merge_tol = (0.35 / s).max(0.02);
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FillStyle {
    pub rule: FillRule,

    pub opts: RasterOptions,
}

impl Default for FillStyle {
    fn default() -> Self {
        Self {
            rule: FillRule::NonZero,
            opts: RasterOptions::default(),
        }
    }
}

impl FillStyle {
    pub fn new(rule: FillRule, opts: RasterOptions) -> Self {
        Self { rule, opts }
    }

    pub fn with_rule(rule: FillRule) -> Self {
        Self {
            rule,
            opts: RasterOptions::default(),
        }
    }

    #[must_use]
    pub fn with_opts(mut self, opts: RasterOptions) -> Self {
        self.opts = opts;
        self
    }
}

#[derive(Clone, Copy)]
struct Edge {
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,

    dir: i32,
}

#[derive(Clone, Copy, Debug)]
struct Run {
    x0: f32,
    x1: f32,
    y0: f32,
    y1: f32,

    alpha: f32,
    claimed: bool,
}

pub fn fill_styled(path: &Path, style: FillStyle, out: &mut Vec<Span>) -> Rect {
    fill(path, style.rule, style.opts, out)
}

pub fn fill(path: &Path, rule: FillRule, opts: RasterOptions, out: &mut Vec<Span>) -> Rect {
    if opts.analytic {
        if let Some(rect) = path.as_rect() {
            if rect.is_empty() {
                return Rect::ZERO;
            }
            out.push(Span::rect(rect));
            return rect;
        }
        if let Some((rect, radius)) = path.as_rounded_rect() {
            if rect.is_empty() {
                return Rect::ZERO;
            }
            out.push(Span::rounded(rect, radius));
            return rect;
        }
        if let Some((center, radius)) = path.as_circle() {
            if radius <= 0.0 {
                return Rect::ZERO;
            }
            let rect = Rect::from_xywh(
                center.x - radius,
                center.y - radius,
                radius * 2.0,
                radius * 2.0,
            );
            out.push(Span::rounded(rect, radius));
            return rect;
        }
    }

    let contours = path.contours(opts.tolerance);
    fill_contours(&contours, rule, opts, out)
}

pub fn fill_contours(
    contours: &[Contour],
    rule: FillRule,
    opts: RasterOptions,
    out: &mut Vec<Span>,
) -> Rect {
    let edges = collect_edges(contours);
    if edges.is_empty() {
        return Rect::ZERO;
    }

    let cell = opts.row_height.clamp(0.05, 16.0);
    let samples = opts.aa_samples.clamp(1, 64) as usize;
    let merge_tol = opts.merge_tol.max(0.0);

    let mut top = f32::INFINITY;
    let mut bottom = f32::NEG_INFINITY;
    let mut left = f32::INFINITY;
    let mut right = f32::NEG_INFINITY;
    for e in &edges {
        top = top.min(e.y0.min(e.y1));
        bottom = bottom.max(e.y0.max(e.y1));
        left = left.min(e.x0.min(e.x1));
        right = right.max(e.x0.max(e.x1));
    }
    if !top.is_finite() || !bottom.is_finite() || bottom <= top {
        return Rect::ZERO;
    }
    if !left.is_finite() || !right.is_finite() || right <= left {
        return Rect::ZERO;
    }

    let first_row = (top / cell).floor();
    let last_row = (bottom / cell).ceil();
    let first_col = (left / cell).floor();
    let last_col = (right / cell).ceil();
    let cols = (last_col - first_col).max(0.0) as usize;
    if cols == 0 {
        return Rect::ZERO;
    }
    let rows = ((last_row - first_row).max(0.0).ceil() as usize) + 1;

    // Large rasters dominate a frame, so they are split into row bands and
    // tessellated on worker threads. UI-sized shapes stay on the calling
    // thread: spawning threads would cost more than the raster itself.
    let bands = if opts.parallel {
        let cores = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);
        band_count(rows, cols, samples, cores)
    } else {
        1
    };

    if bands <= 1 {
        return raster_rows(
            &edges, rule, cell, samples, merge_tol, first_row, first_col, cols, 0, rows, out,
        );
    }

    let band_rows = rows.div_ceil(bands);
    let mut parts: Vec<(usize, Vec<Span>, Rect)> = Vec::with_capacity(bands);
    std::thread::scope(|scope| {
        let mut handles = Vec::with_capacity(bands);
        for band in 0..bands {
            let start = (band * band_rows).min(rows);
            let end = (start + band_rows).min(rows);
            if start >= end {
                continue;
            }
            let edges = &edges;
            handles.push((
                start,
                scope.spawn(move || {
                    let mut spans = Vec::new();
                    let coverage = raster_rows(
                        edges, rule, cell, samples, merge_tol, first_row, first_col, cols, start,
                        end, &mut spans,
                    );
                    (spans, coverage)
                }),
            ));
        }
        for (start, handle) in handles {
            if let Ok((spans, coverage)) = handle.join() {
                parts.push((start, spans, coverage));
            }
        }
    });
    // Bands run in parallel but spans must stay ordered by row for the
    // renderer's batching and for stable output.
    parts.sort_by_key(|(start, _, _)| *start);

    let mut coverage = Rect::ZERO;
    for (_, spans, part_coverage) in parts {
        out.extend(spans);
        coverage = union_rect(coverage, part_coverage);
    }
    coverage
}

const PARALLEL_MIN_ROWS: usize = 64;

/// Parallel floor in raster *work* units: cells times anti-aliasing samples.
///
/// The old floor was 256k cells regardless of sample count, so a one-sample
/// `CRISP` fill spawned threads for a quarter of the work a four-sample fill
/// needed. Keeping the floor in work units makes the decision follow the
/// actual per-cell cost.
const PARALLEL_MIN_WORK: usize = 256 * 1024 * 4;

const MAX_BANDS: usize = 8;

/// Number of row bands to split a raster into.
///
/// `cores` is passed in so the decision can be tested without asking the
/// process how many CPUs it has.
fn band_count(rows: usize, cols: usize, samples: usize, cores: usize) -> usize {
    let work = rows.saturating_mul(cols).saturating_mul(samples.max(1));
    if rows < PARALLEL_MIN_ROWS || work < PARALLEL_MIN_WORK {
        return 1;
    }
    cores.min(MAX_BANDS).min(rows / PARALLEL_MIN_ROWS).max(1)
}

#[allow(clippy::too_many_arguments)]
fn raster_rows(
    edges: &[Edge],
    rule: FillRule,
    cell: f32,
    samples: usize,
    merge_tol: f32,
    first_row: f32,
    first_col: f32,
    cols: usize,
    row_start: usize,
    row_end: usize,
    out: &mut Vec<Span>,
) -> Rect {
    let mut coverage_per_cell = vec![0.0f32; cols];
    let mut crossings: Vec<(f32, i32)> = Vec::new();
    let mut spans: Vec<(f32, f32)> = Vec::new();
    let mut row_runs: Vec<(f32, f32, f32)> = Vec::new();
    let mut emitted: Vec<Run> = Vec::new();
    let mut runs: Vec<Run> = Vec::new();
    let mut coverage = Rect::ZERO;
    let inv_samples = 1.0 / samples as f32;

    for row_index in row_start..row_end {
        let y0 = (first_row + row_index as f32) * cell;
        let y1 = y0 + cell;
        coverage_per_cell.fill(0.0);

        for sample in 0..samples {
            let y = y0 + (sample as f32 + 0.5) * inv_samples * cell;
            crossings.clear();
            for e in edges {
                let (ymin, ymax) = (e.y0.min(e.y1), e.y0.max(e.y1));
                if y < ymin || y >= ymax {
                    continue;
                }
                let t = (y - e.y0) / (e.y1 - e.y0);
                crossings.push((e.x0 + (e.x1 - e.x0) * t, e.dir));
            }
            if crossings.len() < 2 {
                continue;
            }
            crossings.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

            spans.clear();
            match rule {
                FillRule::NonZero => {
                    let mut winding = 0;
                    let mut start = 0.0;
                    for &(x, dir) in &crossings {
                        let was = winding;
                        winding += dir;
                        if was == 0 && winding != 0 {
                            start = x;
                        } else if was != 0 && winding == 0 {
                            spans.push((start, x));
                        }
                    }
                }
                FillRule::EvenOdd => {
                    let mut i = 0;
                    while i + 1 < crossings.len() {
                        spans.push((crossings[i].0, crossings[i + 1].0));
                        i += 2;
                    }
                }
            }

            for &(x0, x1) in &spans {
                add_span_coverage(&mut coverage_per_cell, first_col, cell, x0, x1);
            }
        }

        row_runs.clear();
        let mut i = 0usize;
        while i < cols {
            let a = coverage_per_cell[i] * inv_samples;
            if a <= 0.0 {
                i += 1;
                continue;
            }
            let alpha = quantize_alpha(a);
            let start = i;
            i += 1;
            while i < cols {
                let b = coverage_per_cell[i] * inv_samples;
                if b <= 0.0 || quantize_alpha(b) != alpha {
                    break;
                }
                i += 1;
            }
            row_runs.push((
                (first_col + start as f32) * cell,
                (first_col + i as f32) * cell,
                alpha,
            ));
        }

        for run in runs.iter_mut() {
            run.claimed = false;
        }
        for (x0, x1, alpha) in row_runs.drain(..) {
            let mut extended = false;
            for run in runs.iter_mut() {
                if run.claimed || run.alpha != alpha {
                    continue;
                }
                if (run.x0 - x0).abs() <= merge_tol && (run.x1 - x1).abs() <= merge_tol {
                    run.x0 = run.x0.min(x0);
                    run.x1 = run.x1.max(x1);
                    run.y1 = y1;
                    run.claimed = true;
                    extended = true;
                    break;
                }
            }
            if !extended {
                runs.push(Run {
                    x0,
                    x1,
                    y0,
                    y1,
                    alpha,
                    claimed: true,
                });
            }
        }

        emitted.clear();
        runs.retain_mut(|run| {
            if run.claimed {
                run.claimed = false;
                true
            } else {
                emitted.push(*run);
                false
            }
        });
        for run in emitted.drain(..) {
            coverage = union_rect(coverage, emit_run(run, out));
        }
    }
    for run in runs.drain(..) {
        coverage = union_rect(coverage, emit_run(run, out));
    }
    coverage
}

fn add_span_coverage(coverage: &mut [f32], first_col: f32, cell: f32, x0: f32, x1: f32) {
    if x1 <= x0 {
        return;
    }
    let start = ((x0 / cell).floor() - first_col).max(0.0) as usize;
    let end = (((x1 / cell).ceil() - first_col).max(0.0) as usize).min(coverage.len());
    for (i, cell_cov) in coverage[start..end].iter_mut().enumerate() {
        let cell_left = (first_col + (start + i) as f32) * cell;
        let lo = x0.max(cell_left);
        let hi = x1.min(cell_left + cell);
        if hi > lo {
            *cell_cov += (hi - lo) / cell;
        }
    }
}

fn quantize_alpha(a: f32) -> f32 {
    // 64 levels keep curved edges smooth without exploding the number of
    // distinct span alphas (and therefore GPU instances) on large paths.
    const LEVELS: f32 = 64.0;
    if a >= 1.0 {
        1.0
    } else if a <= 0.0 {
        0.0
    } else {
        ((a * LEVELS).round().max(1.0) / LEVELS).min(1.0)
    }
}

fn emit_run(run: Run, out: &mut Vec<Span>) -> Rect {
    let rect = Rect::from_xywh(
        run.x0,
        run.y0,
        (run.x1 - run.x0).max(0.0),
        (run.y1 - run.y0).max(0.0),
    );
    if rect.is_empty() {
        return Rect::ZERO;
    }
    out.push(Span::rect(rect).with_alpha(run.alpha));
    rect
}

fn collect_edges(contours: &[Contour]) -> Vec<Edge> {
    let mut edges = Vec::new();
    for contour in contours {
        let pts = &contour.points;
        if pts.len() < 2 {
            continue;
        }
        let n = pts.len();
        for i in 0..n {
            let a = pts[i];
            let b = pts[(i + 1) % n];
            if a.y == b.y {
                continue;
            }
            if a == b {
                continue;
            }
            let dir = if b.y > a.y { 1 } else { -1 };
            edges.push(Edge {
                x0: a.x,
                y0: a.y,
                x1: b.x,
                y1: b.y,
                dir,
            });
        }
    }
    edges
}

pub(crate) fn contains(contours: &[Contour], p: Point, rule: FillRule) -> bool {
    let edges = collect_edges(contours);
    let mut winding = 0;
    let mut crossings = 0;
    for e in &edges {
        let (ymin, ymax) = (e.y0.min(e.y1), e.y0.max(e.y1));
        if p.y < ymin || p.y >= ymax {
            continue;
        }
        let t = (p.y - e.y0) / (e.y1 - e.y0);
        let x = e.x0 + (e.x1 - e.x0) * t;
        if x > p.x {
            winding += e.dir;
            crossings += 1;
        }
    }
    match rule {
        FillRule::NonZero => winding != 0,
        FillRule::EvenOdd => crossings % 2 == 1,
    }
}

#[cfg(test)]
mod tests {
    use super::{MAX_BANDS, band_count, quantize_alpha};

    #[test]
    fn band_count_follows_work_not_cells() {
        assert_eq!(band_count(1000, 1000, 4, 4), 4);
        assert_eq!(band_count(1000, 1000, 1, 4), 1, "1 sample = 1/4 work");
        assert_eq!(band_count(1000, 1000, 8, 4), 4);
    }

    #[test]
    fn band_count_keeps_small_and_single_core_shapes_serial() {
        assert_eq!(band_count(32, 4000, 4, 8), 1, "too few rows");
        assert_eq!(band_count(1000, 1000, 4, 1), 1, "single core");
        assert_eq!(band_count(1000, 1000, 4, 64), MAX_BANDS);
    }

    #[test]
    fn alpha_quantization_keeps_64_levels() {
        assert_eq!(quantize_alpha(0.0), 0.0);
        assert_eq!(quantize_alpha(1.0), 1.0);
        assert!((quantize_alpha(0.51) - 33.0 / 64.0).abs() < f32::EPSILON);
        assert!(quantize_alpha(0.01) > 0.0, "thin coverage must survive");
    }
}
