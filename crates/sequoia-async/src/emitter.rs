#![forbid(unsafe_code)]

use std::any::Any;
use std::sync::mpsc::Sender;

use crate::wake::WakeFn;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StreamError;

impl std::fmt::Display for StreamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "async stream closed")
    }
}

impl std::error::Error for StreamError {}

#[doc(hidden)]
pub struct StreamMsgAny {
    pub id: u64,

    pub kind: StreamKind,
}

#[doc(hidden)]
#[non_exhaustive]
pub enum StreamKind {
    Item(Box<dyn Any + Send>),

    Finished,

    Panicked,
}

pub struct Emitter<T: Send + 'static> {
    id: u64,
    tx: Sender<StreamMsgAny>,
    wake: WakeFn,
    _marker: std::marker::PhantomData<T>,
}

impl<T: Send + 'static> Emitter<T> {
    #[doc(hidden)]
    pub fn new(id: u64, tx: Sender<StreamMsgAny>, wake: WakeFn) -> Self {
        Self {
            id,
            tx,
            wake,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn emit(&self, value: T) -> Result<(), StreamError> {
        self.tx
            .send(StreamMsgAny {
                id: self.id,
                kind: StreamKind::Item(Box::new(value)),
            })
            .map_err(|_| StreamError)?;
        (self.wake)();
        Ok(())
    }
}

impl<T: Send + 'static> Clone for Emitter<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            tx: self.tx.clone(),
            wake: self.wake.clone(),
            _marker: std::marker::PhantomData,
        }
    }
}

impl<T: Send + 'static> std::fmt::Debug for Emitter<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Emitter").field("id", &self.id).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wake::noop_wake;
    use std::sync::mpsc::channel;

    #[test]
    fn emitter_sends_item_and_wakes() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};
        let (tx, rx) = channel::<StreamMsgAny>();
        let n = Arc::new(AtomicUsize::new(0));
        let probe = Arc::clone(&n);
        let wake = crate::wake::fn_wake(move || {
            probe.fetch_add(1, Ordering::SeqCst);
        });
        let em: Emitter<u32> = Emitter::new(9, tx, wake);
        assert!(em.emit(42).is_ok());
        assert_eq!(n.load(Ordering::SeqCst), 1);
        let msg = rx.try_recv().expect("item queued");
        assert_eq!(msg.id, 9);
        match msg.kind {
            StreamKind::Item(any) => assert_eq!(*any.downcast::<u32>().unwrap(), 42),
            _ => panic!("expected item"),
        }
        let _ = noop_wake();
    }

    #[test]
    fn emitter_errors_after_receiver_dropped() {
        let (tx, rx) = channel::<StreamMsgAny>();
        drop(rx);
        let em: Emitter<u32> = Emitter::new(1, tx, noop_wake());
        assert_eq!(em.emit(1).unwrap_err(), StreamError);
    }
}
