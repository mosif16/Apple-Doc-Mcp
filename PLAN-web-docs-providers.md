# Implementation Plan: Web Documentation Providers

> **STATUS: COMPLETED** - Implemented in Phase 6 (2025-12-02). See `agents.md` for completion details.
> Additional ML/AI providers (MLX, Hugging Face) were added in Phase 7 (2025-12-02).

## Overview

Expand the docs-mcp query system to support JavaScript/TypeScript (MDN) and Web Framework (React, Next.js, Node.js) documentation with comprehensive usage example extraction.

## Goals

1. **MDN Web Docs Provider** - JavaScript, Web APIs, TypeScript via MDN
2. **Web Frameworks Provider** - React, Next.js, Node.js official docs
3. **Usage Example Focus** - Inline code blocks, runnable examples, real-world patterns
4. **Quality Ranking** - Prioritize examples with setup/context for copy-paste use

---

## Phase 1: MDN Web Docs Provider

### 1.1 Create MDN Module Structure

```
crates/multi-provider-client/src/
├── mdn/
│   ├── mod.rs          # Module exports
│   ├── types.rs        # MDN-specific types
│   └── client.rs       # HTTP client with caching
```

### 1.2 MDN Types (`mdn/types.rs`)

```rust
/// MDN documentation categories
pub enum MdnCategory {
    JavaScript,    // JavaScript language features
    WebApi,        // DOM, Fetch, Canvas, etc.
    Css,           // CSS properties and selectors
    Html,          // HTML elements and attributes
}

/// A searchable MDN article
pub struct MdnArticle {
    pub slug: String,              // "Web/JavaScript/Reference/Array"
    pub title: String,             // "Array"
    pub summary: String,           // Brief description
    pub category: MdnCategory,
    pub url: String,               // Full MDN URL
    pub examples: Vec<MdnExample>, // Code examples
    pub syntax: Option<String>,    // Syntax signature
    pub parameters: Vec<MdnParameter>,
    pub return_value: Option<String>,
    pub browser_compat: Option<String>, // Browser compatibility
}

/// Code example from MDN
pub struct MdnExample {
    pub code: String,
    pub language: String,        // "js", "ts", "css", "html"
    pub description: Option<String>,
    pub is_runnable: bool,       // Has live demo potential
}

/// MDN Technology (for unified interface)
pub struct MdnTechnology {
    pub identifier: String,      // "mdn:javascript", "mdn:webapi"
    pub title: String,
    pub description: String,
    pub url: String,
    pub article_count: usize,
}

/// Search index entry
pub struct MdnSearchEntry {
    pub slug: String,
    pub title: String,
    pub summary: String,
    pub category: MdnCategory,
}
```

### 1.3 MDN Client (`mdn/client.rs`)

Data sources:
- **Primary**: MDN Yari JSON API (`https://developer.mozilla.org/api/v1/search`)
- **Fallback**: HTML scraping from `https://developer.mozilla.org/en-US/docs/`
- **Search Index**: MDN search API

```rust
pub struct MdnClient {
    http: Client,
    disk_cache: DiskCache,
    memory_cache: MemoryCache<Vec<u8>>,
    search_index: RwLock<HashMap<String, Vec<MdnSearchEntry>>>,
    cache_dir: PathBuf,
}

impl MdnClient {
    // Core methods
    pub async fn get_technologies(&self) -> Result<Vec<MdnTechnology>>;
    pub async fn get_category(&self, category: &str) -> Result<Vec<MdnSearchEntry>>;
    pub async fn get_article(&self, slug: &str) -> Result<MdnArticle>;
    pub async fn search(&self, query: &str) -> Result<Vec<MdnSearchEntry>>;

    // Example extraction
    async fn extract_examples_from_html(&self, html: &str) -> Vec<MdnExample>;
}
```

### 1.4 MDN Search API Integration

MDN provides a search API:
```
GET https://developer.mozilla.org/api/v1/search?q={query}&locale=en-US
```

Response structure to parse:
```json
{
  "documents": [
    {
      "slug": "Web/JavaScript/Reference/Global_Objects/Array/map",
      "title": "Array.prototype.map()",
      "summary": "Creates a new array...",
      "mdn_url": "/en-US/docs/..."
    }
  ]
}
```

### 1.5 Example Extraction Strategy

MDN pages have structured code blocks:
```html
<pre class="brush: js notranslate">
  // Code example here
</pre>
```

Extraction selectors:
- `pre.brush` - Language-tagged code blocks
- `.example-good` / `.example-bad` - Best practice examples
- `#examples` section - Curated examples
- `.live-sample` - Interactive examples

---

## Phase 2: Web Frameworks Provider

### 2.1 Create WebFrameworks Module Structure

```
crates/multi-provider-client/src/
├── web_frameworks/
│   ├── mod.rs
│   ├── types.rs
│   ├── client.rs
│   ├── react.rs      # React-specific parsing
│   ├── nextjs.rs     # Next.js-specific parsing
│   └── nodejs.rs     # Node.js-specific parsing
```

### 2.2 Types (`web_frameworks/types.rs`)

```rust
/// Framework identifier
pub enum WebFramework {
    React,
    NextJs,
    NodeJs,
}

/// Documentation article
pub struct WebFrameworkArticle {
    pub framework: WebFramework,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub content: String,           // Full markdown content
    pub examples: Vec<CodeExample>,
    pub api_signature: Option<String>,
    pub related: Vec<String>,
}

/// Code example with metadata
pub struct CodeExample {
    pub code: String,
    pub language: String,          // "jsx", "tsx", "js"
    pub filename: Option<String>,  // "App.jsx", "page.tsx"
    pub description: Option<String>,
    pub is_complete: bool,         // Has imports/setup
    pub has_output: bool,          // Shows expected result
}

/// Technology for unified interface
pub struct WebFrameworkTechnology {
    pub identifier: String,        // "react", "nextjs", "nodejs"
    pub title: String,
    pub description: String,
    pub url: String,
    pub version: String,
}
```

### 2.3 Data Sources

**React** (`react.dev`):
- Uses Next.js with JSON data files
- API reference at `/reference/react/`
- Learn section at `/learn/`
- Content stored in MDX format

**Next.js** (`nextjs.org`):
- Documentation at `/docs/`
- GitHub raw content access for MDX files
- Structured sections: App Router, Pages Router, API Reference

**Node.js** (`nodejs.org`):
- API documentation at `/api/`
- JSON API available: `https://nodejs.org/api/all.json`
- Well-structured with examples

### 2.4 WebFrameworks Client

```rust
pub struct WebFrameworksClient {
    http: Client,
    disk_cache: DiskCache,
    memory_cache: MemoryCache<Vec<u8>>,
    // Per-framework search indexes
    react_index: RwLock<Vec<WebFrameworkSearchEntry>>,
    nextjs_index: RwLock<Vec<WebFrameworkSearchEntry>>,
    nodejs_index: RwLock<Vec<WebFrameworkSearchEntry>>,
}

impl WebFrameworksClient {
    pub async fn get_technologies(&self) -> Result<Vec<WebFrameworkTechnology>>;
    pub async fn search(&self, framework: WebFramework, query: &str) -> Result<Vec<WebFrameworkSearchEntry>>;
    pub async fn get_article(&self, framework: WebFramework, slug: &str) -> Result<WebFrameworkArticle>;

    // Framework-specific methods
    async fn fetch_react_docs(&self, path: &str) -> Result<WebFrameworkArticle>;
    async fn fetch_nextjs_docs(&self, path: &str) -> Result<WebFrameworkArticle>;
    async fn fetch_nodejs_docs(&self, path: &str) -> Result<WebFrameworkArticle>;
}
```

---

## Phase 3: Unified Types Integration

### 3.1 Update `ProviderType` (`types.rs`)

```rust
pub enum ProviderType {
    Apple,
    Telegram,
    TON,
    Cocoon,
    Rust,
    Mdn,           // NEW
    WebFrameworks, // NEW
}

impl ProviderType {
    pub fn name(&self) -> &'static str {
        match self {
            // ...existing...
            Self::Mdn => "MDN",
            Self::WebFrameworks => "Web Frameworks",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            // ...existing...
            Self::Mdn => "MDN Web Documentation (JavaScript, Web APIs, CSS)",
            Self::WebFrameworks => "React, Next.js, and Node.js Documentation",
        }
    }
}
```

### 3.2 Update `TechnologyKind`

```rust
pub enum TechnologyKind {
    Framework,      // Apple
    ApiCategory,    // Telegram
    BlockchainApi,  // TON
    DocSection,     // Cocoon
    RustCrate,      // Rust
    MdnCategory,    // NEW: JavaScript, WebAPI, CSS
    WebFramework,   // NEW: React, Next.js, Node.js
}
```

### 3.3 Update `SymbolContent`

```rust
pub enum SymbolContent {
    // ...existing variants...

    /// MDN article content
    Mdn {
        syntax: Option<String>,
        parameters: Vec<MdnParameter>,
        return_value: Option<String>,
        browser_compat: Option<String>,
        examples: Vec<MdnExample>,
    },

    /// Web framework documentation
    WebFramework {
        framework: String,       // "react", "nextjs", "nodejs"
        api_signature: Option<String>,
        examples: Vec<CodeExample>,
        content_markdown: String,
    },
}
```

### 3.4 Add Conversion Functions

```rust
impl UnifiedTechnology {
    pub fn from_mdn(tech: MdnTechnology) -> Self { ... }
    pub fn from_web_framework(tech: WebFrameworkTechnology) -> Self { ... }
}

impl UnifiedFrameworkData {
    pub fn from_mdn(data: MdnCategory) -> Self { ... }
    pub fn from_web_framework(data: WebFrameworkCategory) -> Self { ... }
}

impl UnifiedSymbolData {
    pub fn from_mdn(data: MdnArticle) -> Self { ... }
    pub fn from_web_framework(data: WebFrameworkArticle) -> Self { ... }
}
```

---

## Phase 4: ProviderClients Integration

### 4.1 Update `lib.rs`

```rust
pub mod mdn;
pub mod web_frameworks;

pub struct ProviderClients {
    pub apple: AppleDocsClient,
    pub telegram: TelegramClient,
    pub ton: TonClient,
    pub cocoon: CocoonClient,
    pub rust: RustClient,
    pub mdn: MdnClient,                   // NEW
    pub web_frameworks: WebFrameworksClient, // NEW
}

impl ProviderClients {
    pub fn new() -> Self {
        Self {
            // ...existing...
            mdn: MdnClient::new(),
            web_frameworks: WebFrameworksClient::new(),
        }
    }

    // Update get_all_technologies, get_technologies_for, get_framework, get_symbol
}
```

---

## Phase 5: Query Tool Integration

### 5.1 Add Detection Keywords (`query.rs`)

```rust
/// MDN-related keywords
static MDN_KEYWORDS: Lazy<Vec<&'static str>> = Lazy::new(|| {
    vec![
        "javascript", "js", "typescript", "ts", "mdn",
        "dom", "fetch", "promise", "async", "await",
        "array", "object", "string", "number", "map", "set",
        "event", "listener", "element", "document", "window",
        "localstorage", "sessionstorage", "indexeddb",
        "webapi", "canvas", "webgl", "audio", "video",
    ]
});

/// React-related keywords
static REACT_KEYWORDS: Lazy<Vec<&'static str>> = Lazy::new(|| {
    vec![
        "react", "jsx", "tsx", "component", "hook",
        "usestate", "useeffect", "usecontext", "usememo", "usecallback",
        "useref", "usereducer", "createcontext", "forwardref",
        "suspense", "errorboundary", "memo", "lazy",
    ]
});

/// Next.js-related keywords
static NEXTJS_KEYWORDS: Lazy<Vec<&'static str>> = Lazy::new(|| {
    vec![
        "nextjs", "next", "app router", "pages router",
        "server component", "client component", "ssr", "ssg", "isr",
        "getstaticprops", "getserversideprops", "middleware",
        "api route", "route handler", "dynamic route",
    ]
});

/// Node.js-related keywords
static NODEJS_KEYWORDS: Lazy<Vec<&'static str>> = Lazy::new(|| {
    vec![
        "nodejs", "node", "npm", "require", "module",
        "fs", "path", "http", "https", "stream", "buffer",
        "eventemitter", "process", "child_process",
        "express", "koa", "fastify",
    ]
});
```

### 5.2 Update Provider Detection

```rust
fn detect_provider_and_technology(query: &str) -> (Option<ProviderType>, Option<String>) {
    // ...existing Apple, Rust, Telegram, TON, Cocoon checks...

    // Check for React keywords
    for keyword in REACT_KEYWORDS.iter() {
        if contains_word(query, keyword) {
            return (Some(ProviderType::WebFrameworks), Some("react".to_string()));
        }
    }

    // Check for Next.js keywords
    for keyword in NEXTJS_KEYWORDS.iter() {
        if contains_word(query, keyword) {
            return (Some(ProviderType::WebFrameworks), Some("nextjs".to_string()));
        }
    }

    // Check for Node.js keywords
    for keyword in NODEJS_KEYWORDS.iter() {
        if contains_word(query, keyword) {
            return (Some(ProviderType::WebFrameworks), Some("nodejs".to_string()));
        }
    }

    // Check for MDN/JavaScript keywords
    for keyword in MDN_KEYWORDS.iter() {
        if contains_word(query, keyword) {
            return (Some(ProviderType::Mdn), Some("mdn:javascript".to_string()));
        }
    }

    // Default fallback...
}
```

### 5.3 Add Search Functions

```rust
/// Search MDN documentation
async fn search_mdn(
    context: &Arc<AppContext>,
    query: &str,
    max_results: usize,
) -> Result<Vec<DocResult>> {
    let entries = context.providers.mdn.search(query).await?;

    let mut results = Vec::new();
    for entry in entries.into_iter().take(max_results) {
        // Fetch full article for top results
        let article = if results.len() < MAX_DETAILED_DOCS {
            context.providers.mdn.get_article(&entry.slug).await.ok()
        } else {
            None
        };

        results.push(DocResult {
            title: entry.title,
            kind: format!("{:?}", entry.category),
            path: entry.slug.clone(),
            summary: entry.summary,
            platforms: Some("MDN Web Docs".to_string()),
            code_sample: article.as_ref()
                .and_then(|a| a.examples.first())
                .map(|e| e.code.clone()),
            related_apis: Vec::new(),
            full_content: article.as_ref().map(|a| format_mdn_content(a)),
            declaration: article.as_ref().and_then(|a| a.syntax.clone()),
            parameters: article.as_ref()
                .map(|a| a.parameters.iter()
                    .map(|p| (p.name.clone(), p.description.clone()))
                    .collect())
                .unwrap_or_default(),
        });
    }

    Ok(results)
}

/// Search Web Frameworks documentation
async fn search_web_frameworks(
    context: &Arc<AppContext>,
    framework: &str,
    query: &str,
    max_results: usize,
) -> Result<Vec<DocResult>> {
    let fw = match framework {
        "react" => WebFramework::React,
        "nextjs" => WebFramework::NextJs,
        "nodejs" => WebFramework::NodeJs,
        _ => WebFramework::React,
    };

    let entries = context.providers.web_frameworks.search(fw, query).await?;

    // Similar to search_mdn but with framework-specific formatting
    // ...
}
```

### 5.4 Update execute_search_query

```rust
async fn execute_search_query(
    context: &Arc<AppContext>,
    intent: &QueryIntent,
    max_results: usize,
) -> Result<Vec<DocResult>> {
    let provider = *context.state.active_provider.read().await;

    // ...keyword filtering...

    match provider {
        ProviderType::Apple => search_apple(context, &search_query, max_results).await,
        ProviderType::Rust => search_rust(context, intent, &search_query, max_results).await,
        ProviderType::Telegram => search_telegram(context, &search_query, max_results).await,
        ProviderType::TON => search_ton(context, &search_query, max_results).await,
        ProviderType::Cocoon => search_cocoon(context, &search_query, max_results).await,
        ProviderType::Mdn => search_mdn(context, &search_query, max_results).await,
        ProviderType::WebFrameworks => {
            let framework = intent.technology.as_deref().unwrap_or("react");
            search_web_frameworks(context, framework, &search_query, max_results).await
        }
    }
}
```

---

## Phase 6: Example Quality Ranking

### 6.1 Example Scoring System

```rust
pub struct ExampleScore {
    pub score: i32,
    pub is_runnable: bool,
    pub has_context: bool,
    pub has_output: bool,
}

impl CodeExample {
    pub fn calculate_score(&self) -> ExampleScore {
        let mut score = 0;

        // Completeness indicators
        let has_imports = self.code.contains("import ") || self.code.contains("require(");
        let has_function = self.code.contains("function ") || self.code.contains("=>");
        let has_export = self.code.contains("export ");

        // Boost complete examples
        if has_imports { score += 10; }
        if has_function { score += 5; }
        if self.is_complete { score += 20; }
        if self.has_output { score += 15; }

        // Boost examples with descriptions
        if self.description.is_some() { score += 10; }

        // Penalize snippets
        if self.code.len() < 50 { score -= 10; }

        ExampleScore {
            score,
            is_runnable: has_imports && has_function,
            has_context: has_imports,
            has_output: self.has_output,
        }
    }
}
```

### 6.2 Example Extraction Priorities

**Tier 1 - Runnable Examples** (highest priority):
- Complete with imports/setup
- Has expected output/result
- Clear description of what it demonstrates

**Tier 2 - Contextual Examples**:
- Has some setup but may need context
- Shows real-world usage pattern
- Contains comments explaining the code

**Tier 3 - Reference Examples**:
- API usage snippets
- Syntax demonstrations
- Minimal but accurate

---

## Phase 7: Caching Strategy

### 7.1 Cache Configuration

| Data Type | Memory TTL | Disk TTL |
|-----------|------------|----------|
| MDN search index | 1h | 24h |
| MDN article content | 30min | 24h |
| React docs index | 1h | 24h |
| Next.js docs index | 1h | 24h |
| Node.js API index | 24h | 7d |

### 7.2 Cache Keys

```rust
// MDN
format!("mdn_article_{}.json", slug.replace('/', "_"))
format!("mdn_search_{}.json", category)

// Web Frameworks
format!("{}_article_{}.json", framework, slug.replace('/', "_"))
format!("{}_index.json", framework)
```

---

## Implementation Order

### Week 1: MDN Provider
1. [ ] Create `mdn/types.rs` with all types
2. [ ] Create `mdn/client.rs` with search and fetch
3. [ ] Add MDN search API integration
4. [ ] Implement HTML example extraction
5. [ ] Add unit tests

### Week 2: Web Frameworks Provider
1. [ ] Create `web_frameworks/types.rs`
2. [ ] Implement React docs client
3. [ ] Implement Next.js docs client
4. [ ] Implement Node.js docs client
5. [ ] Add unit tests

### Week 3: Integration
1. [ ] Update `ProviderType`, `TechnologyKind`, `SymbolContent`
2. [ ] Update `ProviderClients`
3. [ ] Add conversion functions
4. [ ] Update query.rs with detection and search

### Week 4: Polish & Test
1. [ ] End-to-end testing
2. [ ] Example quality ranking
3. [ ] Performance optimization
4. [ ] Documentation update

---

## Testing Commands

```bash
# Test MDN query
printf '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"query","arguments":{"query":"JavaScript Array map"}},"id":1}\n' | ./target/release/docs-mcp-cli

# Test React query
printf '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"query","arguments":{"query":"React useState hook"}},"id":1}\n' | ./target/release/docs-mcp-cli

# Test Next.js query
printf '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"query","arguments":{"query":"Next.js server components"}},"id":1}\n' | ./target/release/docs-mcp-cli

# Test Node.js query
printf '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"query","arguments":{"query":"Node.js fs readFile"}},"id":1}\n' | ./target/release/docs-mcp-cli
```

---

## Success Criteria

1. **Detection Accuracy**: Correctly route 95%+ of JS/React/Next/Node queries
2. **Example Quality**: Top result includes runnable code example 80%+ of time
3. **Response Time**: Search completes in <500ms (cached), <2s (uncached)
4. **Test Coverage**: 20+ unit tests for new providers
5. **Documentation**: CLAUDE.md updated with new provider details

---

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| MDN API rate limiting | Aggressive caching, respect rate limits |
| React docs format changes | Version detection, fallback parsers |
| Large index sizes | Lazy loading, memory-efficient structures |
| Example extraction failures | Multiple selector fallbacks, graceful degradation |

---

## Files to Create/Modify

**New Files:**
- `crates/multi-provider-client/src/mdn/mod.rs`
- `crates/multi-provider-client/src/mdn/types.rs`
- `crates/multi-provider-client/src/mdn/client.rs`
- `crates/multi-provider-client/src/web_frameworks/mod.rs`
- `crates/multi-provider-client/src/web_frameworks/types.rs`
- `crates/multi-provider-client/src/web_frameworks/client.rs`
- `crates/multi-provider-client/src/web_frameworks/react.rs`
- `crates/multi-provider-client/src/web_frameworks/nextjs.rs`
- `crates/multi-provider-client/src/web_frameworks/nodejs.rs`

**Modified Files:**
- `crates/multi-provider-client/src/lib.rs` - Add new clients
- `crates/multi-provider-client/src/types.rs` - Add enums/conversions
- `crates/docs-mcp-core/src/tools/query.rs` - Add detection/search
- `crates/docs-mcp-core/src/state.rs` - Add state if needed
- `CLAUDE.md` - Document new providers
- `agents.md` - Update roadmap
