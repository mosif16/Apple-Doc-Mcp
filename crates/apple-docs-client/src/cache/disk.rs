use std::{
    fs::create_dir_all,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context, Result};
use serde::{de::DeserializeOwned, Serialize};
use tokio::{fs, task};
use tracing::debug;

use crate::types::CacheEntry;
use time::OffsetDateTime;

#[derive(Debug)]
pub struct DiskCache {
    root: PathBuf,
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
}

impl DiskCache {
    pub fn new<P: Into<PathBuf>>(root: P) -> Self {
        Self { root: root.into() }
    }

    pub async fn load<T>(&self, file_name: &str) -> Result<Option<CacheEntry<T>>>
    where
        T: DeserializeOwned + Send + 'static,
    {
        let path = self.root.join(file_name);
        if !path.exists() {
            return Ok(None);
        }

        let data = fs::read(path.clone())
            .await
            .with_context(|| format!("failed to read cache file {path:?}"))?;
        let entry =
            task::spawn_blocking(
                move || match serde_json::from_slice::<CacheEntry<T>>(&data) {
                    Ok(entry) => Ok(entry),
                    Err(primary_err) => serde_json::from_slice::<T>(&data)
                        .map(|value| CacheEntry {
                            value,
                            stored_at: OffsetDateTime::UNIX_EPOCH,
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

        let entry = CacheEntry {
            value,
            stored_at: time::OffsetDateTime::now_utc(),
        };

        let payload = task::spawn_blocking(move || serde_json::to_vec(&entry)).await??;
        fs::write(path.clone(), payload)
            .await
            .with_context(|| format!("failed to write cache file {path:?}"))?;

        debug!(target: "apple_docs_cache", file = ?path, "wrote cache entry");
        Ok(())
    }
}
