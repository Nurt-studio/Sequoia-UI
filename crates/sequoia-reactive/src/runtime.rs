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

    /// Dependencies the observer is already subscribed to. They are not
    /// registered again, which keeps an unchanged rebuild from scanning every
    /// observer list (O(n^2) for many effects on one signal).
    known: Vec<usize>,
    deps: Vec<DepSlot>,
}

thread_local! {
    static FRAMES: RefCell<Vec<TrackingFrame>> = const { RefCell::new(Vec::new()) };
    static BATCH_DEPTH: Cell<u32> = const { Cell::new(0) };
    static PENDING: RefCell<Vec<Weak<EffectNode>>> = const { RefCell::new(Vec::new()) };
    static FLUSHING: Cell<bool> = const { Cell::new(false) };

    static TRANSACTIONS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

/// Backstop for a flush that keeps producing work.
///
/// A real cycle is caught per node ([`MAX_EFFECT_RUNS_PER_NODE`]); this bound
/// is only reached by pathological graphs, so it is high enough that a signal
/// with thousands of legitimate subscribers still runs every one of them.
const MAX_EFFECT_RUNS_PER_FLUSH: u32 = 100_000;

/// How many times one effect may run inside a single flush.
const MAX_EFFECT_RUNS_PER_NODE: u32 = 100;

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
        if let Some(o) = observer
            && !frame.known.contains(&dep_id)
        {
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
                known: Vec::new(),
                deps: Vec::new(),
            });
        });
        Self { dst: Some(dst) }
    }

    /// Tracks reads for `observer`, skipping the ones in `known` because they
    /// are already subscribed.
    pub(crate) fn new_known(
        observer: Option<Weak<dyn Observer>>,
        known: Vec<usize>,
        dst: &'a mut Vec<DepSlot>,
    ) -> Self {
        FRAMES.with_borrow_mut(|f| {
            f.push(TrackingFrame {
                observer,
                collect: false,
                known,
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
                known: Vec::new(),
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
            let depth = BATCH_DEPTH.with(|d| {
                let next = d.get().saturating_sub(1);
                d.set(next);
                next
            });
            if depth != 0 {
                return;
            }
            // Flush even while unwinding. A panic inside `f` skips the normal
            // flush, and effects already queued then stay `queued` forever, so
            // their state silently lags until some unrelated write. A second
            // panic here would abort the process, so it is swallowed: the
            // original panic wins and `PendingTail` restores the queue.
            if std::thread::panicking() {
                let _ = std::panic::catch_unwind(flush_effects);
            } else {
                flush_effects();
            }
        }
    }

    BATCH_DEPTH.with(|d| d.set(d.get().wrapping_add(1)));
    let _guard = DepthGuard;
    f()
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

pub fn untracked<R>(f: impl FnOnce() -> R) -> R {        FRAMES.with_borrow_mut(|f| {
            f.push(TrackingFrame {
                observer: None,
                collect: false,
                known: Vec::new(),
                deps: Vec::new(),
            });
        });
        let _guard = PopGuard;
        f()
}

/// Effects taken out of [`PENDING`] for the current flush that have not run yet.
///
/// The tail is never dropped on the floor: a node that was lifted out of the
/// queue but did not get to run still has `queued = true`, so leaving it
/// behind would silence every later write to its dependencies (`mark_dirty`
/// only re-enqueues on a `false -> true` transition). On unwind the tail goes
/// back to the front of `PENDING`, in its original order, so the next flush
/// still runs it.
struct PendingTail {
    nodes: VecDeque<Rc<EffectNode>>,
}

impl PendingTail {
    fn take() -> Self {
        let nodes = PENDING.with_borrow_mut(|pending| {
            pending
                .drain(..)
                .filter_map(|weak| weak.upgrade())
                .collect()
        });
        Self { nodes }
    }

    fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Hands out the next node with its `queued` flag cleared, so that a signal
    /// write inside its own body enqueues it again.
    fn next(&mut self) -> Option<Rc<EffectNode>> {
        let node = self.nodes.pop_front()?;
        node.clear_queued();
        Some(node)
    }

    fn clear_queued(&mut self) {
        for node in self.nodes.drain(..) {
            node.clear_queued();
        }
    }
}

impl Drop for PendingTail {
    fn drop(&mut self) {
        if self.nodes.is_empty() {
            return;
        }
        if !std::thread::panicking() {
            // The cycle guard cut the flush short on purpose: drop the tail
            // instead of leaving stale `queued` flags behind.
            self.clear_queued();
            return;
        }

        let mut restored: Vec<Weak<EffectNode>> = Vec::new();
        for node in self.nodes.drain(..) {
            if node.is_disposed() {
                node.clear_queued();
            } else {
                restored.push(Rc::downgrade(&node));
            }
        }
        if restored.is_empty() {
            return;
        }
        PENDING.with_borrow_mut(|pending| {
            // The tail was queued before anything the panicking effect added
            // while it ran, so it goes back to the front.
            let mut queue = restored;
            queue.append(pending);
            *pending = queue;
        });
    }
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
    let mut per_node: std::collections::HashMap<usize, u32> = std::collections::HashMap::new();
    let mut banned: std::collections::HashSet<usize> = std::collections::HashSet::new();
    'flush: loop {
        let mut tail = PendingTail::take();
        if tail.is_empty() {
            break;
        }
        while let Some(node) = tail.next() {
            if node.is_disposed() {
                continue;
            }
            let node_id = node.id();
            if banned.contains(&node_id) {
                // Over budget already; another write re-queued it, so keep it
                // quiet until this flush ends instead of running it again.
                node.clear_queued();
                continue;
            }
            runs += 1;
            // Count per node: one effect re-running hundreds of times is a
            // cycle, while a thousand different effects on one signal is a
            // perfectly normal graph that must not lose its tail.
            let node_runs = per_node.entry(node_id).or_insert(0);
            *node_runs += 1;
            if *node_runs > MAX_EFFECT_RUNS_PER_NODE {
                log::error!(
                    "sequoia-reactive: reactive cycle suspected \
                     (one effect ran {node_runs} times in a single flush) — skipping it"
                );
                node.clear_queued();
                banned.insert(node_id);
                continue;
            }
            if runs > MAX_EFFECT_RUNS_PER_FLUSH {
                log::error!(
                    "sequoia-reactive: reactive cycle suspected \
                     (over {MAX_EFFECT_RUNS_PER_FLUSH} effect runs in a single flush) — \
                     dropping the rest of the queue"
                );

                tail.clear_queued();
                let stale: Vec<Rc<EffectNode>> = PENDING.with_borrow_mut(|p| {
                    p.drain(..).filter_map(|w| w.upgrade()).collect()
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

/// Removes a node that queued itself from [`PENDING`] again.
///
/// An effect that reads a memo which recomputes inside `get` is notified while
/// its own body is still running, and the value it receives is already the new
/// one — running it again would just repeat the work.
pub(crate) fn dequeue_effect(node: &EffectNode) {
    let ptr = node as *const EffectNode;
    let removed = PENDING.with_borrow_mut(|pending| {
        let before = pending.len();
        pending.retain(|weak| !weak.upgrade().is_some_and(|n| Rc::as_ptr(&n) == ptr));
        pending.len() != before
    });
    if removed {
        node.clear_queued();
    }
}
