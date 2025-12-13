use std::cmp::Reverse;
use std::path::PathBuf;
use std::time::Duration as StdDuration;

use anyhow::Result;
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
const BUN_BASE: &str = "https://bun.sh";

#[derive(Debug)]
pub struct WebFrameworksClient {
    http: Client,
    disk_cache: DiskCache,
    memory_cache: MemoryCache<Vec<u8>>,
    /// Search indexes per framework
    react_index: RwLock<Vec<WebFrameworkSearchEntry>>,
    nextjs_index: RwLock<Vec<WebFrameworkSearchEntry>>,
    nodejs_index: RwLock<Vec<WebFrameworkSearchEntry>>,
    bun_index: RwLock<Vec<WebFrameworkSearchEntry>>,
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
            bun_index: RwLock::new(Vec::new()),
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
            WebFramework::Bun => self.search_bun(query).await,
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
            WebFramework::Bun => self.fetch_bun_article(slug).await,
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
        let index = self.build_react_index();
        let _ = self
            .disk_cache
            .store("react_index.json", index.clone())
            .await;
        *self.react_index.write().await = index;

        Ok(())
    }

    /// Build React search index
    fn build_react_index(&self) -> Vec<WebFrameworkSearchEntry> {
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

        let index = self.build_nextjs_index();
        let _ = self
            .disk_cache
            .store("nextjs_index.json", index.clone())
            .await;
        *self.nextjs_index.write().await = index;

        Ok(())
    }

    fn build_nextjs_index(&self) -> Vec<WebFrameworkSearchEntry> {
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
                    title: m.display_name.clone().unwrap_or_else(|| m.name.clone()),
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

    // ==================== BUN ====================

    /// Search Bun documentation
    async fn search_bun(&self, query: &str) -> Result<Vec<WebFrameworkSearchEntry>> {
        self.ensure_bun_index().await?;

        let query_lower = query.to_lowercase();
        let query_terms: Vec<&str> = query_lower.split_whitespace().collect();

        let index = self.bun_index.read().await;
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

    async fn ensure_bun_index(&self) -> Result<()> {
        if !self.bun_index.read().await.is_empty() {
            return Ok(());
        }

        if let Ok(Some(entry)) = self
            .disk_cache
            .load::<Vec<WebFrameworkSearchEntry>>("bun_index.json")
            .await
        {
            *self.bun_index.write().await = entry.value;
            return Ok(());
        }

        let index = self.build_bun_index();
        let _ = self
            .disk_cache
            .store("bun_index.json", index.clone())
            .await;
        *self.bun_index.write().await = index;

        Ok(())
    }

    /// Build comprehensive Bun search index
    fn build_bun_index(&self) -> Vec<WebFrameworkSearchEntry> {
        vec![
            // ==================== Runtime APIs ====================
            // Bun global object
            self.bun_entry("docs/api/bun", "Bun", "The Bun global namespace with runtime APIs", "Runtime"),
            self.bun_entry("docs/api/bun#bun-version", "Bun.version", "Get the current Bun version string", "Runtime"),
            self.bun_entry("docs/api/bun#bun-revision", "Bun.revision", "Get the git commit of the Bun build", "Runtime"),
            self.bun_entry("docs/api/bun#bun-env", "Bun.env", "Access environment variables (like process.env)", "Runtime"),
            self.bun_entry("docs/api/bun#bun-main", "Bun.main", "Get the path of the entrypoint script", "Runtime"),
            self.bun_entry("docs/api/bun#bun-sleep", "Bun.sleep", "Sleep for a specified duration (async)", "Runtime"),
            self.bun_entry("docs/api/bun#bun-sleepsync", "Bun.sleepSync", "Sleep synchronously for a duration", "Runtime"),
            self.bun_entry("docs/api/bun#bun-which", "Bun.which", "Find the path to an executable", "Runtime"),
            self.bun_entry("docs/api/bun#bun-peek", "Bun.peek", "Read a promise's value without awaiting", "Runtime"),
            self.bun_entry("docs/api/bun#bun-openineditor", "Bun.openInEditor", "Open a file in the default editor", "Runtime"),
            self.bun_entry("docs/api/bun#bun-deepequals", "Bun.deepEquals", "Deep equality comparison", "Runtime"),
            self.bun_entry("docs/api/bun#bun-escapehtml", "Bun.escapeHTML", "Escape HTML entities in a string", "Runtime"),
            self.bun_entry("docs/api/bun#bun-stringwidth", "Bun.stringWidth", "Get display width of a string", "Runtime"),
            self.bun_entry("docs/api/bun#bun-arraybuffersink", "Bun.ArrayBufferSink", "Streaming sink for ArrayBuffers", "Runtime"),
            self.bun_entry("docs/api/bun#bun-pathtofileurl", "Bun.pathToFileURL", "Convert a file path to file:// URL", "Runtime"),
            self.bun_entry("docs/api/bun#bun-fileURLToPath", "Bun.fileURLToPath", "Convert file:// URL to file path", "Runtime"),
            self.bun_entry("docs/api/bun#bun-gc", "Bun.gc", "Trigger garbage collection manually", "Runtime"),
            self.bun_entry("docs/api/bun#bun-generateheapdiff", "Bun.generateHeapDiff", "Generate heap snapshot diff", "Runtime"),
            self.bun_entry("docs/api/bun#bun-shrink", "Bun.shrink", "Shrink memory usage", "Runtime"),
            self.bun_entry("docs/api/bun#bun-inspect", "Bun.inspect", "Format a value for debugging output", "Runtime"),
            self.bun_entry("docs/api/bun#bun-nanoseconds", "Bun.nanoseconds", "High-resolution nanosecond timer", "Runtime"),
            self.bun_entry("docs/api/bun#bun-readableStreamToArray", "Bun.readableStreamToArray", "Convert ReadableStream to array", "Runtime"),
            self.bun_entry("docs/api/bun#bun-readableStreamToArrayBuffer", "Bun.readableStreamToArrayBuffer", "Convert ReadableStream to ArrayBuffer", "Runtime"),
            self.bun_entry("docs/api/bun#bun-readableStreamToBlob", "Bun.readableStreamToBlob", "Convert ReadableStream to Blob", "Runtime"),
            self.bun_entry("docs/api/bun#bun-readableStreamToJSON", "Bun.readableStreamToJSON", "Convert ReadableStream to JSON", "Runtime"),
            self.bun_entry("docs/api/bun#bun-readableStreamToText", "Bun.readableStreamToText", "Convert ReadableStream to text", "Runtime"),
            self.bun_entry("docs/api/bun#bun-resolveSync", "Bun.resolveSync", "Resolve a module path synchronously", "Runtime"),
            self.bun_entry("docs/api/bun#bun-resolve", "Bun.resolve", "Resolve a module path asynchronously", "Runtime"),

            // ==================== File I/O ====================
            self.bun_entry("docs/api/file-io", "File I/O", "Bun's optimized file I/O APIs", "File I/O"),
            self.bun_entry("docs/api/file-io#bun-file", "Bun.file", "Create a lazy file reference (BunFile)", "File I/O"),
            self.bun_entry("docs/api/file-io#bun-write", "Bun.write", "Write data to a file or BunFile", "File I/O"),
            self.bun_entry("docs/api/file-io#bunfile", "BunFile", "Lazy file reference with streaming support", "File I/O"),
            self.bun_entry("docs/api/file-io#bunfile-text", "BunFile.text()", "Read file contents as text", "File I/O"),
            self.bun_entry("docs/api/file-io#bunfile-json", "BunFile.json()", "Read and parse file as JSON", "File I/O"),
            self.bun_entry("docs/api/file-io#bunfile-arraybuffer", "BunFile.arrayBuffer()", "Read file as ArrayBuffer", "File I/O"),
            self.bun_entry("docs/api/file-io#bunfile-bytes", "BunFile.bytes()", "Read file as Uint8Array", "File I/O"),
            self.bun_entry("docs/api/file-io#bunfile-stream", "BunFile.stream()", "Get file as ReadableStream", "File I/O"),
            self.bun_entry("docs/api/file-io#bunfile-writer", "BunFile.writer()", "Get a FileSink for streaming writes", "File I/O"),
            self.bun_entry("docs/api/file-io#bunfile-slice", "BunFile.slice()", "Get a slice of the file", "File I/O"),
            self.bun_entry("docs/api/file-io#bunfile-exists", "BunFile.exists()", "Check if the file exists", "File I/O"),

            // ==================== HTTP Server ====================
            self.bun_entry("docs/api/http", "HTTP Server", "Bun's fast HTTP server with Bun.serve()", "HTTP"),
            self.bun_entry("docs/api/http#bun-serve", "Bun.serve", "Start an HTTP/HTTPS server", "HTTP"),
            self.bun_entry("docs/api/http#request", "Request", "Incoming HTTP request object", "HTTP"),
            self.bun_entry("docs/api/http#response", "Response", "HTTP response object", "HTTP"),
            self.bun_entry("docs/api/http#server", "Server", "HTTP server instance from Bun.serve()", "HTTP"),
            self.bun_entry("docs/api/http#fetch-handler", "fetch handler", "Request handler function for Bun.serve()", "HTTP"),
            self.bun_entry("docs/api/http#error-handler", "error handler", "Error handling in Bun.serve()", "HTTP"),
            self.bun_entry("docs/api/http#tls", "TLS/HTTPS", "Configure TLS for HTTPS servers", "HTTP"),
            self.bun_entry("docs/api/http#unix-sockets", "Unix Sockets", "Listen on Unix domain sockets", "HTTP"),
            self.bun_entry("docs/api/http#streaming", "Streaming Responses", "Stream responses with ReadableStream", "HTTP"),
            self.bun_entry("docs/api/http#hot-reloading", "Hot Reloading", "Hot reload the server on file changes", "HTTP"),

            // ==================== WebSocket ====================
            self.bun_entry("docs/api/websockets", "WebSocket Server", "Built-in WebSocket server with pub/sub", "WebSocket"),
            self.bun_entry("docs/api/websockets#server-websocket", "ServerWebSocket", "Server-side WebSocket connection", "WebSocket"),
            self.bun_entry("docs/api/websockets#websocket-handlers", "WebSocket Handlers", "open, message, close, drain handlers", "WebSocket"),
            self.bun_entry("docs/api/websockets#publish-subscribe", "Pub/Sub", "Built-in publish/subscribe for WebSockets", "WebSocket"),
            self.bun_entry("docs/api/websockets#serverwebsocket-send", "ServerWebSocket.send()", "Send data to a WebSocket client", "WebSocket"),
            self.bun_entry("docs/api/websockets#serverwebsocket-publish", "ServerWebSocket.publish()", "Publish to a topic", "WebSocket"),
            self.bun_entry("docs/api/websockets#serverwebsocket-subscribe", "ServerWebSocket.subscribe()", "Subscribe to a topic", "WebSocket"),
            self.bun_entry("docs/api/websockets#serverwebsocket-unsubscribe", "ServerWebSocket.unsubscribe()", "Unsubscribe from a topic", "WebSocket"),
            self.bun_entry("docs/api/websockets#serverwebsocket-close", "ServerWebSocket.close()", "Close the WebSocket connection", "WebSocket"),
            self.bun_entry("docs/api/websockets#compression", "WebSocket Compression", "Per-message deflate compression", "WebSocket"),
            self.bun_entry("docs/api/websockets#backpressure", "WebSocket Backpressure", "Handle slow consumers with drain", "WebSocket"),

            // ==================== Workers ====================
            self.bun_entry("docs/api/workers", "Workers", "Web Workers and multi-threading in Bun", "Workers"),
            self.bun_entry("docs/api/workers#worker", "Worker", "Create a new Web Worker", "Workers"),
            self.bun_entry("docs/api/workers#worker-postMessage", "Worker.postMessage()", "Send message to worker", "Workers"),
            self.bun_entry("docs/api/workers#worker-terminate", "Worker.terminate()", "Terminate a worker", "Workers"),
            self.bun_entry("docs/api/workers#structured-clone", "Structured Clone", "Data serialization for workers", "Workers"),
            self.bun_entry("docs/api/workers#sharedarraybuffer", "SharedArrayBuffer", "Share memory between workers", "Workers"),
            self.bun_entry("docs/api/workers#atomics", "Atomics", "Atomic operations on SharedArrayBuffer", "Workers"),

            // ==================== Spawn (Child Processes) ====================
            self.bun_entry("docs/api/spawn", "Spawn", "Run child processes with Bun.spawn()", "Spawn"),
            self.bun_entry("docs/api/spawn#bun-spawn", "Bun.spawn", "Spawn a child process", "Spawn"),
            self.bun_entry("docs/api/spawn#bun-spawnSync", "Bun.spawnSync", "Spawn a child process synchronously", "Spawn"),
            self.bun_entry("docs/api/spawn#subprocess", "Subprocess", "Child process handle", "Spawn"),
            self.bun_entry("docs/api/spawn#subprocess-stdin", "Subprocess.stdin", "Write to child process stdin", "Spawn"),
            self.bun_entry("docs/api/spawn#subprocess-stdout", "Subprocess.stdout", "Read from child process stdout", "Spawn"),
            self.bun_entry("docs/api/spawn#subprocess-stderr", "Subprocess.stderr", "Read from child process stderr", "Spawn"),
            self.bun_entry("docs/api/spawn#subprocess-exited", "Subprocess.exited", "Promise that resolves on exit", "Spawn"),
            self.bun_entry("docs/api/spawn#subprocess-kill", "Subprocess.kill()", "Kill the child process", "Spawn"),
            self.bun_entry("docs/api/spawn#ipc", "IPC", "Inter-process communication", "Spawn"),
            self.bun_entry("docs/api/spawn#$ shell", "$ (shell)", "Shell scripting with tagged template", "Spawn"),

            // ==================== TCP/UDP Sockets ====================
            self.bun_entry("docs/api/tcp", "TCP Sockets", "Low-level TCP socket API", "Sockets"),
            self.bun_entry("docs/api/tcp#bun-listen", "Bun.listen", "Create a TCP server", "Sockets"),
            self.bun_entry("docs/api/tcp#bun-connect", "Bun.connect", "Connect as TCP client", "Sockets"),
            self.bun_entry("docs/api/tcp#socket", "Socket", "TCP socket instance", "Sockets"),
            self.bun_entry("docs/api/tcp#socket-write", "Socket.write()", "Write data to socket", "Sockets"),
            self.bun_entry("docs/api/tcp#socket-end", "Socket.end()", "Close the socket gracefully", "Sockets"),
            self.bun_entry("docs/api/tcp#socket-flush", "Socket.flush()", "Flush buffered data", "Sockets"),
            self.bun_entry("docs/api/tcp#socket-handlers", "Socket Handlers", "open, data, close, error handlers", "Sockets"),
            self.bun_entry("docs/api/udp", "UDP Sockets", "Low-level UDP socket API", "Sockets"),
            self.bun_entry("docs/api/udp#bun-udpsocket", "Bun.udpSocket", "Create a UDP socket", "Sockets"),

            // ==================== SQLite ====================
            self.bun_entry("docs/api/sqlite", "SQLite", "Built-in SQLite database driver", "Database"),
            self.bun_entry("docs/api/sqlite#database", "Database", "SQLite database connection", "Database"),
            self.bun_entry("docs/api/sqlite#database-query", "Database.query()", "Create a prepared statement", "Database"),
            self.bun_entry("docs/api/sqlite#database-run", "Database.run()", "Execute SQL without returning rows", "Database"),
            self.bun_entry("docs/api/sqlite#database-exec", "Database.exec()", "Execute multiple SQL statements", "Database"),
            self.bun_entry("docs/api/sqlite#database-prepare", "Database.prepare()", "Create a prepared statement", "Database"),
            self.bun_entry("docs/api/sqlite#database-transaction", "Database.transaction()", "Create a transaction function", "Database"),
            self.bun_entry("docs/api/sqlite#statement", "Statement", "Prepared SQL statement", "Database"),
            self.bun_entry("docs/api/sqlite#statement-get", "Statement.get()", "Get a single row", "Database"),
            self.bun_entry("docs/api/sqlite#statement-all", "Statement.all()", "Get all matching rows", "Database"),
            self.bun_entry("docs/api/sqlite#statement-run", "Statement.run()", "Execute without returning rows", "Database"),
            self.bun_entry("docs/api/sqlite#statement-values", "Statement.values()", "Get rows as arrays", "Database"),
            self.bun_entry("docs/api/sqlite#statement-columns", "Statement.columns()", "Get column information", "Database"),

            // ==================== Hashing & Crypto ====================
            self.bun_entry("docs/api/hashing", "Hashing", "Fast cryptographic hashing APIs", "Crypto"),
            self.bun_entry("docs/api/hashing#bun-hash", "Bun.hash", "Fast non-cryptographic hash (Wyhash)", "Crypto"),
            self.bun_entry("docs/api/hashing#bun-hash-cityHash32", "Bun.hash.cityHash32", "CityHash32 hashing", "Crypto"),
            self.bun_entry("docs/api/hashing#bun-hash-cityHash64", "Bun.hash.cityHash64", "CityHash64 hashing", "Crypto"),
            self.bun_entry("docs/api/hashing#bun-hash-murmur32v3", "Bun.hash.murmur32v3", "MurmurHash3 32-bit", "Crypto"),
            self.bun_entry("docs/api/hashing#bun-hash-murmur64v2", "Bun.hash.murmur64v2", "MurmurHash2 64-bit", "Crypto"),
            self.bun_entry("docs/api/hashing#bun-hash-adler32", "Bun.hash.adler32", "Adler-32 checksum", "Crypto"),
            self.bun_entry("docs/api/hashing#bun-hash-crc32", "Bun.hash.crc32", "CRC32 checksum", "Crypto"),
            self.bun_entry("docs/api/hashing#bun-cryptohashers", "Bun.CryptoHasher", "Streaming cryptographic hashers", "Crypto"),
            self.bun_entry("docs/api/hashing#md5", "Bun.CryptoHasher MD5", "MD5 hash algorithm", "Crypto"),
            self.bun_entry("docs/api/hashing#sha1", "Bun.CryptoHasher SHA-1", "SHA-1 hash algorithm", "Crypto"),
            self.bun_entry("docs/api/hashing#sha256", "Bun.CryptoHasher SHA-256", "SHA-256 hash algorithm", "Crypto"),
            self.bun_entry("docs/api/hashing#sha512", "Bun.CryptoHasher SHA-512", "SHA-512 hash algorithm", "Crypto"),
            self.bun_entry("docs/api/hashing#bun-password", "Bun.password", "Password hashing with bcrypt/argon2", "Crypto"),
            self.bun_entry("docs/api/hashing#bun-password-hash", "Bun.password.hash()", "Hash a password", "Crypto"),
            self.bun_entry("docs/api/hashing#bun-password-hashSync", "Bun.password.hashSync()", "Hash a password synchronously", "Crypto"),
            self.bun_entry("docs/api/hashing#bun-password-verify", "Bun.password.verify()", "Verify a password hash", "Crypto"),
            self.bun_entry("docs/api/hashing#bun-password-verifySync", "Bun.password.verifySync()", "Verify a password synchronously", "Crypto"),

            // ==================== Streams ====================
            self.bun_entry("docs/api/streams", "Streams", "Web Streams API with Bun optimizations", "Streams"),
            self.bun_entry("docs/api/streams#readablestream", "ReadableStream", "Readable byte stream", "Streams"),
            self.bun_entry("docs/api/streams#writablestream", "WritableStream", "Writable byte stream", "Streams"),
            self.bun_entry("docs/api/streams#transformstream", "TransformStream", "Transform stream pipeline", "Streams"),
            self.bun_entry("docs/api/streams#direct-readablestream", "Direct ReadableStream", "High-performance direct streams", "Streams"),
            self.bun_entry("docs/api/streams#filesink", "FileSink", "Streaming file writer", "Streams"),

            // ==================== Bundler ====================
            self.bun_entry("docs/bundler", "Bundler", "Bun's built-in JavaScript/TypeScript bundler", "Bundler"),
            self.bun_entry("docs/bundler#bun-build", "Bun.build", "Bundle JavaScript/TypeScript files", "Bundler"),
            self.bun_entry("docs/bundler#entrypoints", "entrypoints", "Entry point files for bundling", "Bundler"),
            self.bun_entry("docs/bundler#outdir", "outdir", "Output directory for bundles", "Bundler"),
            self.bun_entry("docs/bundler#target", "target", "Target environment (browser, bun, node)", "Bundler"),
            self.bun_entry("docs/bundler#format", "format", "Output format (esm, cjs, iife)", "Bundler"),
            self.bun_entry("docs/bundler#splitting", "splitting", "Enable code splitting", "Bundler"),
            self.bun_entry("docs/bundler#plugins", "plugins", "Bundler plugin system", "Bundler"),
            self.bun_entry("docs/bundler#sourcemap", "sourcemap", "Generate source maps", "Bundler"),
            self.bun_entry("docs/bundler#minify", "minify", "Minify output code", "Bundler"),
            self.bun_entry("docs/bundler#external", "external", "Mark dependencies as external", "Bundler"),
            self.bun_entry("docs/bundler#define", "define", "Replace global identifiers", "Bundler"),
            self.bun_entry("docs/bundler#loader", "loader", "Configure file type loaders", "Bundler"),
            self.bun_entry("docs/bundler#naming", "naming", "Configure output file naming", "Bundler"),
            self.bun_entry("docs/bundler/loaders", "Loaders", "Built-in loaders (js, ts, jsx, tsx, json, etc.)", "Bundler"),
            self.bun_entry("docs/bundler/plugins", "Plugin API", "Create custom bundler plugins", "Bundler"),
            self.bun_entry("docs/bundler/executables", "Executables", "Compile to standalone executables", "Bundler"),

            // ==================== Test Runner ====================
            self.bun_entry("docs/cli/test", "Test Runner", "Built-in test runner (bun test)", "Testing"),
            self.bun_entry("docs/cli/test#writing-tests", "Writing Tests", "describe, test, it, expect", "Testing"),
            self.bun_entry("docs/cli/test#expect", "expect", "Assertion library", "Testing"),
            self.bun_entry("docs/cli/test#matchers", "Matchers", "toBe, toEqual, toMatch, etc.", "Testing"),
            self.bun_entry("docs/cli/test#lifecycle", "Lifecycle Hooks", "beforeAll, afterAll, beforeEach, afterEach", "Testing"),
            self.bun_entry("docs/cli/test#mock", "Mocking", "Mock functions and modules", "Testing"),
            self.bun_entry("docs/cli/test#spyOn", "spyOn", "Spy on object methods", "Testing"),
            self.bun_entry("docs/cli/test#mock-module", "mock.module", "Mock module imports", "Testing"),
            self.bun_entry("docs/cli/test#snapshots", "Snapshots", "Snapshot testing", "Testing"),
            self.bun_entry("docs/cli/test#coverage", "Code Coverage", "Test coverage reports", "Testing"),
            self.bun_entry("docs/cli/test#dom", "DOM Testing", "happy-dom for DOM testing", "Testing"),
            self.bun_entry("docs/cli/test#timeout", "Timeouts", "Configure test timeouts", "Testing"),
            self.bun_entry("docs/cli/test#skip-todo", "skip/todo", "Skip or mark tests as todo", "Testing"),
            self.bun_entry("docs/cli/test#watch", "Watch Mode", "Re-run tests on file changes", "Testing"),
            self.bun_entry("docs/cli/test#preload", "Preload Scripts", "Run setup before tests", "Testing"),

            // ==================== Package Manager ====================
            self.bun_entry("docs/cli/install", "Package Manager", "Fast npm-compatible package manager", "Package Manager"),
            self.bun_entry("docs/cli/install#bun-install", "bun install", "Install all dependencies", "Package Manager"),
            self.bun_entry("docs/cli/add", "bun add", "Add a dependency", "Package Manager"),
            self.bun_entry("docs/cli/remove", "bun remove", "Remove a dependency", "Package Manager"),
            self.bun_entry("docs/cli/update", "bun update", "Update dependencies", "Package Manager"),
            self.bun_entry("docs/cli/link", "bun link", "Link local packages", "Package Manager"),
            self.bun_entry("docs/cli/pm", "bun pm", "Package manager utilities", "Package Manager"),
            self.bun_entry("docs/install/lockfile", "bun.lockb", "Binary lockfile format", "Package Manager"),
            self.bun_entry("docs/install/workspaces", "Workspaces", "Monorepo workspace support", "Package Manager"),
            self.bun_entry("docs/install/cache", "Global Cache", "Global package cache", "Package Manager"),
            self.bun_entry("docs/install/lifecycle", "Lifecycle Scripts", "postinstall, preinstall hooks", "Package Manager"),
            self.bun_entry("docs/install/registries", "Registries", "Configure npm registries", "Package Manager"),
            self.bun_entry("docs/install/overrides", "Overrides", "Override dependency versions", "Package Manager"),
            self.bun_entry("docs/install/patch", "Patch", "Patch dependencies locally", "Package Manager"),
            self.bun_entry("docs/install/optional", "Optional Dependencies", "Handle optional dependencies", "Package Manager"),
            self.bun_entry("docs/install/trusted", "Trusted Dependencies", "Security for lifecycle scripts", "Package Manager"),

            // ==================== CLI Commands ====================
            self.bun_entry("docs/cli/run", "bun run", "Run JavaScript/TypeScript files", "CLI"),
            self.bun_entry("docs/cli/run#scripts", "bun run (scripts)", "Run package.json scripts", "CLI"),
            self.bun_entry("docs/cli/build", "bun build", "Bundle for production", "CLI"),
            self.bun_entry("docs/cli/create", "bun create", "Scaffold a new project", "CLI"),
            self.bun_entry("docs/cli/init", "bun init", "Initialize a new project", "CLI"),
            self.bun_entry("docs/cli/upgrade", "bun upgrade", "Upgrade Bun to latest version", "CLI"),
            self.bun_entry("docs/cli/repl", "bun repl", "Interactive JavaScript REPL", "CLI"),
            self.bun_entry("docs/cli/bunx", "bunx", "Execute packages (like npx)", "CLI"),
            self.bun_entry("docs/cli/completions", "Shell Completions", "Enable shell autocompletions", "CLI"),

            // ==================== Configuration ====================
            self.bun_entry("docs/runtime/bunfig", "bunfig.toml", "Bun configuration file", "Config"),
            self.bun_entry("docs/runtime/bunfig#runtime", "Runtime Config", "Runtime configuration options", "Config"),
            self.bun_entry("docs/runtime/bunfig#install", "Install Config", "Package manager configuration", "Config"),
            self.bun_entry("docs/runtime/bunfig#test", "Test Config", "Test runner configuration", "Config"),
            self.bun_entry("docs/runtime/bunfig#run", "Run Config", "Script runner configuration", "Config"),

            // ==================== TypeScript ====================
            self.bun_entry("docs/runtime/typescript", "TypeScript", "First-class TypeScript support", "TypeScript"),
            self.bun_entry("docs/runtime/typescript#tsconfig", "tsconfig.json", "TypeScript configuration", "TypeScript"),
            self.bun_entry("docs/runtime/typescript#path-mapping", "Path Mapping", "TypeScript path aliases", "TypeScript"),
            self.bun_entry("docs/runtime/typescript#declaration-files", "Declaration Files", ".d.ts file handling", "TypeScript"),

            // ==================== JSX ====================
            self.bun_entry("docs/runtime/jsx", "JSX", "Built-in JSX/TSX support", "JSX"),
            self.bun_entry("docs/runtime/jsx#react", "React JSX", "JSX with React runtime", "JSX"),
            self.bun_entry("docs/runtime/jsx#solid", "Solid JSX", "JSX with Solid runtime", "JSX"),
            self.bun_entry("docs/runtime/jsx#automatic", "Automatic JSX", "Automatic JSX runtime", "JSX"),
            self.bun_entry("docs/runtime/jsx#classic", "Classic JSX", "Classic JSX transform", "JSX"),

            // ==================== Module Resolution ====================
            self.bun_entry("docs/runtime/modules", "Module Resolution", "How Bun resolves modules", "Modules"),
            self.bun_entry("docs/runtime/modules#esm", "ES Modules", "ESM import/export support", "Modules"),
            self.bun_entry("docs/runtime/modules#commonjs", "CommonJS", "CommonJS require() support", "Modules"),
            self.bun_entry("docs/runtime/modules#resolution", "Resolution Algorithm", "Module resolution algorithm", "Modules"),
            self.bun_entry("docs/runtime/modules#file-types", "File Types", "Supported file extensions", "Modules"),
            self.bun_entry("docs/runtime/modules#import-meta", "import.meta", "Module metadata object", "Modules"),

            // ==================== Environment Variables ====================
            self.bun_entry("docs/runtime/env", "Environment Variables", "Environment variable handling", "Runtime"),
            self.bun_entry("docs/runtime/env#dotenv", ".env Files", "Automatic .env file loading", "Runtime"),
            self.bun_entry("docs/runtime/env#bun-env", "Bun.env", "Access environment variables", "Runtime"),
            self.bun_entry("docs/runtime/env#process-env", "process.env", "Node.js-compatible process.env", "Runtime"),

            // ==================== Hot Reloading ====================
            self.bun_entry("docs/runtime/hot", "Hot Reloading", "Hot module reloading with --hot", "Runtime"),
            self.bun_entry("docs/runtime/hot#watch", "Watch Mode", "Watch mode with --watch", "Runtime"),

            // ==================== FFI ====================
            self.bun_entry("docs/api/ffi", "FFI", "Foreign Function Interface", "FFI"),
            self.bun_entry("docs/api/ffi#dlopen", "dlopen", "Load native libraries", "FFI"),
            self.bun_entry("docs/api/ffi#cc", "cc", "Compile and run C code", "FFI"),
            self.bun_entry("docs/api/ffi#types", "FFI Types", "C type mappings", "FFI"),
            self.bun_entry("docs/api/ffi#ptr", "ptr", "Create pointers from buffers", "FFI"),
            self.bun_entry("docs/api/ffi#cstring", "CString", "Handle C strings", "FFI"),
            self.bun_entry("docs/api/ffi#callback", "Callback", "Create C callbacks from JS functions", "FFI"),

            // ==================== Console & Logging ====================
            self.bun_entry("docs/api/console", "Console", "Console API with colors", "Logging"),
            self.bun_entry("docs/api/console#console-log", "console.log", "Log to stdout", "Logging"),
            self.bun_entry("docs/api/console#console-error", "console.error", "Log to stderr", "Logging"),
            self.bun_entry("docs/api/console#console-table", "console.table", "Log data as table", "Logging"),
            self.bun_entry("docs/api/console#console-time", "console.time", "Performance timing", "Logging"),

            // ==================== Globals ====================
            self.bun_entry("docs/api/globals", "Globals", "Global objects and functions", "Globals"),
            self.bun_entry("docs/api/globals#fetch", "fetch", "HTTP fetch API", "Globals"),
            self.bun_entry("docs/api/globals#TextEncoder", "TextEncoder", "Encode strings to bytes", "Globals"),
            self.bun_entry("docs/api/globals#TextDecoder", "TextDecoder", "Decode bytes to strings", "Globals"),
            self.bun_entry("docs/api/globals#atob-btoa", "atob/btoa", "Base64 encoding/decoding", "Globals"),
            self.bun_entry("docs/api/globals#performance", "performance", "High-resolution timing", "Globals"),
            self.bun_entry("docs/api/globals#crypto", "crypto", "Web Crypto API", "Globals"),
            self.bun_entry("docs/api/globals#structuredClone", "structuredClone", "Deep clone objects", "Globals"),
            self.bun_entry("docs/api/globals#queueMicrotask", "queueMicrotask", "Queue a microtask", "Globals"),
            self.bun_entry("docs/api/globals#reportError", "reportError", "Report unhandled errors", "Globals"),
            self.bun_entry("docs/api/globals#setImmediate", "setImmediate", "Run callback immediately", "Globals"),
            self.bun_entry("docs/api/globals#navigator", "navigator", "Navigator API", "Globals"),
            self.bun_entry("docs/api/globals#alert-confirm-prompt", "alert/confirm/prompt", "Dialog functions", "Globals"),
            self.bun_entry("docs/api/globals#Blob", "Blob", "Binary large object", "Globals"),
            self.bun_entry("docs/api/globals#File", "File", "File object (extends Blob)", "Globals"),
            self.bun_entry("docs/api/globals#FormData", "FormData", "Form data for HTTP requests", "Globals"),
            self.bun_entry("docs/api/globals#Headers", "Headers", "HTTP headers", "Globals"),
            self.bun_entry("docs/api/globals#URL", "URL", "URL parsing and manipulation", "Globals"),
            self.bun_entry("docs/api/globals#URLSearchParams", "URLSearchParams", "Query string handling", "Globals"),

            // ==================== Transpiler ====================
            self.bun_entry("docs/api/transpiler", "Transpiler", "Built-in JavaScript/TypeScript transpiler", "Transpiler"),
            self.bun_entry("docs/api/transpiler#bun-transpiler", "Bun.Transpiler", "Transpiler class", "Transpiler"),
            self.bun_entry("docs/api/transpiler#transform", "transform", "Transform code synchronously", "Transpiler"),
            self.bun_entry("docs/api/transpiler#transformsync", "transformSync", "Transform code asynchronously", "Transpiler"),
            self.bun_entry("docs/api/transpiler#scan", "scan", "Scan for imports/exports", "Transpiler"),
            self.bun_entry("docs/api/transpiler#scanImports", "scanImports", "Get all imports from code", "Transpiler"),

            // ==================== Node.js Compatibility ====================
            self.bun_entry("docs/runtime/nodejs-apis", "Node.js APIs", "Node.js API compatibility", "Node.js"),
            self.bun_entry("docs/runtime/nodejs-apis#fs", "fs", "File system compatibility", "Node.js"),
            self.bun_entry("docs/runtime/nodejs-apis#path", "path", "Path utilities compatibility", "Node.js"),
            self.bun_entry("docs/runtime/nodejs-apis#http", "http/https", "HTTP server compatibility", "Node.js"),
            self.bun_entry("docs/runtime/nodejs-apis#crypto", "crypto", "Crypto module compatibility", "Node.js"),
            self.bun_entry("docs/runtime/nodejs-apis#buffer", "Buffer", "Buffer class compatibility", "Node.js"),
            self.bun_entry("docs/runtime/nodejs-apis#process", "process", "Process object compatibility", "Node.js"),
            self.bun_entry("docs/runtime/nodejs-apis#events", "events", "EventEmitter compatibility", "Node.js"),
            self.bun_entry("docs/runtime/nodejs-apis#child_process", "child_process", "Child process compatibility", "Node.js"),
            self.bun_entry("docs/runtime/nodejs-apis#stream", "stream", "Node.js streams compatibility", "Node.js"),

            // ==================== Plugins ====================
            self.bun_entry("docs/runtime/plugins", "Runtime Plugins", "Bun plugin system", "Plugins"),
            self.bun_entry("docs/runtime/plugins#loaders", "Plugin Loaders", "Custom file loaders", "Plugins"),
            self.bun_entry("docs/runtime/plugins#virtual-modules", "Virtual Modules", "Create virtual modules", "Plugins"),

            // ==================== HTML Imports ====================
            self.bun_entry("docs/runtime/html", "HTML Imports", "Import HTML files directly", "HTML"),
            self.bun_entry("docs/runtime/html#development", "HTML Dev Server", "Development server for HTML", "HTML"),
            self.bun_entry("docs/runtime/html#bundling", "HTML Bundling", "Bundle HTML for production", "HTML"),

            // ==================== Debugging ====================
            self.bun_entry("docs/runtime/debugger", "Debugger", "Debug Bun with Web Inspector", "Debug"),
            self.bun_entry("docs/runtime/debugger#vscode", "VS Code Debugging", "Debug with VS Code", "Debug"),
            self.bun_entry("docs/runtime/debugger#chrome", "Chrome DevTools", "Debug with Chrome DevTools", "Debug"),
        ]
    }

    fn bun_entry(&self, slug: &str, title: &str, description: &str, category: &str) -> WebFrameworkSearchEntry {
        WebFrameworkSearchEntry {
            framework: WebFramework::Bun,
            slug: slug.to_string(),
            title: title.to_string(),
            description: description.to_string(),
            url: format!("{}/{}", BUN_BASE, slug),
            category: Some(category.to_string()),
        }
    }

    async fn fetch_bun_article(&self, slug: &str) -> Result<WebFrameworkArticle> {
        let cache_key = format!("bun_{}.json", slug.replace('/', "_").replace('#', "_"));

        if let Ok(Some(entry)) = self
            .disk_cache
            .load::<WebFrameworkArticle>(&cache_key)
            .await
        {
            return Ok(entry.value);
        }

        // Handle anchor links - remove anchor for fetching
        let fetch_slug = slug.split('#').next().unwrap_or(slug);
        let url = format!("{}/{}", BUN_BASE, fetch_slug);
        debug!(url = %url, "Fetching Bun article");

        let response = self.http.get(&url).send().await?;
        if !response.status().is_success() {
            anyhow::bail!("Bun page not found: {}", slug);
        }

        let html = response.text().await?;
        let article = self.parse_bun_html(&html, slug, &url);

        let _ = self.disk_cache.store(&cache_key, article.clone()).await;

        Ok(article)
    }

    fn parse_bun_html(&self, html: &str, slug: &str, url: &str) -> WebFrameworkArticle {
        let document = Html::parse_document(html);

        // Bun docs use h1 for main title, or extract from breadcrumb
        let title = self
            .extract_text(&document, "h1, .docs-title, article h1")
            .unwrap_or_else(|| {
                // Try to get title from the slug
                slug.split('/').next_back()
                    .unwrap_or("Bun")
                    .replace('-', " ")
                    .replace('#', " - ")
            });

        // Description from first paragraph or meta
        let description = self
            .extract_text(&document, "article > p:first-of-type, .docs-content > p:first-of-type, main p:first-of-type")
            .unwrap_or_else(|| format!("Bun documentation for {title}"));

        // Extract code examples - Bun docs use various code block styles
        let examples = self.extract_bun_code_examples(&document);

        // Get API signature if present
        let api_signature = self.extract_text(&document, ".api-signature, pre.signature, code.language-ts:first-of-type");

        // Get content, limited to reasonable size
        let content = self
            .extract_text(&document, "article, .docs-content, main")
            .map(|s| if s.len() > 4000 { s[..4000].to_string() } else { s })
            .unwrap_or_default();

        WebFrameworkArticle {
            framework: WebFramework::Bun,
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

    #[allow(clippy::unused_self)]
    fn extract_bun_code_examples(&self, document: &Html) -> Vec<CodeExample> {
        let mut examples = Vec::new();

        // Bun docs use various selectors for code blocks
        let selectors = [
            "pre code",
            "pre.language-ts",
            "pre.language-typescript",
            "pre.language-js",
            "pre.language-javascript",
            "pre.language-bash",
            "pre.language-sh",
            ".code-block pre",
            ".highlight pre",
            "pre.shiki",
        ];

        for selector_str in selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                for element in document.select(&selector) {
                    let code = element.text().collect::<String>().trim().to_string();
                    if code.is_empty() || code.len() < 15 {
                        continue;
                    }

                    // Determine language from class
                    let class = element.value().attr("class").unwrap_or("");
                    let language = if class.contains("typescript") || class.contains("language-ts") {
                        "typescript"
                    } else if class.contains("javascript") || class.contains("language-js") {
                        "javascript"
                    } else if class.contains("bash") || class.contains("sh") {
                        "bash"
                    } else if class.contains("json") {
                        "json"
                    } else {
                        "typescript" // Default to TypeScript for Bun
                    };

                    // Skip shell commands for code examples (keep them but lower priority)
                    let is_shell = language == "bash" || code.starts_with("bun ") || code.starts_with("$ ");

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
                                            .is_some_and(|c| c.contains("filename") || c.contains("file-name"))
                        })
                        .map(|e| e.text().collect::<String>().trim().to_string());

                    let is_complete = code.contains("import ") || code.contains("export ") || code.contains("Bun.");
                    let has_output = code.contains("console.log") || code.contains("// Output:") || code.contains("// =>");

                    let mut example = CodeExample {
                        code,
                        language: language.to_string(),
                        filename,
                        description,
                        is_complete,
                        has_output,
                    };

                    // Lower quality for shell commands
                    if is_shell {
                        example.is_complete = false;
                    }

                    examples.push(example);

                    if examples.len() >= 8 {
                        break;
                    }
                }
            }

            if examples.len() >= 8 {
                break;
            }
        }

        // Sort by quality score
        examples.sort_by_key(|example| Reverse(example.quality_score()));

        // Limit to 5 best examples
        examples.truncate(5);

        examples
    }

    // ==================== HELPERS ====================

    #[allow(clippy::unused_self)]
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

    #[allow(clippy::unused_self)]
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
                                            .is_some_and(|c| c.contains("filename"))
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
        examples.sort_by_key(|example| Reverse(example.quality_score()));

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
