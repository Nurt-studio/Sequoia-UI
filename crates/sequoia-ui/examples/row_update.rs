use sequoia_ui::prelude::*;
use sequoia_ui::render::PresentMode;
use sequoia_core::{Rect, RenderList};
use sequoia_view::Runtime;
use sequoia_text::TextSystem;
use sequoia_reactive::Environment;
use sequoia_assets::AssetRegistry;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

const ROWS: usize = 3_000;
const WARMUP: usize = 10;
const RUNS: usize = 100;
const TOTAL_FRAMES: u32 = (WARMUP + RUNS) as u32;
const TARGET_ROW: usize = 10;

#[derive(Clone, PartialEq)]
struct RowData {
    id: usize,
    shade: f32,
}

// ---------------------------------------------------------------------------
// Probe
// ---------------------------------------------------------------------------
// Measures "from frame start until the last rebuilt render closure finished".
// `tick()` is called at the END of each content closure, so `last` is when the
// last rebuild completed. `rebuilt` is the number of closure runs in a frame.
//
// Signals are read inside the closures, but mutations NEVER happen here.

#[derive(Default, Clone)]
struct Probe {
    t0: Arc<Mutex<Option<Instant>>>,
    last: Arc<Mutex<Option<Instant>>>,
    rebuilt: Arc<AtomicUsize>,
    times: Arc<Mutex<Vec<u128>>>,
    frame_times: Arc<Mutex<Vec<u128>>>,
    rebuilds_per_frame: Arc<Mutex<Vec<usize>>>,
    frames: Arc<AtomicUsize>,
}

impl Probe {
    fn begin_frame(&self) {
        *self.t0.lock().unwrap() = Some(Instant::now());
        *self.last.lock().unwrap() = None;
        self.rebuilt.store(0, Ordering::Relaxed);
    }

    fn tick(&self) {
        *self.last.lock().unwrap() = Some(Instant::now());
        self.rebuilt.fetch_add(1, Ordering::Relaxed);
    }

    fn end_frame(&self) {
        let t0 = *self.t0.lock().unwrap();
        let last = *self.last.lock().unwrap();
        if let (Some(start), Some(end)) = (t0, last) {
            self.times
                .lock()
                .unwrap()
                .push(end.duration_since(start).as_micros());
        }
        if let Some(start) = t0 {
            self.frame_times
                .lock()
                .unwrap()
                .push(Instant::now().duration_since(start).as_micros());
        }
        self.rebuilds_per_frame
            .lock()
            .unwrap()
            .push(self.rebuilt.load(Ordering::Relaxed));
        self.frames.fetch_add(1, Ordering::Relaxed);
    }

    fn results(&self) -> ProbeResults {
        ProbeResults {
            times: self.times.lock().unwrap().clone(),
            frame_times: self.frame_times.lock().unwrap().clone(),
            rebuilds: self.rebuilds_per_frame.lock().unwrap().clone(),
            frames: self.frames.load(Ordering::Relaxed),
        }
    }
}

struct ProbeResults {
    times: Vec<u128>,
    frame_times: Vec<u128>,
    rebuilds: Vec<usize>,
    #[allow(dead_code)]
    frames: usize,
}

// ---------------------------------------------------------------------------
// Scenarios
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
enum Scenario {
    FineOne,   // mutates 1 row          -> 1 render closure runs
    CoarseOne, // mutates 1 row + `rev`  -> all 3000 closures run
    FineAll,   // mutates every row      -> all 3000 closures run
}

impl Scenario {
    fn label(self) -> &'static str {
        match self {
            Scenario::FineOne => "S1 fine / 1 wiersz  ",
            Scenario::CoarseOne => "S2 coarse / 1 wiersz",
            Scenario::FineAll => "S3 fine / wszystkie ",
        }
    }

    fn is_coarse(self) -> bool {
        matches!(self, Scenario::CoarseOne)
    }

    /// Mutations — ALWAYS called from outside the reactive graph.
    fn apply(self, rows: &[Signal<RowData>], rev: &Signal<usize>) {
        match self {
            Scenario::FineOne => {
                rows[TARGET_ROW].update(|r| r.shade = 1.0 - r.shade);
            }
            Scenario::CoarseOne => {
                rows[TARGET_ROW].update(|r| r.shade = 1.0 - r.shade);
                rev.update(|v| *v = v.wrapping_add(1));
            }
            Scenario::FineAll => {
                for r in rows {
                    r.update(|rd| rd.shade = 1.0 - rd.shade);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tree construction
// ---------------------------------------------------------------------------
// Built exactly once. The closures only READ signals and never mutate them.

fn build_list(probe: &Probe, coarse: bool) -> (Column, Vec<Signal<RowData>>, Signal<usize>) {
    let rows: Vec<Signal<RowData>> = (0..ROWS)
        // 0.25 keeps `1.0 - shade` changing every frame (0.5 would be a fixed point).
        .map(|i| Signal::new(RowData { id: i, shade: 0.25 }))
        .collect();
    let rev = Signal::new(0usize);

    let mut col = Column::new();
    for row in rows.iter() {
        let row = row.clone();
        let rev = rev.clone();
        let p = probe.clone();

        col = col.child(Text::with(move || {
            if coarse {
                let _ = rev.get(); // subscribe to the coarse trigger
            }
            let r = row.get();
            let s = format!("Row {} shade {:.2}", r.id, r.shade);
            p.tick(); // this closure's rebuild is done
            s
        }));
    }
    (col, rows, rev)
}

// ---------------------------------------------------------------------------
// Report
// ---------------------------------------------------------------------------

fn median_u128(v: &[u128]) -> u128 {
    if v.is_empty() {
        return 0;
    }
    let mut v = v.to_vec();
    v.sort_unstable();
    v[v.len() / 2]
}

fn median_usize(v: &[usize]) -> usize {
    if v.is_empty() {
        return 0;
    }
    let mut v = v.to_vec();
    v.sort_unstable();
    v[v.len() / 2]
}

fn report(name: &str, res: &ProbeResults) {
    let skip = WARMUP.min(res.times.len().saturating_sub(1));
    let times = &res.times[skip..];
    let rebuilds = &res.rebuilds[skip..];
    let frame_skip = WARMUP.min(res.frame_times.len().saturating_sub(1));
    let frames = &res.frame_times[frame_skip..];
    if times.is_empty() && frames.is_empty() {
        println!("{name}: brak próbek");
        return;
    }
    let p50 = median_u128(times);
    let f50 = median_u128(frames);
    let r50 = median_usize(rebuilds);
    println!(
        "{name}: rebuild p50 = {:>6.2} ms | frame p50 = {:>6.2} ms | rebuilds/frame p50 = {:>5} | samples = {}",
        p50 as f64 / 1000.0,
        f50 as f64 / 1000.0,
        r50,
        frames.len().max(times.len()),
    );
}

// ---------------------------------------------------------------------------
// CPU benchmark
// ---------------------------------------------------------------------------

fn bench_cpu(scenario: Scenario) {
    let env = Environment::new();
    env.provide(Theme::dark());
    env.provide(AssetRegistry::with_builtin_icons());

    let mut text = TextSystem::new();
    text.set_scale_factor(1.0);
    let bounds = Rect::from_xywh(0.0, 0.0, 800.0, 600.0);

    let probe = Probe::default();
    let mut measure_calls = Vec::new();
    let mut measure_hits = Vec::new();
    let mut measure_ns = Vec::new();
    let mut arrange_ns = Vec::new();
    // The tree is mounted once; every frame reuses the same signals and views.
    let (root, rows, rev) = build_list(&probe, scenario.is_coarse());

    let rt = Runtime::new();
    rt.set_environment(env);
    rt.mount(root);

    for _frame in 0..TOTAL_FRAMES {
        probe.begin_frame();
        scenario.apply(&rows, &rev); // mutation BEFORE layout, outside the graph

        // `layout` settles pending reactive updates before measuring.
        rt.reset_measure_cache_stats();
        rt.layout(bounds, &mut text);
        let mut list = RenderList::new();
        rt.paint(&mut list, &mut text);
        let (calls, hits) = rt.measure_cache_stats();
        let (m_ns, a_ns) = rt.layout_breakdown();
        measure_calls.push(calls as usize);
        measure_hits.push(hits as usize);
        measure_ns.push(m_ns as usize);
        arrange_ns.push(a_ns as usize);

        probe.end_frame();
    }

    report(scenario.label(), &probe.results());
    let measure_skip = WARMUP.min(measure_calls.len().saturating_sub(1));
    println!(
        "  measure/frame p50: calls = {:>5} | hits = {:>5} | misses = {:>5}",
        median_usize(&measure_calls[measure_skip..]),
        median_usize(&measure_hits[measure_skip..]),
        median_usize(&measure_calls[measure_skip..])
            - median_usize(&measure_hits[measure_skip..]),
    );
    println!(
        "  layout/frame p50: measure = {:>6.2} ms | arrange = {:>6.2} ms",
        median_usize(&measure_ns[measure_skip..]) as f64 / 1_000_000.0,
        median_usize(&arrange_ns[measure_skip..]) as f64 / 1_000_000.0,
    );
}

// ---------------------------------------------------------------------------
// GPU benchmark
// ---------------------------------------------------------------------------
// `Signal<T>` is built on `Rc` and is `!Send`, so all mutations must happen on
// the UI thread. `App::on_update` runs there at the start of every frame,
// before tick/layout, which is exactly what this benchmark needs.

fn bench_gpu(scenario: Scenario, no_vsync: bool) {
    let probe = Probe::default();
    let (root, rows, rev) = build_list(&probe, scenario.is_coarse());

    let start_probe = probe.clone();
    let end_probe = probe.clone();

    let mut app = App::new()
        .title(scenario.label().trim())
        .size(800.0, 600.0)
        .root(root)
        .redraw_mode(RedrawMode::Continuous)
        .fps_limit(1000) // no practical cap, measure raw throughput
        .on_frame_start(move || start_probe.begin_frame())
        .on_update(move || scenario.apply(&rows, &rev))
        .on_frame_end(move || end_probe.end_frame())
        .benchmark(TOTAL_FRAMES);
    if no_vsync {
        // Vsync hides the raw frame cost behind the display refresh rate.
        app = app.present_mode(PresentMode::Immediate);
    }

    let t0 = Instant::now();
    app.run().expect("bench failed");
    let elapsed = t0.elapsed();

    let avg_ms = elapsed.as_secs_f64() * 1000.0 / TOTAL_FRAMES as f64;
    println!();
    println!(
        "GPU ({}): avg frame = {:.2} ms | frames = {}",
        scenario.label().trim(),
        avg_ms,
        TOTAL_FRAMES
    );
    report(scenario.label(), &probe.results());
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let no_vsync = args.iter().any(|arg| arg == "--no-vsync");
    let positional: Vec<&str> = args
        .iter()
        .map(String::as_str)
        .filter(|arg| *arg != "--no-vsync")
        .collect();
    let mode = positional.first().copied().unwrap_or("cpu");

    println!("sequoia-ui benchmark: {ROWS} wierszy\n");

    match mode {
        "cpu" => {
            println!("=== CPU Reactive Update (czysty silnik, bez GPU/okna) ===");
            println!(
                "(Mutacja POZA grafem; p50 = do ostatniej przebudowy, licznik = wywołania closure)\n"
            );
            let only = positional.get(1).copied();
            if only.is_none() || only == Some("s1") {
                bench_cpu(Scenario::FineOne);
            }
            if only.is_none() || only == Some("s2") {
                bench_cpu(Scenario::CoarseOne);
            }
            if only.is_none() || only == Some("s3") {
                bench_cpu(Scenario::FineAll);
            }
            println!("\nGPU bench (osobny proces dla każdego scenariusza):");
            println!("  cargo run --release --example row_update -- gpu s1 [--no-vsync]");
            println!("  cargo run --release --example row_update -- gpu s2");
            println!("  cargo run --release --example row_update -- gpu s3");
        }
        "gpu" => {
            let test_case = positional.get(1).copied().unwrap_or("s1");
            let scenario = match test_case {
                "s1" => Scenario::FineOne,
                "s2" => Scenario::CoarseOne,
                "s3" => Scenario::FineAll,
                _ => {
                    eprintln!("Użyj: s1, s2 lub s3");
                    return;
                }
            };
            bench_gpu(scenario, no_vsync);
        }
        _ => {
            println!(
                "Użycie: cargo run --release --example row_update [cpu [s1|s2|s3] | gpu s1|s2|s3] [--no-vsync]"
            );
        }
    }
}
