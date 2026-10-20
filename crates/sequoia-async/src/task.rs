#![forbid(unsafe_code)]

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TaskId(u64);

impl TaskId {
    pub fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub fn get(self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "task#{}", self.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum JoinError {
    Cancelled,

    Panicked,
}

impl std::fmt::Display for JoinError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cancelled => write!(f, "task was cancelled"),
            Self::Panicked => write!(f, "task panicked on worker"),
        }
    }
}

impl std::error::Error for JoinError {}

#[derive(Clone)]
pub struct TaskHandle {
    id: TaskId,
    cancel: std::rc::Rc<dyn Fn(TaskId) -> bool>,
}

impl TaskHandle {
    pub fn new(id: TaskId, cancel: impl Fn(TaskId) -> bool + 'static) -> Self {
        Self {
            id,
            cancel: std::rc::Rc::new(cancel),
        }
    }

    pub fn id(&self) -> TaskId {
        self.id
    }

    pub fn cancel(&self) -> bool {
        (self.cancel)(self.id)
    }
}

impl std::fmt::Debug for TaskHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaskHandle").field("id", &self.id).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handle_delegates_cancel_to_closure() {
        let seen = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let probe = std::rc::Rc::clone(&seen);
        let h = TaskHandle::new(TaskId::new(7), move |id| {
            probe.borrow_mut().push(id.get());
            true
        });
        assert_eq!(h.id().get(), 7);
        assert!(h.cancel());
        assert_eq!(*seen.borrow(), vec![7]);
    }

    #[test]
    fn task_id_orders_and_displays() {
        assert!(TaskId::new(1) < TaskId::new(2));
        assert_eq!(format!("{}", TaskId::new(3)), "task#3");
    }
}
