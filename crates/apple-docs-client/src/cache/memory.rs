use dashmap::DashMap;
use time::{Duration, OffsetDateTime};

use crate::types::CacheEntry;
use super::stats::CacheStats;

#[derive(Debug)]
pub struct MemoryCache<T> {
    entries: DashMap<String, CacheEntry<T>>,
    ttl: Duration,
    stats: CacheStats,
}

impl<T: Clone> MemoryCache<T> {
    pub fn new(ttl: Duration) -> Self {
        Self {
            entries: DashMap::new(),
            ttl,
            stats: CacheStats::new(),
        }
    }

    pub fn get(&self, key: &str) -> Option<T> {
        let result = self.entries.get(key).and_then(|entry| {
            if OffsetDateTime::now_utc() - entry.stored_at <= self.ttl {
                Some(entry.value.clone())
            } else {
                None
            }
        });

        if result.is_some() {
            self.stats.record_hit();
        } else {
            self.stats.record_miss();
        }

        result
    }

    /// Get value and track bytes served (for Vec<u8> caches)
    pub fn get_with_size(&self, key: &str, size_fn: impl FnOnce(&T) -> usize) -> Option<T> {
        let result = self.entries.get(key).and_then(|entry| {
            if OffsetDateTime::now_utc() - entry.stored_at <= self.ttl {
                let size = size_fn(&entry.value);
                self.stats.record_bytes(size as u64);
                Some(entry.value.clone())
            } else {
                None
            }
        });

        if result.is_some() {
            self.stats.record_hit();
        } else {
            self.stats.record_miss();
        }

        result
    }

    pub fn insert(&self, key: impl Into<String>, value: T) {
        let now = OffsetDateTime::now_utc();
        let entry = CacheEntry {
            value,
            stored_at: now,
            last_accessed: now,
        };
        self.entries.insert(key.into(), entry);
        self.stats.set_entry_count(self.entries.len());
    }

    pub fn clear(&self) {
        self.entries.clear();
        self.stats.set_entry_count(0);
    }

    /// Get a reference to the cache statistics
    pub fn stats(&self) -> &CacheStats {
        &self.stats
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

    #[test]
    fn tracks_cache_hits() {
        let cache = MemoryCache::new(Duration::hours(1));
        cache.insert("key1", "value1".to_string());
        cache.insert("key2", "value2".to_string());

        // First access - should hit
        assert_eq!(cache.get("key1"), Some("value1".to_string()));
        assert_eq!(cache.get("key2"), Some("value2".to_string()));

        let snapshot = cache.stats().snapshot();
        assert_eq!(snapshot.hits, 2, "Should record 2 cache hits");
        assert_eq!(snapshot.misses, 0, "Should have no misses");
    }

    #[test]
    fn tracks_cache_misses() {
        let cache = MemoryCache::<String>::new(Duration::hours(1));

        // Access non-existent keys
        assert_eq!(cache.get("nonexistent1"), None);
        assert_eq!(cache.get("nonexistent2"), None);

        let snapshot = cache.stats().snapshot();
        assert_eq!(snapshot.hits, 0, "Should have no hits");
        assert_eq!(snapshot.misses, 2, "Should record 2 cache misses");
    }

    #[test]
    fn tracks_mixed_hits_and_misses() {
        let cache = MemoryCache::new(Duration::hours(1));
        cache.insert("exists", 42);

        // Hit
        assert_eq!(cache.get("exists"), Some(42));
        // Miss
        assert_eq!(cache.get("not_exists"), None);
        // Hit again
        assert_eq!(cache.get("exists"), Some(42));

        let snapshot = cache.stats().snapshot();
        assert_eq!(snapshot.hits, 2, "Should record 2 hits");
        assert_eq!(snapshot.misses, 1, "Should record 1 miss");
        assert!((snapshot.hit_rate() - 66.666).abs() < 0.01, "Hit rate should be ~66.67%");
    }

    #[test]
    fn tracks_expired_entries_as_misses() {
        let cache = MemoryCache::new(Duration::milliseconds(100));
        cache.insert("key", 42);

        // First access - should hit
        assert_eq!(cache.get("key"), Some(42));

        // Wait for expiration
        std::thread::sleep(std::time::Duration::from_millis(150));

        // Access expired entry - should miss
        assert_eq!(cache.get("key"), None);

        let snapshot = cache.stats().snapshot();
        assert_eq!(snapshot.hits, 1, "Should have 1 hit");
        assert_eq!(snapshot.misses, 1, "Expired entry should count as miss");
    }

    #[test]
    fn tracks_bytes_served() {
        let cache = MemoryCache::new(Duration::hours(1));
        let data1 = vec![1u8; 1024]; // 1KB
        let data2 = vec![2u8; 2048]; // 2KB
        cache.insert("data1", data1.clone());
        cache.insert("data2", data2.clone());

        // Get with size tracking
        cache.get_with_size("data1", |v| v.len());
        cache.get_with_size("data2", |v| v.len());

        let snapshot = cache.stats().snapshot();
        assert_eq!(snapshot.bytes_served, 3072, "Should track 3KB served");
        assert_eq!(snapshot.hits, 2, "Should record 2 hits");
    }

    #[test]
    fn tracks_entry_count() {
        let cache = MemoryCache::new(Duration::hours(1));
        cache.insert("key1", "value1".to_string());
        cache.insert("key2", "value2".to_string());
        cache.insert("key3", "value3".to_string());

        let snapshot = cache.stats().snapshot();
        assert_eq!(snapshot.entry_count, 3, "Should track 3 entries");
    }

    #[test]
    fn clear_resets_entry_count() {
        let cache = MemoryCache::new(Duration::hours(1));
        cache.insert("key1", "value1".to_string());
        cache.insert("key2", "value2".to_string());

        cache.clear();

        let snapshot = cache.stats().snapshot();
        assert_eq!(snapshot.entry_count, 0, "Clear should reset entry count");
        // Note: hits/misses are preserved
    }

    #[test]
    fn calculates_hit_rate() {
        let cache = MemoryCache::new(Duration::hours(1));
        cache.insert("key1", 1);
        cache.insert("key2", 2);

        // 2 hits
        cache.get("key1");
        cache.get("key2");
        // 1 miss
        cache.get("key3");

        let snapshot = cache.stats().snapshot();
        assert!((snapshot.hit_rate() - 66.666).abs() < 0.01, "Hit rate should be ~66.67%");
    }
}
