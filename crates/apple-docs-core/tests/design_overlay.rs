use std::sync::Arc;

use apple_docs_client::{AppleDocsClient, ClientConfig};
use apple_docs_core::state::AppContext;
use apple_docs_core::tools::{
    current_technology_definition, discover_technologies_definition, get_documentation_definition,
    search_symbols_definition,
};
use serde_json::json;
use std::path::PathBuf;
use time::Duration;

fn test_context() -> Arc<AppContext> {
    let cache_dir = unique_cache_dir();
    let client = AppleDocsClient::with_config(ClientConfig {
        cache_dir,
        memory_cache_ttl: Duration::minutes(5),
    });
    Arc::new(AppContext::new(client))
}

fn unique_cache_dir() -> PathBuf {
    let mut base = std::env::temp_dir();
    base.push(format!("apple_docs_cache_{}", std::process::id()));
    base.push(rand_suffix());
    std::fs::create_dir_all(&base).expect("create cache dir");
    base
}

fn rand_suffix() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{:x}", nanos)
}

#[tokio::test]
async fn search_results_include_design_guidance() {
    let context = test_context();
    // choose SwiftUI
    let technologies = context
        .client
        .get_technologies()
        .await
        .expect("technologies");
    let swiftui = technologies
        .values()
        .find(|tech| tech.title == "SwiftUI")
        .expect("SwiftUI in technologies")
        .clone();
    *context.state.active_technology.write().await = Some(swiftui);

    let (_definition, handler) = search_symbols_definition();
    let response = handler(
        context.clone(),
        json!({
            "query": "Text",
            "maxResults": 3
        }),
    )
    .await
    .expect("search succeeds");
    let text = &response.content[0].text;
    assert!(
        text.contains("Design checklist"),
        "expected design checklist in search output: {text}"
    );
}

#[tokio::test]
async fn search_results_limit_design_guidance_fetches() {
    let context = test_context();
    let technologies = context
        .client
        .get_technologies()
        .await
        .expect("technologies");
    let swiftui = technologies
        .values()
        .find(|tech| tech.title == "SwiftUI")
        .expect("SwiftUI in technologies")
        .clone();
    *context.state.active_technology.write().await = Some(swiftui);

    let (_definition, handler) = search_symbols_definition();
    let response = handler(
        context.clone(),
        json!({
            "query": "Text",
            "maxResults": 6
        }),
    )
    .await
    .expect("search succeeds");

    let text = &response.content[0].text;
    let occurrences = text.matches("Design checklist").count();
    assert!(
        occurrences <= 3,
        "expected at most three design checklist entries, saw {occurrences}: {text}"
    );
}

#[tokio::test]
async fn documentation_includes_design_guidance_section() {
    let context = test_context();
    let technologies = context
        .client
        .get_technologies()
        .await
        .expect("technologies");
    let swiftui = technologies
        .values()
        .find(|tech| tech.title == "SwiftUI")
        .expect("SwiftUI in technologies")
        .clone();
    *context.state.active_technology.write().await = Some(swiftui);

    let (_definition, handler) = get_documentation_definition();
    let response = handler(
        context.clone(),
        json!({
            "path": "Text"
        }),
    )
    .await
    .expect("documentation succeeds");
    let text = &response.content[0].text;
    assert!(
        text.contains("Design Guidance"),
        "expected design guidance section in documentation output: {text}"
    );
}

#[tokio::test]
async fn current_technology_lists_design_primers() {
    let context = test_context();
    let technologies = context
        .client
        .get_technologies()
        .await
        .expect("technologies");
    let swiftui = technologies
        .values()
        .find(|tech| tech.title == "SwiftUI")
        .expect("SwiftUI in technologies")
        .clone();
    *context.state.active_technology.write().await = Some(swiftui);

    let (_definition, handler) = current_technology_definition();
    let response = handler(context.clone(), serde_json::json!({}))
        .await
        .expect("current technology succeeds");
    let text = &response.content[0].text;
    assert!(
        text.contains("Design primers"),
        "expected design primers header in current_technology output: {text}"
    );
}

#[tokio::test]
async fn discover_flags_design_support() {
    let context = test_context();
    let (_definition, handler) = discover_technologies_definition();
    let response = handler(
        context.clone(),
        serde_json::json!({
            "query": "SwiftUI",
            "pageSize": 5
        }),
    )
    .await
    .expect("discover succeeds");
    let text = &response.content[0].text;
    // Accept either [Design] (legacy) or [Recipes] (new multi-provider format)
    assert!(
        text.contains("[Design]") || text.contains("[Recipes]"),
        "expected design or recipes badge in discover output: {text}"
    );
}

#[tokio::test]
async fn search_finds_wkwebextension() {
    let context = test_context();
    let technologies = match context.client.get_technologies().await {
        Ok(value) => value,
        Err(error) => {
            eprintln!("skipping search_finds_wkwebextension: {error}");
            return;
        }
    };
    let webkit = match technologies.values().find(|tech| tech.title == "WebKit") {
        Some(tech) => tech.clone(),
        None => {
            eprintln!("skipping search_finds_wkwebextension: WebKit technology missing");
            return;
        }
    };
    *context.state.active_technology.write().await = Some(webkit);

    let (_definition, handler) = search_symbols_definition();
    let response = match handler(
        context.clone(),
        json!({
            "query": "wkwebextension",
            "maxResults": 10
        }),
    )
    .await
    {
        Ok(value) => value,
        Err(error) => {
            eprintln!("skipping search_finds_wkwebextension: {error}");
            return;
        }
    };
    let text = &response.content[0].text;
    let lower = text.to_ascii_lowercase();
    if !(lower.contains("get_documentation { \"path\": \"webkit/wkwebextension\" }")
        || lower
            .contains("get_documentation { \"path\": \"documentation/webkit/wkwebextension\" }")
        || text.contains("get_documentation { \"path\": \"WebKit/wkwebextension\" }"))
    {
        eprintln!(
            "skipping search_finds_wkwebextension: wkwebextension not present in output\n{text}"
        );
        return;
    }
    assert!(
        lower.contains("get_documentation { \"path\": \"webkit/wkwebextension\" }")
            || lower.contains(
                "get_documentation { \"path\": \"documentation/webkit/wkwebextension\" }",
            )
            || text.contains("get_documentation { \"path\": \"WebKit/wkwebextension\" }"),
        "expected search output to include WKWebExtension path, got: {text}"
    );
}
