use std::cell::{Cell, RefCell};
use std::rc::Rc;

use sequoia_reactive::{Effect, Signal, effect};
use sequoia_test::Harness;
use sequoia_view::prelude::*;

#[derive(Clone)]
struct Probe {
    mounts: Rc<Cell<u32>>,
    unmounts: Rc<Cell<u32>>,
    source: Signal<i32>,
    hits: Rc<Cell<u32>>,
    sub: Rc<RefCell<Option<Effect>>>,
}

impl View for Probe {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn is_leaf(&self) -> bool {
        true
    }

    fn on_mount(&self) {
        self.mounts.set(self.mounts.get() + 1);
        let (source, hits) = (self.source.clone(), self.hits.clone());
        *self.sub.borrow_mut() = Some(effect(move || {
            source.get();
            hits.set(hits.get() + 1);
        }));
    }

    fn on_unmount(&self) {
        self.unmounts.set(self.unmounts.get() + 1);
        self.sub.borrow_mut().take();
    }
}

#[derive(Clone)]
struct Switch {
    show: Signal<bool>,
    probe: Probe,
}

impl View for Switch {
    fn body(&self) -> impl View + 'static {
        let _ = self.show.get();
        Unit
    }

    fn children_override(&self) -> Option<Vec<AnyView>> {
        if self.show.get_untracked() {
            Some(vec![AnyView::new(self.probe.clone())])
        } else {
            Some(vec![])
        }
    }

    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle::default()
    }
}

fn setup() -> (
    Harness,
    Signal<bool>,
    Signal<i32>,
    Rc<Cell<u32>>,
    Rc<Cell<u32>>,
    Rc<Cell<u32>>,
) {
    let show = Signal::new(true);
    let source = Signal::new(0);
    let mounts = Rc::new(Cell::new(0));
    let unmounts = Rc::new(Cell::new(0));
    let hits = Rc::new(Cell::new(0));
    let probe = Probe {
        mounts: mounts.clone(),
        unmounts: unmounts.clone(),
        source: source.clone(),
        hits: hits.clone(),
        sub: Rc::new(RefCell::new(None)),
    };
    let h = Harness::new(Switch {
        show: show.clone(),
        probe,
    });
    h.layout(400.0, 300.0);
    h.paint();
    h.tick();
    (h, show, source, mounts, unmounts, hits)
}

#[test]
fn mount_fires_once_then_effect_tracks() {
    let (_h, _show, source, mounts, unmounts, hits) = setup();
    assert_eq!(mounts.get(), 1, "on_mount raz przy starcie");
    assert_eq!(unmounts.get(), 0);
    let first = hits.get();
    assert!(first >= 1, "efekt odpala przy mounucie");
    source.set(1);
    assert_eq!(hits.get(), first + 1, "the effect lives on after the mount");
}

#[test]
fn unmount_cancels_effect_and_remount_restarts() {
    let (h, show, source, mounts, unmounts, hits) = setup();
    show.set(false);
    h.layout(400.0, 300.0);
    h.paint();
    h.tick();
    assert_eq!(unmounts.get(), 1, "on_unmount after photo");
    let frozen = hits.get();
    source.set(2);
    source.set(3);
    h.tick();
    assert_eq!(hits.get(), frozen, "efekt anulowany po unmounucie");

    show.set(true);
    h.layout(400.0, 300.0);
    h.paint();
    h.tick();
    assert_eq!(mounts.get(), 2, "ponowny mount");
    let before = hits.get();
    source.set(4);
    assert_eq!(hits.get(), before + 1, "new effect works");
}
