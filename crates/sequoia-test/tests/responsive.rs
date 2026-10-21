#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_core::Size;
use sequoia_test::Harness;
use sequoia_view::prelude::*;
use sequoia_view::{AnyView, Breakpoint, Responsive, WithSize};

fn branch_name(h: &Harness) -> String {
    let root = h.root().unwrap();
    let branch = h.children(root)[0];
    h.type_name_of(branch).unwrap_or_default()
}

#[test]
fn window_size_signal_tracks_layout_bounds() {
    let h = Harness::new(Unit);
    h.layout(500.0, 800.0);
    assert_eq!(h.runtime().window_size_value(), Size::new(500.0, 800.0));
    assert_eq!(h.runtime().window_rect().size, Size::new(500.0, 800.0));
    h.layout(1280.0, 720.0);
    assert_eq!(h.runtime().window_size_value(), Size::new(1280.0, 720.0));
}

#[test]
fn responsive_switches_column_row_on_resize() {
    let h = Harness::new(Unit);
    h.runtime().mount(Responsive::window(&h.runtime(), |s| {
        if Breakpoint::of(s.width).is_compact() {
            AnyView::new(Column::new())
        } else {
            AnyView::new(Row::new())
        }
    }));

    h.layout(500.0, 800.0);
    h.tick();
    h.layout(500.0, 800.0);
    assert!(
        branch_name(&h).contains("Column"),
        "narrow should be Column, it is: {}",
        branch_name(&h)
    );

    h.layout(1000.0, 800.0);
    h.tick();
    h.layout(1000.0, 800.0);
    assert!(
        branch_name(&h).contains("Row"),
        "the width should be Row, it is: {}",
        branch_name(&h)
    );

    h.layout(400.0, 800.0);
    h.tick();
    h.layout(400.0, 800.0);
    assert!(branch_name(&h).contains("Column"));
}

#[test]
fn responsive_settles_within_a_single_layout_call() {
    let h = Harness::new(Unit);
    h.runtime().mount(Responsive::window(&h.runtime(), |s| {
        if Breakpoint::of(s.width).is_compact() {
            AnyView::new(Column::new())
        } else {
            AnyView::new(Row::new())
        }
    }));

    h.layout(500.0, 800.0);
    assert!(
        branch_name(&h).contains("Column"),
        "narrow should already be Column after one layout, it is: {}",
        branch_name(&h)
    );

    h.layout(1000.0, 800.0);
    assert!(
        branch_name(&h).contains("Row"),
        "wide should already be Row after one layout, it is: {}",
        branch_name(&h)
    );
}

#[test]
fn with_size_reports_arranged_width() {
    let adaptive = WithSize::new(|s| {
        if s.width < 600.0 {
            AnyView::new(Column::new())
        } else {
            AnyView::new(Row::new())
        }
    })
    .fill_size();
    let size_signal = adaptive.size_signal();

    let h = Harness::new(Column::new().child(adaptive));

    h.layout(1000.0, 800.0);
    h.tick();
    h.layout(1000.0, 800.0);
    h.tick();
    h.layout(1000.0, 800.0);
    let reported = size_signal.get_untracked();
    assert!(
        (reported.width - 1000.0).abs() < 1.0,
        "WithSize is supposed to report the width of the parent, it is: {reported:?}"
    );
    let root = h.root().unwrap();
    let with = h.children(root)[0];
    assert!(
        h.type_name_of(with)
            .unwrap_or_default()
            .contains("WithSize")
    );
    let branch = h.children(with)[0];
    assert!(
        h.type_name_of(branch).unwrap_or_default().contains("Row"),
        "wide parent has to give Row"
    );
}

#[test]
fn with_size_settles_within_a_single_layout_call() {
    let adaptive = WithSize::new(|s| {
        if s.width < 600.0 {
            AnyView::new(Column::new())
        } else {
            AnyView::new(Row::new())
        }
    })
    .fill_size();
    let size_signal = adaptive.size_signal();

    let h = Harness::new(Column::new().child(adaptive));

    h.layout(1000.0, 800.0);
    let reported = size_signal.get_untracked();
    assert!(
        (reported.width - 1000.0).abs() < 1.0,
        "WithSize should report the width within one layout, it is: {reported:?}"
    );
    let root = h.root().unwrap();
    let with = h.children(root)[0];
    let branch = h.children(with)[0];
    assert!(
        h.type_name_of(branch).unwrap_or_default().contains("Row"),
        "wide parent has to give Row after one layout"
    );
}
