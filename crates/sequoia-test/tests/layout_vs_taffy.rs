#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Layout benchmark: Sequoia vs taffy (latest release).
//!
//! ```text
//! cargo test --profile release-bench -p sequoia-test --test layout_vs_taffy -- --nocapture
//! ```
//!
//! Four scene shapes (`flex`, `grid`, `wrap`, `text`) at three sizes, in two
//! modes:
//!
//! - `warm`: one mounted/built tree, repeated layout with alternating width so
//!   neither engine reuses the previous frame's cached result.
//! - `cold`: a fresh tree is built and laid out on every iteration.
//!
//! taffy implements CSS Flexbox/Grid and has no text shaping, so `text` uses a
//! measure callback returning an approximate width; Sequoia shapes the text
//! with `TextSystem`. Every other scene contains no text.

use std::hint::black_box;
use std::time::Instant;

use sequoia_core::Rect;
use sequoia_test::Harness;
use sequoia_text::TextSystem;
use sequoia_view::prelude::*;
use taffy::prelude::{
    AvailableSpace, Display, FlexDirection, FlexWrap, NodeId, Size, Style, TaffyTree, auto,
    evenly_sized_tracks, length, percent, zero,
};
use taffy::tree::{LayoutInput, LayoutOutput};

const W: f32 = 800.0;
const H: f32 = 600.0;
const WARMUP: usize = 20;
const WARM_FRAMES: usize = 200;
const COLD_FRAMES: usize = 60;
const ROW_H: f32 = 24.0;
const GAP: f32 = 4.0;
const ROW_GAP: f32 = 8.0;
const COLS: usize = 10;
const SIZES: [usize; 3] = [100, 1_000, 3_000];
const KINDS: [Kind; 4] = [Kind::Flex, Kind::Grid, Kind::Wrap, Kind::Text];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Flex,
    Grid,
    Wrap,
    Text,
}

impl Kind {
    fn name(self) -> &'static str {
        match self {
            Self::Flex => "flex",
            Self::Grid => "grid",
            Self::Wrap => "wrap",
            Self::Text => "text",
        }
    }

    fn has_measure(self) -> bool {
        self == Self::Text
    }
}

fn available_width(i: usize) -> f32 {
    W + (i % 2) as f32 * 0.5
}

fn sequoia_root(kind: Kind, rows: usize) -> AnyView {
    match kind {
        Kind::Flex => {
            let mut root = Column::new().gap(GAP);
            for _ in 0..rows {
                root = root.child(
                    Row::new()
                        .fill_width()
                        .height(ROW_H)
                        .gap(ROW_GAP)
                        .child(Panel::new().width(80.0).height(ROW_H))
                        .child(Panel::new().flex(1.0).height(ROW_H))
                        .child(Panel::new().width(40.0).height(ROW_H)),
                );
            }
            AnyView::new(root.width(W).height(H))
        }
        Kind::Grid => {
            let mut root = Grid::new(vec![Track::Fill(1.0); COLS]).gap(GAP);
            for i in 0..rows {
                root = root.child(Panel::new().height(ROW_H));
                let _ = i;
            }
            AnyView::new(root.width(W).height(H))
        }
        Kind::Wrap => {
            let mut root = Row::new().wrap(true).gap(ROW_GAP).width(W).height(H);
            for _ in 0..rows {
                root = root.child(Panel::new().width(80.0).height(ROW_H));
            }
            AnyView::new(root)
        }
        Kind::Text => {
            let mut root = Column::new().gap(GAP).width(W).height(H);
            for i in 0..rows {
                root = root.child(Text::new(format!("layout benchmark row {i}")).size(14.0));
            }
            AnyView::new(root)
        }
    }
}

fn taffy_tree(kind: Kind, rows: usize) -> (TaffyTree<()>, NodeId) {
    let mut tree = TaffyTree::new();
    let mut children = Vec::with_capacity(rows);

    match kind {
        Kind::Flex => {
            for _ in 0..rows {
                let left = tree
                    .new_leaf(Style {
                        size: Size {
                            width: length(80.0),
                            height: length(ROW_H),
                        },
                        ..Default::default()
                    })
                    .unwrap();
                let middle = tree
                    .new_leaf(Style {
                        flex_grow: 1.0,
                        size: Size {
                            width: auto(),
                            height: length(ROW_H),
                        },
                        ..Default::default()
                    })
                    .unwrap();
                let right = tree
                    .new_leaf(Style {
                        size: Size {
                            width: length(40.0),
                            height: length(ROW_H),
                        },
                        ..Default::default()
                    })
                    .unwrap();
                children.push(
                    tree.new_with_children(
                        Style {
                            flex_direction: FlexDirection::Row,
                            size: Size {
                                width: percent(1.0),
                                height: length(ROW_H),
                            },
                            gap: Size {
                                width: length(ROW_GAP),
                                height: zero(),
                            },
                            ..Default::default()
                        },
                        &[left, middle, right],
                    )
                    .unwrap(),
                );
            }
        }
        Kind::Grid => {
            for _ in 0..rows {
                children.push(
                    tree.new_leaf(Style {
                        size: Size {
                            width: auto(),
                            height: length(ROW_H),
                        },
                        ..Default::default()
                    })
                    .unwrap(),
                );
            }
        }
        Kind::Wrap | Kind::Text => {
            for _ in 0..rows {
                children.push(
                    tree.new_leaf(Style {
                        size: if kind == Kind::Wrap {
                            Size {
                                width: length(80.0),
                                height: length(ROW_H),
                            }
                        } else {
                            Size {
                                width: auto(),
                                height: auto(),
                            }
                        },
                        ..Default::default()
                    })
                    .unwrap(),
                );
            }
        }
    }

    let style = match kind {
        Kind::Flex => Style {
            flex_direction: FlexDirection::Column,
            size: Size {
                width: length(W),
                height: length(H),
            },
            gap: Size {
                width: zero(),
                height: length(GAP),
            },
            ..Default::default()
        },
        Kind::Grid => Style {
            display: Display::Grid,
            grid_template_columns: evenly_sized_tracks(COLS as u16),
            size: Size {
                width: length(W),
                height: length(H),
            },
            gap: Size {
                width: length(GAP),
                height: length(GAP),
            },
            ..Default::default()
        },
        Kind::Wrap => Style {
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            size: Size {
                width: length(W),
                height: length(H),
            },
            gap: Size {
                width: length(ROW_GAP),
                height: length(ROW_GAP),
            },
            ..Default::default()
        },
        Kind::Text => Style {
            flex_direction: FlexDirection::Column,
            size: Size {
                width: length(W),
                height: length(H),
            },
            gap: Size {
                width: zero(),
                height: length(GAP),
            },
            ..Default::default()
        },
    };
    let root = tree.new_with_children(style, &children).unwrap();
    (tree, root)
}

fn measure_text(
    input: LayoutInput,
    _node: NodeId,
    _ctx: Option<&mut ()>,
    _style: &Style,
) -> LayoutOutput {
    const TEXT_W: f32 = 168.0;
    let width = input.known_dimensions.width.unwrap_or(TEXT_W).min(TEXT_W);
    LayoutOutput::from_sizes(
        Size {
            width,
            height: 18.0,
        },
        taffy::geometry::Rect::ZERO,
    )
}

#[derive(Clone, Copy)]
struct Stats {
    avg: f64,
    p50: f64,
    p95: f64,
}

fn stats(mut samples: Vec<f64>) -> Stats {
    samples.sort_by(f64::total_cmp);
    let n = samples.len();
    Stats {
        avg: samples.iter().sum::<f64>() / n as f64,
        p50: samples[n / 2],
        p95: samples[(n as f64 * 0.95) as usize],
    }
}

fn taffy_compute(tree: &mut TaffyTree<()>, root: NodeId, i: usize, with_measure: bool) {
    let space = Size {
        width: AvailableSpace::Definite(available_width(i)),
        height: AvailableSpace::Definite(H),
    };
    if with_measure {
        tree.compute_layout_with_measure(root, space, measure_text)
            .unwrap();
    } else {
        tree.compute_layout(root, space).unwrap();
    }
}

fn bench_sequoia_warm(view: &AnyView) -> (Stats, Stats, Stats) {
    let harness = Harness::new(view.clone());
    let rt = harness.runtime();
    let mut text = TextSystem::new();
    let root = rt.root().expect("mounted root");
    for i in 0..WARMUP {
        rt.layout(Rect::from_xywh(0.0, 0.0, available_width(i), H), &mut text);
        black_box(rt.rect_of(root));
    }
    rt.reset_measure_cache_stats();
    let mut total = Vec::with_capacity(WARM_FRAMES);
    let mut measure = Vec::with_capacity(WARM_FRAMES);
    let mut arrange = Vec::with_capacity(WARM_FRAMES);
    for i in 0..WARM_FRAMES {
        let start = Instant::now();
        rt.layout(Rect::from_xywh(0.0, 0.0, available_width(i), H), &mut text);
        total.push(start.elapsed().as_secs_f64() * 1000.0);
        let (m, a) = rt.layout_breakdown();
        measure.push(m as f64 / 1_000_000.0);
        arrange.push(a as f64 / 1_000_000.0);
        black_box(rt.rect_of(root));
    }
    (stats(total), stats(measure), stats(arrange))
}

fn bench_taffy_warm(tree: &mut TaffyTree<()>, root: NodeId, with_measure: bool) -> Stats {
    for i in 0..WARMUP {
        taffy_compute(tree, root, i, with_measure);
        black_box(tree.layout(root).unwrap());
    }
    let mut samples = Vec::with_capacity(WARM_FRAMES);
    for i in 0..WARM_FRAMES {
        let start = Instant::now();
        taffy_compute(tree, root, i, with_measure);
        samples.push(start.elapsed().as_secs_f64() * 1000.0);
        black_box(tree.layout(root).unwrap());
    }
    stats(samples)
}

fn bench_sequoia_cold(view: &AnyView) -> Stats {
    let mut text = TextSystem::new();
    let mut samples = Vec::with_capacity(COLD_FRAMES);
    for i in 0..COLD_FRAMES {
        let start = Instant::now();
        let harness = Harness::new(view.clone());
        let rt = harness.runtime();
        rt.layout(Rect::from_xywh(0.0, 0.0, available_width(i), H), &mut text);
        samples.push(start.elapsed().as_secs_f64() * 1000.0);
        black_box(rt.root().and_then(|root| rt.rect_of(root)));
    }
    stats(samples)
}

fn bench_taffy_cold(kind: Kind, rows: usize) -> Stats {
    let mut samples = Vec::with_capacity(COLD_FRAMES);
    for i in 0..COLD_FRAMES {
        let start = Instant::now();
        let (mut tree, root) = taffy_tree(kind, rows);
        taffy_compute(&mut tree, root, i, kind.has_measure());
        samples.push(start.elapsed().as_secs_f64() * 1000.0);
        black_box(tree.layout(root).unwrap());
    }
    stats(samples)
}

fn report(kind: Kind, mode: &str, rows: usize, seq: Stats, taffy: Stats, measure: Option<Stats>) {
    println!(
        "[layout-vs-taffy] RESULT kind={} mode={} size={} seq_avg={:.3} seq_p50={:.3} seq_p95={:.3} taffy_avg={:.3} taffy_p50={:.3} taffy_p95={:.3}",
        kind.name(),
        mode,
        rows,
        seq.avg,
        seq.p50,
        seq.p95,
        taffy.avg,
        taffy.p50,
        taffy.p95,
    );
    if let Some(m) = measure {
        println!(
            "[layout-vs-taffy]   sequoia measure avg={:.3}ms p95={:.3}ms",
            m.avg, m.p95
        );
    }
    let faster = if seq.avg < taffy.avg {
        "sequoia"
    } else {
        "taffy"
    };
    let ratio = seq.avg.max(taffy.avg) / seq.avg.min(taffy.avg);
    println!("[layout-vs-taffy]   {faster} is {ratio:.2}x faster");
}

#[test]
fn layout_vs_taffy() {
    let cpus = std::thread::available_parallelism()
        .map(|v| v.get())
        .unwrap_or(1);
    println!(
        "[layout-vs-taffy] cpus={cpus} warmup={WARMUP} warm_frames={WARM_FRAMES} cold_frames={COLD_FRAMES}"
    );

    for kind in KINDS {
        for rows in SIZES {
            let view = sequoia_root(kind, rows);
            let (seq, measure, arrange) = bench_sequoia_warm(&view);
            let (mut tree, root) = taffy_tree(kind, rows);
            let taffy = bench_taffy_warm(&mut tree, root, kind.has_measure());
            report(kind, "warm", rows, seq, taffy, Some(measure));
            let _ = arrange;

            let seq_cold = bench_sequoia_cold(&view);
            let taffy_cold = bench_taffy_cold(kind, rows);
            report(kind, "cold", rows, seq_cold, taffy_cold, None);
        }
    }
}
