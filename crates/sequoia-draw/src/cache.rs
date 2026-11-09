use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use sequoia_core::{Point, Rect, Vec2};

use crate::flatten::Contour;
use crate::path::{FillRule, Path};
use crate::raster::{self, FillStyle, RasterOptions};
use crate::stroke::{self, StrokeStyle};
use crate::{Span, union_rect};

#[derive(Clone, Debug, PartialEq)]
pub struct Shape {
    pub spans: Rc<[Span]>,

    pub bounds: Rect,
}

impl Shape {
    pub fn new(spans: Vec<Span>) -> Self {
        let mut bounds = Rect::ZERO;
        for span in &spans {
            bounds = union_rect(bounds, span.rect);
        }
        Self {
            spans: spans.into(),
            bounds,
        }
    }

    pub fn empty() -> Self {
        Self {
            spans: Rc::from(Vec::new()),
            bounds: Rect::ZERO,
        }
    }

    pub fn len(&self) -> usize {
        self.spans.len()
    }

    pub fn is_empty(&self) -> bool {
        self.spans.is_empty()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Span> {
        self.spans.iter()
    }
}

impl<'a> IntoIterator for &'a Shape {
    type Item = &'a Span;
    type IntoIter = std::slice::Iter<'a, Span>;

    fn into_iter(self) -> Self::IntoIter {
        self.spans.iter()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShapeStats {
    pub hits: u64,

    pub misses: u64,

    pub evictions: u64,
}

impl ShapeStats {
    pub fn hit_rate(&self) -> f32 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f32 / total as f32
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Key {
    kind: u8,
    fingerprint: u64,
    style: u64,
    opts: u64,
    offset: u64,
}

#[derive(Debug)]
struct Entry {
    shape: Rc<Shape>,
    used: u64,
}

/// Owned description of a cache miss, captured so it can be tessellated after
/// the paint pass that asked for it has returned.
#[derive(Debug)]
enum Request {
    Fill {
        path: Path,
        rule: FillRule,
        opts: RasterOptions,
        offset: Vec2,
    },
    Stroke {
        path: Path,
        style: StrokeStyle,
        opts: RasterOptions,
        offset: Vec2,
    },
    Contours {
        contours: Vec<Contour>,
        rule: FillRule,
        opts: RasterOptions,
    },
}

impl Request {
    fn opts(&self) -> RasterOptions {
        match self {
            Request::Fill { opts, .. }
            | Request::Stroke { opts, .. }
            | Request::Contours { opts, .. } => *opts,
        }
    }

    /// Rough work estimate: source vertices, not raster cells. Used only to
    /// decide whether a batch is worth spawning threads for.
    fn cost(&self) -> usize {
        match self {
            Request::Fill { path, .. } | Request::Stroke { path, .. } => path.segs().len(),
            Request::Contours { contours, .. } => contours.iter().map(|c| c.points.len()).sum(),
        }
    }

    fn build(&self, opts: RasterOptions, out: &mut Vec<Span>) {
        match self {
            Request::Fill {
                path, rule, offset, ..
            } => {
                if *offset == Vec2::ZERO {
                    raster::fill(path, *rule, opts, out);
                } else {
                    raster::fill(&path.translated(offset.x, offset.y), *rule, opts, out);
                }
            }
            Request::Stroke {
                path,
                style,
                offset,
                ..
            } => {
                if *offset == Vec2::ZERO {
                    stroke::stroke_path(path, style, opts, out);
                } else {
                    stroke::stroke_path(&path.translated(offset.x, offset.y), style, opts, out);
                }
            }
            Request::Contours { contours, rule, .. } => {
                raster::fill_contours(contours, *rule, opts, out);
            }
        }
    }
}

/// Below this many shapes a batch stays on the calling thread: thread startup
/// costs more than a few small rasters.
const BATCH_MIN_SHAPES: usize = 4;

/// Below this many source vertices a batch stays on the calling thread, even
/// when it has enough shapes.
const BATCH_MIN_COST: usize = 256;

const BATCH_MAX_WORKERS: usize = 8;

pub const DEFAULT_CAPACITY: usize = 256;

/// Upper bound for the default cache. A scene whose *live* working set is
/// bigger than [`DEFAULT_CAPACITY`] grows up to this size instead of
/// re-tessellating the same outlines every frame (see [`ShapeCache::growing`]).
pub const MAX_CAPACITY: usize = 2048;

#[derive(Debug)]
pub struct ShapeCache {
    inner: RefCell<Inner>,
}

#[derive(Debug)]
struct Inner {
    entries: HashMap<Key, Entry>,
    capacity: usize,
    max_capacity: usize,
    tick: u64,
    stats: ShapeStats,
    prefetching: bool,
    pending: Vec<(Key, Request)>,
    pending_keys: HashSet<Key>,
}

impl Default for ShapeCache {
    fn default() -> Self {
        Self::growing(DEFAULT_CAPACITY, MAX_CAPACITY)
    }
}

impl ShapeCache {
    pub fn new(capacity: usize) -> Self {
        Self::growing(capacity, capacity)
    }

    /// A cache that may grow from `capacity` up to `max_capacity` once all of
    /// its entries are live, i.e. every one was touched within the last
    /// `capacity` lookups. Evicting an on-screen shape forces the next frame to
    /// re-tessellate it, so growing once is the cheaper response to a working
    /// set that outgrew the cache.
    pub fn growing(capacity: usize, max_capacity: usize) -> Self {
        let max_capacity = max_capacity.max(capacity);
        Self {
            inner: RefCell::new(Inner {
                entries: HashMap::new(),
                capacity,
                max_capacity,
                tick: 0,
                stats: ShapeStats::default(),
                prefetching: false,
                pending: Vec::new(),
                pending_keys: HashSet::new(),
            }),
        }
    }

    pub fn capacity(&self) -> usize {
        self.inner.borrow().capacity
    }

    pub fn max_capacity(&self) -> usize {
        self.inner.borrow().max_capacity
    }

    pub fn len(&self) -> usize {
        self.inner.borrow().entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.borrow().entries.is_empty()
    }

    pub fn stats(&self) -> ShapeStats {
        self.inner.borrow().stats
    }

    pub fn reset_stats(&self) {
        self.inner.borrow_mut().stats = ShapeStats::default();
    }

    pub fn clear(&self) {
        self.inner.borrow_mut().entries.clear();
    }

    pub fn fill(&self, path: &Path, rule: FillRule, opts: RasterOptions) -> Rc<Shape> {
        self.fill_styled(path, FillStyle::new(rule, opts))
    }

    pub fn fill_styled(&self, path: &Path, style: FillStyle) -> Rc<Shape> {
        let key = Key {
            kind: 0,
            fingerprint: path.fingerprint(),
            style: style.rule as u64,
            opts: opts_key(style.opts),
            offset: 0,
        };
        self.get_or_build(
            key,
            |out| {
                raster::fill(path, style.rule, style.opts, out);
            },
            || Request::Fill {
                path: path.clone(),
                rule: style.rule,
                opts: style.opts,
                offset: Vec2::ZERO,
            },
        )
    }

    pub fn fill_offset(
        &self,
        path: &Path,
        rule: FillRule,
        opts: RasterOptions,
        offset: Vec2,
    ) -> Rc<Shape> {
        let key = Key {
            kind: 0,
            fingerprint: path.fingerprint(),
            style: rule as u64,
            opts: opts_key(opts),
            offset: offset_key(offset),
        };
        self.get_or_build(
            key,
            |out| {
                let translated = path.translated(offset.x, offset.y);
                raster::fill(&translated, rule, opts, out);
            },
            || Request::Fill {
                path: path.clone(),
                rule,
                opts,
                offset,
            },
        )
    }

    pub fn stroke(&self, path: &Path, style: &StrokeStyle, opts: RasterOptions) -> Rc<Shape> {
        let key = Key {
            kind: 1,
            fingerprint: path.fingerprint(),
            style: style.fingerprint(),
            opts: opts_key(opts),
            offset: 0,
        };
        self.get_or_build(
            key,
            |out| {
                stroke::stroke_path(path, style, opts, out);
            },
            || Request::Stroke {
                path: path.clone(),
                style: style.clone(),
                opts,
                offset: Vec2::ZERO,
            },
        )
    }

    pub fn stroke_offset(
        &self,
        path: &Path,
        style: &StrokeStyle,
        opts: RasterOptions,
        offset: Vec2,
    ) -> Rc<Shape> {
        let key = Key {
            kind: 1,
            fingerprint: path.fingerprint(),
            style: style.fingerprint(),
            opts: opts_key(opts),
            offset: offset_key(offset),
        };
        self.get_or_build(
            key,
            |out| {
                let translated = path.translated(offset.x, offset.y);
                stroke::stroke_path(&translated, style, opts, out);
            },
            || Request::Stroke {
                path: path.clone(),
                style: style.clone(),
                opts,
                offset,
            },
        )
    }

    pub fn fill_contours(
        &self,
        contours: &[crate::Contour],
        rule: FillRule,
        opts: RasterOptions,
    ) -> Rc<Shape> {
        let key = Key {
            kind: 2,
            fingerprint: contours_fingerprint(contours),
            style: rule as u64,
            opts: opts_key(opts),
            offset: 0,
        };
        self.get_or_build(
            key,
            |out| {
                raster::fill_contours(contours, rule, opts, out);
            },
            || Request::Contours {
                contours: contours.to_vec(),
                rule,
                opts,
            },
        )
    }

    pub fn disk(&self, center: Point, radius: f32) -> Rc<Shape> {
        if !(radius > 0.0) || !radius.is_finite() {
            return Rc::new(Shape::empty());
        }
        Rc::new(Shape::new(vec![Span::rounded(
            Rect::from_xywh(
                center.x - radius,
                center.y - radius,
                radius * 2.0,
                radius * 2.0,
            ),
            radius,
        )]))
    }

    pub fn rounded_rect(&self, rect: Rect, radius: f32) -> Rc<Shape> {
        let radius = radius
            .clamp(0.0, rect.size.width.min(rect.size.height) * 0.5)
            .max(0.0);
        Rc::new(Shape::new(vec![Span::rounded(rect, radius)]))
    }

    pub fn rect(&self, rect: Rect) -> Rc<Shape> {
        Rc::new(Shape::new(vec![Span::rect(rect)]))
    }

    /// Starts a prefetch pass.
    ///
    /// While the pass is active, cache misses are recorded instead of
    /// tessellated, and each miss returns an empty placeholder [`Shape`]. The
    /// caller is expected to paint again after
    /// [`ShapeCache::tessellate_prefetch`], so the placeholders never reach the
    /// screen. Do not retain placeholder shapes across the pass.
    pub fn begin_prefetch(&self) {
        let mut inner = self.inner.borrow_mut();
        inner.prefetching = true;
        inner.pending.clear();
        inner.pending_keys.clear();
    }

    /// Ends a prefetch pass and returns how many distinct shapes missed the
    /// cache. Nothing has been tessellated yet.
    pub fn end_prefetch(&self) -> usize {
        let mut inner = self.inner.borrow_mut();
        inner.prefetching = false;
        inner.pending.len()
    }

    /// Tessellates the shapes recorded by the last prefetch pass and inserts
    /// them into the cache, returning how many were built.
    ///
    /// Big batches are split across worker threads; each worker builds its
    /// shapes with `parallel(false)` because the batch already keeps the cores
    /// busy. Small batches stay on the calling thread, and a lone shape keeps
    /// its own band-parallel rasterizer. Results are inserted in request order
    /// so a batch produces the same spans every time.
    pub fn tessellate_prefetch(&self) -> usize {
        let pending = {
            let mut inner = self.inner.borrow_mut();
            inner.prefetching = false;
            inner.pending_keys.clear();
            std::mem::take(&mut inner.pending)
        };
        if pending.is_empty() {
            return 0;
        }

        let cost: usize = pending.iter().map(|(_, req)| req.cost()).sum();
        let workers = if pending.len() >= BATCH_MIN_SHAPES && cost >= BATCH_MIN_COST {
            std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1)
                .min(BATCH_MAX_WORKERS)
                .min(pending.len())
        } else {
            1
        };

        let built: Vec<(Key, Vec<Span>)> = if workers <= 1 {
            pending
                .iter()
                .map(|(key, req)| (*key, build_request(req, req.opts())))
                .collect()
        } else {
            let chunk = pending.len().div_ceil(workers);
            let mut parts: Vec<Vec<(Key, Vec<Span>)>> = Vec::with_capacity(workers);
            std::thread::scope(|scope| {
                let mut handles = Vec::with_capacity(workers);
                for chunk in pending.chunks(chunk) {
                    handles.push(scope.spawn(move || {
                        chunk
                            .iter()
                            .map(|(key, req)| {
                                (*key, build_request(req, req.opts().parallel(false)))
                            })
                            .collect::<Vec<_>>()
                    }));
                }
                // Join in submission order so the cache sees requests in the
                // order they were asked for.
                for handle in handles {
                    if let Ok(part) = handle.join() {
                        parts.push(part);
                    }
                }
            });
            parts.into_iter().flatten().collect()
        };

        let count = built.len();
        let mut inner = self.inner.borrow_mut();
        for (key, spans) in built {
            inner.tick += 1;
            let used = inner.tick;
            inner.insert_used(key, Rc::new(Shape::new(spans)), used);
        }
        count
    }

    fn get_or_build(
        &self,
        key: Key,
        build: impl FnOnce(&mut Vec<Span>),
        request: impl FnOnce() -> Request,
    ) -> Rc<Shape> {
        let mut inner = self.inner.borrow_mut();
        inner.tick += 1;
        let tick = inner.tick;
        if let Some(entry) = inner.entries.get_mut(&key) {
            entry.used = tick;
            let shape = entry.shape.clone();
            inner.stats.hits += 1;
            return shape;
        }
        if inner.prefetching {
            if inner.pending_keys.insert(key) {
                inner.stats.misses += 1;
                inner.pending.push((key, request()));
            } else {
                // A repeat lookup in the same pass would have been a hit if
                // the first one had been built, so do not report it as a miss.
                inner.stats.hits += 1;
            }
            return Rc::new(Shape::empty());
        }
        inner.stats.misses += 1;
        if inner.capacity == 0 {
            drop(inner);
            let mut spans = Vec::new();
            build(&mut spans);
            return Rc::new(Shape::new(spans));
        }

        let mut spans = Vec::new();
        build(&mut spans);
        let shape = Rc::new(Shape::new(spans));
        inner.insert_used(key, shape.clone(), tick);
        shape
    }
}

impl Inner {
    fn insert_used(&mut self, key: Key, shape: Rc<Shape>, used: u64) {
        if self.capacity == 0 {
            return;
        }
        if self.entries.len() >= self.capacity
            && let Some((oldest, oldest_used)) = self
                .entries
                .iter()
                .min_by_key(|(_, entry)| entry.used)
                .map(|(key, entry)| (*key, entry.used))
        {
            // Everything is still in use, so the working set outgrew the
            // cache: grow instead of evicting a shape that is on screen.
            let hot = used.saturating_sub(oldest_used) <= self.capacity as u64;
            if hot && self.capacity < self.max_capacity {
                self.capacity = self.capacity.saturating_mul(2).min(self.max_capacity);
            } else {
                self.entries.remove(&oldest);
                self.stats.evictions += 1;
            }
        }
        self.entries.insert(key, Entry { shape, used });
    }
}

fn build_request(req: &Request, opts: RasterOptions) -> Vec<Span> {
    let mut spans = Vec::new();
    req.build(opts, &mut spans);
    spans
}

fn offset_key(offset: Vec2) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for v in [offset.x, offset.y] {
        let v = if v == 0.0 { 0.0 } else { v };
        hash ^= u64::from(v.to_bits());
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn opts_key(opts: RasterOptions) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for v in [opts.tolerance, opts.row_height, opts.merge_tol] {
        hash ^= u64::from(v.to_bits());
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash ^= u64::from(opts.aa_samples);
    hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    hash ^= u64::from(opts.analytic);
    hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    hash ^= u64::from(opts.parallel);
    hash.wrapping_mul(0x0000_0100_0000_01b3)
}

fn contours_fingerprint(contours: &[crate::Contour]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut eat = |v: u64| {
        hash ^= v;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    };
    for c in contours {
        eat(c.points.len() as u64);
        eat(u64::from(c.closed));
        for p in &c.points {
            eat(u64::from(p.x.to_bits()));
            eat(u64::from(p.y.to_bits()));
        }
    }
    hash
}
