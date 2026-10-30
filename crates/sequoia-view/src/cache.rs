//! CPU-side layer cache.
//!
//! [`LayerCache`] stores `RenderList` snapshots for static subtrees. The
//! runtime fills it for elements that opt in through
//! [`View::cache_layer`](crate::View::cache_layer) (for example [`Cached`]).
//!
//! GPU resources are owned by the renderer and are not mirrored here: image
//! textures, embedded surfaces, group layers, and the offscreen frame target
//! all have renderer-specific keys and lifetimes.
//!
//! [`Cached`]: crate::Cached

use std::collections::HashMap;
use std::sync::Arc;

use sequoia_core::{Rect, RenderList, RoundedClip, Transform, WindowId};

use crate::paint_region::CullCtx;

/// Identifies a cached layer by element and content revision.
///
/// `Runtime` bumps `revision` on the element and all ancestors whenever a
/// repaint-relevant dirty flag or direct damage mark reaches the subtree.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct LayerKey {
    pub element_id: u64,
    pub revision: u64,
}

/// Absolute paint context that a cached layer was captured in.
///
/// Cached item rects/clips/transforms are absolute, so a hit is only valid
/// when the parent offset, transform, clip, opacity, DPI, text system/atlas,
/// window, interaction targets inside the subtree, and culling configuration
/// are unchanged.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct LayerStamp {
    pub rect: Rect,
    pub scale: f32,
    pub transform: Transform,
    pub clip: Option<Rect>,
    pub rounded_clip: Option<RoundedClip>,
    pub opacity: f32,
    pub text_atlas: u64,
    pub text_epoch: u64,
    pub window: WindowId,
    pub hovered: Option<u64>,
    pub pressed: Option<u64>,
    pub focused: Option<u64>,
    pub captured: Option<u64>,
    pub cull: CullCtx,
}

struct LayerEntry {
    list: Arc<RenderList>,
    stamp: Option<LayerStamp>,
}

/// CPU-side cache of rendered subtrees.
#[derive(Default)]
pub struct LayerCache {
    map: HashMap<LayerKey, LayerEntry>,
    capacity: usize,
}

impl LayerCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            map: HashMap::new(),
            capacity: capacity.max(1),
        }
    }

    pub fn get(&self, key: LayerKey) -> Option<Arc<RenderList>> {
        self.map.get(&key).map(|entry| Arc::clone(&entry.list))
    }

    pub(crate) fn get_stamped(&self, key: LayerKey, stamp: &LayerStamp) -> Option<Arc<RenderList>> {
        self.map
            .get(&key)
            .filter(|entry| entry.stamp.as_ref() == Some(stamp))
            .map(|entry| Arc::clone(&entry.list))
    }

    pub fn insert(&mut self, key: LayerKey, list: RenderList) -> Arc<RenderList> {
        let list = Arc::new(list);
        self.store(key, Arc::clone(&list), None);
        list
    }

    pub(crate) fn insert_stamped(
        &mut self,
        key: LayerKey,
        list: RenderList,
        stamp: LayerStamp,
    ) -> Arc<RenderList> {
        let list = Arc::new(list);
        self.store(key, Arc::clone(&list), Some(stamp));
        list
    }

    fn store(&mut self, key: LayerKey, list: Arc<RenderList>, stamp: Option<LayerStamp>) {
        if !self.map.contains_key(&key)
            && self.map.len() >= self.capacity
            && let Some(victim) = self.map.keys().next().copied()
        {
            self.map.remove(&victim);
        }
        self.map.insert(key, LayerEntry { list, stamp });
    }

    /// Drops every cached revision of `element_id`.
    pub fn invalidate_element(&mut self, element_id: u64) {
        self.map.retain(|key, _| key.element_id != element_id);
    }

    pub fn invalidate_all(&mut self) {
        self.map.clear();
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sequoia_core::{Color, Rect};

    fn layer(element_id: u64, revision: u64) -> LayerKey {
        LayerKey {
            element_id,
            revision,
        }
    }

    #[test]
    fn layer_cache_hit_returns_same_list() {
        let mut cache = LayerCache::new(4);
        let mut list = RenderList::new();
        list.push_quad(Rect::from_xywh(0.0, 0.0, 10.0, 10.0), Color::RED, 0.0);
        let first = cache.insert(layer(1, 0), list);
        let second = cache.get(layer(1, 0)).unwrap();
        assert!(Arc::ptr_eq(&first, &second));
    }

    #[test]
    fn layer_cache_new_revision_is_miss() {
        let mut cache = LayerCache::new(4);
        let _ = cache.insert(layer(1, 0), RenderList::new());
        assert!(cache.get(layer(1, 0)).is_some());
        assert!(cache.get(layer(1, 1)).is_none());
    }

    #[test]
    fn layer_cache_invalidate_element() {
        let mut cache = LayerCache::new(8);
        let _ = cache.insert(layer(1, 0), RenderList::new());
        let _ = cache.insert(layer(2, 0), RenderList::new());
        cache.invalidate_element(1);
        assert!(cache.get(layer(1, 0)).is_none());
        assert!(cache.get(layer(2, 0)).is_some(), "other element untouched");
    }

    #[test]
    fn layer_cache_evicts_when_full() {
        let mut cache = LayerCache::new(2);
        let _ = cache.insert(layer(1, 0), RenderList::new());
        let _ = cache.insert(layer(2, 0), RenderList::new());
        let _ = cache.insert(layer(3, 0), RenderList::new());
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn layer_cache_replacing_key_does_not_evict_another() {
        let mut cache = LayerCache::new(2);
        let _ = cache.insert(layer(1, 0), RenderList::new());
        let _ = cache.insert(layer(2, 0), RenderList::new());
        let _ = cache.insert(layer(1, 0), RenderList::new());
        assert_eq!(cache.len(), 2);
        assert!(cache.get(layer(2, 0)).is_some());
    }

    fn stamp(rect: Rect) -> LayerStamp {
        LayerStamp {
            rect,
            scale: 1.0,
            transform: Transform::IDENTITY,
            clip: Some(rect),
            rounded_clip: None,
            opacity: 1.0,
            text_atlas: 0,
            text_epoch: 0,
            window: WindowId::PRIMARY,
            hovered: None,
            pressed: None,
            focused: None,
            captured: None,
            cull: CullCtx::ENABLED,
        }
    }

    #[test]
    fn layer_cache_stamp_must_match() {
        let mut cache = LayerCache::new(4);
        let rect = Rect::from_xywh(0.0, 0.0, 10.0, 10.0);
        let _ = cache.insert_stamped(layer(1, 0), RenderList::new(), stamp(rect));
        assert!(
            cache.get_stamped(layer(1, 0), &stamp(rect)).is_some(),
            "same context hits"
        );
        assert!(
            cache
                .get_stamped(layer(1, 0), &stamp(Rect::from_xywh(1.0, 0.0, 10.0, 10.0)))
                .is_none(),
            "moved layer misses"
        );
        let mut hovered = stamp(rect);
        hovered.hovered = Some(7);
        assert!(
            cache.get_stamped(layer(1, 0), &hovered).is_none(),
            "hover target change misses"
        );
    }
}
