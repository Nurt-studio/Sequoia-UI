use std::hint::black_box;
use std::time::Instant;

use sequoia_core::prelude::*;
use sequoia_text::TextSystem;
use sequoia_view::Runtime;
use sequoia_view::prelude::*;
use taffy::prelude::{
    AvailableSpace, Dimension, Display, FlexDirection, GridTemplateComponent, NodeId, Style,
    TaffyTree, length,
};

const WARM_ITERS: usize = 20;

struct Timing {
    cold_ms: f64,
    warm_ms: f64,
}

fn measure<F: FnMut(usize)>(mut f: F) -> Timing {
    let t = Instant::now();
    f(0);
    let cold_ms = t.elapsed().as_secs_f64() * 1000.0;
    let t = Instant::now();
    for i in 0..WARM_ITERS {
        f(i + 1);
    }
    let warm_ms = t.elapsed().as_secs_f64() * 1000.0 / WARM_ITERS as f64;
    Timing { cold_ms, warm_ms }
}

fn taffy_size(w: f32, h: f32) -> taffy::Size<Dimension> {
    taffy::Size {
        width: length(w),
        height: length(h),
    }
}

fn taffy_avail(w: f32, h: f32) -> taffy::Size<AvailableSpace> {
    taffy::Size {
        width: AvailableSpace::Definite(w),
        height: AvailableSpace::Definite(h),
    }
}

fn seq_case(root: impl View + 'static, w: f32, h: f32, resize: bool) -> Timing {
    let rt = Runtime::new();
    rt.mount(root);
    let el = rt.root().expect("root");
    let mut text = TextSystem::new();
    measure(|i| {
        let d = if resize { i as f32 } else { 0.0 };
        rt.layout(Rect::from_xywh(0.0, 0.0, w + d, h + d), &mut text);
        black_box(rt.rect_of(el));
    })
}

fn taffy_case(mut tree: TaffyTree<()>, root: NodeId, w: f32, h: f32, resize: bool) -> Timing {
    let mut style = tree.style(root).expect("style").clone();
    measure(|i| {
        let d = if resize { i as f32 } else { 0.0 };
        style.size = taffy_size(w + d, h + d);
        tree.set_style(root, style.clone()).expect("set_style");
        tree.compute_layout(root, taffy_avail(w + d, h + d))
            .expect("layout");
        black_box(tree.layout(root).expect("result").size);
    })
}

fn report(name: &str, nodes: usize, resize: bool, seq: Timing, taf: Timing) {
    println!(
        "{name:<15} n={nodes:<7} {} | seq {:>7.2} / {:>6.2} ms | taffy {:>7.2} / {:>6.2} ms | seq/taffy {:>5.2}x cold, {:>5.2}x warm",
        if resize { "resize+1px" } else { "static    " },
        seq.cold_ms,
        seq.warm_ms,
        taf.cold_ms,
        taf.warm_ms,
        seq.cold_ms / taf.cold_ms,
        seq.warm_ms / taf.warm_ms,
    );
}

fn seq_flat(n: usize, resize: bool) -> Timing {
    let w = 10.0 * n as f32;
    let children: Vec<Panel> = (0..n).map(|_| Panel::new().size(10.0, 10.0)).collect();
    let root = Row::new().width(w).height(100.0).children(children);
    seq_case(root, w, 100.0, resize)
}

fn taffy_flat(n: usize, resize: bool) -> Timing {
    let w = 10.0 * n as f32;
    let mut tree: TaffyTree<()> = TaffyTree::new();
    let kids: Vec<NodeId> = (0..n)
        .map(|_| {
            tree.new_leaf(Style {
                size: taffy_size(10.0, 10.0),
                ..Default::default()
            })
            .expect("leaf")
        })
        .collect();
    let root = tree
        .new_with_children(
            Style {
                display: Display::Flex,
                size: taffy_size(w, 100.0),
                ..Default::default()
            },
            &kids,
        )
        .expect("root");
    taffy_case(tree, root, w, 100.0, resize)
}

fn seq_grow(n: usize, resize: bool) -> Timing {
    let w = 20.0 * n as f32;
    let children: Vec<Panel> = (0..n).map(|_| Panel::new().flex(1.0)).collect();
    let root = Row::new().width(w).height(100.0).children(children);
    seq_case(root, w, 100.0, resize)
}

fn taffy_grow(n: usize, resize: bool) -> Timing {
    let w = 20.0 * n as f32;
    let mut tree: TaffyTree<()> = TaffyTree::new();
    let kids: Vec<NodeId> = (0..n)
        .map(|_| {
            tree.new_leaf(Style {
                flex_grow: 1.0,
                ..Default::default()
            })
            .expect("leaf")
        })
        .collect();
    let root = tree
        .new_with_children(
            Style {
                display: Display::Flex,
                size: taffy_size(w, 100.0),
                ..Default::default()
            },
            &kids,
        )
        .expect("root");
    taffy_case(tree, root, w, 100.0, resize)
}

fn seq_nested(rows: usize, cols: usize, resize: bool) -> Timing {
    let w = 10.0 * cols as f32;
    let h = 10.0 * rows as f32;
    let rows_v: Vec<Row> = (0..rows)
        .map(|_| {
            Row::new().children(
                (0..cols)
                    .map(|_| Panel::new().size(10.0, 10.0))
                    .collect::<Vec<_>>(),
            )
        })
        .collect();
    let root = Column::new().size(w, h).children(rows_v);
    seq_case(root, w, h, resize)
}

fn taffy_nested(rows: usize, cols: usize, resize: bool) -> Timing {
    let w = 10.0 * cols as f32;
    let h = 10.0 * rows as f32;
    let mut tree: TaffyTree<()> = TaffyTree::new();
    let leaves: Vec<NodeId> = (0..cols)
        .map(|_| {
            tree.new_leaf(Style {
                size: taffy_size(10.0, 10.0),
                ..Default::default()
            })
            .expect("leaf")
        })
        .collect();
    let row_nodes: Vec<NodeId> = (0..rows)
        .map(|_| {
            tree.new_with_children(
                Style {
                    display: Display::Flex,
                    ..Default::default()
                },
                &leaves,
            )
            .expect("row")
        })
        .collect();
    let root = tree
        .new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                size: taffy_size(w, h),
                ..Default::default()
            },
            &row_nodes,
        )
        .expect("root");
    taffy_case(tree, root, w, h, resize)
}

fn seq_grid(cols: usize, rows: usize, resize: bool) -> Timing {
    let w = 10.0 * cols as f32;
    let h = 10.0 * rows as f32;
    let children: Vec<Panel> = (0..cols * rows)
        .map(|_| Panel::new().size(10.0, 10.0))
        .collect();
    let root = Grid::new((0..cols).map(|_| Track::Px(10.0)))
        .size(w, h)
        .children(children);
    seq_case(root, w, h, resize)
}

fn taffy_grid(cols: usize, rows: usize, resize: bool) -> Timing {
    let w = 10.0 * cols as f32;
    let h = 10.0 * rows as f32;
    let mut tree: TaffyTree<()> = TaffyTree::new();
    let kids: Vec<NodeId> = (0..cols * rows)
        .map(|_| {
            tree.new_leaf(Style {
                size: taffy_size(10.0, 10.0),
                ..Default::default()
            })
            .expect("leaf")
        })
        .collect();
    let columns: Vec<GridTemplateComponent<String>> = vec![length(10.0); cols];
    let root = tree
        .new_with_children(
            Style {
                display: Display::Grid,
                grid_template_columns: columns,
                size: taffy_size(w, h),
                ..Default::default()
            },
            &kids,
        )
        .expect("root");
    taffy_case(tree, root, w, h, resize)
}

fn main() {
    println!(
        "sequoia-view vs taffy 0.15.0 (release) — pelny layout drzewa; cold / warm (srednia z {WARM_ITERS} klatek)"
    );
    println!("static = te same bounds co klatke (cache-friendly), resize = root +1 px na klatke\n");

    report(
        "flat flex 1k",
        1_001,
        false,
        seq_flat(1_000, false),
        taffy_flat(1_000, false),
    );
    report(
        "flat flex 10k",
        10_001,
        false,
        seq_flat(10_000, false),
        taffy_flat(10_000, false),
    );
    report(
        "flex grow 10k",
        10_001,
        false,
        seq_grow(10_000, false),
        taffy_grow(10_000, false),
    );
    report(
        "nested 100x100",
        10_101,
        false,
        seq_nested(100, 100, false),
        taffy_nested(100, 100, false),
    );
    report(
        "grid 100x100",
        10_001,
        false,
        seq_grid(100, 100, false),
        taffy_grid(100, 100, false),
    );

    println!();

    report(
        "flat flex 10k",
        10_001,
        true,
        seq_flat(10_000, true),
        taffy_flat(10_000, true),
    );
    report(
        "flex grow 10k",
        10_001,
        true,
        seq_grow(10_000, true),
        taffy_grow(10_000, true),
    );
    report(
        "nested 100x100",
        10_101,
        true,
        seq_nested(100, 100, true),
        taffy_nested(100, 100, true),
    );
    report(
        "grid 100x100",
        10_001,
        true,
        seq_grid(100, 100, true),
        taffy_grid(100, 100, true),
    );
}
