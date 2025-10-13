use std::{collections::HashMap, sync::Arc};

use apple_docs_client::types::{
    CacheEntry, FrameworkData, FrameworkMetadata, PlatformInfo, ReferenceData, RichText, Technology,
};
use apple_docs_client::{AppleDocsClient, ClientConfig};
use apple_docs_core::state::{AppContext, FrameworkIndexEntry};
use apple_docs_core::tools::search_symbols_definition;
use serde_json::json;
use tempfile::tempdir;
use time::{Duration, OffsetDateTime};
use tokio::fs;

fn sample_platform() -> PlatformInfo {
    PlatformInfo {
        name: "iOS".to_string(),
        introduced_at: Some("17.0".to_string()),
        beta: false,
    }
}

fn sample_technology() -> Technology {
    Technology {
        r#abstract: vec![],
        identifier: "doc://com.apple.documentation/SwiftUI".to_string(),
        kind: "symbol".to_string(),
        role: "collection".to_string(),
        title: "SwiftUI".to_string(),
        url: "https://developer.apple.com/documentation/swiftui".to_string(),
    }
}

fn sample_framework() -> FrameworkData {
    let reference = ReferenceData {
        title: Some("PaneTabView".to_string()),
        kind: Some("structure".to_string()),
        r#abstract: Some(vec![RichText {
            text: Some("A container that manages tabs in a pane.".to_string()),
            kind: "text".to_string(),
        }]),
        platforms: Some(vec![sample_platform()]),
        url: Some("documentation/SwiftUI/PaneTabView".to_string()),
    };

    FrameworkData {
        r#abstract: vec![],
        metadata: FrameworkMetadata {
            platforms: vec![sample_platform()],
            role: "collection".to_string(),
            title: "SwiftUI".to_string(),
        },
        references: vec![("pane_tab_view".to_string(), reference)]
            .into_iter()
            .collect(),
        topic_sections: vec![],
    }
}

#[tokio::test]
async fn search_symbols_uses_fallback_when_index_empty() {
    let dir = tempdir().expect("tempdir");
    let client = AppleDocsClient::with_config(ClientConfig {
        cache_dir: dir.path().to_path_buf(),
        memory_cache_ttl: Duration::minutes(10),
    });
    let context = Arc::new(AppContext::new(client));

    let technology = sample_technology();
    *context.state.active_technology.write().await = Some(technology.clone());
    *context.state.framework_cache.write().await = Some(sample_framework());
    context
        .state
        .framework_index
        .write()
        .await
        .replace(Vec::new());

    let (_definition, handler) = search_symbols_definition();
    let response = handler(
        context.clone(),
        json!({
            "query": "pane",
            "maxResults": 5
        }),
    )
    .await
    .expect("handler should succeed");

    let text = &response.content[0].text;
    assert!(
        text.contains("Fallback suggestions"),
        "Expected fallback suggestions in response: {text}"
    );
    assert!(
        text.contains("PaneTabView"),
        "Expected fallback result to include PaneTabView: {text}"
    );
}

#[tokio::test]
async fn search_symbols_primary_results_exclude_fallback() {
    let dir = tempdir().expect("tempdir");
    let client = AppleDocsClient::with_config(ClientConfig {
        cache_dir: dir.path().to_path_buf(),
        memory_cache_ttl: Duration::minutes(10),
    });
    let context = Arc::new(AppContext::new(client));

    let technology = sample_technology();
    *context.state.active_technology.write().await = Some(technology.clone());

    let framework = sample_framework();
    // Pre-build framework index with matching entry.
    let index_entry = FrameworkIndexEntry {
        id: "pane_tab_view".to_string(),
        tokens: vec!["pane".to_string(), "tabview".to_string()],
        reference: framework.references["pane_tab_view"].clone(),
    };

    *context.state.framework_cache.write().await = Some(framework);
    context
        .state
        .framework_index
        .write()
        .await
        .replace(vec![index_entry]);

    let (_definition, handler) = search_symbols_definition();
    let response = handler(
        context.clone(),
        json!({
            "query": "pane",
            "maxResults": 5
        }),
    )
    .await
    .expect("handler should succeed");

    let text = &response.content[0].text;
    assert!(text.contains("PaneTabView"));
    assert!(!text.contains("Fallback suggestions"));
}

#[tokio::test]
async fn search_symbols_global_scope_reads_cached_frameworks() {
    let dir = tempdir().expect("tempdir");
    let client = AppleDocsClient::with_config(ClientConfig {
        cache_dir: dir.path().to_path_buf(),
        memory_cache_ttl: Duration::minutes(10),
    });
    let cache_dir = client.cache_dir().clone();
    let context = Arc::new(AppContext::new(client));

    let technology = sample_technology();
    let framework = sample_framework();

    let mut technologies_map = HashMap::new();
    technologies_map.insert(technology.identifier.clone(), technology.clone());

    let technologies_entry = CacheEntry {
        value: technologies_map,
        stored_at: OffsetDateTime::now_utc(),
    };
    fs::write(
        cache_dir.join("technologies.json"),
        serde_json::to_vec(&technologies_entry).expect("serialize technologies cache"),
    )
    .await
    .expect("write technologies cache");

    let framework_entry = CacheEntry {
        value: framework.clone(),
        stored_at: OffsetDateTime::now_utc(),
    };
    fs::write(
        cache_dir.join("SwiftUI.json"),
        serde_json::to_vec(&framework_entry).expect("serialize framework cache"),
    )
    .await
    .expect("write framework cache");

    let (_definition, handler) = search_symbols_definition();
    let response = handler(
        context.clone(),
        json!({
            "query": "pane",
            "scope": "global",
            "maxResults": 5
        }),
    )
    .await
    .expect("handler should succeed");

    let text = &response.content[0].text;
    assert!(
        text.contains("PaneTabView"),
        "Expected symbol to appear in global search response: {text}"
    );
    assert!(
        text.contains("Technology: SwiftUI"),
        "Expected technology hint in global search response: {text}"
    );
}
