use serde::Serialize;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

/// Cache statistics tracker using atomic counters for thread-safe operation.
///
/// Tracks cache performance metrics including hit/miss rates and data volume.
#[derive(Debug, Default)]
pub struct CacheStats {
    /// Number of successful cache lookups
    pub hits: AtomicUsize,
    /// Number of failed cache lookups (cache miss)
    pub misses: AtomicUsize,
    /// Total bytes served from cache
    pub bytes_served: AtomicU64,
    /// Current number of entries in cache
    pub entry_count: AtomicUsize,
    /// Number of cache evictions performed
    pub evictions: AtomicUsize,
}

impl CacheStats {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a cache hit
    #[inline]
    pub fn record_hit(&self) {
        self.hits.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a cache miss
    #[inline]
    pub fn record_miss(&self) {
        self.misses.fetch_add(1, Ordering::Relaxed);
    }

    /// Record bytes served from cache
    #[inline]
    pub fn record_bytes(&self, bytes: u64) {
        self.bytes_served.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Update the current entry count
    #[inline]
    pub fn set_entry_count(&self, count: usize) {
        self.entry_count.store(count, Ordering::Relaxed);
    }

    /// Increment entry count
    #[inline]
    pub fn increment_entries(&self) {
        self.entry_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrement entry count
    #[inline]
    pub fn decrement_entries(&self, count: usize) {
        self.entry_count.fetch_sub(count, Ordering::Relaxed);
    }

    /// Record a cache eviction
    #[inline]
    pub fn record_eviction(&self, count: usize) {
        self.evictions.fetch_add(count, Ordering::Relaxed);
    }

    /// Get a snapshot of current statistics
    pub fn snapshot(&self) -> CacheStatsSnapshot {
        CacheStatsSnapshot {
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            bytes_served: self.bytes_served.load(Ordering::Relaxed),
            entry_count: self.entry_count.load(Ordering::Relaxed),
            evictions: self.evictions.load(Ordering::Relaxed),
        }
    }

    /// Reset all statistics to zero
    pub fn reset(&self) {
        self.hits.store(0, Ordering::Relaxed);
        self.misses.store(0, Ordering::Relaxed);
        self.bytes_served.store(0, Ordering::Relaxed);
        self.entry_count.store(0, Ordering::Relaxed);
        self.evictions.store(0, Ordering::Relaxed);
    }
}

/// Immutable snapshot of cache statistics at a point in time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct CacheStatsSnapshot {
    pub hits: usize,
    pub misses: usize,
    pub bytes_served: u64,
    pub entry_count: usize,
    pub evictions: usize,
}

/// Combined statistics from both memory and disk caches
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CombinedCacheStats {
    pub memory: CacheStatsSnapshot,
    pub disk: CacheStatsSnapshot,
}

impl CombinedCacheStats {
    /// Get total statistics across both memory and disk caches
    pub fn total(&self) -> CacheStatsSnapshot {
        CacheStatsSnapshot {
            hits: self.memory.hits + self.disk.hits,
            misses: self.memory.misses + self.disk.misses,
            bytes_served: self.memory.bytes_served + self.disk.bytes_served,
            entry_count: self.memory.entry_count + self.disk.entry_count,
            evictions: self.memory.evictions + self.disk.evictions,
        }
    }
}

impl CacheStatsSnapshot {
    /// Calculate the cache hit rate as a percentage (0.0 - 100.0)
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            return 0.0;
        }
        (self.hits as f64 / total as f64) * 100.0
    }

    /// Total number of cache requests
    pub fn total_requests(&self) -> usize {
        self.hits + self.misses
    }

    /// Average bytes per hit
    pub fn avg_bytes_per_hit(&self) -> f64 {
        if self.hits == 0 {
            return 0.0;
        }
        self.bytes_served as f64 / self.hits as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_hits_and_misses() {
        let stats = CacheStats::new();
        stats.record_hit();
        stats.record_hit();
        stats.record_miss();

        let snapshot = stats.snapshot();
        assert_eq!(snapshot.hits, 2);
        assert_eq!(snapshot.misses, 1);
        // Use approximate comparison for floating point
        assert!((snapshot.hit_rate() - 66.666).abs() < 0.01);
    }

    #[test]
    fn records_bytes_served() {
        let stats = CacheStats::new();
        stats.record_bytes(1024);
        stats.record_bytes(2048);

        let snapshot = stats.snapshot();
        assert_eq!(snapshot.bytes_served, 3072);
    }

    #[test]
    fn tracks_entry_count() {
        let stats = CacheStats::new();
        stats.increment_entries();
        stats.increment_entries();
        stats.set_entry_count(5);

        let snapshot = stats.snapshot();
        assert_eq!(snapshot.entry_count, 5);
    }

    #[test]
    fn resets_all_stats() {
        let stats = CacheStats::new();
        stats.record_hit();
        stats.record_miss();
        stats.record_bytes(1024);
        stats.increment_entries();

        stats.reset();

        let snapshot = stats.snapshot();
        assert_eq!(snapshot.hits, 0);
        assert_eq!(snapshot.misses, 0);
        assert_eq!(snapshot.bytes_served, 0);
        assert_eq!(snapshot.entry_count, 0);
        assert_eq!(snapshot.evictions, 0);
    }

    #[test]
    fn calculates_derived_metrics() {
        let snapshot = CacheStatsSnapshot {
            hits: 80,
            misses: 20,
            bytes_served: 8000,
            entry_count: 50,
            evictions: 0,
        };

        assert_eq!(snapshot.hit_rate(), 80.0);
        assert_eq!(snapshot.total_requests(), 100);
        assert_eq!(snapshot.avg_bytes_per_hit(), 100.0);
    }

    #[test]
    fn records_evictions() {
        let stats = CacheStats::new();
        stats.record_eviction(3);
        stats.record_eviction(2);

        let snapshot = stats.snapshot();
        assert_eq!(snapshot.evictions, 5);
    }

    #[test]
    fn decrements_entry_count() {
        let stats = CacheStats::new();
        stats.set_entry_count(10);
        stats.decrement_entries(3);

        let snapshot = stats.snapshot();
        assert_eq!(snapshot.entry_count, 7);
    }

    #[test]
    fn hit_rate_with_no_requests() {
        let snapshot = CacheStatsSnapshot {
            hits: 0,
            misses: 0,
            bytes_served: 0,
            entry_count: 0,
            evictions: 0,
        };

        assert_eq!(snapshot.hit_rate(), 0.0);
        assert_eq!(snapshot.total_requests(), 0);
    }

    #[test]
    fn hit_rate_with_all_hits() {
        let snapshot = CacheStatsSnapshot {
            hits: 100,
            misses: 0,
            bytes_served: 10000,
            entry_count: 50,
            evictions: 0,
        };

        assert_eq!(snapshot.hit_rate(), 100.0);
        assert_eq!(snapshot.total_requests(), 100);
    }

    #[test]
    fn hit_rate_with_all_misses() {
        let snapshot = CacheStatsSnapshot {
            hits: 0,
            misses: 100,
            bytes_served: 0,
            entry_count: 50,
            evictions: 0,
        };

        assert_eq!(snapshot.hit_rate(), 0.0);
        assert_eq!(snapshot.total_requests(), 100);
    }

    #[test]
    fn avg_bytes_with_no_hits() {
        let snapshot = CacheStatsSnapshot {
            hits: 0,
            misses: 10,
            bytes_served: 0,
            entry_count: 0,
            evictions: 0,
        };

        assert_eq!(snapshot.avg_bytes_per_hit(), 0.0);
    }

    #[test]
    fn combined_cache_stats_total() {
        let memory = CacheStatsSnapshot {
            hits: 50,
            misses: 10,
            bytes_served: 5000,
            entry_count: 25,
            evictions: 2,
        };

        let disk = CacheStatsSnapshot {
            hits: 30,
            misses: 20,
            bytes_served: 3000,
            entry_count: 15,
            evictions: 1,
        };

        let combined = CombinedCacheStats { memory, disk };
        let total = combined.total();

        assert_eq!(total.hits, 80, "Total hits should be sum of memory and disk");
        assert_eq!(total.misses, 30, "Total misses should be sum of memory and disk");
        assert_eq!(total.bytes_served, 8000, "Total bytes should be sum of memory and disk");
        assert_eq!(total.entry_count, 40, "Total entries should be sum of memory and disk");
        assert_eq!(total.evictions, 3, "Total evictions should be sum of memory and disk");
    }

    #[test]
    fn combined_cache_stats_hit_rate() {
        let memory = CacheStatsSnapshot {
            hits: 80,
            misses: 20,
            bytes_served: 8000,
            entry_count: 40,
            evictions: 0,
        };

        let disk = CacheStatsSnapshot {
            hits: 60,
            misses: 40,
            bytes_served: 6000,
            entry_count: 30,
            evictions: 0,
        };

        let combined = CombinedCacheStats { memory, disk };
        let total = combined.total();

        // Total: 140 hits, 60 misses = 140/200 = 70%
        assert_eq!(total.hit_rate(), 70.0);
        assert_eq!(total.total_requests(), 200);
    }

    #[test]
    fn combined_cache_stats_avg_bytes() {
        let memory = CacheStatsSnapshot {
            hits: 40,
            misses: 10,
            bytes_served: 4000,
            entry_count: 20,
            evictions: 0,
        };

        let disk = CacheStatsSnapshot {
            hits: 60,
            misses: 20,
            bytes_served: 12000,
            entry_count: 30,
            evictions: 0,
        };

        let combined = CombinedCacheStats { memory, disk };
        let total = combined.total();

        // Total: 100 hits, 16000 bytes = 160 bytes per hit
        assert_eq!(total.avg_bytes_per_hit(), 160.0);
    }

    #[test]
    fn combined_cache_stats_empty() {
        let memory = CacheStatsSnapshot {
            hits: 0,
            misses: 0,
            bytes_served: 0,
            entry_count: 0,
            evictions: 0,
        };

        let disk = CacheStatsSnapshot {
            hits: 0,
            misses: 0,
            bytes_served: 0,
            entry_count: 0,
            evictions: 0,
        };

        let combined = CombinedCacheStats { memory, disk };
        let total = combined.total();

        assert_eq!(total.hits, 0);
        assert_eq!(total.misses, 0);
        assert_eq!(total.bytes_served, 0);
        assert_eq!(total.entry_count, 0);
        assert_eq!(total.evictions, 0);
        assert_eq!(total.hit_rate(), 0.0);
        assert_eq!(total.avg_bytes_per_hit(), 0.0);
    }

    #[test]
    fn thread_safety_concurrent_updates() {
        use std::sync::Arc;
        use std::thread;

        let stats = Arc::new(CacheStats::new());
        let mut handles = vec![];

        // Spawn 10 threads that each record 100 hits
        for _ in 0..10 {
            let stats_clone = Arc::clone(&stats);
            let handle = thread::spawn(move || {
                for _ in 0..100 {
                    stats_clone.record_hit();
                }
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        let snapshot = stats.snapshot();
        assert_eq!(snapshot.hits, 1000, "All hits should be recorded atomically");
    }

    #[test]
    fn thread_safety_concurrent_mixed_operations() {
        use std::sync::Arc;
        use std::thread;

        let stats = Arc::new(CacheStats::new());
        let mut handles = vec![];

        // Spawn threads doing different operations
        for i in 0..5 {
            let stats_clone = Arc::clone(&stats);
            let handle = thread::spawn(move || {
                for _ in 0..50 {
                    if i % 2 == 0 {
                        stats_clone.record_hit();
                    } else {
                        stats_clone.record_miss();
                    }
                    stats_clone.record_bytes(100);
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let snapshot = stats.snapshot();
        assert_eq!(snapshot.hits, 150, "Hits should be correct");
        assert_eq!(snapshot.misses, 100, "Misses should be correct");
        assert_eq!(snapshot.bytes_served, 25000, "Bytes should be correct");
    }
}
