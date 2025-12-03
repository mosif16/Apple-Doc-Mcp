use std::{
    fs::create_dir_all,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context, Result};
use serde::{de::DeserializeOwned, Serialize};
use tokio::{fs, task};
use tracing::debug;

use crate::types::CacheEntry;
use super::stats::CacheStats;
use time::OffsetDateTime;

/// Default maximum cache size: 500MB
const DEFAULT_MAX_SIZE_BYTES: u64 = 500 * 1024 * 1024;

#[derive(Debug)]
pub struct DiskCache {
    root: PathBuf,
    stats: CacheStats,
    max_size_bytes: u64,
}

impl DiskCache {
    pub fn new<P: Into<PathBuf>>(root: P) -> Self {
        Self::with_max_size(root, DEFAULT_MAX_SIZE_BYTES)
    }

    pub fn with_max_size<P: Into<PathBuf>>(root: P, max_size_bytes: u64) -> Self {
        Self {
            root: root.into(),
            stats: CacheStats::new(),
            max_size_bytes,
        }
    }

    pub async fn load<T>(&self, file_name: &str) -> Result<Option<CacheEntry<T>>>
    where
        T: DeserializeOwned + Send + 'static,
    {
        let path = self.root.join(file_name);
        if !path.exists() {
            self.stats.record_miss();
            return Ok(None);
        }

        let data = fs::read(path.clone())
            .await
            .with_context(|| format!("failed to read cache file {path:?}"))?;

        let bytes_read = data.len() as u64;

        let entry =
            task::spawn_blocking(
                move || match serde_json::from_slice::<CacheEntry<T>>(&data) {
                    Ok(entry) => Ok(entry),
                    Err(primary_err) => serde_json::from_slice::<T>(&data)
                        .map(|value| CacheEntry {
                            value,
                            stored_at: OffsetDateTime::UNIX_EPOCH,
                            last_accessed: OffsetDateTime::now_utc(),
                        })
                        .map_err(|legacy_err| {
                            anyhow!(
                                "failed to deserialize cache file {:?}: {}; legacy parse error: {}",
                                path,
                                primary_err,
                                legacy_err
                            )
                        }),
                },
            )
            .await??;

        self.stats.record_hit();
        self.stats.record_bytes(bytes_read);

        Ok(Some(entry))
    }

    pub async fn store<T>(&self, file_name: &str, value: T) -> Result<()>
    where
        T: Serialize + Send + 'static,
    {
        let path = self.root.join(file_name);
        let parent = path.parent().map(Path::to_path_buf);
        if let Some(parent) = parent {
            create_dir_all(&parent)
                .with_context(|| format!("failed to create cache dir {parent:?}"))?;
        }

        let now = time::OffsetDateTime::now_utc();
        let entry = CacheEntry {
            value,
            stored_at: now,
            last_accessed: now,
        };

        let payload = task::spawn_blocking(move || serde_json::to_vec(&entry)).await??;
        fs::write(path.clone(), payload)
            .await
            .with_context(|| format!("failed to write cache file {path:?}"))?;

        self.stats.increment_entries();
        debug!(target: "docs_mcp_cache", file = ?path, "wrote cache entry");

        // Evict old entries if cache exceeds size limit
        self.evict_if_needed().await?;

        Ok(())
    }

    /// Get a reference to the cache statistics
    pub fn stats(&self) -> &CacheStats {
        &self.stats
    }

    /// Evict least recently accessed entries if cache exceeds size limit
    /// Uses file modification time (mtime) as a proxy for last access time
    async fn evict_if_needed(&self) -> Result<()> {
        use std::collections::BTreeMap;
        use std::time::SystemTime;

        // Calculate current cache size and collect entries with their metadata
        // BTreeMap keeps entries sorted by modification time (oldest first)
        let mut entries: BTreeMap<SystemTime, (String, u64)> = BTreeMap::new();
        let mut total_size: u64 = 0;

        let mut read_dir = fs::read_dir(&self.root).await?;
        while let Some(entry) = read_dir.next_entry().await? {
            let path = entry.path();
            if path.is_file() {
                if let Ok(metadata) = fs::metadata(&path).await {
                    let file_size = metadata.len();
                    total_size += file_size;

                    let file_name = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_string();

                    // Use file modification time as proxy for last access
                    let modified_time = metadata
                        .modified()
                        .unwrap_or(SystemTime::UNIX_EPOCH);

                    // Handle potential collisions by finding an available slot
                    let mut key = modified_time;
                    let mut nanos_offset = 1;
                    while entries.contains_key(&key) {
                        key = modified_time + std::time::Duration::from_nanos(nanos_offset);
                        nanos_offset += 1;
                    }

                    entries.insert(key, (file_name, file_size));
                }
            }
        }

        // If under limit, no eviction needed
        if total_size <= self.max_size_bytes {
            return Ok(());
        }

        // Evict oldest entries (by modification time) until under limit
        let mut evicted_count = 0;
        for (_, (file_name, file_size)) in entries.iter() {
            if total_size <= self.max_size_bytes {
                break;
            }

            let file_path = self.root.join(file_name);
            if let Ok(()) = fs::remove_file(&file_path).await {
                total_size -= file_size;
                evicted_count += 1;
                debug!(
                    target: "docs_mcp_cache",
                    file = ?file_path,
                    "evicted cache entry"
                );
            }
        }

        if evicted_count > 0 {
            self.stats.record_eviction(evicted_count);
            self.stats.decrement_entries(evicted_count);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    #[tokio::test]
    async fn round_trip_persists_entry() {
        let dir = tempdir().expect("tempdir");
        let cache = DiskCache::new(dir.path());

        cache
            .store("example.json", json!({"hello": "world"}))
            .await
            .unwrap();
        let entry: Option<CacheEntry<serde_json::Value>> =
            cache.load("example.json").await.unwrap();
        let entry = entry.expect("expected cache entry");
        assert_eq!(entry.value["hello"], "world");
    }

    #[tokio::test]
    async fn tracks_cache_hits() {
        let dir = tempdir().expect("tempdir");
        let cache = DiskCache::new(dir.path());

        cache.store("file1.json", json!({"data": 1})).await.unwrap();
        cache.store("file2.json", json!({"data": 2})).await.unwrap();

        // Load both files
        let _: Option<CacheEntry<serde_json::Value>> = cache.load("file1.json").await.unwrap();
        let _: Option<CacheEntry<serde_json::Value>> = cache.load("file2.json").await.unwrap();

        let snapshot = cache.stats().snapshot();
        assert_eq!(snapshot.hits, 2, "Should record 2 cache hits");
        assert_eq!(snapshot.misses, 0, "Should have no misses");
    }

    #[tokio::test]
    async fn tracks_cache_misses() {
        let dir = tempdir().expect("tempdir");
        let cache = DiskCache::new(dir.path());

        // Try to load non-existent files
        let result1: Option<CacheEntry<serde_json::Value>> =
            cache.load("nonexistent1.json").await.unwrap();
        let result2: Option<CacheEntry<serde_json::Value>> =
            cache.load("nonexistent2.json").await.unwrap();

        assert!(result1.is_none());
        assert!(result2.is_none());

        let snapshot = cache.stats().snapshot();
        assert_eq!(snapshot.hits, 0, "Should have no hits");
        assert_eq!(snapshot.misses, 2, "Should record 2 cache misses");
    }

    #[tokio::test]
    async fn tracks_bytes_served() {
        let dir = tempdir().expect("tempdir");
        let cache = DiskCache::new(dir.path());

        // Store a file
        let large_data = json!({"data": "x".repeat(1000)});
        cache.store("large.json", large_data).await.unwrap();

        // Load it and check bytes served
        let _: Option<CacheEntry<serde_json::Value>> = cache.load("large.json").await.unwrap();

        let snapshot = cache.stats().snapshot();
        assert!(snapshot.bytes_served > 0, "Should track bytes served");
        assert_eq!(snapshot.hits, 1, "Should record 1 hit");
    }

    #[tokio::test]
    async fn evicts_oldest_entries_when_over_limit() {
        let dir = tempdir().expect("tempdir");
        // Create cache with very small limit (1KB)
        let cache = DiskCache::with_max_size(dir.path(), 1024);

        // Store multiple files that will exceed the limit
        for i in 0..5 {
            let data = json!({"data": "x".repeat(300)});
            cache.store(&format!("file{}.json", i), data).await.unwrap();
            // Small delay to ensure different modification times
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        // Check that evictions occurred
        let snapshot = cache.stats().snapshot();
        assert!(snapshot.evictions > 0, "Should have evicted some entries");
    }

    #[tokio::test]
    async fn does_not_evict_when_under_limit() {
        let dir = tempdir().expect("tempdir");
        // Create cache with large limit (100MB)
        let cache = DiskCache::with_max_size(dir.path(), 100 * 1024 * 1024);

        // Store a few small files
        for i in 0..3 {
            let data = json!({"data": i});
            cache.store(&format!("file{}.json", i), data).await.unwrap();
        }

        let snapshot = cache.stats().snapshot();
        assert_eq!(snapshot.evictions, 0, "Should not evict when under limit");
        assert_eq!(snapshot.entry_count, 3, "Should have 3 entries");
    }

    #[tokio::test]
    async fn evicts_least_recently_accessed() {
        let dir = tempdir().expect("tempdir");
        // Create cache with very small limit (1KB)
        let cache = DiskCache::with_max_size(dir.path(), 1024);

        // Store first file (oldest) - larger to ensure eviction
        cache.store("old.json", json!({"data": "x".repeat(800)})).await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Store second file - this should trigger eviction of the old file
        cache.store("new.json", json!({"data": "x".repeat(800)})).await.unwrap();

        let snapshot = cache.stats().snapshot();
        assert!(snapshot.evictions > 0, "Should have evicted at least one entry");

        // The newest file should still be there
        let newest: Option<CacheEntry<serde_json::Value>> =
            cache.load("new.json").await.unwrap();
        assert!(newest.is_some(), "Newest file should not be evicted");
    }

    #[tokio::test]
    async fn tracks_entry_count() {
        let dir = tempdir().expect("tempdir");
        let cache = DiskCache::new(dir.path());

        cache.store("file1.json", json!({"a": 1})).await.unwrap();
        cache.store("file2.json", json!({"b": 2})).await.unwrap();
        cache.store("file3.json", json!({"c": 3})).await.unwrap();

        let snapshot = cache.stats().snapshot();
        assert_eq!(snapshot.entry_count, 3, "Should track entry count");
    }

    #[tokio::test]
    async fn eviction_updates_entry_count() {
        let dir = tempdir().expect("tempdir");
        // Small cache limit to force eviction
        let cache = DiskCache::with_max_size(dir.path(), 1024);

        // Store files that will exceed limit
        for i in 0..5 {
            let data = json!({"data": "x".repeat(300)});
            cache.store(&format!("file{}.json", i), data).await.unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        let snapshot = cache.stats().snapshot();
        // Entry count should be reduced after eviction
        assert!(snapshot.entry_count < 5, "Entry count should be reduced after eviction");
        assert_eq!(
            snapshot.evictions,
            5 - snapshot.entry_count,
            "Eviction count should match reduction in entries"
        );
    }
}
