use std::time::Instant;

use sequoia_layout::Axis;
use sequoia_layout::{ContainerStyle, distribute_flex, resolve_tracks};

fn main() {
    let n = 100_000usize;

    let items: Vec<sequoia_layout::FlexItem> = (0..n)
        .map(|_| sequoia_layout::FlexItem::new(10.0, 1.0, 1.0, 0.0, f32::INFINITY))
        .collect();
    let mut sizes = vec![10.0; n];
    let t = Instant::now();
    distribute_flex(&items, &mut sizes, 0.0, 1_000_000.0);
    println!("sequoia distribute_flex {n}: {:?}", t.elapsed());

    let tracks = vec![sequoia_layout::Track::Fill(1.0); n.min(1000)];
    let content = vec![0.0; tracks.len()];
    let t = Instant::now();
    let _ = resolve_tracks(&tracks, &content, 1_000_000.0, 0.0);
    println!("sequoia resolve_tracks {}: {:?}", tracks.len(), t.elapsed());

    use taffy::prelude::*;
    let mut taffy = TaffyTree::<()>::new();
    let children: Vec<NodeId> = (0..n)
        .map(|_| taffy.new_leaf(Style::default()).unwrap())
        .collect();
    let root = taffy
        .new_with_children(
            Style {
                display: Display::Flex,
                size: Size {
                    width: Dimension::from_length(1_000_000.0),
                    height: Dimension::from_length(100.0),
                },
                ..Default::default()
            },
            &children,
        )
        .unwrap();
    let t = Instant::now();
    taffy.compute_layout(root, Size::MAX_CONTENT).unwrap();
    println!("taffy flex {n}: {:?}", t.elapsed());

    let _ = ContainerStyle::default();
    let _ = Axis::Horizontal;
}
