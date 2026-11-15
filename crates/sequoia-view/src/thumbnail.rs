use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::hash::Hash;
use std::rc::Rc;

use sequoia_core::{ImageSource, Pixels};

/// Thumbnails are decoded RGBA pixels, so an unbounded cache grows with every
/// distinct key and size a view has ever shown. Both limits are soft: the entry
/// just inserted stays even when it alone exceeds them.
const DEFAULT_MAX_BYTES: usize = 64 << 20;
const DEFAULT_MAX_ENTRIES: usize = 512;

fn entry_bytes(width: u32, height: u32) -> usize {
    (width as usize)
        .saturating_mul(height as usize)
        .saturating_mul(4)
}

#[derive(Default)]
struct Inner {
    map: HashMap<(String, u32, u32), ImageSource>,
    order: VecDeque<(String, u32, u32)>,
    bytes: usize,
    max_bytes: usize,
    max_entries: usize,
}

#[derive(Clone)]
pub struct ThumbnailCache {
    inner: Rc<RefCell<Inner>>,
}

impl Default for ThumbnailCache {
    fn default() -> Self {
        Self::with_limits(DEFAULT_MAX_BYTES, DEFAULT_MAX_ENTRIES)
    }
}

impl ThumbnailCache {
    pub fn new() -> Self {
        Self::default()
    }

    fn with_limits(max_bytes: usize, max_entries: usize) -> Self {
        Self {
            inner: Rc::new(RefCell::new(Inner {
                max_bytes: max_bytes.max(1),
                max_entries: max_entries.max(1),
                ..Inner::default()
            })),
        }
    }

    pub fn get_or_insert<K>(
        &self,
        key: K,
        width: u32,
        height: u32,
        build: impl FnOnce(Pixels) -> Pixels,
    ) -> ImageSource
    where
        K: Hash + Eq + ToString,
    {
        let w = width.max(1);
        let h = height.max(1);
        let k = (key.to_string(), w, h);
        let mut inner = self.inner.borrow_mut();

        if let Some(src) = inner.map.get(&k) {
            let src = src.clone();
            if let Some(pos) = inner.order.iter().position(|entry| entry == &k) {
                inner.order.remove(pos);
            }
            inner.order.push_back(k);
            return src;
        }

        let buf = build(Pixels::new(w, h));
        let src = buf.into_source();
        inner.bytes = inner.bytes.saturating_add(entry_bytes(w, h));
        inner.map.insert(k.clone(), src.clone());
        inner.order.push_back(k);

        while inner.map.len() > inner.max_entries
            || (inner.bytes > inner.max_bytes && inner.map.len() > 1)
        {
            let Some(oldest) = inner.order.pop_front() else {
                break;
            };
            if inner.map.remove(&oldest).is_some() {
                inner.bytes = inner.bytes.saturating_sub(entry_bytes(oldest.1, oldest.2));
            }
        }

        src
    }

    pub fn len(&self) -> usize {
        self.inner.borrow().map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.borrow().map.is_empty()
    }

    pub fn clear(&self) {
        let mut inner = self.inner.borrow_mut();
        inner.map.clear();
        inner.order.clear();
        inner.bytes = 0;
    }
}

pub fn shared_cache() -> ThumbnailCache {
    thread_local! {
        static SHARED: ThumbnailCache = ThumbnailCache::new();
    }
    SHARED.with(|c| c.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sequoia_core::Color;

    const SIDE: u32 = 64;

    fn key(name: &str) -> (String, u32, u32) {
        (name.to_string(), SIDE, SIDE)
    }

    #[test]
    fn build_runs_once_per_key_and_size() {
        let cache = ThumbnailCache::new();
        let calls = Rc::new(std::cell::Cell::new(0));
        let build = {
            let calls = calls.clone();
            move |mut px: Pixels| {
                calls.set(calls.get() + 1);
                px.set(0, 0, Color::WHITE);
                px
            }
        };
        let a = cache.get_or_insert("n", 4, 4, build);
        let b = cache.get_or_insert("n", 4, 4, |px| px);
        assert_eq!(calls.get(), 1, "drugi raz to hit, nie build");
        assert_eq!(a.cache_key(), b.cache_key());
        let _ = cache.get_or_insert("n", 8, 8, |px| px);
        assert_eq!(cache.len(), 2, "inny rozmiar = osobny wpis");
    }

    #[test]
    fn degenerate_size_clamps_to_one() {
        let cache = ThumbnailCache::new();
        let src = cache.get_or_insert("z", 0, 0, |px| px);
        let _ = src;
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn byte_limit_evicts_the_least_recently_used_entry() {
        let cache = ThumbnailCache::with_limits(2 * entry_bytes(SIDE, SIDE), 10);
        let _ = cache.get_or_insert("a", SIDE, SIDE, |px| px);
        let _ = cache.get_or_insert("b", SIDE, SIDE, |px| px);
        // Touch "a" so "b" becomes the oldest entry.
        let _ = cache.get_or_insert("a", SIDE, SIDE, |px| px);
        let _ = cache.get_or_insert("c", SIDE, SIDE, |px| px);

        let inner = cache.inner.borrow();
        assert_eq!(inner.map.len(), 2);
        assert!(inner.map.contains_key(&key("a")), "recent entry stays");
        assert!(inner.map.contains_key(&key("c")), "new entry stays");
        assert!(!inner.map.contains_key(&key("b")), "oldest entry evicted");
        assert_eq!(inner.bytes, 2 * entry_bytes(SIDE, SIDE));
    }

    #[test]
    fn entry_limit_evicts_oldest_entries() {
        let cache = ThumbnailCache::with_limits(usize::MAX, 2);
        let _ = cache.get_or_insert("a", SIDE, SIDE, |px| px);
        let _ = cache.get_or_insert("b", SIDE, SIDE, |px| px);
        let _ = cache.get_or_insert("c", SIDE, SIDE, |px| px);

        let inner = cache.inner.borrow();
        assert_eq!(inner.map.len(), 2);
        assert!(!inner.map.contains_key(&key("a")));
        assert!(inner.map.contains_key(&key("b")));
        assert!(inner.map.contains_key(&key("c")));
    }

    #[test]
    fn oversized_entry_is_kept_and_replaces_the_cache() {
        let cache = ThumbnailCache::with_limits(1, 10);
        let _ = cache.get_or_insert("big", SIDE, SIDE, |px| px);

        let inner = cache.inner.borrow();
        assert_eq!(inner.map.len(), 1, "the newest entry is not evicted");
        assert!(inner.map.contains_key(&key("big")));
    }

    #[test]
    fn clear_resets_bytes_and_order() {
        let cache = ThumbnailCache::with_limits(usize::MAX, 10);
        let _ = cache.get_or_insert("a", SIDE, SIDE, |px| px);
        cache.clear();

        let inner = cache.inner.borrow();
        assert!(inner.map.is_empty());
        assert!(inner.order.is_empty());
        assert_eq!(inner.bytes, 0);
    }
}
