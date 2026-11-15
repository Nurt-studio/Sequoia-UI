use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Default undo depth shared by every history-backed type.
///
/// `History` used to derive `Default` with a limit of zero, which silently
/// disabled history, while `Store` kept 100 steps. One default keeps the
/// semantics of `History`, `Store` and `UndoSignal` aligned.
pub(crate) const DEFAULT_HISTORY_LIMIT: usize = 100;

#[derive(Clone, Debug)]
pub struct History<T> {
    // Bounded stacks: dropping the oldest entry must not shift the rest, which
    // made `checkpoint`/`undo` O(n) for a large limit.
    undo: VecDeque<T>,
    redo: VecDeque<T>,
    limit: usize,

    last_coalesced: Option<Instant>,
    /// Tag of the step `last_coalesced` refers to. Steps with different tags
    /// (typing vs. dragging) must never merge into one undo entry.
    last_tag: Option<String>,
}

impl<T> Default for History<T> {
    fn default() -> Self {
        Self::new(DEFAULT_HISTORY_LIMIT)
    }
}

impl<T> History<T> {
    pub fn new(limit: usize) -> Self {
        Self {
            undo: VecDeque::new(),
            redo: VecDeque::new(),
            limit,
            last_coalesced: None,
            last_tag: None,
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
        self.last_tag = None;
    }

    pub fn checkpoint(&mut self, value: T) {
        if self.limit == 0 {
            return;
        }
        if self.undo.len() >= self.limit {
            self.undo.pop_front();
        }
        self.undo.push_back(value);
        self.redo.clear();
        self.last_coalesced = None;
        self.last_tag = None;
    }

    pub fn checkpoint_coalesced(&mut self, tag: &str, value: T, window: Duration) {
        if self.limit == 0 {
            return;
        }
        if let Some(t) = self.last_coalesced
            && self.last_tag.as_deref() == Some(tag)
            && t.elapsed() < window
        {
            self.last_coalesced = Some(Instant::now());
            return;
        }
        if self.undo.len() >= self.limit {
            self.undo.pop_front();
        }
        self.undo.push_back(value);
        self.redo.clear();
        self.last_coalesced = Some(Instant::now());
        self.last_tag = Some(tag.to_owned());
    }

    pub fn undo(&mut self, current: T) -> Option<T> {
        let prev = self.undo.pop_back()?;
        if self.redo.len() >= self.limit.max(1) {
            self.redo.pop_front();
        }
        self.redo.push_back(current);
        self.last_coalesced = None;
        self.last_tag = None;
        Some(prev)
    }

    pub fn redo(&mut self, current: T) -> Option<T> {
        let next = self.redo.pop_back()?;
        if self.undo.len() >= self.limit.max(1) {
            self.undo.pop_front();
        }
        self.undo.push_back(current);
        self.last_coalesced = None;
        self.last_tag = None;
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
        h.checkpoint_coalesced("typing", 1, w);
        h.checkpoint_coalesced("typing", 2, w);
        h.checkpoint_coalesced("typing", 3, w);

        assert_eq!(h.depths(), (1, 0));
        h.checkpoint_coalesced("typing", 4, Duration::from_nanos(0));
        assert_eq!(h.depths(), (2, 0));
    }

    #[test]
    fn coalescing_respects_the_tag() {
        let mut h = History::new(60);
        let w = Duration::from_secs(3600);
        h.checkpoint_coalesced("typing", 1, w);
        h.checkpoint_coalesced("drag", 2, w);
        h.checkpoint_coalesced("typing", 3, w);
        assert_eq!(
            h.depths(),
            (3, 0),
            "typing and drag in the same window are separate steps"
        );
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
    fn default_keeps_the_same_depth_as_the_other_histories() {
        let mut h: History<i32> = History::default();
        for v in 0..(DEFAULT_HISTORY_LIMIT as i32 + 10) {
            h.checkpoint(v);
        }
        assert_eq!(
            h.depths(),
            (DEFAULT_HISTORY_LIMIT, 0),
            "Default must not silently disable history"
        );
        assert_eq!(DEFAULT_HISTORY_LIMIT, 100);
    }

    #[test]
    fn zero_limit_disables_history() {
        let mut h: History<i32> = History::new(0);
        h.checkpoint(1);
        assert!(!h.can_undo());
        assert!(h.undo(2).is_none());
    }
}
