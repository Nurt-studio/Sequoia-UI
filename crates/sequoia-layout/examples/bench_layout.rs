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

    let _ = ContainerStyle::default();
    let _ = Axis::Horizontal;
}
