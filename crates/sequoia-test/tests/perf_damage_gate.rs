#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Benchmarks for the parts of the damage work that only exist after the
//! change: frame skipping (`paint_if_dirty`) and the layer cache (`Cached`).
//!
//! ```text
//! cargo test -p sequoia-test --test perf_damage_gate -- --nocapture
//! ```

use std::hint::black_box;
use std::time::Instant;

use sequoia_core::{Color, Length, Point, Rect, RenderList};
use sequoia_elements::Container;
use sequoia_test::Harness;
use sequoia_text::TextSystem;
use sequoia_view::prelude::*;
use sequoia_view::{FrameOutcome, Runtime};

const W: f32 = 800.0;
const H: f32 = 600.0;
const ROWS: usize = 800;
const FRAMES: usize = 300;
const WARMUP: usize = 20;

fn row(i: usize) -> impl View {
    Container::new()
        .width(Length::Fill(1.0))
        .height(36.0)
        .background(Color::rgb8(0x20, 0x20, 0x24))
        .hover(Color::rgb8(0x2e, 0x2e, 0x36))
        .padding_xy(12.0, 8.0)
        .child(
            Row::new()
                .gap(12.0)
                .child(Text::new(format!("Row {i:04}")).size(14.0))
                .child(
                    Text::new("composite dashboard detail row")
                        .size(12.0)
                        .color(Color::rgb8(0x88, 0x88, 0x88)),
                )
                .child(Text::new(format!("{:>8}", i * 37)).size(12.0)),
        )
}

fn list(cached_rows: bool) -> impl View {
    let mut col = Column::new();
    for i in 0..ROWS {
        if cached_rows {
            col = col.child(Cached::new(row(i)));
        } else {
            col = col.child(row(i));
        }
    }
    ScrollView::new(col)
}

fn setup(cached_rows: bool) -> (Harness, Runtime, TextSystem, RenderList, Rect) {
    let h = Harness::new(list(cached_rows));
    let rt = h.runtime();
    let mut text = TextSystem::new();
    let mut list = RenderList::new();
    let bounds = Rect::from_xywh(0.0, 0.0, W, H);
    rt.layout(bounds, &mut text);
    rt.paint(&mut list, &mut text);
    black_box(list.len());
    h.tick();
    (h, rt, text, list, bounds)
}

struct PaintStats {
    avg_ms: f64,
    p50_ms: f64,
    p95_ms: f64,
    items: usize,
    painted: u64,
    skipped: u64,
}

fn summarize(name: &str, stats: &PaintStats) {
    println!(
        "[gate] {name:<28} avg={:8.4}ms  p50={:8.4}ms  p95={:8.4}ms  items={}  painted={}  skipped={}",
        stats.avg_ms, stats.p50_ms, stats.p95_ms, stats.items, stats.painted, stats.skipped
    );
}

fn always_paint(
    rt: &Runtime,
    text: &mut TextSystem,
    list: &mut RenderList,
    bounds: Rect,
    mut before: impl FnMut(usize, &mut TextSystem),
) -> PaintStats {
    for i in 0..WARMUP {
        before(i, text);
        rt.tick();
        rt.layout(bounds, text);
        list.clear();
        rt.paint(list, text);
        black_box(list.len());
    }
    let mut samples = Vec::with_capacity(FRAMES);
    for i in 0..FRAMES {
        let t0 = Instant::now();
        before(i, text);
        rt.tick();
        rt.layout(bounds, text);
        list.clear();
        rt.paint(list, text);
        black_box(list.len());
        samples.push(t0.elapsed().as_secs_f64() * 1000.0);
    }
    let avg_ms = samples.iter().sum::<f64>() / samples.len() as f64;
    let mut sorted = samples.clone();
    sorted.sort_by(|a, b| a.total_cmp(b));
    PaintStats {
        avg_ms,
        p50_ms: sorted[sorted.len() / 2],
        p95_ms: sorted[(sorted.len() as f64 * 0.95) as usize],
        items: list.len(),
        painted: FRAMES as u64,
        skipped: 0,
    }
}

fn gated(
    rt: &Runtime,
    text: &mut TextSystem,
    list: &mut RenderList,
    bounds: Rect,
    mut before: impl FnMut(usize, &mut TextSystem),
) -> PaintStats {
    for i in 0..WARMUP {
        before(i, text);
        rt.tick();
        rt.layout(bounds, text);
        list.clear();
        let _ = rt.paint_if_dirty(list, text);
        black_box(list.len());
    }
    let mut samples = Vec::with_capacity(FRAMES);
    let (mut painted, mut skipped) = (0u64, 0u64);
    for i in 0..FRAMES {
        let t0 = Instant::now();
        before(i, text);
        rt.tick();
        rt.layout(bounds, text);
        list.clear();
        match rt.paint_if_dirty(list, text) {
            FrameOutcome::Painted => painted += 1,
            FrameOutcome::Skipped => skipped += 1,
        }
        black_box(list.len());
        samples.push(t0.elapsed().as_secs_f64() * 1000.0);
    }
    let avg_ms = samples.iter().sum::<f64>() / samples.len() as f64;
    let mut sorted = samples.clone();
    sorted.sort_by(|a, b| a.total_cmp(b));
    PaintStats {
        avg_ms,
        p50_ms: sorted[sorted.len() / 2],
        p95_ms: sorted[(sorted.len() as f64 * 0.95) as usize],
        items: list.len(),
        painted,
        skipped,
    }
}

fn report_pair(name: &str, always: &PaintStats, gate: &PaintStats) {
    summarize(&format!("{name}/always_paint"), always);
    summarize(&format!("{name}/paint_if_dirty"), gate);
    let speedup = always.avg_ms / gate.avg_ms.max(1e-9);
    println!(
        "[gate] {name:<28} speedup={speedup:6.1}x  (always p95={:.4}ms, gate p95={:.4}ms)",
        always.p95_ms, gate.p95_ms
    );
}

#[test]
fn damage_frame_gate_and_layer_cache() {
    println!("[gate] ---- frame gate / layer cache ({ROWS} rows, {FRAMES} frames) ----");

    // 1. Nothing changes, but the app keeps scheduling frames (Continuous
    //    redraw, spurious wakeups). Old platform painted the whole viewport.
    {
        let (_h, rt, mut text, mut list, bounds) = setup(false);
        let always = always_paint(&rt, &mut text, &mut list, bounds, |_, _| {});
        let gate = gated(&rt, &mut text, &mut list, bounds, |_, _| {});
        report_pair("idle", &always, &gate);
    }

    // 2. Pointer moves inside one hovered row: hover target does not change,
    //    so nothing visible is dirty.
    {
        let (_h, rt, mut text, mut list, bounds) = setup(false);
        let always = always_paint(&rt, &mut text, &mut list, bounds, |i, text| {
            rt.pointer_move(Point::new(700.0 + (i % 7) as f32, 18.0), text);
        });
        let gate = gated(&rt, &mut text, &mut list, bounds, |i, text| {
            rt.pointer_move(Point::new(700.0 + (i % 7) as f32, 18.0), text);
        });
        report_pair("pointer_same_target", &always, &gate);
    }

    // 3. Hover flips between two rows every frame: there is real damage, so
    //    the gate must paint every frame. Check that it does not regress.
    {
        let (_h, rt, mut text, mut list, bounds) = setup(false);
        let always = always_paint(&rt, &mut text, &mut list, bounds, |i, text| {
            let y = if i % 2 == 0 { 18.0 } else { 54.0 };
            rt.pointer_move(Point::new(700.0, y), text);
        });
        let gate = gated(&rt, &mut text, &mut list, bounds, |i, text| {
            let y = if i % 2 == 0 { 18.0 } else { 54.0 };
            rt.pointer_move(Point::new(700.0, y), text);
        });
        report_pair("hover_flip", &always, &gate);
    }

    // 4. Layer cache: static rows wrapped in `Cached` replay their RenderList
    //    instead of walking/painting the subtree. Idle forced repaint.
    {
        let (_h, rt, mut text, mut list, bounds) = setup(false);
        let plain = always_paint(&rt, &mut text, &mut list, bounds, |_, _| {});
        summarize("idle/plain_rows", &plain);

        let (_h, rt, mut text, mut list, bounds) = setup(true);
        let cached = always_paint(&rt, &mut text, &mut list, bounds, |_, _| {});
        let (hits, misses) = rt.layer_cache_stats();
        summarize("idle/cached_rows", &cached);
        println!(
            "[gate] idle/cached_rows             speedup={:6.1}x  hits={hits} misses={misses}",
            plain.avg_ms / cached.avg_ms.max(1e-9)
        );
    }

    // 5. Layer cache with real damage elsewhere: hover flips between two
    //    visible rows, the remaining static rows can replay from the cache.
    {
        let (_h, rt, mut text, mut list, bounds) = setup(false);
        let plain = always_paint(&rt, &mut text, &mut list, bounds, |i, text| {
            let y = if i % 2 == 0 { 18.0 } else { 54.0 };
            rt.pointer_move(Point::new(700.0, y), text);
        });
        summarize("hover/plain_rows", &plain);

        let (_h, rt, mut text, mut list, bounds) = setup(true);
        let cached = always_paint(&rt, &mut text, &mut list, bounds, |i, text| {
            let y = if i % 2 == 0 { 18.0 } else { 54.0 };
            rt.pointer_move(Point::new(700.0, y), text);
        });
        let (hits, misses) = rt.layer_cache_stats();
        summarize("hover/cached_rows", &cached);
        println!(
            "[gate] hover/cached_rows            speedup={:6.1}x  hits={hits} misses={misses}",
            plain.avg_ms / cached.avg_ms.max(1e-9)
        );
    }

    // 6. Coarse cache: a pulse animation elsewhere forces a paint every
    //    frame, while the whole static list replays as one cached layer.
    {
        let (_h, rt, mut text, mut list, bounds) = setup_split(false);
        let plain = always_paint(&rt, &mut text, &mut list, bounds, |_, _| {});
        summarize("anim/plain_list", &plain);

        let (_h, rt, mut text, mut list, bounds) = setup_split(true);
        let cached = always_paint(&rt, &mut text, &mut list, bounds, |_, _| {});
        let (hits, misses) = rt.layer_cache_stats();
        summarize("anim/cached_list", &cached);
        println!(
            "[gate] anim/cached_list             speedup={:6.1}x  hits={hits} misses={misses}",
            plain.avg_ms / cached.avg_ms.max(1e-9)
        );
    }
}

fn split_scene(cache_list: bool) -> impl View {
    let heavy: AnyView = if cache_list {
        AnyView::new(Cached::new(list(false)))
    } else {
        AnyView::new(list(false))
    };
    Column::new()
        .child(
            Pulse::new(Text::new("anim").size(12.0))
                .min_opacity(0.2)
                .period_duration(std::time::Duration::from_millis(400)),
        )
        .child(
            Container::new()
                .width(Length::Fill(1.0))
                .height(560.0)
                .clip(true)
                .child(heavy),
        )
}

fn setup_split(cache_list: bool) -> (Harness, Runtime, TextSystem, RenderList, Rect) {
    let h = Harness::new(split_scene(cache_list));
    let rt = h.runtime();
    let mut text = TextSystem::new();
    let mut list = RenderList::new();
    let bounds = Rect::from_xywh(0.0, 0.0, W, H);
    rt.layout(bounds, &mut text);
    rt.paint(&mut list, &mut text);
    black_box(list.len());
    h.tick();
    (h, rt, text, list, bounds)
}
