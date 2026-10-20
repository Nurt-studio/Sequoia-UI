use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::{Rc, Weak};
use std::thread_local;

use crate::effect::EffectNode;

pub(crate) trait Observer {
    fn id(&self) -> usize;

    fn mark_dirty(&self) -> bool;

    fn dependents(&self) -> Vec<Weak<dyn Observer>>;
}

pub(crate) trait Dependency {
    fn id(&self) -> usize;

    fn version(&self) -> u64;

    fn validate(&self);

    fn observers(&self) -> Vec<Weak<dyn Observer>>;

    fn register_observer(&self, observer: Weak<dyn Observer>);

    fn unregister_observer(&self, observer_id: usize);
}

pub(crate) type DepSlot = (usize, Weak<dyn Dependency>, u64);

struct TrackingFrame {
    observer: Option<Weak<dyn Observer>>,

    collect: bool,
    deps: Vec<DepSlot>,
}

thread_local! {
    static FRAMES: RefCell<Vec<TrackingFrame>> = const { RefCell::new(Vec::new()) };
    static BATCH_DEPTH: Cell<u32> = const { Cell::new(0) };
    static PENDING: RefCell<Vec<Weak<EffectNode>>> = const { RefCell::new(Vec::new()) };
    static FLUSHING: Cell<bool> = const { Cell::new(false) };

    static TRANSACTIONS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

const MAX_EFFECT_RUNS_PER_FLUSH: u32 = 1_000;

pub(crate) fn track_read<D: Dependency + 'static>(dep: &Rc<D>) {
    FRAMES.with_borrow_mut(|frames| {
        let Some(frame) = frames.last_mut() else {
            return;
        };
        let dep_id = dep.id();
        if frame.deps.iter().any(|(id, _, _)| *id == dep_id) {
            return;
        }
        let observer = frame.observer.clone();
        if observer.is_none() && !frame.collect {
            return;
        }
        if let Some(o) = observer {
            dep.register_observer(o);
        }
        let version = dep.version();

        let dep_dyn: Rc<dyn Dependency> = dep.clone();
        let weak: Weak<dyn Dependency> = Rc::downgrade(&dep_dyn);
        frame.deps.push((dep_id, weak, version));
    });
}

pub(crate) struct FrameGuard<'a> {
    dst: Option<&'a mut Vec<DepSlot>>,
}

impl<'a> FrameGuard<'a> {
    pub(crate) fn new(observer: Option<Weak<dyn Observer>>, dst: &'a mut Vec<DepSlot>) -> Self {
        FRAMES.with_borrow_mut(|f| {
            f.push(TrackingFrame {
                observer,
                collect: false,
                deps: Vec::new(),
            });
        });
        Self { dst: Some(dst) }
    }

    pub(crate) fn new_capture(dst: &'a mut Vec<DepSlot>) -> Self {
        FRAMES.with_borrow_mut(|f| {
            f.push(TrackingFrame {
                observer: None,
                collect: true,
                deps: Vec::new(),
            });
        });
        Self { dst: Some(dst) }
    }
}

impl Drop for FrameGuard<'_> {
    fn drop(&mut self) {
        let frame = FRAMES.with_borrow_mut(|f| f.pop());
        if let (Some(dst), Some(frame)) = (self.dst.as_deref_mut(), frame) {
            *dst = frame.deps;
        }
    }
}

pub(crate) struct FlagGuard<'a> {
    flag: &'a Cell<bool>,
}

impl<'a> FlagGuard<'a> {
    pub(crate) fn new(flag: &'a Cell<bool>) -> Self {
        flag.set(true);
        Self { flag }
    }
}

impl Drop for FlagGuard<'_> {
    fn drop(&mut self) {
        self.flag.set(false);
    }
}

pub(crate) fn propagate(start: &dyn Dependency) {
    let mut queue: VecDeque<Weak<dyn Observer>> = start.observers().into();
    while let Some(weak) = queue.pop_front() {
        let Some(node) = weak.upgrade() else { continue };
        if node.mark_dirty() {
            queue.extend(node.dependents());
        }
    }
}

pub(crate) fn register_observer_in(
    list: &RefCell<Vec<Weak<dyn Observer>>>,
    observer: Weak<dyn Observer>,
) {
    let mut list = list.borrow_mut();
    for existing in list.iter() {
        if let (Some(a), Some(b)) = (existing.upgrade(), observer.upgrade())
            && Rc::ptr_eq(&a, &b)
        {
            return;
        }
    }
    list.retain(|w| w.upgrade().is_some());
    list.push(observer);
}

pub(crate) fn unregister_observer_in(list: &RefCell<Vec<Weak<dyn Observer>>>, observer_id: usize) {
    let mut list = list.borrow_mut();
    if let Some(pos) = list
        .iter()
        .position(|w| w.upgrade().is_some_and(|o| o.id() == observer_id))
    {
        list.swap_remove(pos);
    }
}

pub(crate) fn observers_in(list: &RefCell<Vec<Weak<dyn Observer>>>) -> Vec<Weak<dyn Observer>> {
    let mut list = list.borrow_mut();
    list.retain(|w| w.upgrade().is_some());
    list.clone()
}

pub(crate) fn unregister_all(deps: &RefCell<Vec<DepSlot>>, observer_id: usize) {
    let drained: Vec<DepSlot> = deps.borrow_mut().drain(..).collect();
    for (_, dep, _) in drained {
        if let Some(d) = dep.upgrade() {
            d.unregister_observer(observer_id);
        }
    }
}

pub(crate) fn batch_depth() -> u32 {
    BATCH_DEPTH.with(Cell::get)
}

pub fn batch<R>(f: impl FnOnce() -> R) -> R {
    struct DepthGuard;
    impl Drop for DepthGuard {
        fn drop(&mut self) {
            BATCH_DEPTH.with(|d| d.set(d.get().saturating_sub(1)));
        }
    }

    BATCH_DEPTH.with(|d| d.set(d.get().wrapping_add(1)));
    let guard = DepthGuard;
    let result = f();
    drop(guard);
    if BATCH_DEPTH.with(Cell::get) == 0 {
        flush_effects();
    }
    result
}

pub fn transaction<R>(name: &str, f: impl FnOnce() -> R) -> R {
    TRANSACTIONS.with(|t| t.borrow_mut().push(name.to_owned()));
    struct PopGuard;
    impl Drop for PopGuard {
        fn drop(&mut self) {
            TRANSACTIONS.with(|t| {
                t.borrow_mut().pop();
            });
        }
    }
    let _guard = PopGuard;
    batch(f)
}

pub fn current_transaction() -> Option<String> {
    TRANSACTIONS.with(|t| t.borrow().first().cloned())
}

struct PopGuard;
impl Drop for PopGuard {
    fn drop(&mut self) {
        FRAMES.with_borrow_mut(|f| {
            f.pop();
        });
    }
}

pub fn untracked<R>(f: impl FnOnce() -> R) -> R {
    FRAMES.with_borrow_mut(|f| {
        f.push(TrackingFrame {
            observer: None,
            collect: false,
            deps: Vec::new(),
        });
    });
    let _guard = PopGuard;
    f()
}

pub fn flush_effects() {
    if FLUSHING.with(Cell::get) {
        return;
    }
    FLUSHING.with(|f| f.set(true));
    struct FlushGuard;
    impl Drop for FlushGuard {
        fn drop(&mut self) {
            FLUSHING.with(|f| f.set(false));
        }
    }
    let _guard = FlushGuard;

    let mut runs: u32 = 0;
    'flush: loop {
        let ready: Vec<Rc<EffectNode>> = PENDING.with_borrow_mut(|p| {
            let ready: Vec<_> = p.iter().filter_map(|w| w.upgrade()).collect();
            p.clear();
            ready
        });
        if ready.is_empty() {
            break;
        }
        for (i, node) in ready.iter().enumerate() {
            node.clear_queued();
            if node.is_disposed() {
                continue;
            }
            runs += 1;
            if runs > MAX_EFFECT_RUNS_PER_FLUSH {
                log::error!(
                    "sequoia-reactive: reactive cycle suspected \
                     (over {MAX_EFFECT_RUNS_PER_FLUSH} effect runs in a single flush) — \
                     dropping the rest of the queue"
                );

                for rest in &ready[i..] {
                    rest.clear_queued();
                }
                let stale: Vec<Rc<EffectNode>> = PENDING.with_borrow_mut(|p| {
                    let stale: Vec<_> = p.iter().filter_map(|w| w.upgrade()).collect();
                    p.clear();
                    stale
                });
                for node in stale {
                    node.clear_queued();
                }
                break 'flush;
            }
            node.run();
        }
    }
}

pub(crate) fn enqueue_effect(node: &Weak<EffectNode>) {
    PENDING.with_borrow_mut(|p| p.push(node.clone()));
}
