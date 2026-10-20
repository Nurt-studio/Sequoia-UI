use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::Hash;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

use sequoia_core::{ImageSource, Pixels};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Default)]
pub struct ThumbnailCache {
    inner: Rc<RefCell<HashMap<(String, u32, u32), ImageSource>>>,
}

impl ThumbnailCache {
    pub fn new() -> Self {
        Self::default()
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
        let mut map = self.inner.borrow_mut();
        if let Some(src) = map.get(&k) {
            return src.clone();
        }
        let buf = build(Pixels::new(w, h));
        let src = buf.into_source(NEXT_ID.fetch_add(1, Ordering::Relaxed), 0);
        map.insert(k, src.clone());
        src
    }

    pub fn len(&self) -> usize {
        self.inner.borrow().len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.borrow().is_empty()
    }

    pub fn clear(&self) {
        self.inner.borrow_mut().clear();
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
}
