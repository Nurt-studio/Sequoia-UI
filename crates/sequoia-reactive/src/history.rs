use std::time::{Duration, Instant};

#[derive(Clone, Debug, Default)]
pub struct History<T> {
    undo: Vec<T>,
    redo: Vec<T>,
    limit: usize,

    last_coalesced: Option<Instant>,
}

impl<T> History<T> {
    pub fn new(limit: usize) -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
            limit,
            last_coalesced: None,
        }
    }

    pub fn undo_len(&self) -> usize {
        self.undo.len()
    }

    pub fn redo_len(&self) -> usize {
        self.redo.len()
    }

    pub fn depths(&self) -> (usize, usize) {
        (self.undo.len(), self.redo.len())
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
        self.last_coalesced = None;
    }

    pub fn checkpoint(&mut self, value: T) {
        if self.limit == 0 {
            return;
        }
        if self.undo.len() >= self.limit {
            self.undo.remove(0);
        }
        self.undo.push(value);
        self.redo.clear();
        self.last_coalesced = None;
    }

    pub fn checkpoint_coalesced(&mut self, value: T, window: Duration) {
        if self.limit == 0 {
            return;
        }
        if let Some(t) = self.last_coalesced
            && t.elapsed() < window
        {
            self.last_coalesced = Some(Instant::now());
            return;
        }
        if self.undo.len() >= self.limit {
            self.undo.remove(0);
        }
        self.undo.push(value);
        self.redo.clear();
        self.last_coalesced = Some(Instant::now());
    }

    pub fn undo(&mut self, current: T) -> Option<T> {
        let prev = self.undo.pop()?;
        if self.redo.len() >= self.limit.max(1) {
            self.redo.remove(0);
        }
        self.redo.push(current);
        self.last_coalesced = None;
        Some(prev)
    }

    pub fn redo(&mut self, current: T) -> Option<T> {
        let next = self.redo.pop()?;
        if self.undo.len() >= self.limit.max(1) {
            self.undo.remove(0);
        }
        self.undo.push(current);
        self.last_coalesced = None;
        Some(next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checkpoint_undo_redo_roundtrip() {
        let mut h = History::new(60);
        assert!(!h.can_undo());
        h.checkpoint(vec!["a".to_string()]);
        h.checkpoint(vec!["a".to_string(), "b".to_string()]);
        assert_eq!(h.depths(), (2, 0));
        let prev = h.undo(vec!["a".to_string(), "b".to_string(), "c".to_string()]);
        assert_eq!(prev, Some(vec!["a".to_string(), "b".to_string()]));
        assert_eq!(h.depths(), (1, 1));
        let next = h.redo(vec!["a".to_string(), "b".to_string()]);
        assert_eq!(
            next,
            Some(vec!["a".to_string(), "b".to_string(), "c".to_string()])
        );
        assert_eq!(h.depths(), (2, 0));
    }

    #[test]
    fn checkpoint_clears_redo() {
        let mut h = History::new(60);
        h.checkpoint(1);
        h.checkpoint(2);
        let _ = h.undo(3);
        assert!(h.can_redo());
        h.checkpoint(4);
        assert!(!h.can_redo());
        assert_eq!(h.depths(), (2, 0));
    }

    #[test]
    fn coalescing_collapses_rapid_checkpoints() {
        let mut h = History::new(60);
        let w = Duration::from_secs(3600);
        h.checkpoint_coalesced(1, w);
        h.checkpoint_coalesced(2, w);
        h.checkpoint_coalesced(3, w);

        assert_eq!(h.depths(), (1, 0));
        h.checkpoint_coalesced(4, Duration::from_nanos(0));
        assert_eq!(h.depths(), (2, 0));
    }

    #[test]
    fn limit_bounds_both_stacks() {
        let mut h = History::new(2);
        h.checkpoint(1);
        h.checkpoint(2);
        h.checkpoint(3);
        assert_eq!(h.depths(), (2, 0));
        let _ = h.undo(4);
        let _ = h.undo(5);

        assert_eq!(h.depths(), (0, 2));
        assert!(h.redo(6).is_some());
    }

    #[test]
    fn zero_limit_disables_history() {
        let mut h: History<i32> = History::new(0);
        h.checkpoint(1);
        assert!(!h.can_undo());
        assert!(h.undo(2).is_none());
    }
}
