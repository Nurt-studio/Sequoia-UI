use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use sequoia_core::{Point, Rect};

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
}

#[derive(Debug)]
struct Entry {
    shape: Rc<Shape>,
    used: u64,
}

pub const DEFAULT_CAPACITY: usize = 256;

#[derive(Debug)]
pub struct ShapeCache {
    inner: RefCell<Inner>,
}

#[derive(Debug)]
struct Inner {
    entries: HashMap<Key, Entry>,
    capacity: usize,
    tick: u64,
    stats: ShapeStats,
}

impl Default for ShapeCache {
    fn default() -> Self {
        Self::new(DEFAULT_CAPACITY)
    }
}

impl ShapeCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: RefCell::new(Inner {
                entries: HashMap::new(),
                capacity,
                tick: 0,
                stats: ShapeStats::default(),
            }),
        }
    }

    pub fn capacity(&self) -> usize {
        self.inner.borrow().capacity
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
        };
        self.get_or_build(key, |out| {
            raster::fill(path, style.rule, style.opts, out);
        })
    }

    pub fn stroke(&self, path: &Path, style: &StrokeStyle, opts: RasterOptions) -> Rc<Shape> {
        let key = Key {
            kind: 1,
            fingerprint: path.fingerprint(),
            style: style.fingerprint(),
            opts: opts_key(opts),
        };
        self.get_or_build(key, |out| {
            stroke::stroke_path(path, style, opts, out);
        })
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
        };
        self.get_or_build(key, |out| {
            raster::fill_contours(contours, rule, opts, out);
        })
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

    fn get_or_build(&self, key: Key, build: impl FnOnce(&mut Vec<Span>)) -> Rc<Shape> {
        let mut inner = self.inner.borrow_mut();
        inner.tick += 1;
        let tick = inner.tick;
        if let Some(entry) = inner.entries.get_mut(&key) {
            entry.used = tick;
            let shape = entry.shape.clone();
            inner.stats.hits += 1;
            return shape;
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
        if inner.entries.len() >= inner.capacity
            && let Some(oldest) = inner
                .entries
                .iter()
                .min_by_key(|(_, entry)| entry.used)
                .map(|(key, _)| *key)
        {
            inner.entries.remove(&oldest);
            inner.stats.evictions += 1;
        }
        inner.entries.insert(
            key,
            Entry {
                shape: shape.clone(),
                used: tick,
            },
        );
        shape
    }
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
