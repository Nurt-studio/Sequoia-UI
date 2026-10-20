#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::cell::Cell;
use std::rc::Rc;

use sequoia_core::{Length, WindowId};
use sequoia_reactive::Signal;
use sequoia_test::Harness;
use sequoia_view::prelude::*;

#[derive(Clone)]
struct Counter {
    count: Signal<u32>,
}

impl View for Counter {
    fn body(&self) -> impl View + 'static {
        Text::new(format!("n={}", self.count.get()))
    }
}

fn shown(h: &Harness) -> Option<String> {
    let root = h.root()?;
    let mut stack = h.children(root);
    while let Some(id) = stack.pop() {
        if let Some(t) = h.text(id) {
            return Some(t);
        }
        stack.extend(h.children(id));
    }
    None
}

#[test]
fn shared_signal_updates_both_runtimes() {
    let count = Signal::new(0u32);
    let h1 = Harness::new(Counter {
        count: count.clone(),
    });
    let h2 = Harness::new(Counter {
        count: count.clone(),
    });
    h1.layout(400.0, 300.0);
    h2.layout(400.0, 300.0);
    assert_eq!(shown(&h1).as_deref(), Some("n=0"));
    assert_eq!(shown(&h2).as_deref(), Some("n=0"));

    count.set(7);
    h1.tick();
    h2.tick();
    h1.layout(400.0, 300.0);
    h2.layout(400.0, 300.0);
    assert_eq!(
        shown(&h1).as_deref(),
        Some("n=7"),
        "window 1 sees the write"
    );
    assert_eq!(
        shown(&h2).as_deref(),
        Some("n=7"),
        "window 2 sees the write"
    );

    assert!(h1.stats().rebuilds > 0);
    assert!(h2.stats().rebuilds > 0);
}

#[test]
fn runtimes_are_independent_trees() {
    let count = Signal::new(1u32);
    let h1 = Harness::new(Counter {
        count: count.clone(),
    });
    let h2 = Harness::new(Counter {
        count: count.clone(),
    });
    h1.layout(400.0, 300.0);
    h2.layout(400.0, 300.0);
    let n1 = h1.element_count();
    let n2 = h2.element_count();
    assert!(n1 > 0 && n2 > 0);
    drop(h1);
    h2.tick();
    h2.layout(400.0, 300.0);
    assert_eq!(shown(&h2).as_deref(), Some("n=1"));
}

struct Probe {
    seen: Rc<Cell<WindowId>>,
}

impl View for Probe {
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

    fn on_event(&self, _event: &Event, ctx: &mut EventCtx) {
        self.seen.set(ctx.window);
    }
}

#[test]
fn event_ctx_carries_window_id() {
    let seen = Rc::new(Cell::new(WindowId::new(999)));
    let h = Harness::new(Probe { seen: seen.clone() });
    h.layout(200.0, 200.0);

    h.click(10.0, 10.0);
    assert_eq!(seen.get(), WindowId::PRIMARY, "main window by default");

    h.runtime().set_window_id(WindowId::new(3));
    assert_eq!(h.runtime().window_id(), WindowId::new(3));
    h.click(10.0, 10.0);
    assert_eq!(seen.get(), WindowId::new(3), "runtime carries id to ctx");
}
