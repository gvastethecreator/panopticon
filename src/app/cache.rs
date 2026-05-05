//! Simple bounded LRU cache for per-app icon and other ephemeral data.

use std::collections::{HashMap, VecDeque};

/// Capacity-bounded cache with LRU eviction.
pub(crate) struct BoundedCache<V> {
    capacity: usize,
    entries: HashMap<String, V>,
    access_order: VecDeque<String>,
}

impl<V> BoundedCache<V> {
    pub(crate) fn new(capacity: usize) -> Self {
        Self {
            capacity,
            entries: HashMap::with_capacity(capacity),
            access_order: VecDeque::with_capacity(capacity),
        }
    }

    pub(crate) fn insert(&mut self, key: &str, value: V) {
        if self.capacity == 0 {
            return;
        }

        self.entries.insert(key.to_owned(), value);
        self.touch(key);

        while self.entries.len() > self.capacity {
            let Some(stale_key) = self.access_order.pop_front() else {
                break;
            };
            self.entries.remove(&stale_key);
        }
    }

    pub(crate) fn get_cloned(&mut self, key: &str) -> Option<V>
    where
        V: Clone,
    {
        let value = self.entries.get(key).cloned();
        if value.is_some() {
            self.touch(key);
        }
        value
    }

    pub(crate) fn remove(&mut self, key: &str) -> Option<V> {
        self.access_order.retain(|existing| existing != key);
        self.entries.remove(key)
    }

    #[cfg(test)]
    pub(crate) fn clear(&mut self) {
        self.entries.clear();
        self.access_order.clear();
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }

    fn touch(&mut self, key: &str) {
        self.access_order.retain(|existing| existing != key);
        self.access_order.push_back(key.to_owned());
    }
}

#[cfg(test)]
mod tests {
    use super::BoundedCache;

    #[test]
    fn bounded_cache_evicts_oldest_entry_when_capacity_is_exceeded() {
        let mut cache = BoundedCache::new(2);
        cache.insert("alpha", 1);
        cache.insert("bravo", 2);
        cache.insert("charlie", 3);

        assert_eq!(cache.get_cloned("alpha"), None);
        assert_eq!(cache.get_cloned("bravo"), Some(2));
        assert_eq!(cache.get_cloned("charlie"), Some(3));
    }

    #[test]
    fn bounded_cache_refreshes_recently_accessed_entries() {
        let mut cache = BoundedCache::new(2);
        cache.insert("alpha", 1);
        cache.insert("bravo", 2);

        assert_eq!(cache.get_cloned("alpha"), Some(1));

        cache.insert("charlie", 3);

        assert_eq!(cache.get_cloned("alpha"), Some(1));
        assert_eq!(cache.get_cloned("bravo"), None);
        assert_eq!(cache.get_cloned("charlie"), Some(3));
    }
}
