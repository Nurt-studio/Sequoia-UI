#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::cell::Cell;
use std::rc::Rc;

use sequoia_reactive::Signal;
use sequoia_test::Harness;
use sequoia_view::prelude::*;

struct Label(&'static str);

impl View for Label {
    fn body(&self) -> impl View + 'static {
        Text::new(self.0)
    }
}

#[derive(Clone)]
struct Counter {
    value: Signal<i32>,
    mounts: Rc<Cell<u32>>,
    unmounts: Rc<Cell<u32>>,
}

impl Counter {
    fn new(value: Signal<i32>) -> (Self, Rc<Cell<u32>>, Rc<Cell<u32>>) {
        let mounts = Rc::new(Cell::new(0u32));
        let unmounts = Rc::new(Cell::new(0u32));
        (
            Self {
                value,
                mounts: mounts.clone(),
                unmounts: unmounts.clone(),
            },
            mounts,
            unmounts,
        )
    }

    fn with_counters(value: Signal<i32>, mounts: Rc<Cell<u32>>, unmounts: Rc<Cell<u32>>) -> Self {
        Self {
            value,
            mounts,
            unmounts,
        }
    }
}

impl View for Counter {
    fn body(&self) -> impl View + 'static {
        let n = self.value.get();
        Text::new(format!("Count: {n}"))
    }

    fn on_mount(&self) {
        self.mounts.set(self.mounts.get() + 1);
    }

    fn on_unmount(&self) {
        self.unmounts.set(self.unmounts.get() + 1);
    }
}

struct Shell;

impl View for Shell {
    fn body(&self) -> impl View + 'static {
        Column::new().child(Label("A")).child(Label("B"))
    }
}

struct StaticRoot(Counter);

impl View for StaticRoot {
    fn body(&self) -> impl View + 'static {
        Column::new().child(self.0.clone()).child(Label("static"))
    }
}

struct Toggle {
    flag: Signal<bool>,
}

impl View for Toggle {
    fn body(&self) -> impl View + 'static {
        if self.flag.get() {
            AnyView::new(Label("ON"))
        } else {
            AnyView::new(Text::new("OFF"))
        }
    }
}

struct ToggleRoot(Signal<bool>);

impl View for ToggleRoot {
    fn body(&self) -> impl View + 'static {
        Column::new().child(Toggle {
            flag: self.0.clone(),
        })
    }
}

struct ToggleHooks {
    flag: Signal<bool>,
    mounts: Rc<Cell<u32>>,
    unmounts: Rc<Cell<u32>>,
}

impl View for ToggleHooks {
    fn body(&self) -> impl View + 'static {
        if self.flag.get() {
            AnyView::new(Text::new("plain"))
        } else {
            let counter =
                Counter::with_counters(Signal::new(0), self.mounts.clone(), self.unmounts.clone());
            AnyView::new(counter)
        }
    }
}

struct Rows(Signal<usize>);

impl View for Rows {
    fn body(&self) -> impl View + 'static {
        let n = self.0.get();
        let mut col = Column::new();
        for i in 0..n {
            col = col.child(Text::new(format!("row {i}")));
        }
        col
    }
}

struct Swap {
    which: Signal<u8>,
}

impl View for Swap {
    fn body(&self) -> impl View + 'static {
        match self.which.get() {
            0 => Text::new("A"),
            _ => Text::new("B"),
        }
    }
}

struct SwapRoot(Signal<u8>);

impl View for SwapRoot {
    fn body(&self) -> impl View + 'static {
        Column::new().child(Swap {
            which: self.0.clone(),
        })
    }
}

#[test]
fn mounts_expected_tree() {
    let h = Harness::new(Shell);
    let root = h.root().unwrap();
    let col = h.children(root)[0];
    assert_eq!(h.children(col).len(), 2);
    assert_eq!(h.stats().rebuilds, 0, "initial build is not a rebuild");
    h.tick();
    assert_eq!(h.stats().rebuilds, 0, "unchanged tick must not rebuild");
}

#[test]
fn unreferenced_signal_change_is_inert() {
    let orphan = Signal::new(1);
    let h = Harness::new(Shell);
    h.set(&orphan, 2);
    assert_eq!(h.stats().rebuilds, 0, "unread signal must not rebuild");
}

#[test]
fn children_override_reads_subscribe_rebuild() {
    struct ConditionalKids {
        flag: Signal<bool>,
    }

    impl View for ConditionalKids {
        fn body(&self) -> impl View + 'static {
            Unit
        }

        fn children_override(&self) -> Option<Vec<AnyView>> {
            if self.flag.get() {
                Some(vec![AnyView::new(Label("A")), AnyView::new(Label("B"))])
            } else {
                Some(vec![AnyView::new(Label("A"))])
            }
        }
    }

    let flag = Signal::new(false);
    let h = Harness::new(ConditionalKids { flag: flag.clone() });
    let root = h.root().unwrap();
    assert_eq!(h.children(root).len(), 1);

    h.set(&flag, true);
    let kids = h.children(root);
    assert_eq!(kids.len(), 2, "flag in children_override rebuilds");
    let texts: Vec<_> = kids
        .iter()
        .flat_map(|&k| h.children(k))
        .filter_map(|t| h.text(t))
        .collect();
    assert_eq!(texts, vec!["A".to_string(), "B".to_string()]);
}

#[test]
fn paint_request_frame_calls_redraw_hook() {
    struct Thirsty;
    impl View for Thirsty {
        fn body(&self) -> impl View + 'static {
            Unit
        }
        fn is_leaf(&self) -> bool {
            true
        }
        fn paint_self(&self, ctx: &mut PaintCtx) {
            ctx.request_frame();
        }
    }

    let frames = Rc::new(Cell::new(0u32));
    let h = Harness::new(Thirsty);
    h.layout(100.0, 100.0);
    let hook_frames = frames.clone();
    h.runtime().set_redraw_hook(move || {
        hook_frames.set(hook_frames.get() + 1);
    });
    h.paint();
    assert_eq!(
        frames.get(),
        1,
        "request_frame in paint_self requests a frame"
    );

    let frames = Rc::new(Cell::new(0u32));
    let h = Harness::new(Unit);
    h.layout(100.0, 100.0);
    let hook_frames = frames.clone();
    h.runtime().set_redraw_hook(move || {
        hook_frames.set(hook_frames.get() + 1);
    });
    h.paint();
    assert_eq!(frames.get(), 0, "no request means no frame (OnDemand idle)");
}

#[test]
fn signal_change_rebuilds_reader_only() {
    let value = Signal::new(0);
    let (counter, mounts, unmounts) = Counter::new(value.clone());
    let h = Harness::new(StaticRoot(counter));

    let root = h.root().unwrap();
    let col = h.children(root)[0];
    let counter_el = h.children(col)[0];
    let text_el = h.children(counter_el)[0];

    h.set(&value, 5);

    assert_eq!(h.text(text_el).as_deref(), Some("Count: 5"));
    assert_eq!(h.stats().rebuilds, 1, "exactly one element read the signal");
    assert_eq!(mounts.get(), 1, "rebuild must not remount the component");
    assert_eq!(unmounts.get(), 0);
    assert!(h.is_alive(counter_el));
    assert!(h.is_alive(text_el), "child identity kept");
}

#[test]
fn type_change_in_slot_remounts() {
    let flag = Signal::new(true);
    let h = Harness::new(ToggleRoot(flag.clone()));

    let root = h.root().unwrap();
    let col = h.children(root)[0];
    let toggle_el = h.children(col)[0];
    let branch = h.children(toggle_el)[0];
    assert!(h.type_name_of(branch).unwrap().contains("Label"));

    h.set(&flag, false);

    assert!(!h.is_alive(branch), "old branch must be unmounted");
    let branch2 = h.children(toggle_el)[0];
    assert_ne!(branch, branch2, "remount = new ElementId");
    assert!(h.type_name_of(branch2).unwrap().contains("::Text"));
    assert_eq!(h.text(branch2).as_deref(), Some("OFF"));
    assert!(h.stats().unmounts >= 1);
    assert!(h.stats().mounts > h.stats().unmounts);
}

#[test]
fn lifecycle_hooks_called_on_mount_and_unmount() {
    let flag = Signal::new(true);
    let mounts = Rc::new(Cell::new(0u32));
    let unmounts = Rc::new(Cell::new(0u32));

    struct Root(Signal<bool>, Rc<Cell<u32>>, Rc<Cell<u32>>);

    impl View for Root {
        fn body(&self) -> impl View + 'static {
            Column::new().child(ToggleHooks {
                flag: self.0.clone(),
                mounts: self.1.clone(),
                unmounts: self.2.clone(),
            })
        }
    }

    let h = Harness::new(Root(flag.clone(), mounts.clone(), unmounts.clone()));
    assert_eq!(mounts.get(), 0, "Text branch has no counter hooks");
    assert_eq!(unmounts.get(), 0);

    h.set(&flag, false);
    assert_eq!(mounts.get(), 1);
    assert_eq!(unmounts.get(), 0);

    h.set(&flag, true);
    assert_eq!(unmounts.get(), 1);

    h.set(&flag, false);
    assert_eq!(mounts.get(), 2);
}

#[test]
fn children_grow_and_shrink() {
    let n = Signal::new(2usize);
    let h = Harness::new(Rows(n.clone()));

    let root = h.root().unwrap();
    let col = h.children(root)[0];
    assert_eq!(h.children(col).len(), 2);
    let initial = h.children(col);

    h.set(&n, 4);
    assert_eq!(h.children(col).len(), 4);
    assert_eq!(h.children(col)[0], initial[0], "identity kept on append");
    assert!(h.is_alive(initial[1]));

    h.set(&n, 1);
    assert_eq!(h.children(col).len(), 1);
    assert!(!h.is_alive(initial[1]), "extra child unmounted");
    assert_eq!(h.text(h.children(col)[0]).as_deref(), Some("row 0"));
}

#[test]
fn children_shrink_from_four_to_two() {
    let n = Signal::new(4usize);
    let h = Harness::new(Rows(n.clone()));
    let root = h.root().unwrap();
    let col = h.children(root)[0];
    let initial = h.children(col);
    assert_eq!(initial.len(), 4);

    h.set(&n, 2);
    assert_eq!(h.children(col).len(), 2);
    assert_eq!(h.children(col)[0], initial[0]);
    assert_eq!(h.children(col)[1], initial[1]);
    assert!(h.is_alive(initial[0]));
    assert!(h.is_alive(initial[1]));
    assert!(!h.is_alive(initial[2]));
    assert!(!h.is_alive(initial[3]));
}

#[test]
fn same_type_replacement_keeps_identity() {
    let which = Signal::new(0u8);
    let h = Harness::new(SwapRoot(which.clone()));

    let root = h.root().unwrap();
    let col = h.children(root)[0];
    let swap_el = h.children(col)[0];
    let text_el = h.children(swap_el)[0];
    assert_eq!(h.text(text_el).as_deref(), Some("A"));

    h.set(&which, 1);

    let text_el2 = h.children(swap_el)[0];
    assert_eq!(text_el, text_el2, "same type in slot = identity kept");
    assert_eq!(h.text(text_el2).as_deref(), Some("B"));
}

#[test]
fn double_wrapped_anyview_unwraps_correctly() {
    struct DoubleWrap;
    impl View for DoubleWrap {
        fn body(&self) -> impl View + 'static {
            let inner = AnyView::new(Text::new("inner"));
            AnyView::new(inner)
        }
    }

    let h = Harness::new(DoubleWrap);
    let root = h.root().unwrap();
    let text = h.children(root)[0];

    assert!(h.type_name_of(text).unwrap().contains("::Text"));
    assert_eq!(h.text(text).as_deref(), Some("inner"));
    assert_eq!(h.element_count(), 2);
}

#[test]
fn stress_updates_stabilize() {
    let value = Signal::new(0);
    let (counter, mounts, unmounts) = Counter::new(value.clone());
    let h = Harness::new(StaticRoot(counter));

    for i in 0..1_000 {
        h.set(&value, i);
    }

    let root = h.root().unwrap();
    let col = h.children(root)[0];
    let counter_el = h.children(col)[0];
    let text_el = h.children(counter_el)[0];

    assert_eq!(h.text(text_el).as_deref(), Some("Count: 999"));
    assert_eq!(mounts.get(), 1);
    assert_eq!(unmounts.get(), 0);
    assert_eq!(
        h.element_count(),
        6,
        "no element leaks: Root+Column+Counter+Text+Label+Text"
    );
}
