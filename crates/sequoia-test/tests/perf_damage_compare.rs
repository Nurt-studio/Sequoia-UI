#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Cross-version benchmark for the damage/layout-skip work.
//!
//! Uses only APIs that exist both before and after the change, so the same
//! file can be run against a baseline checkout:
//!
//! ```text
//! cargo test -p sequoia-test --test perf_damage_compare -- --nocapture
//! ```
//!
//! Every scenario drives a full frame the way the platform did before the
//! change: `tick` -> `layout` -> `paint` (always, no frame gate).

use std::hint::black_box;
use std::time::{Duration, Instant};

use sequoia_core::{Color, Length, Rect, RenderList, Vec2};
use sequoia_elements::Container;
use sequoia_reactive::Signal;
use sequoia_test::Harness;
use sequoia_text::TextSystem;
use sequoia_view::prelude::*;

const W: f32 = 800.0;
const H: f32 = 600.0;
const ROWS: usize = 800;
const FRAMES: usize = 200;
const WARMUP: usize = 20;
const LIVE_ROWS: usize = 120;
const LIVE_FRAMES: usize = 80;

#[derive(Clone, Copy)]
struct Sample {
    total_ms: f64,
    layout_ms: f64,
    paint_ms: f64,
    items: usize,
}

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

fn list_n(rows: usize, extra: impl View + 'static) -> impl View {
    let mut col = Column::new().child(extra);
    for i in 0..rows {
        col = col.child(row(i));
    }
    ScrollView::new(col)
}

fn list(extra: impl View + 'static) -> impl View {
    list_n(ROWS, extra)
}

struct Live {
    tick: Signal<u64>,
    rows: usize,
}

impl View for Live {
    fn body(&self) -> impl View + 'static {
        let t = self.tick.clone();
        list_n(
            self.rows,
            Text::with(move || format!("tick {}", t.get())).size(16.0),
        )
    }
}

struct Pulsing;

impl View for Pulsing {
    fn body(&self) -> impl View + 'static {
        list(
            Pulse::new(Text::new("live").size(16.0))
                .min_opacity(0.2)
                .period_duration(Duration::from_millis(400)),
        )
    }
}

fn summarize(name: &str, samples: &[Sample]) {
    let n = samples.len() as f64;
    let avg = samples.iter().map(|s| s.total_ms).sum::<f64>() / n;
    let avg_layout = samples.iter().map(|s| s.layout_ms).sum::<f64>() / n;
    let avg_paint = samples.iter().map(|s| s.paint_ms).sum::<f64>() / n;
    let mut sorted: Vec<f64> = samples.iter().map(|s| s.total_ms).collect();
    sorted.sort_by(|a, b| a.total_cmp(b));
    let p50 = sorted[sorted.len() / 2];
    let p95 = sorted[(sorted.len() as f64 * 0.95) as usize];
    let max = sorted[sorted.len() - 1];
    println!(
        "[perf] {name:<18} avg={avg:8.3}ms  layout={avg_layout:8.3}ms  paint={avg_paint:8.3}ms  p50={p50:8.3}ms  p95={p95:8.3}ms  max={max:8.3}ms  items={}",
        samples.last().map(|s| s.items).unwrap_or(0)
    );
}

fn bench<F>(frames: usize, mut frame: F) -> Vec<Sample>
where
    F: FnMut(usize) -> Sample,
{
    for i in 0..WARMUP {
        let s = frame(i);
        black_box((s.total_ms, s.items));
    }
    let mut out = Vec::with_capacity(frames);
    for i in 0..frames {
        out.push(frame(i));
    }
    out
}

fn scene_harness(root: impl View + 'static) -> (Harness, Runtime, TextSystem, RenderList, Rect) {
    let h = Harness::new(root);
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

#[test]
fn recent_changes_performance() {
    let cpus = std::thread::available_parallelism()
        .map(|v| v.get())
        .unwrap_or(1);
    println!(
        "[perf] ---- damage/layout-skip comparison ({ROWS} rows, {FRAMES} frames, {cpus} cpu) ----"
    );

    // 1. Forced idle frames: nothing changed, but the old pipeline still
    //    relayouts and repaints every frame (e.g. a Continuous redraw loop).
    {
        let (_h, rt, mut text, mut list, bounds) = scene_harness(list(Text::new("header")));
        let samples = bench(FRAMES, |_| {
            let t0 = Instant::now();
            rt.tick();
            let t1 = Instant::now();
            rt.layout(bounds, &mut text);
            let layout_ms = t1.elapsed().as_secs_f64() * 1000.0;
            let t2 = Instant::now();
            list.clear();
            rt.paint(&mut list, &mut text);
            let paint_ms = t2.elapsed().as_secs_f64() * 1000.0;
            black_box(list.len());
            Sample {
                total_ms: t0.elapsed().as_secs_f64() * 1000.0,
                layout_ms,
                paint_ms,
                items: list.len(),
            }
        });
        summarize("idle", &samples);
    }

    // 2. Paint-only animation (opacity pulse): frames keep coming, but the
    //    tree does not need layout. This is the layout-skip win.
    {
        let (_h, rt, mut text, mut list, bounds) = scene_harness(Pulsing);
        let samples = bench(FRAMES, |_| {
            let t0 = Instant::now();
            rt.tick();
            let t1 = Instant::now();
            rt.layout(bounds, &mut text);
            let layout_ms = t1.elapsed().as_secs_f64() * 1000.0;
            let t2 = Instant::now();
            list.clear();
            rt.paint(&mut list, &mut text);
            let paint_ms = t2.elapsed().as_secs_f64() * 1000.0;
            black_box(list.len());
            Sample {
                total_ms: t0.elapsed().as_secs_f64() * 1000.0,
                layout_ms,
                paint_ms,
                items: list.len(),
            }
        });
        summarize("pulse_anim", &samples);
    }

    // 3. Hover repaints: paint-only damage. Old code relayouts the whole
    //    dashboard for every hover transition.
    {
        let (_h, rt, mut text, mut list, bounds) = scene_harness(list(Unit));
        let samples = bench(FRAMES, |i| {
            let y = if i % 2 == 0 { 18.0 } else { 54.0 };
            let t0 = Instant::now();
            rt.pointer_move(sequoia_core::Point::new(700.0, y), &mut text);
            rt.tick();
            let t1 = Instant::now();
            rt.layout(bounds, &mut text);
            let layout_ms = t1.elapsed().as_secs_f64() * 1000.0;
            let t2 = Instant::now();
            list.clear();
            rt.paint(&mut list, &mut text);
            let paint_ms = t2.elapsed().as_secs_f64() * 1000.0;
            black_box(list.len());
            Sample {
                total_ms: t0.elapsed().as_secs_f64() * 1000.0,
                layout_ms,
                paint_ms,
                items: list.len(),
            }
        });
        summarize("hover", &samples);
    }

    // 4. Scrolling: layout is legitimately dirty every frame. This is the
    //    regression guard - the damage work must not make it slower.
    {
        let (h, rt, mut text, mut list, bounds) = scene_harness(list(Text::new("header")));
        let root = h.root().expect("root");
        let scroll_el = if h.view::<ScrollView>(root).is_some() {
            root
        } else {
            h.children(root)[0]
        };
        let scroll = h.view::<ScrollView>(scroll_el).expect("ScrollView");
        let max = scroll.max_offset(&rt, scroll_el);
        assert!(max > 0.0, "content must scroll");
        let samples = bench(FRAMES, |i| {
            let dir = if i % 120 < 60 { 1.0 } else { -1.0 };
            let t0 = Instant::now();
            scroll.scroll_by(&rt, scroll_el, Vec2::new(0.0, dir * 5.0));
            rt.tick();
            let t1 = Instant::now();
            rt.layout(bounds, &mut text);
            let layout_ms = t1.elapsed().as_secs_f64() * 1000.0;
            let t2 = Instant::now();
            list.clear();
            rt.paint(&mut list, &mut text);
            let paint_ms = t2.elapsed().as_secs_f64() * 1000.0;
            black_box(list.len());
            Sample {
                total_ms: t0.elapsed().as_secs_f64() * 1000.0,
                layout_ms,
                paint_ms,
                items: list.len(),
            }
        });
        summarize("scroll", &samples);
    }

    // 5. Live signal: text rebuild means layout is dirty every frame. Also a
    //    regression guard.
    {
        let tick = Signal::new(0u64);
        let (_h, rt, mut text, mut list, bounds) = scene_harness(Live {
            tick: tick.clone(),
            rows: LIVE_ROWS,
        });
        let samples = bench(LIVE_FRAMES, |i| {
            let t0 = Instant::now();
            tick.set(i as u64);
            rt.tick();
            let t1 = Instant::now();
            rt.layout(bounds, &mut text);
            let layout_ms = t1.elapsed().as_secs_f64() * 1000.0;
            let t2 = Instant::now();
            list.clear();
            rt.paint(&mut list, &mut text);
            let paint_ms = t2.elapsed().as_secs_f64() * 1000.0;
            black_box(list.len());
            Sample {
                total_ms: t0.elapsed().as_secs_f64() * 1000.0,
                layout_ms,
                paint_ms,
                items: list.len(),
            }
        });
        summarize("live_signal", &samples);
    }
}
