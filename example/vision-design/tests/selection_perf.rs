//! Headless benchmark for marquee (rubber-band) selection on the design canvas.
//!
//! The scene is built procedurally with 10, 50, 200 and 4000 rectangles, the
//! marquee drags over all of them, and the run reports:
//!
//! * selection time (pointer down + drag + commit) - CPU work for the gesture,
//! * frame time for layout + paint with the selection drawn - FPS budget,
//! * estimated memory held by layers, the selection and the render list.
//!
//! The same report is written to `selection_perf_report.txt` in the example
//! directory on every run.
//!
//! Run with:
//!
//! ```text
//! cargo test -p vision-design --test selection_perf -- --nocapture
//! ```

#![allow(dead_code)]
#![allow(unreachable_pub)]

#[path = "../src/app/mod.rs"]
mod app;
#[path = "../src/canvas/mod.rs"]
mod canvas;
#[path = "../src/codegen/mod.rs"]
mod codegen;
#[path = "../src/components/mod.rs"]
mod components;
#[path = "../src/core/mod.rs"]
mod core;
#[path = "../src/document/mod.rs"]
mod document;
#[path = "../src/model/mod.rs"]
mod model;
#[path = "../src/node_graph/mod.rs"]
mod node_graph;
#[path = "../src/overlays/mod.rs"]
mod overlays;
#[path = "../src/panels/mod.rs"]
mod panels;
#[path = "../src/state/mod.rs"]
mod state;
#[path = "../src/theme/mod.rs"]
mod theme;

use std::hint::black_box;
use std::mem::size_of;
use std::time::Instant;

use sequoia_ui::core::{RenderItem, RenderList};
use sequoia_ui::prelude::*;
use sequoia_ui::text::TextSystem;
use sequoia_ui::view::Runtime;

use crate::core::layer::Layer;
use crate::model::svelte_ast::{Dimension, Display, Element, Position, Style, SvelteNode, Tag};
use crate::state::EditorState;

use crate::canvas::canvas;

const W: f32 = 1440.0;
const H: f32 = 900.0;
const CELL: f32 = 12.0;
const GAP: f32 = 2.0;
const MARGIN: f32 = 10.0;
const FRAMES: usize = 120;
const WARMUP: usize = 10;
const DRAG_STEPS: usize = 8;

/// Builds a document with `n` small rectangles laid out in a grid.
fn scene_tree(n: usize) -> SvelteNode {
    let cols = ((n as f32 * (W / H)).sqrt().ceil() as usize).max(1);
    let rows = n.div_ceil(cols);
    let step = CELL + GAP;
    let mut children = Vec::with_capacity(n);

    for i in 0..n {
        let col = i % cols;
        let row = i / cols;
        children.push(SvelteNode::Element(Element {
            id: i as u64 + 2,
            name: format!("Rect {i}"),
            tag: Tag::Div,
            style: Style {
                display: Display::None,
                position: Position::Absolute,
                top: Some(MARGIN + row as f32 * step),
                left: Some(MARGIN + col as f32 * step),
                width: Dimension::Fixed(CELL),
                height: Dimension::Fixed(CELL),
                fill: Some(Color::hex(0x0D99FF)),
                fill_hex: "#0D99FF".into(),
                ..Default::default()
            },
            children: Vec::new(),
            visible: true,
            locked: false,
            component: false,
            instance: false,
            interface: None,
            overrides: Vec::new(),
            slot_content: Vec::new(),
            behavior: None,
            binding: None,
            hover: None,
        }));
    }

    SvelteNode::Element(Element {
        id: 1,
        name: "Frame".into(),
        tag: Tag::Div,
        style: Style {
            display: Display::Flex,
            position: Position::Relative,
            width: Dimension::Fixed(MARGIN * 2.0 + cols as f32 * step),
            height: Dimension::Fixed(MARGIN * 2.0 + rows as f32 * step),
            top: Some(0.0),
            left: Some(0.0),
            fill: Some(Color::hex(0x1B1B1B)),
            fill_hex: "#1B1B1B".into(),
            ..Default::default()
        },
        children,
        visible: true,
        locked: false,
        component: false,
        instance: false,
        interface: None,
        overrides: Vec::new(),
        slot_content: Vec::new(),
        behavior: None,
        binding: None,
        hover: None,
    })
}

struct SceneStats {
    selected: usize,
    selection_ms: f64,
    avg_ms: f64,
    p95_ms: f64,
    layout_ms: f64,
    paint_ms: f64,
    items: usize,
    quads: usize,
    layers_bytes: usize,
    selection_bytes: usize,
    render_bytes: usize,
}

fn layer_bytes(l: &Layer) -> usize {
    size_of::<Layer>()
        + l.name.capacity()
        + l.fill_hex.capacity()
        + l.stroke_hex.capacity()
        + l.text.capacity()
}

fn bench(n: usize) -> SceneStats {
    let state = EditorState::with_seed(Some(scene_tree(n)), None);
    let rt = Runtime::new();
    rt.set_root_fill(true);
    rt.mount(canvas(&state));
    rt.tick();

    let mut text = TextSystem::new();
    let mut list = RenderList::new();
    let bounds = Rect::from_xywh(0.0, 0.0, W, H);
    rt.layout(bounds, &mut text);
    list.clear();
    rt.paint(&mut list, &mut text);

    let layer_count = state.layers.with_untracked(|layers| layers.len());
    assert_eq!(
        layer_count,
        n + 1,
        "scene should hold the frame plus {n} rects"
    );

    // Start outside the frame (which would otherwise start a move gesture) and
    // drag across the whole grid.
    let start = Point::new(W - 10.0, H - 10.0);
    let end = Point::new(5.0, 5.0);

    let t0 = Instant::now();
    rt.pointer_down(start, PointerButton::Left, Modifiers::NONE, &mut text);
    for step in 1..=DRAG_STEPS {
        let t = step as f32 / DRAG_STEPS as f32;
        let pos = Point::new(
            start.x + (end.x - start.x) * t,
            start.y + (end.y - start.y) * t,
        );
        rt.pointer_move_with_mods(pos, Modifiers::NONE, &mut text);
    }
    rt.pointer_up(end, PointerButton::Left, Modifiers::NONE, &mut text);
    let selection_ms = t0.elapsed().as_secs_f64() * 1000.0;

    let selected = state.selected_layers.with_untracked(|v| v.len());
    assert_eq!(
        selected, n,
        "marquee should select exactly {n} rectangles, got {selected}"
    );

    for _ in 0..WARMUP {
        rt.layout(bounds, &mut text);
        list.clear();
        rt.paint(&mut list, &mut text);
    }

    let mut dts = Vec::with_capacity(FRAMES);
    let mut layout_acc = 0.0f64;
    let mut paint_acc = 0.0f64;
    for _ in 0..FRAMES {
        let frame = Instant::now();

        let t_layout = Instant::now();
        rt.layout(bounds, &mut text);
        let layout_ms = t_layout.elapsed().as_secs_f64() * 1000.0;

        let t_paint = Instant::now();
        list.clear();
        rt.paint(&mut list, &mut text);
        let paint_ms = t_paint.elapsed().as_secs_f64() * 1000.0;

        black_box(list.len());
        dts.push(frame.elapsed().as_secs_f64() * 1000.0);
        layout_acc += layout_ms;
        paint_acc += paint_ms;
    }

    dts.sort_by(|a, b| a.total_cmp(b));
    let avg_ms = dts.iter().sum::<f64>() / dts.len() as f64;
    let p95_ms = dts[(dts.len() as f64 * 0.95) as usize];
    let frames = FRAMES as f64;

    let layers_bytes = state.layers.with_untracked(|layers| {
        layers.capacity() * size_of::<Layer>() + layers.iter().map(layer_bytes).sum::<usize>()
    });
    let selection_bytes = state
        .selected_layers
        .with_untracked(|ids| ids.capacity().max(ids.len()) * size_of::<u64>());
    let items = list.len();
    let render_bytes = items * size_of::<RenderItem>();
    let quads = list
        .items()
        .iter()
        .filter(|item| matches!(item, RenderItem::Quad(_)))
        .count();

    SceneStats {
        selected,
        selection_ms,
        avg_ms,
        p95_ms,
        layout_ms: layout_acc / frames,
        paint_ms: paint_acc / frames,
        items,
        quads,
        layers_bytes,
        selection_bytes,
        render_bytes,
    }
}

fn cpus() -> f64 {
    std::thread::available_parallelism()
        .map(|v| v.get() as f64)
        .unwrap_or(1.0)
}

/// Groups thousands with a thin space, so `20305` reads as `20 305` and not as
/// `20`.
fn grouped(value: f64) -> String {
    let digits = format!("{:.0}", value);
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(' ');
        }
        out.push(ch);
    }
    out
}

fn report(n: usize, s: &SceneStats) -> String {
    let cpus = cpus();
    let fps = grouped(1000.0 / s.avg_ms.max(1e-9));
    let one_core_pct = s.avg_ms / 16.6667 * 100.0;
    let machine_pct = one_core_pct / cpus;
    let selected = s.selected;
    let selection_ms = s.selection_ms;
    let avg_ms = s.avg_ms;
    let p95_ms = s.p95_ms;
    let layout_ms = s.layout_ms;
    let paint_ms = s.paint_ms;
    let layers_mb = s.layers_bytes as f64 / (1024.0 * 1024.0);
    let selection_kb = s.selection_bytes as f64 / 1024.0;
    let render_mb = s.render_bytes as f64 / (1024.0 * 1024.0);
    let total_mb = layers_mb + selection_kb / 1024.0 + render_mb;
    let items = grouped(s.items as f64);
    let quads = grouped(s.quads as f64);

    format!(
        "[vision-design] N={n:<4} | zaznaczono={selected:<4} | selekcja={selection_ms:7.2}ms | \
         klatka avg={avg_ms:7.3}ms p95={p95_ms:7.3}ms (layout {layout_ms:6.3} + paint {paint_ms:6.3}) | \
         ~{fps} FPS | CPU {one_core_pct:5.1}% rdzenia @60FPS ({machine_pct:4.2}% maszyny {cpus:.0} CPU) | \
         RAM ~{total_mb:5.2} MB (warstwy {layers_mb:5.2} MB, zaznaczenie {selection_kb:5.1} KB, render-list {render_mb:5.2} MB) | \
         itemy={items} quady={quads}"
    )
}

#[test]
fn marquee_selection_cpu_ram_fps() {
    let header = format!(
        "[vision-design] marquee selection benchmark: {} klatek na scenariusz, okno {}x{}",
        FRAMES, W as u32, H as u32
    );
    println!("{header}");

    let mut saved = String::new();
    saved.push_str("Vision Design - benchmark zaznaczania prostokatnego (marquee)\n");
    saved.push_str(&format!(
        "System: {} | rdzenie: {:.0} | okno: {}x{} | klatek na scenariusz: {}\n",
        std::env::consts::OS,
        cpus(),
        W as u32,
        H as u32,
        FRAMES
    ));
    saved.push_str(
        "FPS mierzy budowe klatki (layout + paint) na CPU, bez GPU i vsync; RAM to szacunek struktur, nie RSS.\n",
    );
    saved.push_str(
        "Uruchamiaj pojedynczo (cargo test ... -- --nocapture): pelny `cargo test` moze zanizyc FPS przez rownolegle testy.\n\n",
    );
    saved.push_str(&header);
    saved.push('\n');

    for n in [10usize, 50, 200, 4000] {
        let stats = bench(n);
        let line = report(n, &stats);
        println!("{line}");
        saved.push_str(&line);
        saved.push('\n');

        assert!(
            stats.selection_ms < 2000.0,
            "N={n}: marquee selection took {:.1}ms",
            stats.selection_ms
        );
        assert!(
            stats.avg_ms.is_finite() && stats.avg_ms > 0.0,
            "N={n}: invalid frame time"
        );
        assert!(
            stats.avg_ms < 250.0,
            "N={n}: frame budget exploded ({:.1}ms)",
            stats.avg_ms
        );
    }

    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("selection_perf_report.txt");
    std::fs::write(&path, saved).expect("zapis raportu z benchmarku");
    println!("[vision-design] wynik zapisany: {}", path.display());
}
