use dashmap::DashMap;
use time::{Duration, OffsetDateTime};

use crate::types::CacheEntry;

#[derive(Debug)]
pub struct MemoryCache<T> {
    entries: DashMap<String, CacheEntry<T>>,
    ttl: Duration,
}

impl<T: Clone> MemoryCache<T> {
    pub fn new(ttl: Duration) -> Self {
        Self {
            entries: DashMap::new(),
            ttl,
        }
    }

    pub fn get(&self, key: &str) -> Option<T> {
        self.entries.get(key).and_then(|entry| {
            if OffsetDateTime::now_utc() - entry.stored_at <= self.ttl {
                Some(entry.value.clone())
            } else {
                None
            }
        })
    }

    pub fn insert(&self, key: impl Into<String>, value: T) {
        let entry = CacheEntry {
            value,
            stored_at: OffsetDateTime::now_utc(),
        };
        self.entries.insert(key.into(), entry);
    }

    pub fn clear(&self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn respects_ttl() {
        let cache = MemoryCache::new(Duration::seconds(1));
        cache.insert("key", 42);
        assert_eq!(cache.get("key"), Some(42));

        std::thread::sleep(std::time::Duration::from_secs(2));
        assert!(cache.get("key").is_none());
    }
}
