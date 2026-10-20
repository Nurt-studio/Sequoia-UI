#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_core::Length;
use sequoia_reactive::Signal;
use sequoia_test::Harness;
use sequoia_view::prelude::*;

struct Bar {
    h: f32,
}

impl View for Bar {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn is_leaf(&self) -> bool {
        true
    }

    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fill(1.0),
            height: Length::Px(self.h),
            ..LayoutStyle::default()
        }
    }
}

struct FreshScrollEachBody {
    n: Signal<usize>,
}

impl View for FreshScrollEachBody {
    fn body(&self) -> impl View + 'static {
        let n = self.n.get();
        let mut col = Column::new();
        for _ in 0..n {
            col = col.child(Bar { h: 40.0 });
        }
        ScrollView::new(col)
    }
}

#[test]
fn scroll_new_in_body_survives_signal_rebuild() {
    let n = Signal::new(20usize);
    let h = Harness::new(FreshScrollEachBody { n: n.clone() });
    h.layout(300.0, 200.0);

    h.wheel(150.0, 100.0, -40.0);
    let el = h.children(h.root().unwrap())[0];
    let rt = h.runtime();
    let scroll = h.view::<ScrollView>(el).unwrap();
    assert_eq!(sequoia_view::ScrollView::offset(&scroll, &rt, el), 40.0);

    h.set(&n, 25);
    h.layout(300.0, 200.0);

    let el2 = h.children(h.root().unwrap())[0];
    assert_eq!(el, el2, "ten sam element po reuse (same_type)");
    let scroll2 = h.view::<ScrollView>(el2).unwrap();
    assert_eq!(
        sequoia_view::ScrollView::offset(&scroll2, &rt, el2),
        40.0,
        "offset does not reset despite ::new() in body()"
    );
    assert_eq!(scroll2.content_height(&rt, el2), 1000.0);
}

#[test]
fn unmount_drops_state_slots() {
    let h = Harness::new(ScrollView::new(Bar { h: 800.0 }));
    assert_eq!(h.state_count(), 0, "slot is created lazily");
    h.layout(300.0, 200.0);
    assert_eq!(h.state_count(), 1, "jeden slot scrolla po layout");

    h.runtime().mount(Unit);
    h.tick();
    assert_eq!(h.state_count(), 0, "unmount dropi sloty");
    assert_eq!(h.scoped_count(), 0);
    assert_eq!(h.element_count(), 1, "only new root left");
}

#[test]
fn text_field_new_in_body_keeps_caret() {
    use sequoia_elements::TextField;

    struct FreshField {
        value: Signal<String>,
        tick: Signal<u32>,
    }

    impl View for FreshField {
        fn body(&self) -> impl View + 'static {
            let _ = self.tick.get();
            TextField::new(self.value.clone())
        }
    }

    let value = Signal::new("hello".to_string());
    let tick = Signal::new(0u32);
    let h = Harness::new(FreshField {
        value: value.clone(),
        tick: tick.clone(),
    });
    h.layout(300.0, 40.0);

    let el = h.children(h.root().unwrap())[0];
    h.view::<TextField>(el).unwrap().select_range(1, 4);
    assert_eq!(h.view::<TextField>(el).unwrap().selection(), Some((1, 4)));

    h.set(&tick, 1);
    h.layout(300.0, 40.0);
    assert_eq!(
        h.view::<TextField>(el).unwrap().selection(),
        Some((1, 4)),
        "caret does not reset despite ::new() in body()"
    );
}
