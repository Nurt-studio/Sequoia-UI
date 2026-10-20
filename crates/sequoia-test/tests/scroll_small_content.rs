#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_core::{Length, Size};
use sequoia_elements::prelude::*;
use sequoia_reactive::Signal;
use sequoia_test::Harness;

struct FillBar;

impl View for FillBar {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn is_leaf(&self) -> bool {
        true
    }

    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fill(1.0),
            height: Length::Fill(1.0),
            ..LayoutStyle::default()
        }
    }

    fn intrinsic_size(&self, _ctx: &mut LayoutCtx) -> Option<Size> {
        Some(Size::new(100.0, 28.0))
    }
}

struct Tree {
    n: Signal<usize>,
}

impl View for Tree {
    fn body(&self) -> impl View + 'static {
        let n = self.n.get();
        let mut col = Column::new().fill_width();
        for _ in 0..n {
            col = col.child(
                Container::new()
                    .child(FillBar)
                    .width(Length::Fill(1.0))
                    .on_click(|| {}),
            );
        }
        ScrollView::new(col)
    }
}

fn row_rects(h: &Harness) -> Vec<sequoia_core::Rect> {
    let root = h.root().unwrap();
    let scroll_el = h.children(root)[0];
    let col = h.children(scroll_el)[0];
    h.children(col)
        .iter()
        .map(|id| h.rect(*id).unwrap())
        .collect()
}

#[test]
fn small_content_hugs_instead_of_stretching() {
    let n = Signal::new(3usize);
    let h = Harness::new(Tree { n: n.clone() });
    h.layout(248.0, 700.0);

    let rows = row_rects(&h);
    assert_eq!(rows.len(), 3);
    for (i, r) in rows.iter().enumerate() {
        assert_eq!(
            r.size.height, 28.0,
            "row {i} hugs content (28px), not splits viewport"
        );
        assert_eq!(r.top(), i as f32 * 28.0, "row {i} follows the previous one");
    }

    let root = h.root().unwrap();
    let scroll_el = h.children(root)[0];
    let scroll = h.view::<ScrollView>(scroll_el).unwrap();
    assert_eq!(scroll.content_height(&h.runtime(), scroll_el), 84.0);
    assert_eq!(scroll.max_offset(&h.runtime(), scroll_el), 0.0);
}

#[test]
fn removing_children_moves_rest_up_without_gaps() {
    let n = Signal::new(3usize);
    let h = Harness::new(Tree { n: n.clone() });
    h.layout(248.0, 700.0);

    h.set(&n, 1);
    h.layout(248.0, 700.0);

    let rows = row_rects(&h);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].size.height, 28.0);
    assert_eq!(rows[0].top(), 0.0, "single row pinned to the top");
}
