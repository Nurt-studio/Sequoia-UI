#![forbid(unsafe_code)]

use crate::task::{TaskHandle, TaskId};

pub struct ScopedTask {
    handle: Option<TaskHandle>,
    cancelled: bool,
}

impl ScopedTask {
    pub fn from_handle(handle: TaskHandle) -> Self {
        Self {
            handle: Some(handle),
            cancelled: false,
        }
    }

    pub fn new(id: TaskId, cancel: impl Fn(TaskId) -> bool + 'static) -> Self {
        Self {
            handle: Some(TaskHandle::new(id, cancel)),
            cancelled: false,
        }
    }

    pub fn id(&self) -> Option<TaskId> {
        self.handle.as_ref().map(|h| h.id())
    }

    pub fn cancel(&mut self) -> bool {
        if self.cancelled {
            return false;
        }
        self.cancelled = true;
        match &self.handle {
            Some(h) => h.cancel(),
            None => false,
        }
    }

    pub fn detach(mut self) -> Option<TaskId> {
        let id = self.id();
        self.cancelled = true;
        self.handle.take();

        id
    }

    pub fn is_active(&self) -> bool {
        !self.cancelled && self.handle.is_some()
    }
}

impl Drop for ScopedTask {
    fn drop(&mut self) {
        if !self.cancelled
            && let Some(h) = &self.handle
        {
            h.cancel();
        }
    }
}

impl std::fmt::Debug for ScopedTask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScopedTask")
            .field("id", &self.handle.as_ref().map(|h| h.id().get()))
            .field("cancelled", &self.cancelled)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn drop_cancels_once() {
        let n = Rc::new(Cell::new(0));
        let probe = Rc::clone(&n);
        {
            let _guard = ScopedTask::new(crate::task::TaskId::new(1), move |_| {
                probe.set(probe.get() + 1);
                true
            });
        }
        assert_eq!(n.get(), 1);
    }

    #[test]
    fn detach_skips_cancel() {
        let n = Rc::new(Cell::new(0));
        let probe = Rc::clone(&n);
        {
            let guard = ScopedTask::new(crate::task::TaskId::new(2), move |_| {
                probe.set(probe.get() + 1);
                true
            });
            let _ = guard.detach();
        }
        assert_eq!(n.get(), 0);
    }

    #[test]
    fn explicit_cancel_is_idempotent() {
        let mut guard = ScopedTask::new(crate::task::TaskId::new(3), |_| true);
        assert!(guard.cancel());
        assert!(!guard.cancel());
        assert!(!guard.is_active());
    }

    #[test]
    fn from_handle_cancels_on_drop() {
        let n = Rc::new(Cell::new(0));
        let probe = Rc::clone(&n);
        let handle = TaskHandle::new(crate::task::TaskId::new(4), move |_| {
            probe.set(probe.get() + 1);
            true
        });
        {
            let _guard = ScopedTask::from_handle(handle);
        }
        assert_eq!(n.get(), 1);
    }
}
