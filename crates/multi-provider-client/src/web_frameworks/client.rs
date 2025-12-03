use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration as StdDuration;

use anyhow::{Context, Result};
use directories::ProjectDirs;
use reqwest::Client;
use scraper::{Html, Selector};
use serde_json::Value;
use tokio::sync::RwLock;
use tracing::{debug, instrument, warn};

use super::types::{
    CodeExample, NodeApiModule, WebFramework, WebFrameworkArticle, WebFrameworkSearchEntry,
    WebFrameworkTechnology,
};
use docs_mcp_client::cache::{DiskCache, MemoryCache};

// API endpoints
const NODEJS_API_JSON: &str = "https://nodejs.org/api/all.json";
const REACT_DEV_BASE: &str = "https://react.dev";
const NEXTJS_BASE: &str = "https://nextjs.org";

#[derive(Debug)]
pub struct WebFrameworksClient {
    http: Client,
    disk_cache: DiskCache,
    memory_cache: MemoryCache<Vec<u8>>,
    /// Search indexes per framework
    react_index: RwLock<Vec<WebFrameworkSearchEntry>>,
    nextjs_index: RwLock<Vec<WebFrameworkSearchEntry>>,
    nodejs_index: RwLock<Vec<WebFrameworkSearchEntry>>,
    cache_dir: PathBuf,
}

impl Default for WebFrameworksClient {
    fn default() -> Self {
        Self::new()
    }
}

impl WebFrameworksClient {
    #[must_use]
    pub fn new() -> Self {
        let project_dirs = ProjectDirs::from("com", "RecordAndLearn", "multi-docs-mcp")
            .expect("unable to resolve project directories");

        let cache_dir = project_dirs.cache_dir().join("web_frameworks");
        if let Err(e) = std::fs::create_dir_all(&cache_dir) {
            warn!(error = %e, "Failed to create web_frameworks cache directory");
        }

        let http = Client::builder()
            .user_agent("MultiDocsMCP/1.0 (Documentation Search Tool)")
            .timeout(StdDuration::from_secs(30))
            .gzip(true)
            .build()
            .expect("failed to build reqwest client");

        Self {
            http,
            disk_cache: DiskCache::new(&cache_dir),
            memory_cache: MemoryCache::new(time::Duration::hours(1)),
            react_index: RwLock::new(Vec::new()),
            nextjs_index: RwLock::new(Vec::new()),
            nodejs_index: RwLock::new(Vec::new()),
            cache_dir,
        }
    }

    /// Get available technologies
    #[instrument(name = "webfw_client.get_technologies", skip(self))]
    pub async fn get_technologies(&self) -> Result<Vec<WebFrameworkTechnology>> {
        Ok(WebFrameworkTechnology::predefined())
    }

    /// Search documentation for a specific framework
    #[instrument(name = "webfw_client.search", skip(self))]
    pub async fn search(
        &self,
        framework: WebFramework,
        query: &str,
    ) -> Result<Vec<WebFrameworkSearchEntry>> {
        match framework {
            WebFramework::React => self.search_react(query).await,
            WebFramework::NextJs => self.search_nextjs(query).await,
            WebFramework::NodeJs => self.search_nodejs(query).await,
        }
    }

    /// Get article for a specific framework
    #[instrument(name = "webfw_client.get_article", skip(self))]
    pub async fn get_article(
        &self,
        framework: WebFramework,
        slug: &str,
    ) -> Result<WebFrameworkArticle> {
        match framework {
            WebFramework::React => self.fetch_react_article(slug).await,
            WebFramework::NextJs => self.fetch_nextjs_article(slug).await,
            WebFramework::NodeJs => self.fetch_nodejs_article(slug).await,
        }
    }

    // ==================== REACT ====================

    /// Search React documentation
    async fn search_react(&self, query: &str) -> Result<Vec<WebFrameworkSearchEntry>> {
        // Ensure index is loaded
        self.ensure_react_index().await?;

        let query_lower = query.to_lowercase();
        let query_terms: Vec<&str> = query_lower.split_whitespace().collect();

        let index = self.react_index.read().await;
        let mut results: Vec<(i32, &WebFrameworkSearchEntry)> = index
            .iter()
            .filter_map(|entry| {
                let title_lower = entry.title.to_lowercase();
                let desc_lower = entry.description.to_lowercase();

                let mut score = 0i32;
                for term in &query_terms {
                    if title_lower.contains(term) {
                        score += 15;
                    }
                    if desc_lower.contains(term) {
                        score += 5;
                    }
                }

                if score > 0 {
                    Some((score, entry))
                } else {
                    None
                }
            })
            .collect();

        results.sort_by(|a, b| b.0.cmp(&a.0));

        Ok(results
            .into_iter()
            .take(20)
            .map(|(_, e)| e.clone())
            .collect())
    }

    /// Ensure React search index is loaded
    async fn ensure_react_index(&self) -> Result<()> {
        if !self.react_index.read().await.is_empty() {
            return Ok(());
        }

        // Check disk cache
        if let Ok(Some(entry)) = self
            .disk_cache
            .load::<Vec<WebFrameworkSearchEntry>>("react_index.json")
            .await
        {
            *self.react_index.write().await = entry.value;
            return Ok(());
        }

        // Build index from known React API pages
        let index = self.build_react_index().await;
        let _ = self
            .disk_cache
            .store("react_index.json", index.clone())
            .await;
        *self.react_index.write().await = index;

        Ok(())
    }

    /// Build React search index
    async fn build_react_index(&self) -> Vec<WebFrameworkSearchEntry> {
        // Predefined React API entries based on react.dev structure
        vec![
            // Hooks
            self.react_entry("reference/react/useState", "useState", "State hook for functional components"),
            self.react_entry("reference/react/useEffect", "useEffect", "Side effect hook for functional components"),
            self.react_entry("reference/react/useContext", "useContext", "Context hook for accessing context values"),
            self.react_entry("reference/react/useReducer", "useReducer", "Reducer hook for complex state logic"),
            self.react_entry("reference/react/useCallback", "useCallback", "Memoize callbacks to prevent unnecessary re-renders"),
            self.react_entry("reference/react/useMemo", "useMemo", "Memoize expensive computations"),
            self.react_entry("reference/react/useRef", "useRef", "Reference hook for mutable values and DOM refs"),
            self.react_entry("reference/react/useId", "useId", "Generate unique IDs for accessibility"),
            self.react_entry("reference/react/useTransition", "useTransition", "Mark state updates as non-blocking transitions"),
            self.react_entry("reference/react/useDeferredValue", "useDeferredValue", "Defer updating part of the UI"),
            self.react_entry("reference/react/useImperativeHandle", "useImperativeHandle", "Customize ref exposed to parent components"),
            self.react_entry("reference/react/useLayoutEffect", "useLayoutEffect", "Fire effect synchronously after DOM mutations"),
            self.react_entry("reference/react/useDebugValue", "useDebugValue", "Display label for custom hooks in React DevTools"),
            // Components
            self.react_entry("reference/react/Component", "Component", "Base class for React class components"),
            self.react_entry("reference/react/Fragment", "Fragment", "Group elements without adding extra DOM nodes"),
            self.react_entry("reference/react/Suspense", "Suspense", "Display fallback while children are loading"),
            self.react_entry("reference/react/StrictMode", "StrictMode", "Enable additional development checks"),
            self.react_entry("reference/react/Profiler", "Profiler", "Measure rendering performance"),
            // APIs
            self.react_entry("reference/react/createContext", "createContext", "Create a context for passing data through component tree"),
            self.react_entry("reference/react/forwardRef", "forwardRef", "Expose DOM node to parent with ref"),
            self.react_entry("reference/react/lazy", "lazy", "Define lazy-loaded component"),
            self.react_entry("reference/react/memo", "memo", "Skip re-rendering when props are unchanged"),
            self.react_entry("reference/react/startTransition", "startTransition", "Mark updates as transitions"),
            self.react_entry("reference/react/cache", "cache", "Cache the result of a data fetch or computation"),
            self.react_entry("reference/react/use", "use", "Read the value of a resource like Promise or context"),
            // DOM
            self.react_entry("reference/react-dom/createPortal", "createPortal", "Render children into different DOM node"),
            self.react_entry("reference/react-dom/flushSync", "flushSync", "Force React to flush pending updates synchronously"),
            self.react_entry("reference/react-dom/client/createRoot", "createRoot", "Create root to render React components"),
            self.react_entry("reference/react-dom/client/hydrateRoot", "hydrateRoot", "Hydrate server-rendered HTML"),
            // Server
            self.react_entry("reference/rsc/server-components", "Server Components", "Components that run only on the server"),
            self.react_entry("reference/rsc/server-actions", "Server Actions", "Functions that run on the server"),
            self.react_entry("reference/rsc/use-server", "use server", "Mark server-side functions"),
            self.react_entry("reference/rsc/use-client", "use client", "Mark client-side components"),
        ]
    }

    fn react_entry(&self, slug: &str, title: &str, description: &str) -> WebFrameworkSearchEntry {
        WebFrameworkSearchEntry {
            framework: WebFramework::React,
            slug: slug.to_string(),
            title: title.to_string(),
            description: description.to_string(),
            url: format!("{}/{}", REACT_DEV_BASE, slug),
            category: Some(
                if slug.contains("Hook") || slug.contains("use") {
                    "Hook"
                } else if slug.contains("Component") || slug.contains("Fragment") || slug.contains("Suspense") {
                    "Component"
                } else {
                    "API"
                }
                .to_string(),
            ),
        }
    }

    /// Fetch React article
    async fn fetch_react_article(&self, slug: &str) -> Result<WebFrameworkArticle> {
        let cache_key = format!("react_{}.json", slug.replace('/', "_"));

        // Check cache
        if let Ok(Some(entry)) = self
            .disk_cache
            .load::<WebFrameworkArticle>(&cache_key)
            .await
        {
            return Ok(entry.value);
        }

        // Fetch HTML page and scrape
        let url = format!("{}/{}", REACT_DEV_BASE, slug);
        debug!(url = %url, "Fetching React article");

        let response = self.http.get(&url).send().await?;
        if !response.status().is_success() {
            anyhow::bail!("React page not found: {}", slug);
        }

        let html = response.text().await?;
        let article = self.parse_react_html(&html, slug, &url);

        let _ = self.disk_cache.store(&cache_key, article.clone()).await;

        Ok(article)
    }

    fn parse_react_html(&self, html: &str, slug: &str, url: &str) -> WebFrameworkArticle {
        let document = Html::parse_document(html);

        let title = self
            .extract_text(&document, "h1")
            .unwrap_or_else(|| slug.split('/').last().unwrap_or("React").to_string());

        let description = self
            .extract_text(&document, "article > p:first-of-type, .intro")
            .unwrap_or_default();

        let examples = self.extract_code_examples(&document, "jsx");

        let api_signature = self.extract_text(&document, "pre.language-js:first-of-type, .signature");

        let content = self
            .extract_text(&document, "article")
            .map(|s| if s.len() > 4000 { s[..4000].to_string() } else { s })
            .unwrap_or_default();

        WebFrameworkArticle {
            framework: WebFramework::React,
            slug: slug.to_string(),
            title,
            description,
            content,
            examples,
            api_signature,
            related: Vec::new(),
            url: url.to_string(),
        }
    }

    // ==================== NEXT.JS ====================

    async fn search_nextjs(&self, query: &str) -> Result<Vec<WebFrameworkSearchEntry>> {
        self.ensure_nextjs_index().await?;

        let query_lower = query.to_lowercase();
        let query_terms: Vec<&str> = query_lower.split_whitespace().collect();

        let index = self.nextjs_index.read().await;
        let mut results: Vec<(i32, &WebFrameworkSearchEntry)> = index
            .iter()
            .filter_map(|entry| {
                let title_lower = entry.title.to_lowercase();
                let desc_lower = entry.description.to_lowercase();

                let mut score = 0i32;
                for term in &query_terms {
                    if title_lower.contains(term) {
                        score += 15;
                    }
                    if desc_lower.contains(term) {
                        score += 5;
                    }
                }

                if score > 0 {
                    Some((score, entry))
                } else {
                    None
                }
            })
            .collect();

        results.sort_by(|a, b| b.0.cmp(&a.0));

        Ok(results
            .into_iter()
            .take(20)
            .map(|(_, e)| e.clone())
            .collect())
    }

    async fn ensure_nextjs_index(&self) -> Result<()> {
        if !self.nextjs_index.read().await.is_empty() {
            return Ok(());
        }

        if let Ok(Some(entry)) = self
            .disk_cache
            .load::<Vec<WebFrameworkSearchEntry>>("nextjs_index.json")
            .await
        {
            *self.nextjs_index.write().await = entry.value;
            return Ok(());
        }

        let index = self.build_nextjs_index().await;
        let _ = self
            .disk_cache
            .store("nextjs_index.json", index.clone())
            .await;
        *self.nextjs_index.write().await = index;

        Ok(())
    }

    async fn build_nextjs_index(&self) -> Vec<WebFrameworkSearchEntry> {
        // Predefined Next.js docs entries
        vec![
            // App Router
            self.nextjs_entry("docs/app/building-your-application/routing", "Routing", "Define routes and handle navigation in App Router"),
            self.nextjs_entry("docs/app/building-your-application/routing/layouts-and-templates", "Layouts", "Shared UI across multiple pages"),
            self.nextjs_entry("docs/app/building-your-application/routing/pages", "Pages", "Create unique UI for routes"),
            self.nextjs_entry("docs/app/building-your-application/routing/loading-ui-and-streaming", "Loading UI", "Loading and streaming states"),
            self.nextjs_entry("docs/app/building-your-application/routing/error-handling", "Error Handling", "Handle errors in routes"),
            self.nextjs_entry("docs/app/building-your-application/routing/route-handlers", "Route Handlers", "Create API endpoints"),
            self.nextjs_entry("docs/app/building-your-application/routing/middleware", "Middleware", "Run code before request is completed"),
            // Data Fetching
            self.nextjs_entry("docs/app/building-your-application/data-fetching", "Data Fetching", "Fetch data in Server Components"),
            self.nextjs_entry("docs/app/building-your-application/data-fetching/server-actions-and-mutations", "Server Actions", "Server-side form handling and mutations"),
            self.nextjs_entry("docs/app/building-your-application/caching", "Caching", "Caching mechanisms in Next.js"),
            // Rendering
            self.nextjs_entry("docs/app/building-your-application/rendering/server-components", "Server Components", "React Server Components in Next.js"),
            self.nextjs_entry("docs/app/building-your-application/rendering/client-components", "Client Components", "Client-side React components"),
            self.nextjs_entry("docs/app/building-your-application/rendering/composition-patterns", "Composition Patterns", "Server and Client component patterns"),
            // Styling
            self.nextjs_entry("docs/app/building-your-application/styling", "Styling", "Style your Next.js application"),
            self.nextjs_entry("docs/app/building-your-application/styling/css-modules", "CSS Modules", "Locally scoped CSS classes"),
            self.nextjs_entry("docs/app/building-your-application/styling/tailwind-css", "Tailwind CSS", "Using Tailwind with Next.js"),
            // API Reference
            self.nextjs_entry("docs/app/api-reference/components/image", "Image", "Next.js Image component"),
            self.nextjs_entry("docs/app/api-reference/components/link", "Link", "Client-side navigation"),
            self.nextjs_entry("docs/app/api-reference/components/script", "Script", "Load third-party scripts"),
            self.nextjs_entry("docs/app/api-reference/functions/fetch", "fetch", "Extended fetch with caching"),
            self.nextjs_entry("docs/app/api-reference/functions/cookies", "cookies", "Read and set cookies"),
            self.nextjs_entry("docs/app/api-reference/functions/headers", "headers", "Read request headers"),
            self.nextjs_entry("docs/app/api-reference/functions/redirect", "redirect", "Redirect to another URL"),
            self.nextjs_entry("docs/app/api-reference/functions/notFound", "notFound", "Render not found page"),
            self.nextjs_entry("docs/app/api-reference/file-conventions/page", "page.js", "Define a page component"),
            self.nextjs_entry("docs/app/api-reference/file-conventions/layout", "layout.js", "Define a layout component"),
            self.nextjs_entry("docs/app/api-reference/file-conventions/loading", "loading.js", "Define loading UI"),
            self.nextjs_entry("docs/app/api-reference/file-conventions/error", "error.js", "Define error UI"),
            self.nextjs_entry("docs/app/api-reference/next-config-js", "next.config.js", "Configuration options"),
        ]
    }

    fn nextjs_entry(&self, slug: &str, title: &str, description: &str) -> WebFrameworkSearchEntry {
        WebFrameworkSearchEntry {
            framework: WebFramework::NextJs,
            slug: slug.to_string(),
            title: title.to_string(),
            description: description.to_string(),
            url: format!("{}/{}", NEXTJS_BASE, slug),
            category: Some(
                if slug.contains("routing") {
                    "Routing"
                } else if slug.contains("data-fetching") || slug.contains("caching") {
                    "Data"
                } else if slug.contains("api-reference") {
                    "API"
                } else {
                    "Guide"
                }
                .to_string(),
            ),
        }
    }

    async fn fetch_nextjs_article(&self, slug: &str) -> Result<WebFrameworkArticle> {
        let cache_key = format!("nextjs_{}.json", slug.replace('/', "_"));

        if let Ok(Some(entry)) = self
            .disk_cache
            .load::<WebFrameworkArticle>(&cache_key)
            .await
        {
            return Ok(entry.value);
        }

        let url = format!("{}/{}", NEXTJS_BASE, slug);
        debug!(url = %url, "Fetching Next.js article");

        let response = self.http.get(&url).send().await?;
        if !response.status().is_success() {
            anyhow::bail!("Next.js page not found: {}", slug);
        }

        let html = response.text().await?;
        let article = self.parse_nextjs_html(&html, slug, &url);

        let _ = self.disk_cache.store(&cache_key, article.clone()).await;

        Ok(article)
    }

    fn parse_nextjs_html(&self, html: &str, slug: &str, url: &str) -> WebFrameworkArticle {
        let document = Html::parse_document(html);

        let title = self
            .extract_text(&document, "h1")
            .unwrap_or_else(|| slug.split('/').last().unwrap_or("Next.js").to_string());

        let description = self
            .extract_text(&document, "article > p:first-of-type, .description")
            .unwrap_or_default();

        let examples = self.extract_code_examples(&document, "tsx");

        let content = self
            .extract_text(&document, "article, main")
            .map(|s| if s.len() > 4000 { s[..4000].to_string() } else { s })
            .unwrap_or_default();

        WebFrameworkArticle {
            framework: WebFramework::NextJs,
            slug: slug.to_string(),
            title,
            description,
            content,
            examples,
            api_signature: None,
            related: Vec::new(),
            url: url.to_string(),
        }
    }

    // ==================== NODE.JS ====================

    async fn search_nodejs(&self, query: &str) -> Result<Vec<WebFrameworkSearchEntry>> {
        self.ensure_nodejs_index().await?;

        let query_lower = query.to_lowercase();
        let query_terms: Vec<&str> = query_lower.split_whitespace().collect();

        let index = self.nodejs_index.read().await;
        let mut results: Vec<(i32, &WebFrameworkSearchEntry)> = index
            .iter()
            .filter_map(|entry| {
                let title_lower = entry.title.to_lowercase();
                let desc_lower = entry.description.to_lowercase();

                let mut score = 0i32;
                for term in &query_terms {
                    if title_lower.contains(term) {
                        score += 15;
                    }
                    if desc_lower.contains(term) {
                        score += 5;
                    }
                }

                if score > 0 {
                    Some((score, entry))
                } else {
                    None
                }
            })
            .collect();

        results.sort_by(|a, b| b.0.cmp(&a.0));

        Ok(results
            .into_iter()
            .take(20)
            .map(|(_, e)| e.clone())
            .collect())
    }

    async fn ensure_nodejs_index(&self) -> Result<()> {
        if !self.nodejs_index.read().await.is_empty() {
            return Ok(());
        }

        if let Ok(Some(entry)) = self
            .disk_cache
            .load::<Vec<WebFrameworkSearchEntry>>("nodejs_index.json")
            .await
        {
            *self.nodejs_index.write().await = entry.value;
            return Ok(());
        }

        let index = self.build_nodejs_index().await;
        let _ = self
            .disk_cache
            .store("nodejs_index.json", index.clone())
            .await;
        *self.nodejs_index.write().await = index;

        Ok(())
    }

    async fn build_nodejs_index(&self) -> Vec<WebFrameworkSearchEntry> {
        // Try to fetch from Node.js API JSON
        if let Ok(modules) = self.fetch_nodejs_api_json().await {
            return modules
                .iter()
                .map(|m| WebFrameworkSearchEntry {
                    framework: WebFramework::NodeJs,
                    slug: format!("api/{}", m.name),
                    title: m.displayName.clone().unwrap_or_else(|| m.name.clone()),
                    description: m.desc.clone().unwrap_or_default(),
                    url: format!("https://nodejs.org/api/{}.html", m.name),
                    category: Some("Module".to_string()),
                })
                .collect();
        }

        // Fallback to predefined entries
        vec![
            self.nodejs_entry("fs", "File System (fs)", "File system operations"),
            self.nodejs_entry("path", "Path", "File path utilities"),
            self.nodejs_entry("http", "HTTP", "HTTP server and client"),
            self.nodejs_entry("https", "HTTPS", "HTTPS server and client"),
            self.nodejs_entry("stream", "Stream", "Streaming data handling"),
            self.nodejs_entry("buffer", "Buffer", "Binary data handling"),
            self.nodejs_entry("events", "Events", "Event emitter pattern"),
            self.nodejs_entry("child_process", "Child Process", "Spawn child processes"),
            self.nodejs_entry("crypto", "Crypto", "Cryptographic functions"),
            self.nodejs_entry("os", "OS", "Operating system utilities"),
            self.nodejs_entry("url", "URL", "URL parsing and formatting"),
            self.nodejs_entry("querystring", "Query Strings", "Parse and format URL query strings"),
            self.nodejs_entry("util", "Util", "Utility functions"),
            self.nodejs_entry("assert", "Assert", "Assertion testing"),
            self.nodejs_entry("process", "Process", "Process information and control"),
            self.nodejs_entry("net", "Net", "TCP/IPC networking"),
            self.nodejs_entry("dns", "DNS", "DNS lookups"),
            self.nodejs_entry("readline", "Readline", "Read lines from stream"),
            self.nodejs_entry("zlib", "Zlib", "Compression utilities"),
            self.nodejs_entry("cluster", "Cluster", "Multi-process Node.js"),
            self.nodejs_entry("worker_threads", "Worker Threads", "Multi-threaded JavaScript"),
            self.nodejs_entry("async_hooks", "Async Hooks", "Track async resources"),
            self.nodejs_entry("timers", "Timers", "setTimeout, setInterval, etc."),
        ]
    }

    async fn fetch_nodejs_api_json(&self) -> Result<Vec<NodeApiModule>> {
        let response = self.http.get(NODEJS_API_JSON).send().await?;
        if !response.status().is_success() {
            anyhow::bail!("Failed to fetch Node.js API JSON");
        }

        let json: Value = response.json().await?;

        // Parse modules from the JSON structure
        if let Some(modules) = json.get("modules").and_then(|m| m.as_array()) {
            let parsed: Vec<NodeApiModule> = modules
                .iter()
                .filter_map(|m| serde_json::from_value(m.clone()).ok())
                .collect();
            return Ok(parsed);
        }

        anyhow::bail!("Invalid Node.js API JSON structure")
    }

    fn nodejs_entry(&self, name: &str, title: &str, description: &str) -> WebFrameworkSearchEntry {
        WebFrameworkSearchEntry {
            framework: WebFramework::NodeJs,
            slug: format!("api/{}", name),
            title: title.to_string(),
            description: description.to_string(),
            url: format!("https://nodejs.org/api/{}.html", name),
            category: Some("Module".to_string()),
        }
    }

    async fn fetch_nodejs_article(&self, slug: &str) -> Result<WebFrameworkArticle> {
        let cache_key = format!("nodejs_{}.json", slug.replace('/', "_"));

        if let Ok(Some(entry)) = self
            .disk_cache
            .load::<WebFrameworkArticle>(&cache_key)
            .await
        {
            return Ok(entry.value);
        }

        // Extract module name from slug (e.g., "api/fs" -> "fs")
        let module_name = slug.strip_prefix("api/").unwrap_or(slug);
        let url = format!("https://nodejs.org/api/{}.html", module_name);
        debug!(url = %url, "Fetching Node.js article");

        let response = self.http.get(&url).send().await?;
        if !response.status().is_success() {
            anyhow::bail!("Node.js page not found: {}", slug);
        }

        let html = response.text().await?;
        let article = self.parse_nodejs_html(&html, slug, &url);

        let _ = self.disk_cache.store(&cache_key, article.clone()).await;

        Ok(article)
    }

    fn parse_nodejs_html(&self, html: &str, slug: &str, url: &str) -> WebFrameworkArticle {
        let document = Html::parse_document(html);

        let title = self
            .extract_text(&document, "h1, #toc h2:first-of-type")
            .unwrap_or_else(|| slug.split('/').last().unwrap_or("Node.js").to_string());

        let description = self
            .extract_text(&document, "#apicontent > p:first-of-type, .api_stability + p")
            .unwrap_or_default();

        let examples = self.extract_code_examples(&document, "javascript");

        let content = self
            .extract_text(&document, "#apicontent")
            .map(|s| if s.len() > 4000 { s[..4000].to_string() } else { s })
            .unwrap_or_default();

        WebFrameworkArticle {
            framework: WebFramework::NodeJs,
            slug: slug.to_string(),
            title,
            description,
            content,
            examples,
            api_signature: None,
            related: Vec::new(),
            url: url.to_string(),
        }
    }

    // ==================== HELPERS ====================

    fn extract_text(&self, document: &Html, selector_str: &str) -> Option<String> {
        if let Ok(selector) = Selector::parse(selector_str) {
            document
                .select(&selector)
                .next()
                .map(|e| e.text().collect::<String>().trim().to_string())
                .filter(|s| !s.is_empty())
        } else {
            None
        }
    }

    fn extract_code_examples(&self, document: &Html, default_lang: &str) -> Vec<CodeExample> {
        let mut examples = Vec::new();

        let selectors = [
            "pre code",
            "pre.language-js",
            "pre.language-jsx",
            "pre.language-tsx",
            "pre.language-typescript",
            ".code-block pre",
            ".highlight pre",
        ];

        for selector_str in selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                for element in document.select(&selector) {
                    let code = element.text().collect::<String>().trim().to_string();
                    if code.is_empty() || code.len() < 20 {
                        continue;
                    }

                    // Determine language from class
                    let class = element.value().attr("class").unwrap_or("");
                    let language = if class.contains("tsx") || class.contains("typescript") {
                        "typescript"
                    } else if class.contains("jsx") {
                        "jsx"
                    } else if class.contains("js") || class.contains("javascript") {
                        "javascript"
                    } else {
                        default_lang
                    };

                    // Get description from preceding element
                    let description = element
                        .prev_siblings()
                        .filter_map(scraper::ElementRef::wrap)
                        .find(|e| e.value().name() == "p")
                        .map(|e| e.text().collect::<String>().trim().to_string());

                    // Get filename if present
                    let filename = element
                        .prev_siblings()
                        .filter_map(scraper::ElementRef::wrap)
                        .find(|e| {
                            e.value().name() == "div"
                                && e.value()
                                    .attr("class")
                                    .map(|c| c.contains("filename"))
                                    .unwrap_or(false)
                        })
                        .map(|e| e.text().collect::<String>().trim().to_string());

                    let is_complete = code.contains("import ") || code.contains("export ");
                    let has_output =
                        code.contains("console.log") || code.contains("// Output:");

                    examples.push(CodeExample {
                        code,
                        language: language.to_string(),
                        filename,
                        description,
                        is_complete,
                        has_output,
                    });

                    if examples.len() >= 5 {
                        break;
                    }
                }
            }

            if examples.len() >= 5 {
                break;
            }
        }

        // Sort by quality score
        examples.sort_by(|a, b| b.quality_score().cmp(&a.quality_score()));

        examples
    }

    pub fn cache_dir(&self) -> &PathBuf {
        &self.cache_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let _client = WebFrameworksClient::new();
    }
}
