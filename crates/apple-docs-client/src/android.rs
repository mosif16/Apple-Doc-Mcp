use std::{path::PathBuf, time::Duration as StdDuration};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use directories::ProjectDirs;
use reqwest::Client;
use thiserror::Error;
use time::Duration;
use tokio::sync::Mutex;
use tracing::{debug, instrument, warn};

use crate::cache::{DiskCache, MemoryCache};

const ANDROID_BASE_URL: &str = "https://developer.android.com";
const ANDROID_INDEX_KEY: &str = "android_index";

#[derive(Debug, Clone, Error)]
pub enum AndroidClientError {
    #[error("HTTP request failed: {0}")]
    Http(String),
    #[error("unexpected status code: {0}")]
    Status(u16),
    #[error("cache miss")]
    CacheMiss,
    #[error("parse error: {0}")]
    Parse(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AndroidPackage {
    pub name: String,
    pub path: String,
    pub description: Option<String>,
    pub category: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AndroidClass {
    pub name: String,
    pub qualified_name: String,
    pub package: String,
    pub path: String,
    pub kind: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AndroidLibrary {
    pub name: String,
    pub group_id: String,
    pub artifact_id: String,
    pub description: Option<String>,
    pub category: AndroidCategory,
    pub packages: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum AndroidCategory {
    Core,
    UI,
    Compose,
    Architecture,
    Behavior,
    Media,
    Connectivity,
    Performance,
    Security,
    Test,
    Other,
}

impl std::fmt::Display for AndroidCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Core => write!(f, "Core"),
            Self::UI => write!(f, "UI"),
            Self::Compose => write!(f, "Compose"),
            Self::Architecture => write!(f, "Architecture"),
            Self::Behavior => write!(f, "Behavior"),
            Self::Media => write!(f, "Media"),
            Self::Connectivity => write!(f, "Connectivity"),
            Self::Performance => write!(f, "Performance"),
            Self::Security => write!(f, "Security"),
            Self::Test => write!(f, "Test"),
            Self::Other => write!(f, "Other"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AndroidClientConfig {
    pub cache_dir: PathBuf,
    pub memory_cache_ttl: Duration,
}

impl Default for AndroidClientConfig {
    fn default() -> Self {
        let project_dirs = ProjectDirs::from("com", "RecordAndLearn", "dev-docs-mcp")
            .expect("unable to resolve project directories");

        Self {
            cache_dir: project_dirs.cache_dir().join("android"),
            memory_cache_ttl: Duration::minutes(10),
        }
    }
}

#[derive(Debug)]
pub struct AndroidDocsClient {
    #[allow(dead_code)]
    http: Client,
    disk_cache: DiskCache,
    index_lock: Mutex<()>,
    #[allow(dead_code)]
    memory_cache: MemoryCache<Vec<u8>>,
    config: AndroidClientConfig,
}

impl AndroidDocsClient {
    pub fn with_config(config: AndroidClientConfig) -> Self {
        let http = Client::builder()
            .user_agent("DevDocsMCP/1.0")
            .timeout(StdDuration::from_secs(30))
            .gzip(true)
            .build()
            .expect("failed to build reqwest client");

        if let Err(error) = std::fs::create_dir_all(&config.cache_dir) {
            warn!(
                error = %error,
                cache_dir = %config.cache_dir.display(),
                "failed to create Android cache directory"
            );
        }

        let disk_cache = DiskCache::new(&config.cache_dir);
        Self {
            http,
            disk_cache,
            index_lock: Mutex::new(()),
            memory_cache: MemoryCache::new(config.memory_cache_ttl),
            config,
        }
    }

    #[must_use]
    pub fn new() -> Self {
        Self::with_config(AndroidClientConfig::default())
    }

    pub fn cache_dir(&self) -> &PathBuf {
        &self.config.cache_dir
    }

    #[instrument(name = "android_client.get_libraries", skip(self))]
    pub async fn get_libraries(&self) -> Result<Vec<AndroidLibrary>> {
        let file_name = format!("{ANDROID_INDEX_KEY}.json");

        if let Some(entry) = self.disk_cache.load::<Vec<AndroidLibrary>>(&file_name).await? {
            debug!("Android libraries served from disk cache");
            return Ok(entry.value);
        }

        let _lock = self.index_lock.lock().await;
        if let Some(entry) = self.disk_cache.load::<Vec<AndroidLibrary>>(&file_name).await? {
            debug!("Android libraries served from disk cache after lock");
            return Ok(entry.value);
        }

        let libraries = Self::get_curated_libraries();
        self.disk_cache.store(&file_name, libraries.clone()).await?;
        Ok(libraries)
    }

    fn get_curated_libraries() -> Vec<AndroidLibrary> {
        vec![
            // Compose UI
            AndroidLibrary {
                name: "Compose UI".to_string(),
                group_id: "androidx.compose.ui".to_string(),
                artifact_id: "ui".to_string(),
                description: Some("Fundamental components of compose UI needed to interact with the device".to_string()),
                category: AndroidCategory::Compose,
                packages: vec![
                    "androidx.compose.ui".to_string(),
                    "androidx.compose.ui.draw".to_string(),
                    "androidx.compose.ui.graphics".to_string(),
                    "androidx.compose.ui.input".to_string(),
                    "androidx.compose.ui.layout".to_string(),
                    "androidx.compose.ui.text".to_string(),
                ],
            },
            AndroidLibrary {
                name: "Compose Foundation".to_string(),
                group_id: "androidx.compose.foundation".to_string(),
                artifact_id: "foundation".to_string(),
                description: Some("Write Jetpack Compose applications with ready to use building blocks".to_string()),
                category: AndroidCategory::Compose,
                packages: vec![
                    "androidx.compose.foundation".to_string(),
                    "androidx.compose.foundation.gestures".to_string(),
                    "androidx.compose.foundation.layout".to_string(),
                    "androidx.compose.foundation.lazy".to_string(),
                ],
            },
            AndroidLibrary {
                name: "Compose Material3".to_string(),
                group_id: "androidx.compose.material3".to_string(),
                artifact_id: "material3".to_string(),
                description: Some("Build Jetpack Compose UIs with Material Design 3 components".to_string()),
                category: AndroidCategory::Compose,
                packages: vec![
                    "androidx.compose.material3".to_string(),
                ],
            },
            AndroidLibrary {
                name: "Compose Material".to_string(),
                group_id: "androidx.compose.material".to_string(),
                artifact_id: "material".to_string(),
                description: Some("Build Jetpack Compose UIs with Material Design components".to_string()),
                category: AndroidCategory::Compose,
                packages: vec![
                    "androidx.compose.material".to_string(),
                    "androidx.compose.material.icons".to_string(),
                ],
            },
            AndroidLibrary {
                name: "Compose Runtime".to_string(),
                group_id: "androidx.compose.runtime".to_string(),
                artifact_id: "runtime".to_string(),
                description: Some("Fundamental building blocks of Compose's programming model and state management".to_string()),
                category: AndroidCategory::Compose,
                packages: vec![
                    "androidx.compose.runtime".to_string(),
                    "androidx.compose.runtime.saveable".to_string(),
                ],
            },
            AndroidLibrary {
                name: "Compose Animation".to_string(),
                group_id: "androidx.compose.animation".to_string(),
                artifact_id: "animation".to_string(),
                description: Some("Build animations in their Jetpack Compose applications".to_string()),
                category: AndroidCategory::Compose,
                packages: vec![
                    "androidx.compose.animation".to_string(),
                    "androidx.compose.animation.core".to_string(),
                ],
            },
            AndroidLibrary {
                name: "Compose Navigation".to_string(),
                group_id: "androidx.navigation".to_string(),
                artifact_id: "navigation-compose".to_string(),
                description: Some("Navigation component for Jetpack Compose".to_string()),
                category: AndroidCategory::Compose,
                packages: vec![
                    "androidx.navigation.compose".to_string(),
                ],
            },
            // Architecture
            AndroidLibrary {
                name: "ViewModel".to_string(),
                group_id: "androidx.lifecycle".to_string(),
                artifact_id: "lifecycle-viewmodel".to_string(),
                description: Some("Manage UI-related data in a lifecycle-conscious way".to_string()),
                category: AndroidCategory::Architecture,
                packages: vec![
                    "androidx.lifecycle".to_string(),
                ],
            },
            AndroidLibrary {
                name: "LiveData".to_string(),
                group_id: "androidx.lifecycle".to_string(),
                artifact_id: "lifecycle-livedata".to_string(),
                description: Some("Observable data holder class that respects the lifecycle".to_string()),
                category: AndroidCategory::Architecture,
                packages: vec![
                    "androidx.lifecycle".to_string(),
                ],
            },
            AndroidLibrary {
                name: "Room".to_string(),
                group_id: "androidx.room".to_string(),
                artifact_id: "room-runtime".to_string(),
                description: Some("Persistence library providing abstraction over SQLite".to_string()),
                category: AndroidCategory::Architecture,
                packages: vec![
                    "androidx.room".to_string(),
                ],
            },
            AndroidLibrary {
                name: "Hilt".to_string(),
                group_id: "com.google.dagger".to_string(),
                artifact_id: "hilt-android".to_string(),
                description: Some("Dependency injection library for Android built on Dagger".to_string()),
                category: AndroidCategory::Architecture,
                packages: vec![
                    "dagger.hilt.android".to_string(),
                ],
            },
            AndroidLibrary {
                name: "DataStore".to_string(),
                group_id: "androidx.datastore".to_string(),
                artifact_id: "datastore".to_string(),
                description: Some("Data storage solution using Kotlin coroutines and Flow".to_string()),
                category: AndroidCategory::Architecture,
                packages: vec![
                    "androidx.datastore".to_string(),
                    "androidx.datastore.preferences".to_string(),
                ],
            },
            AndroidLibrary {
                name: "WorkManager".to_string(),
                group_id: "androidx.work".to_string(),
                artifact_id: "work-runtime".to_string(),
                description: Some("Schedule deferrable, asynchronous tasks".to_string()),
                category: AndroidCategory::Architecture,
                packages: vec![
                    "androidx.work".to_string(),
                ],
            },
            // Core
            AndroidLibrary {
                name: "Activity".to_string(),
                group_id: "androidx.activity".to_string(),
                artifact_id: "activity".to_string(),
                description: Some("Access composable APIs built on top of Activity".to_string()),
                category: AndroidCategory::Core,
                packages: vec![
                    "androidx.activity".to_string(),
                    "androidx.activity.compose".to_string(),
                ],
            },
            AndroidLibrary {
                name: "Fragment".to_string(),
                group_id: "androidx.fragment".to_string(),
                artifact_id: "fragment".to_string(),
                description: Some("Segment your app into multiple, independent screens".to_string()),
                category: AndroidCategory::Core,
                packages: vec![
                    "androidx.fragment".to_string(),
                    "androidx.fragment.app".to_string(),
                ],
            },
            AndroidLibrary {
                name: "Core KTX".to_string(),
                group_id: "androidx.core".to_string(),
                artifact_id: "core-ktx".to_string(),
                description: Some("Kotlin extensions for Android framework APIs".to_string()),
                category: AndroidCategory::Core,
                packages: vec![
                    "androidx.core".to_string(),
                    "androidx.core.content".to_string(),
                    "androidx.core.view".to_string(),
                ],
            },
            // UI
            AndroidLibrary {
                name: "RecyclerView".to_string(),
                group_id: "androidx.recyclerview".to_string(),
                artifact_id: "recyclerview".to_string(),
                description: Some("Display large sets of data in your UI while minimizing memory usage".to_string()),
                category: AndroidCategory::UI,
                packages: vec![
                    "androidx.recyclerview.widget".to_string(),
                ],
            },
            AndroidLibrary {
                name: "ConstraintLayout".to_string(),
                group_id: "androidx.constraintlayout".to_string(),
                artifact_id: "constraintlayout".to_string(),
                description: Some("Position and size widgets in a flexible way with relative positioning".to_string()),
                category: AndroidCategory::UI,
                packages: vec![
                    "androidx.constraintlayout.widget".to_string(),
                    "androidx.constraintlayout.motion.widget".to_string(),
                ],
            },
            AndroidLibrary {
                name: "ViewPager2".to_string(),
                group_id: "androidx.viewpager2".to_string(),
                artifact_id: "viewpager2".to_string(),
                description: Some("Display Views or Fragments in a swipeable format".to_string()),
                category: AndroidCategory::UI,
                packages: vec![
                    "androidx.viewpager2.widget".to_string(),
                ],
            },
            AndroidLibrary {
                name: "Material Components".to_string(),
                group_id: "com.google.android.material".to_string(),
                artifact_id: "material".to_string(),
                description: Some("Material Design components for Android".to_string()),
                category: AndroidCategory::UI,
                packages: vec![
                    "com.google.android.material".to_string(),
                ],
            },
            // Media
            AndroidLibrary {
                name: "Media3 ExoPlayer".to_string(),
                group_id: "androidx.media3".to_string(),
                artifact_id: "media3-exoplayer".to_string(),
                description: Some("Media playback library for Android".to_string()),
                category: AndroidCategory::Media,
                packages: vec![
                    "androidx.media3.exoplayer".to_string(),
                ],
            },
            AndroidLibrary {
                name: "CameraX".to_string(),
                group_id: "androidx.camera".to_string(),
                artifact_id: "camera-core".to_string(),
                description: Some("Build camera apps more easily".to_string()),
                category: AndroidCategory::Media,
                packages: vec![
                    "androidx.camera.core".to_string(),
                    "androidx.camera.camera2".to_string(),
                    "androidx.camera.view".to_string(),
                ],
            },
            // Connectivity
            AndroidLibrary {
                name: "Retrofit".to_string(),
                group_id: "com.squareup.retrofit2".to_string(),
                artifact_id: "retrofit".to_string(),
                description: Some("Type-safe HTTP client for Android and Java".to_string()),
                category: AndroidCategory::Connectivity,
                packages: vec![
                    "retrofit2".to_string(),
                ],
            },
            AndroidLibrary {
                name: "OkHttp".to_string(),
                group_id: "com.squareup.okhttp3".to_string(),
                artifact_id: "okhttp".to_string(),
                description: Some("HTTP client that's efficient by default".to_string()),
                category: AndroidCategory::Connectivity,
                packages: vec![
                    "okhttp3".to_string(),
                ],
            },
            // Security
            AndroidLibrary {
                name: "Security Crypto".to_string(),
                group_id: "androidx.security".to_string(),
                artifact_id: "security-crypto".to_string(),
                description: Some("Safely manage keys and encrypt files and sharedpreferences".to_string()),
                category: AndroidCategory::Security,
                packages: vec![
                    "androidx.security.crypto".to_string(),
                ],
            },
            AndroidLibrary {
                name: "Biometric".to_string(),
                group_id: "androidx.biometric".to_string(),
                artifact_id: "biometric".to_string(),
                description: Some("Authenticate with biometrics or device credentials".to_string()),
                category: AndroidCategory::Security,
                packages: vec![
                    "androidx.biometric".to_string(),
                ],
            },
            // Test
            AndroidLibrary {
                name: "Compose UI Test".to_string(),
                group_id: "androidx.compose.ui".to_string(),
                artifact_id: "ui-test".to_string(),
                description: Some("Testing utilities for Compose UI".to_string()),
                category: AndroidCategory::Test,
                packages: vec![
                    "androidx.compose.ui.test".to_string(),
                ],
            },
            AndroidLibrary {
                name: "Espresso".to_string(),
                group_id: "androidx.test.espresso".to_string(),
                artifact_id: "espresso-core".to_string(),
                description: Some("UI testing framework for Android".to_string()),
                category: AndroidCategory::Test,
                packages: vec![
                    "androidx.test.espresso".to_string(),
                ],
            },
        ]
    }

    #[instrument(name = "android_client.get_library", skip(self))]
    pub async fn get_library(&self, name: &str) -> Result<Option<AndroidLibrary>> {
        let libraries = self.get_libraries().await?;
        let name_lower = name.to_lowercase();

        Ok(libraries.into_iter().find(|lib| {
            lib.name.to_lowercase() == name_lower
                || lib.artifact_id.to_lowercase() == name_lower
                || lib.group_id.to_lowercase().contains(&name_lower)
        }))
    }

    #[instrument(name = "android_client.search", skip(self))]
    pub async fn search(&self, query: &str, max_results: usize) -> Result<Vec<AndroidLibrary>> {
        let libraries = self.get_libraries().await?;
        let query_lower = query.to_lowercase();

        let mut results: Vec<(i32, AndroidLibrary)> = libraries
            .into_iter()
            .filter_map(|lib| {
                let name_lower = lib.name.to_lowercase();
                let desc_lower = lib.description.as_ref().map(|d| d.to_lowercase()).unwrap_or_default();
                let artifact_lower = lib.artifact_id.to_lowercase();

                let score = if name_lower == query_lower || artifact_lower == query_lower {
                    100
                } else if name_lower.starts_with(&query_lower) || artifact_lower.starts_with(&query_lower) {
                    80
                } else if name_lower.contains(&query_lower) || artifact_lower.contains(&query_lower) {
                    60
                } else if desc_lower.contains(&query_lower) {
                    40
                } else if lib.packages.iter().any(|p| p.to_lowercase().contains(&query_lower)) {
                    30
                } else {
                    return None;
                };

                Some((score, lib))
            })
            .collect();

        results.sort_by(|a, b| b.0.cmp(&a.0));

        Ok(results.into_iter().take(max_results).map(|(_, lib)| lib).collect())
    }

    #[instrument(name = "android_client.get_by_category", skip(self))]
    pub async fn get_by_category(&self, category: AndroidCategory) -> Result<Vec<AndroidLibrary>> {
        let libraries = self.get_libraries().await?;
        Ok(libraries.into_iter().filter(|lib| lib.category == category).collect())
    }

    pub fn get_reference_url(package: &str) -> String {
        let path = package.replace('.', "/");
        format!("{}/reference/kotlin/{}/package-summary", ANDROID_BASE_URL, path)
    }

    pub fn get_class_url(qualified_name: &str) -> String {
        let path = qualified_name.replace('.', "/");
        format!("{}/reference/kotlin/{}", ANDROID_BASE_URL, path)
    }

    pub fn clear_memory_cache(&self) {
        self.memory_cache.clear();
    }
}

impl Default for AndroidDocsClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn defaults_provide_cache_dir() {
        let client = AndroidDocsClient::new();
        assert!(client.cache_dir().to_string_lossy().contains("android"));
    }

    #[tokio::test]
    async fn curated_libraries_not_empty() {
        let libraries = AndroidDocsClient::get_curated_libraries();
        assert!(!libraries.is_empty());
    }
}
