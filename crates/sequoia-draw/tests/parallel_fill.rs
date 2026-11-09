//! Parallel tessellation: band-split shapes and batches of cache misses must
//! match the serial rasterizer.

use sequoia_draw::{FillRule, Path, RasterOptions, ShapeCache, Span, fill};

fn star(points: usize, radius: f32) -> Path {
    let mut p = Path::new();
    for i in 0..points {
        let angle = i as f32 / points as f32 * std::f32::consts::TAU;
        let point = (angle.cos() * radius, angle.sin() * radius);
        if i == 0 {
            p.move_to(point);
        } else {
            p.line_to(point);
        }
    }
    p.close();
    p
}

fn alpha_at(spans: &[Span], x: f32, y: f32) -> f32 {
    spans
        .iter()
        .filter(|s| {
            x >= s.rect.left() && x < s.rect.right() && y >= s.rect.top() && y < s.rect.bottom()
        })
        .map(|s| s.alpha)
        .sum::<f32>()
        .min(1.0)
}

#[test]
fn parallel_fill_matches_sequential_fill() {
    let path = star(400, 300.0);
    let mut serial = Vec::new();
    let mut parallel = Vec::new();

    let serial_bounds = fill(
        &path,
        FillRule::NonZero,
        RasterOptions::default().parallel(false),
        &mut serial,
    );
    let parallel_bounds = fill(
        &path,
        FillRule::NonZero,
        RasterOptions::default().parallel(true),
        &mut parallel,
    );

    assert!(!serial.is_empty());
    assert_eq!(serial_bounds, parallel_bounds);
    for i in 0..=48 {
        for j in 0..=48 {
            let x = serial_bounds.left() + serial_bounds.size.width * i as f32 / 48.0;
            let y = serial_bounds.top() + serial_bounds.size.height * j as f32 / 48.0;
            let (a, b) = (alpha_at(&serial, x, y), alpha_at(&parallel, x, y));
            assert!(
                (a - b).abs() <= 0.02,
                "coverage differs at ({x}, {y}): serial {a}, parallel {b}"
            );
        }
    }
}

#[test]
fn small_shapes_stay_on_the_calling_thread() {
    let path = star(8, 6.0);
    let mut spans = Vec::new();
    fill(
        &path,
        FillRule::NonZero,
        RasterOptions::default(),
        &mut spans,
    );
    assert!(!spans.is_empty());
}

#[test]
#[ignore = "timing probe: run with --ignored --nocapture"]
fn timing_probe() {
    let path = star(400, 500.0);
    for parallel in [false, true] {
        let opts = RasterOptions::default().parallel(parallel);
        let mut spans = Vec::new();
        fill(&path, FillRule::NonZero, opts, &mut spans);
        let start = std::time::Instant::now();
        for _ in 0..10 {
            spans.clear();
            fill(&path, FillRule::NonZero, opts, &mut spans);
        }
        let ms = start.elapsed().as_secs_f64() * 1000.0 / 10.0;
        println!(
            "parallel={parallel}: {ms:.2} ms/fill ({} spans)",
            spans.len()
        );
    }
}

#[test]
fn prefetch_batches_misses_and_matches_serial_spans() {
    let cache = ShapeCache::default();
    let paths: Vec<Path> = (0..8)
        .map(|i| star(64 + i, 200.0 + i as f32 * 5.0))
        .collect();
    let opts = RasterOptions::default();

    cache.begin_prefetch();
    for path in &paths {
        let placeholder = cache.fill(path, FillRule::NonZero, opts);
        assert!(placeholder.is_empty(), "prefetch must not tessellate yet");
    }
    assert_eq!(cache.end_prefetch(), paths.len());
    assert_eq!(cache.tessellate_prefetch(), paths.len());

    for path in &paths {
        let cached = cache.fill(path, FillRule::NonZero, opts);
        let mut serial = Vec::new();
        fill(path, FillRule::NonZero, opts.parallel(false), &mut serial);
        assert!(!serial.is_empty());
        assert_eq!(
            cached.spans.as_ref(),
            serial.as_slice(),
            "batch spans must equal the serial rasterizer"
        );
    }
    assert!(cache.stats().hits >= paths.len() as u64);
}

#[test]
fn prefetch_counts_a_repeated_key_once() {
    let cache = ShapeCache::default();
    let path = star(64, 200.0);
    let opts = RasterOptions::default();

    cache.begin_prefetch();
    let _ = cache.fill(&path, FillRule::NonZero, opts);
    let _ = cache.fill(&path, FillRule::NonZero, opts);
    assert_eq!(cache.end_prefetch(), 1);
    assert_eq!(cache.tessellate_prefetch(), 1);

    let stats = cache.stats();
    assert_eq!(stats.misses, 1, "{stats:?}");
    assert_eq!(stats.hits, 1, "{stats:?}");
}

/// A canvas-like mix: unique positions and sizes so every cache key misses.
fn canvas_paths(count: usize) -> Vec<Path> {
    (0..count)
        .map(|i| {
            let points = 8 + (i % 4) * 8;
            let radius = 4.0 + (i % 37) as f32 * 5.0;
            let x = (i % 100) as f32 * 18.0;
            let y = (i / 100) as f32 * 12.0;
            star(points, radius).translated(x, y)
        })
        .collect()
}

#[test]
#[ignore = "benchmark: run with --ignored --nocapture"]
fn canvas_scene_benchmark() {
    let cores = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    let opts = RasterOptions::default();
    for count in [1_000usize, 10_000] {
        let paths = canvas_paths(count);
        let mut spans = Vec::new();

        let start = std::time::Instant::now();
        for path in &paths {
            spans.clear();
            fill(path, FillRule::NonZero, opts.parallel(false), &mut spans);
        }
        let serial = start.elapsed().as_secs_f64() * 1000.0;

        let start = std::time::Instant::now();
        for path in &paths {
            spans.clear();
            fill(path, FillRule::NonZero, opts, &mut spans);
        }
        let bands = start.elapsed().as_secs_f64() * 1000.0;

        let cache = ShapeCache::new(paths.len());
        let start = std::time::Instant::now();
        cache.begin_prefetch();
        for path in &paths {
            let _ = cache.fill(path, FillRule::NonZero, opts);
        }
        let misses = cache.end_prefetch();
        let built = cache.tessellate_prefetch();
        let batch = start.elapsed().as_secs_f64() * 1000.0;

        println!(
            "{cores} cores, {count} paths: serial {serial:.1} ms, \
             bands {bands:.1} ms, batch {batch:.1} ms \
             ({misses} misses, {built} built)"
        );
    }
}

#[test]
#[ignore = "timing probe: run with --ignored --nocapture"]
fn batch_timing_probe() {
    let paths: Vec<Path> = (0..200).map(|i| star(48, 100.0 + i as f32 * 0.5)).collect();
    let opts = RasterOptions::default();

    let cache = ShapeCache::default();
    let start = std::time::Instant::now();
    for path in &paths {
        let _ = cache.fill(path, FillRule::NonZero, opts);
    }
    let serial = start.elapsed().as_secs_f64() * 1000.0;

    let batch_cache = ShapeCache::default();
    let start = std::time::Instant::now();
    batch_cache.begin_prefetch();
    for path in &paths {
        let _ = batch_cache.fill(path, FillRule::NonZero, opts);
    }
    let misses = batch_cache.end_prefetch();
    let built = batch_cache.tessellate_prefetch();
    let batched = start.elapsed().as_secs_f64() * 1000.0;

    let cores = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    println!(
        "{cores} cores, {} shapes: serial {serial:.2} ms, batch {batched:.2} ms \
         ({misses} misses, {built} built)",
        paths.len()
    );
}
