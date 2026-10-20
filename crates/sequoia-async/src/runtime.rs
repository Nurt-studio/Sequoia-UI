#![forbid(unsafe_code)]

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;
use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures_core::Future;
use sequoia_reactive::Signal;

use crate::emitter::{Emitter, StreamKind, StreamMsgAny};
use crate::task::{TaskHandle, TaskId};
use crate::wake::{WakeFn, noop_wake};

enum Completion {
    Done(u64, Box<dyn Any + Send>),
    Panicked(u64),
}

type Apply = Box<dyn FnOnce(Box<dyn Any + Send>)>;
type StreamApply = std::rc::Rc<dyn Fn(Box<dyn Any + Send>)>;
type Job = Box<dyn FnOnce() + Send + 'static>;

const THREAD_NAME: &str = "sequoia-async";

#[derive(Clone)]
pub struct AsyncRuntime {
    inner: Rc<Inner>,
}

struct Inner {
    pool: Arc<Pool>,
    done: Receiver<Completion>,
    applies: RefCell<HashMap<u64, Apply>>,
    streams: RefCell<HashMap<u64, StreamApply>>,
    stream_tx: Sender<StreamMsgAny>,
    stream_rx: Receiver<StreamMsgAny>,
    pending: Signal<u64>,
    next_id: Cell<u64>,
    wake: WakeFn,
}

impl AsyncRuntime {
    pub fn new(threads: usize) -> Self {
        Self::with_wake(threads, noop_wake())
    }

    pub fn with_default_threads() -> Self {
        Self::new(default_threads())
    }

    pub fn with_wake(threads: usize, wake: WakeFn) -> Self {
        let (tx, rx) = channel::<Completion>();
        let (stx, srx) = channel::<StreamMsgAny>();
        Self {
            inner: Rc::new(Inner {
                pool: Pool::new(threads, tx),
                done: rx,
                applies: RefCell::new(HashMap::new()),
                streams: RefCell::new(HashMap::new()),
                stream_tx: stx,
                stream_rx: srx,
                pending: Signal::new(0),
                next_id: Cell::new(1),
                wake,
            }),
        }
    }

    fn reserve(&self) -> u64 {
        let id = self.inner.next_id.get();
        self.inner.next_id.set(id + 1);
        self.inner.pending.update(|n| *n += 1);
        id
    }

    pub fn spawn_future<T, F, A>(&self, future: F, apply: A) -> TaskHandle
    where
        T: Send + 'static,
        F: Future<Output = T> + Send + 'static,
        A: FnOnce(T) + 'static,
    {
        let id = self.reserve();
        self.inner.applies.borrow_mut().insert(
            id,
            Box::new(move |any: Box<dyn Any + Send>| {
                if let Ok(value) = any.downcast::<T>() {
                    apply(*value);
                }
            }),
        );
        let tx = self.inner.pool.results_tx();
        let wake = self.inner.wake.clone();
        self.inner.pool.submit(Box::new(move || {
            let msg = match catch_unwind(AssertUnwindSafe(|| pollster::block_on(future))) {
                Ok(value) => Completion::Done(id, Box::new(value)),
                Err(_) => Completion::Panicked(id),
            };
            let _ = tx.send(msg);
            wake();
        }));
        let rt = self.clone();
        TaskHandle::new(TaskId::new(id), move |tid| rt.cancel_raw(tid.get()))
    }

    pub fn spawn_task<F>(&self, future: F) -> TaskHandle
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.spawn_future(future, |()| {})
    }

    pub fn spawn_blocking<T, W, A>(&self, work: W, apply: A) -> TaskHandle
    where
        T: Send + 'static,
        W: FnOnce() -> T + Send + 'static,
        A: FnOnce(T) + 'static,
    {
        let id = self.reserve();
        self.inner.applies.borrow_mut().insert(
            id,
            Box::new(move |any: Box<dyn Any + Send>| {
                if let Ok(value) = any.downcast::<T>() {
                    apply(*value);
                }
            }),
        );
        let tx = self.inner.pool.results_tx();
        let wake = self.inner.wake.clone();
        self.inner.pool.submit(Box::new(move || {
            let msg = match catch_unwind(AssertUnwindSafe(work)) {
                Ok(value) => Completion::Done(id, Box::new(value)),
                Err(_) => Completion::Panicked(id),
            };
            let _ = tx.send(msg);
            wake();
        }));
        let rt = self.clone();
        TaskHandle::new(TaskId::new(id), move |tid| rt.cancel_raw(tid.get()))
    }

    fn cancel_raw(&self, id: u64) -> bool {
        let mut hit = false;
        if self.inner.applies.borrow_mut().remove(&id).is_some() {
            hit = true;
        }
        if self.inner.streams.borrow_mut().remove(&id).is_some() {
            hit = true;
        }
        if hit {
            self.inner.pending.update(|n| *n = n.saturating_sub(1));
        }
        hit
    }

    pub fn cancel(&self, id: TaskId) -> bool {
        self.cancel_raw(id.get())
    }

    pub fn spawn_stream<T, F, Make, C>(&self, make_future: Make, on_item: C) -> TaskHandle
    where
        T: Send + 'static,
        F: Future<Output = ()> + Send + 'static,
        Make: FnOnce(Emitter<T>) -> F + Send + 'static,
        C: Fn(T) + 'static,
    {
        let id = self.reserve();
        let apply: StreamApply = std::rc::Rc::new(move |any: Box<dyn Any + Send>| {
            if let Ok(value) = any.downcast::<T>() {
                on_item(*value);
            }
        });
        self.inner.streams.borrow_mut().insert(id, apply);
        let stx = self.inner.stream_tx.clone();
        let wake = self.inner.wake.clone();
        let emitter = Emitter::new(id, stx.clone(), wake.clone());
        let future = make_future(emitter);
        self.inner.pool.submit(Box::new(move || {
            let outcome = catch_unwind(AssertUnwindSafe(|| pollster::block_on(future)));
            let kind = match outcome {
                Ok(()) => StreamKind::Finished,
                Err(_) => StreamKind::Panicked,
            };
            let _ = stx.send(StreamMsgAny { id, kind });
            wake();
        }));
        let rt = self.clone();
        TaskHandle::new(TaskId::new(id), move |tid| rt.cancel_raw(tid.get()))
    }

    pub fn spawn_to_signal<T, F>(&self, signal: &Signal<T>, future: F) -> TaskHandle
    where
        T: Clone + PartialEq + Send + 'static,
        F: Future<Output = T> + Send + 'static,
    {
        let signal = signal.clone();
        self.spawn_future(future, move |v| signal.set(v))
    }

    pub fn pending(&self) -> Signal<u64> {
        self.inner.pending.clone()
    }

    pub fn is_busy(&self) -> bool {
        self.inner.pending.get_untracked() > 0
    }

    pub fn threads(&self) -> usize {
        self.inner.pool.threads()
    }

    pub fn drain(&self) -> usize {
        let mut handled = 0;
        loop {
            let msg = match self.inner.done.try_recv() {
                Ok(msg) => msg,
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            };
            let id = match &msg {
                Completion::Done(id, _) | Completion::Panicked(id) => *id,
            };
            let apply = self.inner.applies.borrow_mut().remove(&id);
            match (msg, apply) {
                (Completion::Done(_, value), Some(apply)) => {
                    apply(value);
                    handled += 1;
                    self.inner.pending.update(|n| *n = n.saturating_sub(1));
                }
                (Completion::Panicked(..), Some(_)) => {
                    log::error!("sequoia-async: task #{id} panicked — result dropped");
                    handled += 1;
                    self.inner.pending.update(|n| *n = n.saturating_sub(1));
                }
                (Completion::Done(..), None) | (Completion::Panicked(..), None) => {}
            }
        }
        handled += self.drain_streams();
        handled
    }

    fn drain_streams(&self) -> usize {
        let mut handled = 0;
        loop {
            let msg = match self.inner.stream_rx.try_recv() {
                Ok(msg) => msg,
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            };
            match msg.kind {
                StreamKind::Item(value) => {
                    if let Some(apply) = self.inner.streams.borrow().get(&msg.id).cloned() {
                        apply(value);
                        handled += 1;
                    }
                }
                StreamKind::Finished | StreamKind::Panicked => {
                    let was = self.inner.streams.borrow_mut().remove(&msg.id).is_some();
                    if was {
                        if matches!(msg.kind, StreamKind::Panicked) {
                            log::error!("sequoia-async: stream #{} panicked — closed", msg.id);
                        }
                        self.inner.pending.update(|n| *n = n.saturating_sub(1));
                        handled += 1;
                    }
                }
            }
        }
        handled
    }

    pub fn drain_until(&self, timeout: Duration) -> usize {
        crate::testing::drain_until(self, timeout)
    }
}

struct Pool {
    jobs: Mutex<Option<Sender<Job>>>,
    results: Mutex<Sender<Completion>>,
    threads: usize,
}

impl Pool {
    fn new(threads: usize, results_tx: Sender<Completion>) -> Arc<Self> {
        let threads = threads.max(1);
        let (jobs_tx, jobs_rx) = channel::<Job>();
        let jobs_rx = Arc::new(Mutex::new(jobs_rx));
        let mut started = 0;
        for index in 0..threads {
            let rx = Arc::clone(&jobs_rx);
            let spawned = std::thread::Builder::new()
                .name(format!("{THREAD_NAME}-{index}"))
                .spawn(move || {
                    loop {
                        let job = rx.lock().unwrap_or_else(|e| e.into_inner()).recv();
                        match job {
                            Ok(job) => job(),
                            Err(_) => break,
                        }
                    }
                });
            if spawned.is_ok() {
                started += 1;
            } else {
                log::error!("sequoia-async: failed to start worker thread");
            }
        }
        Arc::new(Self {
            jobs: Mutex::new(Some(jobs_tx)),
            results: Mutex::new(results_tx),
            threads: started.max(1),
        })
    }

    fn submit(&self, job: Job) {
        let guard = self.jobs.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(tx) = guard.as_ref()
            && tx.send(job).is_err()
        {
            log::error!("sequoia-async: failed to queue task");
        }
    }

    fn results_tx(&self) -> Sender<Completion> {
        self.results
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    fn threads(&self) -> usize {
        self.threads
    }
}

impl Drop for Pool {
    fn drop(&mut self) {
        self.jobs.lock().unwrap_or_else(|e| e.into_inner()).take();
    }
}

pub(crate) fn default_threads() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get().saturating_sub(1).max(1))
        .unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn future_runs_off_thread_and_apply_on_caller() {
        use std::sync::{Arc, Mutex};
        let rt = AsyncRuntime::new(1);
        let ui = std::thread::current().id();
        let worker = Arc::new(Mutex::new(None));
        let out = Arc::new(Mutex::new(None));
        let w = Arc::clone(&worker);
        let o = Arc::clone(&out);
        rt.spawn_future(
            async move {
                w.lock().unwrap().replace(std::thread::current().id());
                21 * 2
            },
            move |v: i32| {
                o.lock().unwrap().replace((v, std::thread::current().id()));
            },
        );
        assert!(rt.is_busy());
        assert_eq!(rt.drain_until(Duration::from_secs(5)), 1);
        assert_eq!(*out.lock().unwrap(), Some((42, ui)));
        assert_ne!(worker.lock().unwrap().unwrap(), ui);
        assert!(!rt.is_busy());
    }

    #[test]
    fn cancel_drops_result() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};
        let rt = AsyncRuntime::new(1);
        let flag = Arc::new(AtomicBool::new(false));
        let f = Arc::clone(&flag);
        let h = rt.spawn_future(
            async move {
                crate::time::sleep(Duration::from_millis(30)).await;
            },
            move |()| f.store(true, Ordering::SeqCst),
        );
        assert!(h.cancel());
        std::thread::sleep(Duration::from_millis(120));
        assert_eq!(rt.drain(), 0);
        assert!(!flag.load(Ordering::SeqCst));
        assert_eq!(rt.pending().get_untracked(), 0);
    }

    #[test]
    fn threads_never_below_one() {
        let rt = AsyncRuntime::new(0);
        assert_eq!(rt.threads(), 1);
        assert_eq!(rt.drain(), 0);
    }

    #[test]
    fn stream_delivers_multiple_items_then_finishes() {
        use std::sync::{Arc, Mutex};
        let rt = AsyncRuntime::new(2);
        let items = Arc::new(Mutex::new(Vec::new()));
        let probe = Arc::clone(&items);
        let _h = rt.spawn_stream(
            |em: Emitter<i32>| async move {
                for v in [1, 2, 3] {
                    em.emit(v).unwrap();
                }
            },
            move |v: i32| probe.lock().unwrap().push(v),
        );
        assert!(rt.is_busy());
        let ok = crate::testing::run_until(&rt, Duration::from_secs(5), || {
            items.lock().unwrap().len() == 3 && !rt.is_busy()
        });
        assert!(ok, "stream items + finish drained");
        assert_eq!(*items.lock().unwrap(), vec![1, 2, 3]);
        assert_eq!(rt.pending().get_untracked(), 0);
    }

    #[test]
    fn stream_cancel_stops_delivery() {
        use std::sync::{Arc, Mutex};
        let rt = AsyncRuntime::new(1);
        let items = Arc::new(Mutex::new(Vec::new()));
        let probe = Arc::clone(&items);
        let h = rt.spawn_stream(
            |em: Emitter<i32>| async move {
                crate::time::sleep(Duration::from_millis(50)).await;
                let _ = em.emit(99);
            },
            move |v: i32| probe.lock().unwrap().push(v),
        );
        assert!(h.cancel());
        std::thread::sleep(Duration::from_millis(150));
        rt.drain();
        assert!(items.lock().unwrap().is_empty());
        assert_eq!(rt.pending().get_untracked(), 0);
    }

    #[test]
    fn spawn_to_signal_writes_value() {
        let rt = AsyncRuntime::new(1);
        let s = Signal::new(0u32);
        rt.spawn_to_signal(&s, async { 11u32 });
        assert!(crate::testing::run_until(
            &rt,
            Duration::from_secs(5),
            || { s.get_untracked() == 11 }
        ));
    }
}
