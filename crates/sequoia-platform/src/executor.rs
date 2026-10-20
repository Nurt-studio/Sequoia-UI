use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;
use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};
use std::sync::{Arc, Mutex};

use futures_core::Future;
use sequoia_async::{Emitter, Resource, ScopedTask, StreamKind, StreamMsgAny, TaskId};
use sequoia_reactive::Signal;
use sequoia_view::{ElementId, Runtime};
use winit::event_loop::EventLoopProxy;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct JobId(u64);

impl JobId {
    pub fn get(self) -> u64 {
        self.0
    }

    pub fn from_task(id: TaskId) -> Self {
        Self(id.get())
    }

    pub fn task_id(self) -> TaskId {
        TaskId::new(self.0)
    }
}

impl From<TaskId> for JobId {
    fn from(id: TaskId) -> Self {
        Self(id.get())
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct JobWake;

enum JobMsg {
    Done(u64, Box<dyn Any + Send>),

    Panicked(u64),
}

type Apply = Box<dyn FnOnce(Box<dyn Any + Send>)>;

type StreamApply = Rc<dyn Fn(Box<dyn Any + Send>)>;

type Job = Box<dyn FnOnce() + Send + 'static>;

const THREAD_NAME: &str = "sequoia-job";

#[derive(Clone)]
pub struct Executor {
    inner: Rc<Inner>,
}

struct Inner {
    pool: Arc<Pool>,

    done: Receiver<JobMsg>,

    applies: RefCell<HashMap<u64, Apply>>,

    streams: RefCell<HashMap<u64, StreamApply>>,

    stream_tx: Sender<StreamMsgAny>,

    stream_rx: Receiver<StreamMsgAny>,

    pending: Signal<u64>,

    next_id: Cell<u64>,

    proxy: Option<EventLoopProxy<JobWake>>,
}

impl Executor {
    pub(crate) fn new(proxy: Option<EventLoopProxy<JobWake>>, threads: usize) -> Self {
        let (results_tx, results_rx) = channel::<JobMsg>();
        let (stream_tx, stream_rx) = channel::<StreamMsgAny>();
        Self {
            inner: Rc::new(Inner {
                pool: Pool::new(threads, results_tx),
                done: results_rx,
                applies: RefCell::new(HashMap::new()),
                streams: RefCell::new(HashMap::new()),
                stream_tx,
                stream_rx,
                pending: Signal::new(0),
                next_id: Cell::new(1),
                proxy,
            }),
        }
    }

    pub fn spawn<T, W, A>(&self, work: W, apply: A) -> JobId
    where
        T: Send + 'static,
        W: FnOnce() -> T + Send + 'static,
        A: FnOnce(T) + 'static,
    {
        let id = self.inner.next_id.get();
        self.inner.next_id.set(id + 1);

        self.inner.applies.borrow_mut().insert(
            id,
            Box::new(move |any: Box<dyn Any + Send>| {
                if let Ok(value) = any.downcast::<T>() {
                    apply(*value);
                }
            }),
        );
        self.inner.pending.update(|n| *n += 1);

        let tx = self.inner.pool.results_tx();
        let proxy = self.inner.proxy.clone();
        self.inner.pool.submit(Box::new(move || {
            let msg = match catch_unwind(AssertUnwindSafe(work)) {
                Ok(value) => JobMsg::Done(id, Box::new(value)),
                Err(_) => JobMsg::Panicked(id),
            };
            let _ = tx.send(msg);

            if let Some(proxy) = &proxy {
                let _ = proxy.send_event(JobWake);
            }
        }));
        JobId(id)
    }

    pub fn spawn_scoped_rt<T, W, A>(&self, rt: &Runtime, el: ElementId, work: W, apply: A) -> JobId
    where
        T: Send + 'static,
        W: FnOnce() -> T + Send + 'static,
        A: FnOnce(T) + 'static,
    {
        let alive = rt.clone();
        let id = self.spawn(work, move |v| {
            if alive.is_alive(el) {
                apply(v);
            }
        });
        let exec = self.clone();
        rt.track_scoped(el, move || {
            exec.cancel(id);
        });
        id
    }

    pub fn spawn_future_scoped_rt<T, F, A>(
        &self,
        rt: &Runtime,
        el: ElementId,
        future: F,
        apply: A,
    ) -> JobId
    where
        T: Send + 'static,
        F: Future<Output = T> + Send + 'static,
        A: FnOnce(T) + 'static,
    {
        let alive = rt.clone();
        let id = self.spawn_future(future, move |v| {
            if alive.is_alive(el) {
                apply(v);
            }
        });
        let exec = self.clone();
        rt.track_scoped(el, move || {
            exec.cancel(id);
        });
        id
    }

    pub fn cancel(&self, id: JobId) -> bool {
        let mut hit = false;
        if self.inner.applies.borrow_mut().remove(&id.0).is_some() {
            hit = true;
        }
        if self.inner.streams.borrow_mut().remove(&id.0).is_some() {
            hit = true;
        }
        if hit {
            self.inner.pending.update(|n| *n = n.saturating_sub(1));
        }
        hit
    }

    pub fn scoped(&self, id: JobId) -> ScopedTask {
        let exec = self.clone();
        ScopedTask::new(id.task_id(), move |tid| exec.cancel(JobId(tid.get())))
    }

    pub fn spawn_future<T, F, A>(&self, future: F, apply: A) -> JobId
    where
        T: Send + 'static,
        F: Future<Output = T> + Send + 'static,
        A: FnOnce(T) + 'static,
    {
        let id = self.inner.next_id.get();
        self.inner.next_id.set(id + 1);
        self.inner.applies.borrow_mut().insert(
            id,
            Box::new(move |any: Box<dyn Any + Send>| {
                if let Ok(value) = any.downcast::<T>() {
                    apply(*value);
                }
            }),
        );
        self.inner.pending.update(|n| *n += 1);
        let tx = self.inner.pool.results_tx();
        let proxy = self.inner.proxy.clone();
        self.inner.pool.submit(Box::new(move || {
            let msg = match catch_unwind(AssertUnwindSafe(|| pollster::block_on(future))) {
                Ok(value) => JobMsg::Done(id, Box::new(value)),
                Err(_) => JobMsg::Panicked(id),
            };
            let _ = tx.send(msg);
            if let Some(proxy) = &proxy {
                let _ = proxy.send_event(JobWake);
            }
        }));
        JobId(id)
    }

    pub fn spawn_future_scoped<T, F, A>(&self, future: F, apply: A) -> ScopedTask
    where
        T: Send + 'static,
        F: Future<Output = T> + Send + 'static,
        A: FnOnce(T) + 'static,
    {
        let id = self.spawn_future(future, apply);
        self.scoped(id)
    }

    pub fn spawn_task<F>(&self, future: F) -> JobId
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.spawn_future(future, |()| {})
    }

    pub fn spawn_to_signal<T, F>(&self, signal: &Signal<T>, future: F) -> JobId
    where
        T: Clone + PartialEq + Send + 'static,
        F: Future<Output = T> + Send + 'static,
    {
        let signal = signal.clone();
        self.spawn_future(future, move |v| signal.set(v))
    }

    pub fn spawn_resource<T, F>(&self, resource: &Resource<T>, future: F) -> JobId
    where
        T: Clone + PartialEq + Send + 'static,
        F: Future<Output = T> + Send + 'static,
    {
        let loading = resource.begin_loading();
        let resource = resource.clone();
        self.spawn_future(future, move |v| resource.complete(loading, v))
    }

    pub fn spawn_resource_result<T, E, F>(&self, resource: &Resource<T>, future: F) -> JobId
    where
        T: Clone + PartialEq + Send + 'static,
        E: std::fmt::Display + Send + 'static,
        F: Future<Output = Result<T, E>> + Send + 'static,
    {
        let loading = resource.begin_loading();
        let resource = resource.clone();
        self.spawn_future(future, move |r| match r {
            Ok(v) => resource.complete(loading, v),
            Err(e) => resource.fail(loading, e.to_string()),
        })
    }

    pub fn spawn_stream<T, F, Make, C>(&self, make_future: Make, on_item: C) -> JobId
    where
        T: Send + 'static,
        F: Future<Output = ()> + Send + 'static,
        Make: FnOnce(Emitter<T>) -> F + Send + 'static,
        C: Fn(T) + 'static,
    {
        let id = self.inner.next_id.get();
        self.inner.next_id.set(id + 1);
        let apply: StreamApply = Rc::new(move |any: Box<dyn Any + Send>| {
            if let Ok(value) = any.downcast::<T>() {
                on_item(*value);
            }
        });
        self.inner.streams.borrow_mut().insert(id, apply);
        self.inner.pending.update(|n| *n += 1);
        let stx = self.inner.stream_tx.clone();
        let proxy = self.inner.proxy.clone();
        let wake_done = move || {
            if let Some(proxy) = &proxy {
                let _ = proxy.send_event(JobWake);
            }
        };
        let emitter = Emitter::new(
            id,
            stx.clone(),
            std::sync::Arc::new({
                let proxy = self.inner.proxy.clone();
                move || {
                    if let Some(proxy) = &proxy {
                        let _ = proxy.send_event(JobWake);
                    }
                }
            }),
        );
        let future = make_future(emitter);
        self.inner.pool.submit(Box::new(move || {
            let outcome = catch_unwind(AssertUnwindSafe(|| pollster::block_on(future)));
            let kind = match outcome {
                Ok(()) => StreamKind::Finished,
                Err(_) => StreamKind::Panicked,
            };
            let _ = stx.send(StreamMsgAny { id, kind });
            wake_done();
        }));
        JobId(id)
    }

    pub fn spawn_stream_scoped<T, F, Make, C>(&self, make_future: Make, on_item: C) -> ScopedTask
    where
        T: Send + 'static,
        F: Future<Output = ()> + Send + 'static,
        Make: FnOnce(Emitter<T>) -> F + Send + 'static,
        C: Fn(T) + 'static,
    {
        let id = self.spawn_stream(make_future, on_item);
        self.scoped(id)
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

    pub(crate) fn drain(&self) -> usize {
        let mut handled = 0;
        loop {
            let msg = match self.inner.done.try_recv() {
                Ok(msg) => msg,

                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            };
            let id = match &msg {
                JobMsg::Done(id, _) | JobMsg::Panicked(id) => *id,
            };

            let apply = self.inner.applies.borrow_mut().remove(&id);
            match (msg, apply) {
                (JobMsg::Done(_, value), Some(apply)) => {
                    apply(value);
                    handled += 1;
                    self.inner.pending.update(|n| *n = n.saturating_sub(1));
                }
                (JobMsg::Panicked(..), Some(_)) => {
                    log::error!("sequoia-platform: task #{id} panicked — result dropped");
                    handled += 1;
                    self.inner.pending.update(|n| *n = n.saturating_sub(1));
                }

                (JobMsg::Done(..), None) | (JobMsg::Panicked(..), None) => {}
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
                            log::error!("sequoia-platform: stream #{} panicked — closed", msg.id);
                        }
                        self.inner.pending.update(|n| *n = n.saturating_sub(1));
                        handled += 1;
                    }
                }

                _ => {}
            }
        }
        handled
    }
}

impl std::fmt::Debug for Executor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Executor")
            .field("threads", &self.threads())
            .field("pending", &self.inner.pending.get_untracked())
            .finish()
    }
}

struct Pool {
    jobs: Mutex<Option<Sender<Job>>>,

    results: Mutex<Sender<JobMsg>>,

    threads: usize,
}

impl Pool {
    fn new(threads: usize, results_tx: Sender<JobMsg>) -> Arc<Self> {
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
            match spawned {
                Ok(_) => started += 1,
                Err(e) => log::error!("sequoia-platform: failed to start worker thread: {e}"),
            }
        }
        if started == 0 {
            log::error!("sequoia-platform: worker pool has no threads — tasks will wait");
        }
        Arc::new(Self {
            jobs: Mutex::new(Some(jobs_tx)),
            results: Mutex::new(results_tx),
            threads: started,
        })
    }

    fn submit(&self, job: Job) {
        let guard = self.jobs.lock().unwrap_or_else(|e| e.into_inner());
        match guard.as_ref() {
            Some(tx) => {
                if tx.send(job).is_err() {
                    log::error!("sequoia-platform: failed to queue task");
                }
            }
            None => log::error!("sequoia-platform: executor already released"),
        }
    }

    fn results_tx(&self) -> Sender<JobMsg> {
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
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::{Duration, Instant};

    fn detached(threads: usize) -> Executor {
        Executor::new(None, threads)
    }

    fn drain_until(exec: &Executor, timeout: Duration) -> usize {
        let deadline = Instant::now() + timeout;
        loop {
            let n = exec.drain();
            if n > 0 || Instant::now() >= deadline {
                return n;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    fn drain_for(exec: &Executor, timeout: Duration) {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            exec.drain();
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    #[test]
    fn work_runs_off_thread_and_apply_runs_on_caller() {
        let exec = detached(1);
        let ui_thread = std::thread::current().id();
        let worker_thread = Arc::new(Mutex::new(None));
        let applied = Arc::new(Mutex::new(None));

        let seen = Arc::clone(&worker_thread);
        let out = Arc::clone(&applied);
        let id = exec.spawn(
            move || {
                *seen.lock().unwrap() = Some(std::thread::current().id());
                21 * 2
            },
            move |value: i32| {
                *out.lock().unwrap() = Some((value, std::thread::current().id()));
            },
        );

        assert_eq!(id.get(), 1, "ids grow from 1");
        assert!(exec.is_busy(), "task is in flight right after spawn");
        assert_eq!(drain_until(&exec, Duration::from_secs(5)), 1);
        assert_eq!(
            *applied.lock().unwrap(),
            Some((42, ui_thread)),
            "apply runs on the UI thread"
        );
        let worker = worker_thread.lock().unwrap().expect("work ran");
        assert_ne!(worker, ui_thread, "work must not run on the UI thread");
        assert!(!exec.is_busy(), "pending returns to zero");
        assert_eq!(exec.pending().get_untracked(), 0);
    }

    #[test]
    fn pending_signal_tracks_running_jobs() {
        let exec = detached(1);
        let pending = exec.pending();
        assert_eq!(pending.get_untracked(), 0);

        let (release, wait) = channel::<()>();
        exec.spawn(
            move || {
                let _ = wait.recv();
            },
            |_: ()| {},
        );
        assert_eq!(pending.get_untracked(), 1, "spawn raises the counter");

        let _ = release.send(());
        assert_eq!(drain_until(&exec, Duration::from_secs(5)), 1);
        assert_eq!(pending.get_untracked(), 0, "apply lowers the counter");
    }

    #[test]
    fn cancel_drops_the_result() {
        let exec = detached(1);
        let applied = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&applied);
        let id = exec.spawn(
            || std::thread::sleep(Duration::from_millis(10)),
            move |_: ()| flag.store(true, Ordering::SeqCst),
        );

        assert!(exec.cancel(id), "unapplied task can be cancelled");
        assert!(!exec.cancel(id), "second cancel is a no-op");
        assert_eq!(exec.pending().get_untracked(), 0);

        drain_for(&exec, Duration::from_millis(200));
        assert!(!applied.load(Ordering::SeqCst), "cancelled = not applied");
        assert_eq!(
            exec.pending().get_untracked(),
            0,
            "cancel does not double-count"
        );
        assert!(!exec.is_busy());
    }

    #[test]
    fn cancelled_job_does_not_disturb_other_pending_jobs() {
        let exec = detached(1);
        let (release, wait) = channel::<()>();
        let long = exec.spawn(
            move || {
                let _ = wait.recv();
                1
            },
            |_: i32| {},
        );
        let short = exec.spawn(|| 2, |_: i32| {});
        assert_eq!(exec.pending().get_untracked(), 2);

        assert!(exec.cancel(short));
        assert_eq!(
            exec.pending().get_untracked(),
            1,
            "only the long task remains"
        );
        drain_for(&exec, Duration::from_millis(100));
        assert_eq!(
            exec.pending().get_untracked(),
            1,
            "cancelled result does not roll back the long task"
        );

        let _ = release.send(());
        assert_eq!(drain_until(&exec, Duration::from_secs(5)), 1);
        assert_eq!(exec.pending().get_untracked(), 0);
        assert_eq!(long.get(), 1);
    }

    #[test]
    fn panic_in_work_clears_pending_without_applying() {
        let exec = detached(1);
        let applied = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&applied);
        exec.spawn(
            || -> u32 { panic!("import failed") },
            move |_: u32| flag.store(true, Ordering::SeqCst),
        );

        assert_eq!(drain_until(&exec, Duration::from_secs(5)), 1);
        assert!(!applied.load(Ordering::SeqCst));
        assert_eq!(exec.pending().get_untracked(), 0);
    }

    #[test]
    fn jobs_share_the_pool_and_all_results_arrive() {
        let exec = detached(2);
        let results = Arc::new(Mutex::new(Vec::new()));
        for value in 0..8 {
            let out = Arc::clone(&results);
            exec.spawn(move || value * 3, move |v: i32| out.lock().unwrap().push(v));
        }
        assert_eq!(drain_until(&exec, Duration::from_secs(5)), 8);
        let mut sorted = results.lock().unwrap().clone();
        sorted.sort_unstable();
        assert_eq!(sorted, vec![0, 3, 6, 9, 12, 15, 18, 21]);
        assert_eq!(exec.threads(), 2);
    }

    #[test]
    fn threads_never_drop_below_one() {
        let exec = detached(0);
        assert_eq!(exec.threads(), 1, "pool always has a worker thread");
        assert_eq!(exec.drain(), 0, "no tasks = nothing to settle");
    }

    #[test]
    fn default_threads_leaves_one_core_for_the_ui() {
        let threads = default_threads();
        assert!(threads >= 1);
        if let Ok(cores) = std::thread::available_parallelism() {
            assert!(threads <= cores.get());
        }
    }

    #[test]
    fn future_runs_off_thread_and_apply_on_caller() {
        let exec = detached(2);
        let ui_thread = std::thread::current().id();
        let worker_thread = Arc::new(Mutex::new(None));
        let applied = Arc::new(Mutex::new(None));
        let seen = Arc::clone(&worker_thread);
        let out = Arc::clone(&applied);
        exec.spawn_future(
            async move {
                *seen.lock().unwrap() = Some(std::thread::current().id());
                21 * 2
            },
            move |value: i32| {
                *out.lock().unwrap() = Some((value, std::thread::current().id()));
            },
        );
        assert!(exec.is_busy());
        assert_eq!(drain_until(&exec, Duration::from_secs(5)), 1);
        assert_eq!(*applied.lock().unwrap(), Some((42, ui_thread)));
        assert_ne!(worker_thread.lock().unwrap().unwrap(), ui_thread);
        assert!(!exec.is_busy());
    }

    #[test]
    fn future_cancel_drops_result() {
        let exec = detached(1);
        let applied = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&applied);
        let id = exec.spawn_future(
            async move {
                sequoia_async::sleep(std::time::Duration::from_millis(40)).await;
            },
            move |()| flag.store(true, Ordering::SeqCst),
        );
        assert!(exec.cancel(id));
        std::thread::sleep(Duration::from_millis(150));
        assert_eq!(exec.drain(), 0);
        assert!(!applied.load(Ordering::SeqCst));
        assert_eq!(exec.pending().get_untracked(), 0);
    }

    #[test]
    fn future_panic_clears_pending_without_applying() {
        let exec = detached(1);
        let applied = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&applied);
        exec.spawn_future(async { panic!("async import failed") }, move |_: ()| {
            flag.store(true, Ordering::SeqCst)
        });
        assert_eq!(drain_until(&exec, Duration::from_secs(5)), 1);
        assert!(!applied.load(Ordering::SeqCst));
        assert_eq!(exec.pending().get_untracked(), 0);
    }

    #[test]
    fn spawn_to_signal_writes_on_ui() {
        let exec = detached(1);
        let signal = Signal::new(0u32);
        exec.spawn_to_signal(&signal, async { 11u32 });
        let deadline = Instant::now() + Duration::from_secs(5);
        while signal.get_untracked() != 11 && Instant::now() < deadline {
            exec.drain();
            std::thread::sleep(Duration::from_millis(2));
        }
        assert_eq!(signal.get_untracked(), 11);
    }

    #[test]
    fn spawn_resource_guards_stale_results() {
        use sequoia_async::Resource;
        let exec = detached(2);
        let resource: Resource<String> = Resource::new();
        let (slow_gate, slow_wait) = channel::<()>();
        let res_slow = resource.clone();
        let slow_gen = resource.begin_loading();
        exec.spawn_future(
            async move {
                let _ = slow_wait.recv();
                "stale".to_string()
            },
            move |v: String| res_slow.complete(slow_gen, v),
        );
        let res_fast = resource.clone();
        exec.spawn_resource(&res_fast, async { "fresh".to_string() });
        let deadline = Instant::now() + Duration::from_secs(5);
        while exec.is_busy() && resource.data_untracked().as_deref() != Some("fresh") {
            exec.drain();
            if Instant::now() > deadline {
                break;
            }
            std::thread::sleep(Duration::from_millis(2));
        }

        exec.drain();
        let _ = slow_gate.send(());
        drain_for(&exec, Duration::from_millis(300));
        assert_eq!(resource.data_untracked().as_deref(), Some("fresh"));
    }

    #[test]
    fn stream_delivers_items_then_finishes() {
        use std::sync::Mutex;
        let exec = detached(2);
        let items = Arc::new(Mutex::new(Vec::new()));
        let probe = Arc::clone(&items);
        exec.spawn_stream(
            |em: Emitter<i32>| async move {
                for v in [1, 2, 3] {
                    em.emit(v).unwrap();
                }
            },
            move |v: i32| probe.lock().unwrap().push(v),
        );
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            exec.drain();
            let done = items.lock().unwrap().len() == 3 && !exec.is_busy();
            if done || Instant::now() >= deadline {
                break;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        assert_eq!(*items.lock().unwrap(), vec![1, 2, 3]);
        assert_eq!(exec.pending().get_untracked(), 0);
    }

    #[test]
    fn scoped_rt_cancels_on_unmount() {
        use sequoia_view::{Runtime, Unit};
        let exec = detached(1);
        let rt = Runtime::new();
        rt.mount(Unit);
        let root = rt.root().unwrap();
        assert_eq!(rt.scoped_count(), 0);

        let applied = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&applied);
        exec.spawn_scoped_rt(
            &rt,
            root,
            || std::thread::sleep(Duration::from_millis(200)),
            move |_: ()| flag.store(true, Ordering::SeqCst),
        );
        assert_eq!(rt.scoped_count(), 1);

        rt.mount(Unit);
        assert_eq!(rt.scoped_count(), 0, "unmount drenuje anulacje");
        drain_for(&exec, Duration::from_millis(400));
        assert!(
            !applied.load(Ordering::SeqCst),
            "anulowany = nie aplikowany"
        );
    }

    #[test]
    fn scoped_rt_apply_is_noop_for_dead_element() {
        use sequoia_view::{Runtime, Unit};
        let exec = detached(1);
        let rt = Runtime::new();
        rt.mount(Unit);
        let dead = rt.root().unwrap();
        rt.mount(Unit);

        let applied = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&applied);
        exec.spawn_scoped_rt(
            &rt,
            dead,
            || 7,
            move |_: i32| flag.store(true, Ordering::SeqCst),
        );
        assert_eq!(rt.scoped_count(), 0, "martwy el = cancel od razu");
        drain_for(&exec, Duration::from_millis(300));
        assert!(!applied.load(Ordering::SeqCst));
    }

    #[test]
    fn scoped_rt_applies_while_alive() {
        use sequoia_view::{Runtime, Unit};
        let exec = detached(1);
        let rt = Runtime::new();
        rt.mount(Unit);
        let root = rt.root().unwrap();

        let applied = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&applied);
        exec.spawn_scoped_rt(
            &rt,
            root,
            || 1,
            move |_: i32| flag.store(true, Ordering::SeqCst),
        );
        assert_eq!(drain_until(&exec, Duration::from_secs(5)), 1);
        assert!(applied.load(Ordering::SeqCst), "live el = apply works");
    }

    #[test]
    fn scoped_task_cancels_on_drop() {
        let exec = detached(1);
        let applied = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&applied);
        {
            let _guard = exec.spawn_future_scoped(
                async move {
                    sequoia_async::sleep(std::time::Duration::from_millis(40)).await;
                },
                move |()| flag.store(true, Ordering::SeqCst),
            );
            assert!(exec.is_busy());
        }
        std::thread::sleep(Duration::from_millis(150));
        exec.drain();
        assert!(!applied.load(Ordering::SeqCst));
        assert_eq!(exec.pending().get_untracked(), 0);
    }
}
