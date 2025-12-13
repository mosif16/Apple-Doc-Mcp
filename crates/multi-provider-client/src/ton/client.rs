use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration as StdDuration;

use anyhow::{Context, Result};
use directories::ProjectDirs;
use reqwest::Client;
use tokio::sync::Mutex;
use tracing::{debug, instrument};

use super::types::{
    OpenApiSpec, TonCategory, TonCodeExample, TonDocArticle, TonDocSource, TonEndpoint,
    TonEndpointSummary, TonResultType, TonSearchResult, TonSecurityCategory, TonSecurityPattern,
    TonTechnology,
};
use docs_mcp_client::cache::{DiskCache, MemoryCache};

const OPENAPI_URL: &str =
    "https://raw.githubusercontent.com/tonkeeper/opentonapi/master/api/openapi.yml";
const CACHE_KEY: &str = "ton_openapi_spec";

#[derive(Debug)]
pub struct TonClient {
    http: Client,
    disk_cache: DiskCache,
    memory_cache: MemoryCache<Vec<u8>>,
    spec_lock: Mutex<()>,
    cache_dir: PathBuf,
}

impl Default for TonClient {
    fn default() -> Self {
        Self::new()
    }
}

impl TonClient {
    #[must_use]
    pub fn new() -> Self {
        let project_dirs = ProjectDirs::from("com", "RecordAndLearn", "multi-docs-mcp")
            .expect("unable to resolve project directories");

        let cache_dir = project_dirs.cache_dir().join("ton");
        if let Err(e) = std::fs::create_dir_all(&cache_dir) {
            tracing::warn!(error = %e, "Failed to create TON cache directory");
        }

        let http = Client::builder()
            .user_agent("MultiDocsMCP/1.0")
            .timeout(StdDuration::from_secs(30))
            .gzip(true)
            .build()
            .expect("failed to build reqwest client");

        Self {
            http,
            disk_cache: DiskCache::new(&cache_dir),
            memory_cache: MemoryCache::new(time::Duration::minutes(30)),
            spec_lock: Mutex::new(()),
            cache_dir,
        }
    }

    /// Fetch the TON API OpenAPI specification
    #[instrument(name = "ton_client.get_spec", skip(self))]
    async fn get_spec(&self) -> Result<OpenApiSpec> {
        let cache_key = format!("{CACHE_KEY}.json");

        // Check disk cache (we store as JSON after parsing YAML)
        if let Ok(Some(entry)) = self.disk_cache.load::<OpenApiSpec>(&cache_key).await {
            debug!("TON OpenAPI spec served from disk cache");
            return Ok(entry.value);
        }

        // Lock to prevent concurrent fetches
        let _lock = self.spec_lock.lock().await;

        // Double-check after acquiring lock
        if let Ok(Some(entry)) = self.disk_cache.load::<OpenApiSpec>(&cache_key).await {
            debug!("TON OpenAPI spec served from disk cache (after lock)");
            return Ok(entry.value);
        }

        // Fetch from remote (YAML format)
        debug!(url = OPENAPI_URL, "Fetching TON OpenAPI spec (YAML)");
        let response = self
            .http
            .get(OPENAPI_URL)
            .send()
            .await
            .context("Failed to fetch TON OpenAPI spec")?;

        if !response.status().is_success() {
            anyhow::bail!("TON OpenAPI spec fetch failed: {}", response.status());
        }

        let yaml_text = response
            .text()
            .await
            .context("Failed to read TON OpenAPI response")?;

        // Parse YAML
        let spec: OpenApiSpec = serde_yaml::from_str(&yaml_text).map_err(|e| {
            tracing::error!(error = %e, "YAML parsing error details");
            anyhow::anyhow!("Failed to parse TON OpenAPI YAML spec: {}", e)
        })?;

        // Store in cache (as JSON for faster subsequent loads)
        self.disk_cache.store(&cache_key, spec.clone()).await?;

        Ok(spec)
    }

    /// Get available technologies (API categories by tag + additional documentation sections)
    #[instrument(name = "ton_client.get_technologies", skip(self))]
    pub async fn get_technologies(&self) -> Result<Vec<TonTechnology>> {
        let spec = self.get_spec().await?;

        // Group endpoints by tag
        let mut tag_counts: HashMap<String, usize> = HashMap::new();
        for path_item in spec.paths.values() {
            for (_method, operation) in path_item.operations() {
                for tag in &operation.tags {
                    *tag_counts.entry(tag.clone()).or_insert(0) += 1;
                }
            }
        }

        // Build tag descriptions map
        let tag_descriptions: HashMap<String, String> = spec
            .tags
            .iter()
            .map(|t| {
                (
                    t.name.clone(),
                    t.description.clone().unwrap_or_default(),
                )
            })
            .collect();

        let mut technologies: Vec<TonTechnology> = tag_counts
            .into_iter()
            .map(|(tag, count)| TonTechnology {
                identifier: format!("ton:{}", tag.to_lowercase().replace(' ', "-")),
                title: format!("TON {}", tag),
                description: tag_descriptions
                    .get(&tag)
                    .cloned()
                    .unwrap_or_else(|| format!("TON API endpoints for {}", tag)),
                url: format!("https://tonapi.io/api-doc#/{}", tag),
                endpoint_count: count,
                source: TonDocSource::TonApi,
            })
            .collect();

        // Add additional TON documentation sections
        technologies.extend(self.get_additional_technologies());

        technologies.sort_by(|a, b| a.title.cmp(&b.title));
        Ok(technologies)
    }

    /// Get additional TON documentation sections beyond the API
    fn get_additional_technologies(&self) -> Vec<TonTechnology> {
        vec![
            TonTechnology {
                identifier: "ton:smart-contracts".to_string(),
                title: "Smart Contracts".to_string(),
                description: "Smart contract development on TON using FunC, Tact, and Tolk languages".to_string(),
                url: "https://docs.ton.org/v3/documentation/smart-contracts/overview".to_string(),
                endpoint_count: 15,
                source: TonDocSource::TonDocs,
            },
            TonTechnology {
                identifier: "ton:tact".to_string(),
                title: "Tact Language".to_string(),
                description: "High-level, TypeScript-like language for TON smart contracts".to_string(),
                url: "https://docs.tact-lang.org/".to_string(),
                endpoint_count: 50,
                source: TonDocSource::TactLang,
            },
            TonTechnology {
                identifier: "ton:func".to_string(),
                title: "FunC Language".to_string(),
                description: "Low-level, C-like language for TON smart contracts".to_string(),
                url: "https://docs.ton.org/v3/documentation/smart-contracts/func/overview".to_string(),
                endpoint_count: 30,
                source: TonDocSource::FunC,
            },
            TonTechnology {
                identifier: "ton:tolk".to_string(),
                title: "Tolk Language".to_string(),
                description: "Next-generation language replacing FunC with modern syntax".to_string(),
                url: "https://docs.ton.org/v3/documentation/smart-contracts/tolk/overview".to_string(),
                endpoint_count: 20,
                source: TonDocSource::Tolk,
            },
            TonTechnology {
                identifier: "ton:security".to_string(),
                title: "Security Best Practices".to_string(),
                description: "Security guidelines and vulnerability patterns for TON smart contracts".to_string(),
                url: "https://docs.ton.org/v3/guidelines/smart-contracts/security/secure-programming/".to_string(),
                endpoint_count: self.get_security_patterns().len(),
                source: TonDocSource::Security,
            },
            TonTechnology {
                identifier: "ton:tvm".to_string(),
                title: "TVM (TON Virtual Machine)".to_string(),
                description: "TON Virtual Machine opcodes, instructions, and architecture".to_string(),
                url: "https://docs.ton.org/v3/documentation/tvm/overview".to_string(),
                endpoint_count: 100,
                source: TonDocSource::Tvm,
            },
            TonTechnology {
                identifier: "ton:jettons".to_string(),
                title: "Jettons (TEP-74)".to_string(),
                description: "Fungible token standard for TON, similar to ERC-20".to_string(),
                url: "https://docs.ton.org/v3/guidelines/dapps/tutorials/jetton".to_string(),
                endpoint_count: 10,
                source: TonDocSource::TonDocs,
            },
            TonTechnology {
                identifier: "ton:nft".to_string(),
                title: "NFT (TEP-62)".to_string(),
                description: "Non-fungible token standard for TON, similar to ERC-721".to_string(),
                url: "https://docs.ton.org/v3/guidelines/dapps/tutorials/nft".to_string(),
                endpoint_count: 10,
                source: TonDocSource::TonDocs,
            },
            TonTechnology {
                identifier: "ton:wallets".to_string(),
                title: "TON Wallets".to_string(),
                description: "Wallet versions, message handling, and transaction patterns".to_string(),
                url: "https://docs.ton.org/v3/documentation/smart-contracts/contracts-specs/wallet-contracts".to_string(),
                endpoint_count: 8,
                source: TonDocSource::TonDocs,
            },
            TonTechnology {
                identifier: "ton:ton-connect".to_string(),
                title: "TON Connect".to_string(),
                description: "Protocol for connecting dApps with TON wallets".to_string(),
                url: "https://docs.ton.org/v3/guidelines/ton-connect/overview".to_string(),
                endpoint_count: 12,
                source: TonDocSource::TonDocs,
            },
        ]
    }

    /// Get endpoints for a specific tag/category
    #[instrument(name = "ton_client.get_category", skip(self))]
    pub async fn get_category(&self, identifier: &str) -> Result<TonCategory> {
        let spec = self.get_spec().await?;

        // Extract tag from identifier (e.g., "ton:accounts" -> "Accounts")
        let tag_search = identifier
            .strip_prefix("ton:")
            .unwrap_or(identifier)
            .to_lowercase();

        // Find matching tag
        let tag = spec
            .tags
            .iter()
            .find(|t| t.name.to_lowercase().replace(' ', "-") == tag_search)
            .map(|t| t.name.clone())
            .ok_or_else(|| anyhow::anyhow!("TON tag not found: {identifier}"))?;

        let description = spec
            .tags
            .iter()
            .find(|t| t.name == tag)
            .and_then(|t| t.description.clone())
            .unwrap_or_default();

        // Collect endpoints for this tag
        let mut endpoints: Vec<TonEndpointSummary> = Vec::new();
        for (path, path_item) in &spec.paths {
            for (method, operation) in path_item.operations() {
                if operation.tags.contains(&tag) {
                    endpoints.push(TonEndpointSummary::from_openapi(path, method, operation));
                }
            }
        }

        endpoints.sort_by(|a, b| a.path.cmp(&b.path));

        Ok(TonCategory {
            tag,
            description,
            endpoints,
            source: TonDocSource::TonApi,
        })
    }

    /// Get a specific endpoint by operation ID
    #[instrument(name = "ton_client.get_endpoint", skip(self))]
    pub async fn get_endpoint(&self, operation_id: &str) -> Result<TonEndpoint> {
        let spec = self.get_spec().await?;

        for (path, path_item) in &spec.paths {
            for (method, operation) in path_item.operations() {
                let op_id = operation.operation_id.as_deref().unwrap_or("");

                if op_id == operation_id {
                    return Ok(TonEndpoint::from_openapi(path, method, operation));
                }
            }
        }

        anyhow::bail!("TON endpoint not found: {operation_id}")
    }

    /// Search for endpoints matching a query
    #[instrument(name = "ton_client.search", skip(self))]
    pub async fn search(&self, query: &str) -> Result<Vec<TonEndpoint>> {
        let spec = self.get_spec().await?;
        let query_lower = query.to_lowercase();

        let mut results: Vec<TonEndpoint> = Vec::new();

        for (path, path_item) in &spec.paths {
            for (method, operation) in path_item.operations() {
                let matches = path.to_lowercase().contains(&query_lower)
                    || operation
                        .operation_id
                        .as_deref()
                        .is_some_and(|s| s.to_lowercase().contains(&query_lower))
                    || operation
                        .summary
                        .as_deref()
                        .is_some_and(|s| s.to_lowercase().contains(&query_lower))
                    || operation
                        .description
                        .as_deref()
                        .is_some_and(|s| s.to_lowercase().contains(&query_lower));

                if matches {
                    results.push(TonEndpoint::from_openapi(path, method, operation));
                }
            }
        }

        Ok(results)
    }

    /// Unified search across all TON documentation sources
    #[instrument(name = "ton_client.search_all", skip(self))]
    pub async fn search_all(&self, query: &str) -> Result<Vec<TonSearchResult>> {
        let query_lower = query.to_lowercase();
        let mut results: Vec<TonSearchResult> = Vec::new();

        // Search API endpoints
        let api_results = self.search(&query_lower).await?;
        for endpoint in api_results {
            let score = self.calculate_api_score(&endpoint, &query_lower);
            results.push(TonSearchResult {
                id: endpoint.operation_id.clone(),
                title: endpoint
                    .summary
                    .clone()
                    .unwrap_or_else(|| endpoint.operation_id.clone()),
                description: endpoint.description.clone().unwrap_or_default(),
                source: TonDocSource::TonApi,
                url: format!(
                    "https://tonapi.io/api-doc#/{}",
                    endpoint.tags.first().unwrap_or(&"default".to_string())
                ),
                result_type: TonResultType::ApiEndpoint,
                score,
                code_examples: vec![],
            });
        }

        // Search security patterns
        let security_results = self.search_security_patterns(&query_lower);
        results.extend(security_results);

        // Search documentation articles
        let doc_results = self.search_documentation(&query_lower);
        results.extend(doc_results);

        // Sort by score
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        Ok(results)
    }

    /// Calculate relevance score for API endpoint
    fn calculate_api_score(&self, endpoint: &TonEndpoint, query: &str) -> f32 {
        let mut score = 0.0;

        // Exact match in operation_id
        if endpoint.operation_id.to_lowercase() == query {
            score += 10.0;
        } else if endpoint.operation_id.to_lowercase().contains(query) {
            score += 5.0;
        }

        // Match in path
        if endpoint.path.to_lowercase().contains(query) {
            score += 3.0;
        }

        // Match in summary
        if let Some(ref summary) = endpoint.summary {
            if summary.to_lowercase().contains(query) {
                score += 2.0;
            }
        }

        // Match in description
        if let Some(ref desc) = endpoint.description {
            if desc.to_lowercase().contains(query) {
                score += 1.0;
            }
        }

        score
    }

    /// Search security patterns
    fn search_security_patterns(&self, query: &str) -> Vec<TonSearchResult> {
        let patterns = self.get_security_patterns();
        let mut results = Vec::new();

        for pattern in patterns {
            let mut score = 0.0;

            // Title match
            if pattern.title.to_lowercase().contains(query) {
                score += 5.0;
            }

            // Category match
            if pattern.category.name().to_lowercase().contains(query) {
                score += 3.0;
            }

            // Description match
            if pattern.description.to_lowercase().contains(query) {
                score += 2.0;
            }

            // Keyword matches
            let security_keywords = [
                "security",
                "vulnerability",
                "attack",
                "exploit",
                "safe",
                "secure",
                "audit",
                "best practice",
            ];
            for keyword in security_keywords {
                if query.contains(keyword) {
                    score += 1.0;
                }
            }

            if score > 0.0 {
                let mut code_examples = Vec::new();
                if let Some(ref vuln) = pattern.vulnerable_pattern {
                    code_examples.push(vuln.clone());
                }
                if let Some(ref secure) = pattern.secure_pattern {
                    code_examples.push(secure.clone());
                }

                results.push(TonSearchResult {
                    id: pattern.id.clone(),
                    title: pattern.title.clone(),
                    description: pattern.description.clone(),
                    source: TonDocSource::Security,
                    url: format!(
                        "https://docs.ton.org/v3/guidelines/smart-contracts/security/secure-programming/#{}",
                        pattern.id
                    ),
                    result_type: TonResultType::Security,
                    score,
                    code_examples,
                });
            }
        }

        results
    }

    /// Search embedded documentation articles
    fn search_documentation(&self, query: &str) -> Vec<TonSearchResult> {
        let articles = self.get_documentation_articles();
        let mut results = Vec::new();

        for article in articles {
            let mut score = 0.0;

            // Title match (highest weight)
            if article.title.to_lowercase().contains(query) {
                score += 5.0;
            }

            // Tag match
            for tag in &article.tags {
                if tag.to_lowercase().contains(query) {
                    score += 3.0;
                }
            }

            // Description match
            if article.description.to_lowercase().contains(query) {
                score += 2.0;
            }

            // Content match
            if article.content.to_lowercase().contains(query) {
                score += 1.0;
            }

            if score > 0.0 {
                results.push(TonSearchResult {
                    id: article.id.clone(),
                    title: article.title.clone(),
                    description: article.description.clone(),
                    source: article.source,
                    url: article.url.clone(),
                    result_type: TonResultType::Article,
                    score,
                    code_examples: article.code_examples.clone(),
                });
            }
        }

        results
    }

    /// Get embedded security patterns (built-in knowledge base)
    pub fn get_security_patterns(&self) -> Vec<TonSecurityPattern> {
        vec![
            // Integer Handling
            TonSecurityPattern {
                id: "integer-overflow".to_string(),
                title: "Signed/Unsigned Integer Issues".to_string(),
                category: TonSecurityCategory::IntegerHandling,
                severity: "critical".to_string(),
                description: "Improper integer handling allows overflow/underflow attacks. In TON, integers are 257-bit signed by default. Always validate balances before operations.".to_string(),
                vulnerable_pattern: Some(TonCodeExample {
                    language: "func".to_string(),
                    code: ";; Vulnerable: No validation before subtraction\nint from_balance = get_balance(from);\nint new_balance = from_balance - amount;  ;; Can underflow!".to_string(),
                    description: Some("Missing balance validation allows underflow".to_string()),
                    is_complete: false,
                }),
                secure_pattern: Some(TonCodeExample {
                    language: "func".to_string(),
                    code: ";; Secure: Validate before operation\nint from_balance = get_balance(from);\nthrow_unless(998, from_balance >= amount);\nint new_balance = from_balance - amount;".to_string(),
                    description: Some("Validate balance before subtraction to prevent underflow".to_string()),
                    is_complete: false,
                }),
                mitigations: vec![
                    "Always validate values before arithmetic operations".to_string(),
                    "Use throw_unless() to check preconditions".to_string(),
                    "Be aware that TVM integers are 257-bit signed".to_string(),
                ],
                related: vec!["gas-exhaustion".to_string()],
            },

            // Message Handling
            TonSecurityPattern {
                id: "unconditional-accept".to_string(),
                title: "Unconditional External Message Acceptance".to_string(),
                category: TonSecurityCategory::MessageHandling,
                severity: "critical".to_string(),
                description: "Never call accept_message() without proper guards. Attackers can drain contract balance by repeatedly sending external messages.".to_string(),
                vulnerable_pattern: Some(TonCodeExample {
                    language: "func".to_string(),
                    code: "() recv_external(slice in_msg) impure {\n    accept_message();  ;; DANGEROUS: No validation!\n    ;; ... process message\n}".to_string(),
                    description: Some("Accepting messages without validation drains gas".to_string()),
                    is_complete: false,
                }),
                secure_pattern: Some(TonCodeExample {
                    language: "func".to_string(),
                    code: "() recv_external(slice in_msg) impure {\n    ;; First: verify sender authenticity\n    slice signature = in_msg~load_bits(512);\n    int hash = slice_hash(in_msg);\n    throw_unless(35, check_signature(hash, signature, public_key));\n    \n    ;; Then: accept message\n    accept_message();\n}".to_string(),
                    description: Some("Validate signature before accepting external message".to_string()),
                    is_complete: false,
                }),
                mitigations: vec![
                    "Always verify sender authentication before accept_message()".to_string(),
                    "Validate message contents and parameters first".to_string(),
                    "Consider using sequence numbers for replay protection".to_string(),
                ],
                related: vec!["replay-attack".to_string()],
            },

            // Replay Protection
            TonSecurityPattern {
                id: "replay-attack".to_string(),
                title: "Missing Replay Protection".to_string(),
                category: TonSecurityCategory::ReplayProtection,
                severity: "high".to_string(),
                description: "External messages can be reused multiple times if not protected. Implement sequence numbers to prevent replay attacks.".to_string(),
                vulnerable_pattern: Some(TonCodeExample {
                    language: "func".to_string(),
                    code: "() recv_external(slice in_msg) impure {\n    ;; No sequence number check - vulnerable to replay!\n    var signature = in_msg~load_bits(512);\n    accept_message();\n}".to_string(),
                    description: Some("Without sequence numbers, messages can be replayed".to_string()),
                    is_complete: false,
                }),
                secure_pattern: Some(TonCodeExample {
                    language: "func".to_string(),
                    code: "() recv_external(slice in_msg) impure {\n    var signature = in_msg~load_bits(512);\n    var msg_seqno = in_msg~load_uint(32);\n    var stored_seqno = get_data().begin_parse().preload_uint(32);\n    \n    ;; Verify sequence number\n    throw_unless(33, msg_seqno == stored_seqno);\n    \n    accept_message();\n    \n    ;; Increment sequence number\n    set_data(begin_cell().store_uint(stored_seqno + 1, 32).end_cell());\n}".to_string(),
                    description: Some("Use sequence numbers to prevent replay attacks".to_string()),
                    is_complete: false,
                }),
                mitigations: vec![
                    "Implement sequence numbers (seqno) for external messages".to_string(),
                    "Store and increment seqno in contract state".to_string(),
                    "Consider using expiration timestamps for time-limited validity".to_string(),
                ],
                related: vec!["unconditional-accept".to_string()],
            },

            // Gas Management
            TonSecurityPattern {
                id: "gas-exhaustion".to_string(),
                title: "Gas Exhaustion Vulnerability".to_string(),
                category: TonSecurityCategory::GasManagement,
                severity: "high".to_string(),
                description: "Insufficient gas validation can cause transactions to fail mid-execution, potentially leaving contract in inconsistent state.".to_string(),
                vulnerable_pattern: Some(TonCodeExample {
                    language: "tact".to_string(),
                    code: "receive(msg: Process) {\n    // No gas check - might fail mid-execution!\n    self.expensiveOperation();\n}".to_string(),
                    description: Some("Operation may fail due to insufficient gas".to_string()),
                    is_complete: false,
                }),
                secure_pattern: Some(TonCodeExample {
                    language: "tact".to_string(),
                    code: "receive(msg: Process) {\n    // Pre-calculate required gas\n    let gasUsage: Int = 50000;  // Estimated gas for operation\n    require(context().value > getComputeFee(gasUsage), \"Insufficient gas\");\n    \n    self.expensiveOperation();\n}".to_string(),
                    description: Some("Validate gas before expensive operations".to_string()),
                    is_complete: false,
                }),
                mitigations: vec![
                    "Pre-calculate gas consumption for complex operations".to_string(),
                    "Use require() or throw_unless() to validate gas availability".to_string(),
                    "Consider splitting large operations into multiple messages".to_string(),
                    "Document gas requirements for external callers".to_string(),
                ],
                related: vec!["unbounded-loop".to_string()],
            },

            // Unbounded Loops
            TonSecurityPattern {
                id: "unbounded-loop".to_string(),
                title: "Dangerous Loop Patterns".to_string(),
                category: TonSecurityCategory::GasManagement,
                severity: "high".to_string(),
                description: "Sending messages from loops or unbounded iterations can lead to out-of-gas attacks and DoS vulnerabilities.".to_string(),
                vulnerable_pattern: Some(TonCodeExample {
                    language: "func".to_string(),
                    code: ";; Dangerous: Unbounded loop sending messages\nint i = 0;\nwhile (i < count) {  ;; count could be attacker-controlled!\n    send_raw_message(msg, 0);\n    i += 1;\n}".to_string(),
                    description: Some("Attacker can set count to exhaust gas".to_string()),
                    is_complete: false,
                }),
                secure_pattern: Some(TonCodeExample {
                    language: "func".to_string(),
                    code: ";; Safe: Bounded loop with maximum limit\nint MAX_ITERATIONS = 10;\nint iterations = min(count, MAX_ITERATIONS);\nint i = 0;\nwhile (i < iterations) {\n    send_raw_message(msg, 0);\n    i += 1;\n}".to_string(),
                    description: Some("Limit iterations to prevent gas exhaustion".to_string()),
                    is_complete: false,
                }),
                mitigations: vec![
                    "Always bound loop iterations with a maximum limit".to_string(),
                    "Avoid sending messages from within loops".to_string(),
                    "Consider pagination for large data sets".to_string(),
                    "Split large operations across multiple transactions".to_string(),
                ],
                related: vec!["gas-exhaustion".to_string()],
            },

            // Access Control
            TonSecurityPattern {
                id: "missing-access-control".to_string(),
                title: "Missing Access Control".to_string(),
                category: TonSecurityCategory::AccessControl,
                severity: "critical".to_string(),
                description: "Sensitive operations must verify sender authorization. In TON, check sender address against stored admin/owner addresses.".to_string(),
                vulnerable_pattern: Some(TonCodeExample {
                    language: "tact".to_string(),
                    code: "receive(msg: Upgrade) {\n    // VULNERABLE: Anyone can upgrade!\n    self.code = msg.newCode;\n}".to_string(),
                    description: Some("No sender verification for critical operation".to_string()),
                    is_complete: false,
                }),
                secure_pattern: Some(TonCodeExample {
                    language: "tact".to_string(),
                    code: "receive(msg: Upgrade) {\n    // Verify sender is authorized admin\n    require(sender() == self.admin, \"Unauthorized\");\n    self.code = msg.newCode;\n}".to_string(),
                    description: Some("Check sender authorization before sensitive operations".to_string()),
                    is_complete: false,
                }),
                mitigations: vec![
                    "Always verify sender() for administrative functions".to_string(),
                    "Store admin/owner address in contract state".to_string(),
                    "Consider multi-signature schemes for critical operations".to_string(),
                    "Emit events for audit trail".to_string(),
                ],
                related: vec!["code-upgrade-vuln".to_string()],
            },

            // Data Storage
            TonSecurityPattern {
                id: "sensitive-data-onchain".to_string(),
                title: "Sensitive Data On-Chain".to_string(),
                category: TonSecurityCategory::DataStorage,
                severity: "critical".to_string(),
                description: "All contract computation is transparent and emulatable. Never store passwords, private keys, or confidential data on-chain.".to_string(),
                vulnerable_pattern: Some(TonCodeExample {
                    language: "tact".to_string(),
                    code: "contract Vault {\n    // VULNERABLE: Private key stored on-chain!\n    privateKey: Int;\n    password: String;\n}".to_string(),
                    description: Some("Sensitive data is visible to everyone".to_string()),
                    is_complete: false,
                }),
                secure_pattern: Some(TonCodeExample {
                    language: "tact".to_string(),
                    code: "contract Vault {\n    // Store hash instead of sensitive data\n    passwordHash: Int as uint256;\n    // Use commit-reveal for secrets\n    commitments: map<Address, Int>;\n}".to_string(),
                    description: Some("Store hashes and use commit-reveal schemes".to_string()),
                    is_complete: false,
                }),
                mitigations: vec![
                    "Never store secrets directly on-chain".to_string(),
                    "Use cryptographic hashes for sensitive data".to_string(),
                    "Implement commit-reveal schemes when needed".to_string(),
                    "Consider off-chain computation with on-chain verification".to_string(),
                ],
                related: vec![],
            },

            // Randomness
            TonSecurityPattern {
                id: "insecure-randomness".to_string(),
                title: "Insecure Randomness".to_string(),
                category: TonSecurityCategory::Randomness,
                severity: "high".to_string(),
                description: "Built-in random functions are pseudo-random and predictable. For critical applications, use commit-and-disclose schemes or off-chain randomization.".to_string(),
                vulnerable_pattern: Some(TonCodeExample {
                    language: "func".to_string(),
                    code: ";; VULNERABLE: Predictable randomness\nint winner = random() % participants_count;\nsend_prize(winner);".to_string(),
                    description: Some("Validators can predict and manipulate random()".to_string()),
                    is_complete: false,
                }),
                secure_pattern: Some(TonCodeExample {
                    language: "func".to_string(),
                    code: ";; Commit-reveal scheme for fair randomness\n;; Phase 1: All participants commit hash(secret)\n;; Phase 2: All participants reveal secrets\n;; Phase 3: Combine all secrets for final random\nint combined_seed = 0;\nforall (secret in revealed_secrets) {\n    combined_seed = combined_seed ^ secret;\n}\nint winner = combined_seed % participants_count;".to_string(),
                    description: Some("Use commit-reveal for unpredictable randomness".to_string()),
                    is_complete: false,
                }),
                mitigations: vec![
                    "Use commit-reveal schemes for critical randomness".to_string(),
                    "Consider VRF (Verifiable Random Function) solutions".to_string(),
                    "Combine multiple entropy sources".to_string(),
                    "Add delay between commit and reveal phases".to_string(),
                ],
                related: vec![],
            },

            // Race Conditions
            TonSecurityPattern {
                id: "race-condition-destroy".to_string(),
                title: "Account Destruction Race Conditions".to_string(),
                category: TonSecurityCategory::RaceConditions,
                severity: "medium".to_string(),
                description: "Using send mode 128+32 to destroy accounts creates race condition vulnerabilities. Messages sent before destruction may fail.".to_string(),
                vulnerable_pattern: Some(TonCodeExample {
                    language: "func".to_string(),
                    code: ";; DANGEROUS: Race condition with destroy\nsend_raw_message(important_msg, 0);\nsend_raw_message(destroy_msg, 128 + 32);  ;; Destroys account".to_string(),
                    description: Some("First message may fail if account destroyed".to_string()),
                    is_complete: false,
                }),
                secure_pattern: Some(TonCodeExample {
                    language: "func".to_string(),
                    code: ";; Safe: Verify all messages sent before destruction\nint msg_count = send_raw_message(important_msg, 1);  ;; mode 1 = pay fees separately\nthrow_unless(100, msg_count > 0);\n;; Only destroy after confirming message sent\nsend_raw_message(destroy_msg, 128 + 32);".to_string(),
                    description: Some("Verify message delivery before destruction".to_string()),
                    is_complete: false,
                }),
                mitigations: vec![
                    "Avoid combining critical messages with account destruction".to_string(),
                    "Implement proper cleanup sequences".to_string(),
                    "Consider keeping minimal balance instead of destruction".to_string(),
                ],
                related: vec!["gas-exhaustion".to_string()],
            },

            // Code Upgrade
            TonSecurityPattern {
                id: "code-upgrade-vuln".to_string(),
                title: "Code Update Vulnerabilities".to_string(),
                category: TonSecurityCategory::CodeUpgrade,
                severity: "critical".to_string(),
                description: "Contract upgrades must be protected with proper authorization. Unauthorized code changes can completely compromise a contract.".to_string(),
                vulnerable_pattern: Some(TonCodeExample {
                    language: "func".to_string(),
                    code: "() upgrade(cell new_code) impure {\n    ;; VULNERABLE: No auth check!\n    set_code(new_code);\n}".to_string(),
                    description: Some("Anyone can upgrade the contract code".to_string()),
                    is_complete: false,
                }),
                secure_pattern: Some(TonCodeExample {
                    language: "func".to_string(),
                    code: "() upgrade(cell new_code) impure {\n    throw_unless(error::unauthorized, authorized_admin?(sender()));\n    \n    ;; Optional: Add timelock for safety\n    throw_unless(error::too_early, now() >= upgrade_timestamp);\n    \n    set_code(new_code);\n    \n    ;; Emit upgrade event for transparency\n    emit_log(\"contract_upgraded\", new_code_hash);\n}".to_string(),
                    description: Some("Authorize and log all upgrades".to_string()),
                    is_complete: false,
                }),
                mitigations: vec![
                    "Always verify sender authorization for upgrades".to_string(),
                    "Consider timelock mechanisms for upgrade safety".to_string(),
                    "Implement upgrade events for transparency".to_string(),
                    "Use multi-sig for critical upgrade decisions".to_string(),
                ],
                related: vec!["missing-access-control".to_string()],
            },

            // Front-running
            TonSecurityPattern {
                id: "front-running".to_string(),
                title: "Front-Running via Signature Reuse".to_string(),
                category: TonSecurityCategory::ExternalCalls,
                severity: "high".to_string(),
                description: "If signatures don't include recipient address, attackers can redirect transactions to different recipients.".to_string(),
                vulnerable_pattern: Some(TonCodeExample {
                    language: "tact".to_string(),
                    code: "// VULNERABLE: Signature doesn't include recipient\nstruct Request {\n    seqno: Int;\n    amount: Int;\n    // Missing: recipient address!\n}".to_string(),
                    description: Some("Attacker can redirect funds to their address".to_string()),
                    is_complete: false,
                }),
                secure_pattern: Some(TonCodeExample {
                    language: "tact".to_string(),
                    code: "// SECURE: Include recipient in signed data\nstruct Request {\n    to: Address;      // Recipient bound to signature\n    seqno: Int;\n    amount: Int;\n    validUntil: Int;  // Expiration for safety\n}".to_string(),
                    description: Some("Include all critical parameters in signed data".to_string()),
                    is_complete: false,
                }),
                mitigations: vec![
                    "Include recipient address in signed message data".to_string(),
                    "Add expiration timestamps to prevent delayed attacks".to_string(),
                    "Consider using commit-reveal for sensitive operations".to_string(),
                ],
                related: vec!["replay-attack".to_string()],
            },

            // Cross-shard calls
            TonSecurityPattern {
                id: "cross-shard-getter".to_string(),
                title: "Pulling Data From Other Contracts".to_string(),
                category: TonSecurityCategory::ExternalCalls,
                severity: "medium".to_string(),
                description: "Contracts cannot call getter functions across shards. Use asynchronous message-based communication instead.".to_string(),
                vulnerable_pattern: Some(TonCodeExample {
                    language: "tact".to_string(),
                    code: "// WRONG: Cannot call getters on other contracts!\nlet balance = otherContract.getBalance();  // This doesn't work!".to_string(),
                    description: Some("Getter calls across contracts are not possible".to_string()),
                    is_complete: false,
                }),
                secure_pattern: Some(TonCodeExample {
                    language: "tact".to_string(),
                    code: "// CORRECT: Use message-based communication\nmessage GetBalanceRequest { queryId: Int; }\nmessage GetBalanceResponse { queryId: Int; balance: Int; }\n\nreceive(msg: GetBalanceRequest) {\n    send(SendParameters{\n        to: sender(),\n        value: 0,\n        mode: SendRemainingValue,\n        body: GetBalanceResponse{ queryId: msg.queryId, balance: self.balance }.toCell()\n    });\n}".to_string(),
                    description: Some("Use async messages for cross-contract communication".to_string()),
                    is_complete: false,
                }),
                mitigations: vec![
                    "Design contracts with async message patterns".to_string(),
                    "Use query-response pattern for data retrieval".to_string(),
                    "Handle potential message failures gracefully".to_string(),
                    "Consider caching frequently needed external data".to_string(),
                ],
                related: vec![],
            },
        ]
    }

    /// Get embedded documentation articles
    fn get_documentation_articles(&self) -> Vec<TonDocArticle> {
        vec![
            // Smart Contract Overview
            TonDocArticle {
                id: "smart-contracts-overview".to_string(),
                title: "Smart Contracts on TON".to_string(),
                description: "Overview of smart contract development on TON blockchain".to_string(),
                content: "TON smart contracts are programs deployed on the TON blockchain. They can hold TON coins, process messages, and manage data. Unlike Ethereum, TON uses an actor model where contracts communicate asynchronously via messages.".to_string(),
                source: TonDocSource::TonDocs,
                url: "https://docs.ton.org/v3/documentation/smart-contracts/overview".to_string(),
                category: "Smart Contracts".to_string(),
                code_examples: vec![],
                related: vec!["tact-intro".to_string(), "func-intro".to_string()],
                tags: vec!["smart contract".to_string(), "overview".to_string(), "introduction".to_string()],
            },

            // Tact Introduction
            TonDocArticle {
                id: "tact-intro".to_string(),
                title: "Introduction to Tact".to_string(),
                description: "Getting started with Tact - the high-level smart contract language for TON".to_string(),
                content: "Tact is a high-level programming language for TON Blockchain focused on efficiency and simplicity. It features TypeScript-like syntax, strong static typing, and automatic (de)serialization.".to_string(),
                source: TonDocSource::TactLang,
                url: "https://docs.tact-lang.org/".to_string(),
                category: "Tact Language".to_string(),
                code_examples: vec![
                    TonCodeExample {
                        language: "tact".to_string(),
                        code: "contract Counter {\n    value: Int;\n\n    init() {\n        self.value = 0;\n    }\n\n    receive(\"increment\") {\n        self.value += 1;\n    }\n\n    get fun value(): Int {\n        return self.value;\n    }\n}".to_string(),
                        description: Some("Simple counter contract in Tact".to_string()),
                        is_complete: true,
                    }
                ],
                related: vec!["func-intro".to_string(), "tolk-intro".to_string()],
                tags: vec!["tact".to_string(), "language".to_string(), "tutorial".to_string(), "beginner".to_string()],
            },

            // FunC Introduction
            TonDocArticle {
                id: "func-intro".to_string(),
                title: "Introduction to FunC".to_string(),
                description: "Getting started with FunC - the low-level smart contract language for TON".to_string(),
                content: "FunC is a domain-specific, C-like, statically typed language used to program smart contracts on TON. It's designed for writing low-level contracts tightly bound to the TVM model.".to_string(),
                source: TonDocSource::FunC,
                url: "https://docs.ton.org/v3/documentation/smart-contracts/func/overview".to_string(),
                category: "FunC Language".to_string(),
                code_examples: vec![
                    TonCodeExample {
                        language: "func".to_string(),
                        code: "() recv_internal(int my_balance, int msg_value, cell in_msg_full, slice in_msg_body) impure {\n    if (in_msg_body.slice_empty?()) {\n        return ();\n    }\n    int op = in_msg_body~load_uint(32);\n    ;; Handle operations...\n}".to_string(),
                        description: Some("Basic FunC message handler".to_string()),
                        is_complete: false,
                    }
                ],
                related: vec!["tact-intro".to_string(), "tolk-intro".to_string()],
                tags: vec!["func".to_string(), "language".to_string(), "low-level".to_string()],
            },

            // Tolk Introduction
            TonDocArticle {
                id: "tolk-intro".to_string(),
                title: "Introduction to Tolk".to_string(),
                description: "Getting started with Tolk - the next-generation smart contract language for TON".to_string(),
                content: "Tolk is a next-generation language for developing smart contracts on TON. It replaces FunC with an expressive syntax, a robust type system, and built-in serialization — while generating highly optimized assembly code.".to_string(),
                source: TonDocSource::Tolk,
                url: "https://docs.ton.org/v3/documentation/smart-contracts/tolk/overview".to_string(),
                category: "Tolk Language".to_string(),
                code_examples: vec![],
                related: vec!["func-intro".to_string(), "tact-intro".to_string()],
                tags: vec!["tolk".to_string(), "language".to_string(), "next-generation".to_string()],
            },

            // Jettons
            TonDocArticle {
                id: "jettons".to_string(),
                title: "Jettons (Fungible Tokens)".to_string(),
                description: "TEP-74 Jetton standard - fungible tokens on TON".to_string(),
                content: "Jettons are TON's implementation of fungible tokens, similar to ERC-20 on Ethereum. The standard uses a sharded architecture with a master contract and individual wallet contracts for each holder.".to_string(),
                source: TonDocSource::TonDocs,
                url: "https://docs.ton.org/v3/guidelines/dapps/tutorials/jetton".to_string(),
                category: "Token Standards".to_string(),
                code_examples: vec![
                    TonCodeExample {
                        language: "tact".to_string(),
                        code: "message Transfer {\n    queryId: Int as uint64;\n    amount: Int as coins;\n    destination: Address;\n    responseDestination: Address?;\n    customPayload: Cell?;\n    forwardTonAmount: Int as coins;\n    forwardPayload: Slice as remaining;\n}".to_string(),
                        description: Some("Jetton transfer message structure".to_string()),
                        is_complete: false,
                    }
                ],
                related: vec!["nft".to_string(), "sbt".to_string()],
                tags: vec!["jetton".to_string(), "token".to_string(), "tep-74".to_string(), "fungible".to_string()],
            },

            // NFT
            TonDocArticle {
                id: "nft".to_string(),
                title: "NFT (Non-Fungible Tokens)".to_string(),
                description: "TEP-62 NFT standard - non-fungible tokens on TON".to_string(),
                content: "TON NFTs follow the TEP-62 standard. Like Jettons, they use a sharded architecture with a collection contract and individual item contracts for each NFT.".to_string(),
                source: TonDocSource::TonDocs,
                url: "https://docs.ton.org/v3/guidelines/dapps/tutorials/nft".to_string(),
                category: "Token Standards".to_string(),
                code_examples: vec![],
                related: vec!["jettons".to_string(), "sbt".to_string()],
                tags: vec!["nft".to_string(), "token".to_string(), "tep-62".to_string(), "non-fungible".to_string()],
            },

            // Wallets
            TonDocArticle {
                id: "wallets".to_string(),
                title: "TON Wallet Contracts".to_string(),
                description: "Understanding TON wallet versions and architecture".to_string(),
                content: "TON wallets are smart contracts that manage user funds. Different versions (v3r2, v4r2, v5) offer various features. The latest v5 wallet supports plugins and gasless transactions.".to_string(),
                source: TonDocSource::TonDocs,
                url: "https://docs.ton.org/v3/documentation/smart-contracts/contracts-specs/wallet-contracts".to_string(),
                category: "Wallets".to_string(),
                code_examples: vec![],
                related: vec!["ton-connect".to_string()],
                tags: vec!["wallet".to_string(), "v3".to_string(), "v4".to_string(), "v5".to_string()],
            },

            // TON Connect
            TonDocArticle {
                id: "ton-connect".to_string(),
                title: "TON Connect".to_string(),
                description: "Protocol for connecting dApps with TON wallets".to_string(),
                content: "TON Connect is the standard protocol for connecting decentralized applications with TON wallets. It enables secure communication and transaction signing between dApps and wallets.".to_string(),
                source: TonDocSource::TonDocs,
                url: "https://docs.ton.org/v3/guidelines/ton-connect/overview".to_string(),
                category: "TON Connect".to_string(),
                code_examples: vec![
                    TonCodeExample {
                        language: "typescript".to_string(),
                        code: "import { TonConnect } from '@tonconnect/sdk';\n\nconst connector = new TonConnect({\n    manifestUrl: 'https://yourapp.com/tonconnect-manifest.json'\n});\n\nawait connector.connect({ jsBridgeKey: 'tonkeeper' });".to_string(),
                        description: Some("Initialize TON Connect in JavaScript".to_string()),
                        is_complete: false,
                    }
                ],
                related: vec!["wallets".to_string()],
                tags: vec!["ton-connect".to_string(), "dapp".to_string(), "wallet".to_string(), "connection".to_string()],
            },

            // TVM Overview
            TonDocArticle {
                id: "tvm-overview".to_string(),
                title: "TVM (TON Virtual Machine) Overview".to_string(),
                description: "Understanding the TON Virtual Machine architecture".to_string(),
                content: "The TON Virtual Machine (TVM) executes all TON smart contracts. It operates as a stack machine supporting seven variable types: Integer (257-bit signed), Tuple, Cell, Slice, Builder, Continuation, and Null.".to_string(),
                source: TonDocSource::Tvm,
                url: "https://docs.ton.org/v3/documentation/tvm/overview".to_string(),
                category: "TVM".to_string(),
                code_examples: vec![],
                related: vec!["tvm-instructions".to_string()],
                tags: vec!["tvm".to_string(), "virtual machine".to_string(), "stack".to_string(), "architecture".to_string()],
            },

            // Cells and BOC
            TonDocArticle {
                id: "cells-boc".to_string(),
                title: "Cells and Bag of Cells (BOC)".to_string(),
                description: "Understanding TON's fundamental data structure".to_string(),
                content: "Cells are the fundamental data unit in TON. Each cell can store up to 1023 bits of data and up to 4 references to other cells. BOC (Bag of Cells) is the serialization format for cells.".to_string(),
                source: TonDocSource::TonDocs,
                url: "https://docs.ton.org/v3/concepts/dive-into-ton/ton-blockchain/cells-as-data-storage".to_string(),
                category: "Core Concepts".to_string(),
                code_examples: vec![
                    TonCodeExample {
                        language: "tact".to_string(),
                        code: "// Building a cell\nlet builder = beginCell();\nbuilder.storeUint(123, 32);\nbuilder.storeAddress(myAddress);\nlet cell = builder.endCell();\n\n// Reading from a cell\nlet slice = cell.beginParse();\nlet value = slice.loadUint(32);\nlet addr = slice.loadAddress();".to_string(),
                        description: Some("Working with cells in Tact".to_string()),
                        is_complete: false,
                    }
                ],
                related: vec!["tvm-overview".to_string()],
                tags: vec!["cell".to_string(), "boc".to_string(), "data".to_string(), "serialization".to_string()],
            },

            // Message Modes
            TonDocArticle {
                id: "message-modes".to_string(),
                title: "Message Sending Modes".to_string(),
                description: "Understanding TON message modes and flags".to_string(),
                content: "When sending messages in TON, you specify a mode that controls how the message is processed. Common modes: 0 (ordinary), 64 (carry remaining value), 128 (carry all balance), +32 (destroy on zero balance).".to_string(),
                source: TonDocSource::TonDocs,
                url: "https://docs.ton.org/v3/documentation/smart-contracts/message-management/sending-messages".to_string(),
                category: "Messages".to_string(),
                code_examples: vec![
                    TonCodeExample {
                        language: "tact".to_string(),
                        code: "// Common send modes in Tact\nSendIgnoreErrors      // mode 0\nSendPayGasSeparately  // mode 1\nSendRemainingValue    // mode 64\nSendRemainingBalance  // mode 128\nSendDestroyIfZero     // flag +32".to_string(),
                        description: Some("Message modes in Tact".to_string()),
                        is_complete: false,
                    }
                ],
                related: vec!["gas-management".to_string()],
                tags: vec!["message".to_string(), "mode".to_string(), "send".to_string(), "flag".to_string()],
            },

            // Gas and Fees
            TonDocArticle {
                id: "gas-fees".to_string(),
                title: "Gas and Transaction Fees".to_string(),
                description: "Understanding gas costs and fee structure in TON".to_string(),
                content: "TON uses gas to measure computational resources. Gas costs vary by operation: throwing exceptions costs 50 gas, tuple creation costs 1 gas per element, jumps cost 10 gas. Storage fees are charged based on cell count and duration.".to_string(),
                source: TonDocSource::TonDocs,
                url: "https://docs.ton.org/v3/documentation/smart-contracts/transaction-fees/fees-low-level".to_string(),
                category: "Core Concepts".to_string(),
                code_examples: vec![],
                related: vec!["gas-exhaustion".to_string()],
                tags: vec!["gas".to_string(), "fees".to_string(), "transaction".to_string(), "cost".to_string()],
            },

            // Blueprint Development
            TonDocArticle {
                id: "blueprint".to_string(),
                title: "Blueprint Development Framework".to_string(),
                description: "All-in-one tool for writing, testing and deploying TON smart contracts".to_string(),
                content: "Blueprint is the recommended development environment for TON. It provides project scaffolding, compilation, testing with TON Sandbox, and deployment tools. Start with 'npm create ton@latest'.".to_string(),
                source: TonDocSource::TonDocs,
                url: "https://docs.ton.org/v3/documentation/smart-contracts/getting-started/your-first-contract".to_string(),
                category: "Development Tools".to_string(),
                code_examples: vec![
                    TonCodeExample {
                        language: "bash".to_string(),
                        code: "# Create new TON project\nnpm create ton@latest\n\n# Available commands\nnpx blueprint build   # Compile contracts\nnpx blueprint test    # Run tests\nnpx blueprint run     # Deploy or interact".to_string(),
                        description: Some("Blueprint CLI commands".to_string()),
                        is_complete: true,
                    }
                ],
                related: vec!["sandbox".to_string()],
                tags: vec!["blueprint".to_string(), "development".to_string(), "tooling".to_string(), "cli".to_string()],
            },

            // Sandbox Testing
            TonDocArticle {
                id: "sandbox".to_string(),
                title: "TON Sandbox Testing".to_string(),
                description: "Testing framework to emulate TON smart contracts".to_string(),
                content: "TON Sandbox (@ton/sandbox) is a testing framework that emulates arbitrary TON smart contracts. It allows sending messages, running get methods, and testing contract behavior as if deployed on a real network.".to_string(),
                source: TonDocSource::TonDocs,
                url: "https://docs.ton.org/v3/documentation/smart-contracts/getting-started/your-first-contract#testing".to_string(),
                category: "Development Tools".to_string(),
                code_examples: vec![
                    TonCodeExample {
                        language: "typescript".to_string(),
                        code: "import { Blockchain } from '@ton/sandbox';\nimport { Counter } from '../wrappers/Counter';\n\ndescribe('Counter', () => {\n    it('should increment', async () => {\n        const blockchain = await Blockchain.create();\n        const counter = blockchain.openContract(\n            await Counter.fromInit()\n        );\n        \n        await counter.send(\n            deployer.getSender(),\n            { value: toNano('0.05') },\n            'increment'\n        );\n        \n        expect(await counter.getValue()).toBe(1n);\n    });\n});".to_string(),
                        description: Some("Testing a counter contract with Sandbox".to_string()),
                        is_complete: true,
                    }
                ],
                related: vec!["blueprint".to_string()],
                tags: vec!["sandbox".to_string(), "testing".to_string(), "emulator".to_string(), "jest".to_string()],
            },

            // ============================================================================
            // TEP Standards
            // ============================================================================

            TonDocArticle {
                id: "tep-62-nft".to_string(),
                title: "TEP-62: NFT Standard".to_string(),
                description: "Non-fungible token standard for TON blockchain".to_string(),
                content: "TEP-62 defines the NFT standard for TON. It uses a sharded architecture with a collection contract and individual item contracts for each NFT. The standard defines get_nft_data(), get_collection_data(), and transfer ownership mechanisms.".to_string(),
                source: TonDocSource::TonDocs,
                url: "https://github.com/ton-blockchain/TEPs/blob/master/text/0062-nft-standard.md".to_string(),
                category: "TEP Standards".to_string(),
                code_examples: vec![
                    TonCodeExample {
                        language: "tact".to_string(),
                        code: "// NFT Item contract interface\ncontract NftItem {\n    collection: Address;\n    index: Int;\n    owner: Address;\n    content: Cell;\n\n    get fun get_nft_data(): NftData {\n        return NftData{\n            init: true,\n            index: self.index,\n            collection: self.collection,\n            owner: self.owner,\n            content: self.content\n        };\n    }\n}".to_string(),
                        description: Some("NFT Item contract structure".to_string()),
                        is_complete: false,
                    }
                ],
                related: vec!["tep-64-metadata".to_string(), "tep-85-sbt".to_string()],
                tags: vec!["nft".to_string(), "tep-62".to_string(), "token".to_string(), "standard".to_string()],
            },

            TonDocArticle {
                id: "tep-64-metadata".to_string(),
                title: "TEP-64: Token Data Standard".to_string(),
                description: "Standard for token metadata in the TON ecosystem".to_string(),
                content: "TEP-64 defines how token metadata should be structured and stored. It supports on-chain metadata, off-chain JSON, and semi-chain (chunked) storage. Metadata includes name, description, image, and custom attributes.".to_string(),
                source: TonDocSource::TonDocs,
                url: "https://github.com/ton-blockchain/TEPs/blob/master/text/0064-token-data-standard.md".to_string(),
                category: "TEP Standards".to_string(),
                code_examples: vec![
                    TonCodeExample {
                        language: "json".to_string(),
                        code: "{\n  \"name\": \"My NFT\",\n  \"description\": \"Description of my NFT\",\n  \"image\": \"https://example.com/image.png\",\n  \"attributes\": [\n    {\"trait_type\": \"Color\", \"value\": \"Blue\"},\n    {\"trait_type\": \"Rarity\", \"value\": \"Legendary\"}\n  ]\n}".to_string(),
                        description: Some("Off-chain metadata JSON format".to_string()),
                        is_complete: true,
                    }
                ],
                related: vec!["tep-62-nft".to_string(), "jettons".to_string()],
                tags: vec!["metadata".to_string(), "tep-64".to_string(), "token".to_string(), "json".to_string()],
            },

            TonDocArticle {
                id: "tep-74-jetton".to_string(),
                title: "TEP-74: Jetton Standard".to_string(),
                description: "Fungible token standard for TON (like ERC-20)".to_string(),
                content: "TEP-74 defines the Jetton (fungible token) standard. It uses a master contract (jetton-minter) and individual wallet contracts for each holder. Key operations: transfer, burn, mint. Each wallet stores its own balance.".to_string(),
                source: TonDocSource::TonDocs,
                url: "https://github.com/ton-blockchain/TEPs/blob/master/text/0074-jettons-standard.md".to_string(),
                category: "TEP Standards".to_string(),
                code_examples: vec![
                    TonCodeExample {
                        language: "tact".to_string(),
                        code: "message JettonTransfer {\n    queryId: Int as uint64;\n    amount: Int as coins;\n    destination: Address;\n    responseDestination: Address;\n    customPayload: Cell?;\n    forwardTonAmount: Int as coins;\n    forwardPayload: Slice as remaining;\n}\n\nreceive(msg: JettonTransfer) {\n    let ctx = context();\n    require(ctx.sender == self.owner, \"Not owner\");\n    self.balance -= msg.amount;\n    // Send to destination wallet...\n}".to_string(),
                        description: Some("Jetton transfer implementation".to_string()),
                        is_complete: false,
                    }
                ],
                related: vec!["tep-89-discoverable".to_string(), "jettons".to_string()],
                tags: vec!["jetton".to_string(), "tep-74".to_string(), "fungible".to_string(), "token".to_string()],
            },

            TonDocArticle {
                id: "tep-81-dns".to_string(),
                title: "TEP-81: TON DNS Standard".to_string(),
                description: "Domain name system standard for TON blockchain".to_string(),
                content: "TEP-81 defines TON DNS - a service translating human-readable .ton domains to smart contract addresses. Domains are NFTs following TEP-62. Support for subdomains via dns_next_resolver. Minimum 4 characters, max 126.".to_string(),
                source: TonDocSource::TonDocs,
                url: "https://github.com/ton-blockchain/TEPs/blob/master/text/0081-dns-standard.md".to_string(),
                category: "TEP Standards".to_string(),
                code_examples: vec![
                    TonCodeExample {
                        language: "typescript".to_string(),
                        code: "import { TonClient } from '@ton/ton';\n\n// Resolve .ton domain to address\nconst client = new TonClient({ endpoint: 'https://toncenter.com/api/v2/jsonRPC' });\nconst resolved = await client.resolveDomain('myname.ton');\nconsole.log('Address:', resolved.toString());".to_string(),
                        description: Some("Resolving TON DNS domain".to_string()),
                        is_complete: false,
                    }
                ],
                related: vec!["ton-dns".to_string()],
                tags: vec!["dns".to_string(), "tep-81".to_string(), "domain".to_string(), ".ton".to_string()],
            },

            TonDocArticle {
                id: "tep-85-sbt".to_string(),
                title: "TEP-85: SBT (Soulbound Token) Standard".to_string(),
                description: "Non-transferable token standard for identity and credentials".to_string(),
                content: "TEP-85 defines Soul Bound Tokens - non-transferable NFTs used for identity, credentials, and reputation. Based on TEP-62 but with transfer restrictions. Useful for KYC, achievements, and membership verification.".to_string(),
                source: TonDocSource::TonDocs,
                url: "https://github.com/ton-blockchain/TEPs/blob/master/text/0085-sbt-standard.md".to_string(),
                category: "TEP Standards".to_string(),
                code_examples: vec![
                    TonCodeExample {
                        language: "tact".to_string(),
                        code: "contract SoulboundToken {\n    owner: Address;\n    authority: Address;  // Can revoke\n    content: Cell;\n    revoked: Bool;\n\n    // SBTs cannot be transferred!\n    receive(msg: Transfer) {\n        require(false, \"SBT: transfers not allowed\");\n    }\n\n    receive(msg: Revoke) {\n        require(sender() == self.authority, \"Not authority\");\n        self.revoked = true;\n    }\n}".to_string(),
                        description: Some("SBT contract with transfer restriction".to_string()),
                        is_complete: false,
                    }
                ],
                related: vec!["tep-62-nft".to_string()],
                tags: vec!["sbt".to_string(), "tep-85".to_string(), "soulbound".to_string(), "identity".to_string()],
            },

            TonDocArticle {
                id: "tep-89-discoverable".to_string(),
                title: "TEP-89: Discoverable Jettons".to_string(),
                description: "Standard for jetton wallet discovery".to_string(),
                content: "TEP-89 extends TEP-74 to allow discovering jetton wallets by owner address. Adds provide_wallet_address operation to the minter contract, enabling efficient wallet lookups without off-chain indexing.".to_string(),
                source: TonDocSource::TonDocs,
                url: "https://github.com/ton-blockchain/TEPs/blob/master/text/0089-jetton-wallet-discovery.md".to_string(),
                category: "TEP Standards".to_string(),
                code_examples: vec![],
                related: vec!["tep-74-jetton".to_string()],
                tags: vec!["jetton".to_string(), "tep-89".to_string(), "discovery".to_string(), "wallet".to_string()],
            },

            // ============================================================================
            // TON DNS & Domains
            // ============================================================================

            TonDocArticle {
                id: "ton-dns".to_string(),
                title: "TON DNS System".to_string(),
                description: "Human-readable domain names for TON addresses".to_string(),
                content: "TON DNS translates human-readable .ton domains into smart contract addresses, ADNL addresses, and more. Domains are NFTs, purchased via auction at dns.ton.org. Must be renewed yearly. Supports subdomains.".to_string(),
                source: TonDocSource::TonDocs,
                url: "https://docs.ton.org/v3/guidelines/web3/ton-dns/dns".to_string(),
                category: "TON DNS".to_string(),
                code_examples: vec![
                    TonCodeExample {
                        language: "typescript".to_string(),
                        code: "// Using TonWeb to resolve DNS\nimport TonWeb from 'tonweb';\n\nconst tonweb = new TonWeb();\nconst domain = 'wallet.ton';\nconst result = await tonweb.dns.resolve(domain);\nconsole.log('Wallet address:', result.wallet?.toString());\nconsole.log('Site address:', result.site?.toString());".to_string(),
                        description: Some("Resolve TON DNS using TonWeb SDK".to_string()),
                        is_complete: false,
                    }
                ],
                related: vec!["tep-81-dns".to_string(), "ton-sites".to_string()],
                tags: vec!["dns".to_string(), "domain".to_string(), ".ton".to_string(), "nft".to_string()],
            },

            // ============================================================================
            // TON Storage
            // ============================================================================

            TonDocArticle {
                id: "ton-storage".to_string(),
                title: "TON Storage".to_string(),
                description: "Decentralized file storage on TON network".to_string(),
                content: "TON Storage is a decentralized file storage solution based on torrent-like technology. Files are encrypted, split into fragments, and distributed across nodes. Uses RLDP protocol via ADNL. Ideal for NFT metadata, TON Sites, and dApp assets.".to_string(),
                source: TonDocSource::TonDocs,
                url: "https://docs.ton.org/v3/guidelines/web3/ton-storage/storage-provider".to_string(),
                category: "TON Storage".to_string(),
                code_examples: vec![
                    TonCodeExample {
                        language: "bash".to_string(),
                        code: "# Create a bag from files\nstorage-daemon-cli -c \"create /path/to/files -d 'My files'\"\n\n# Download a bag by bag-id\nstorage-daemon-cli -c \"download <bag-id> /download/path\"\n\n# Get bag info\nstorage-daemon-cli -c \"get <bag-id>\"".to_string(),
                        description: Some("TON Storage daemon CLI commands".to_string()),
                        is_complete: true,
                    }
                ],
                related: vec!["ton-sites".to_string()],
                tags: vec!["storage".to_string(), "decentralized".to_string(), "files".to_string(), "torrent".to_string()],
            },

            TonDocArticle {
                id: "ton-storage-provider".to_string(),
                title: "TON Storage Provider".to_string(),
                description: "Run a storage provider service for TON".to_string(),
                content: "Storage providers store files for a fee. They run storage-daemon, deploy a smart contract for payment handling, and serve files to clients. Clients pay per-byte fees. Providers earn by offering reliable storage.".to_string(),
                source: TonDocSource::TonDocs,
                url: "https://docs.ton.org/v3/guidelines/web3/ton-storage/storage-provider".to_string(),
                category: "TON Storage".to_string(),
                code_examples: vec![],
                related: vec!["ton-storage".to_string()],
                tags: vec!["storage".to_string(), "provider".to_string(), "service".to_string(), "monetization".to_string()],
            },

            // ============================================================================
            // TON Sites & WWW
            // ============================================================================

            TonDocArticle {
                id: "ton-sites".to_string(),
                title: "TON Sites".to_string(),
                description: "Decentralized websites hosted on TON".to_string(),
                content: "TON Sites are fully decentralized websites with no central server. They use TON DNS for domain resolution and TON Storage for file hosting. Accessible via TON Proxy or special browsers. Perfect for censorship-resistant content.".to_string(),
                source: TonDocSource::TonDocs,
                url: "https://docs.ton.org/v3/guidelines/web3/ton-www".to_string(),
                category: "TON WWW".to_string(),
                code_examples: vec![
                    TonCodeExample {
                        language: "bash".to_string(),
                        code: "# Host a TON Site\n# 1. Create your static website with index.html\nmkdir my-ton-site && cd my-ton-site\necho '<html><body>Hello TON!</body></html>' > index.html\n\n# 2. Create a bag from the folder\nstorage-daemon-cli -c \"create . -d 'My TON Site'\"\n\n# 3. Register .ton domain and point to bag-id".to_string(),
                        description: Some("Steps to host a TON Site".to_string()),
                        is_complete: true,
                    }
                ],
                related: vec!["ton-dns".to_string(), "ton-storage".to_string(), "ton-proxy".to_string()],
                tags: vec!["website".to_string(), "decentralized".to_string(), "hosting".to_string(), "www".to_string()],
            },

            TonDocArticle {
                id: "ton-proxy".to_string(),
                title: "TON Proxy".to_string(),
                description: "Access TON Sites through HTTP proxy".to_string(),
                content: "TON Proxy allows accessing TON Sites through regular browsers via HTTP. It resolves TON DNS, fetches content from TON Storage, and serves it over HTTP. Can run locally or as a public gateway.".to_string(),
                source: TonDocSource::TonDocs,
                url: "https://docs.ton.org/v3/guidelines/web3/ton-proxy".to_string(),
                category: "TON WWW".to_string(),
                code_examples: vec![],
                related: vec!["ton-sites".to_string(), "ton-dns".to_string()],
                tags: vec!["proxy".to_string(), "gateway".to_string(), "http".to_string(), "access".to_string()],
            },

            // ============================================================================
            // Tact Standard Library
            // ============================================================================

            TonDocArticle {
                id: "tact-stdlib-ownable".to_string(),
                title: "Tact @stdlib/ownable".to_string(),
                description: "Ownable trait for access control in Tact contracts".to_string(),
                content: "The Ownable trait provides basic access control. It declares an owner address and requireOwner() helper. OwnableTransferable extends it to allow ownership transfer via ChangeOwner message.".to_string(),
                source: TonDocSource::TactLang,
                url: "https://docs.tact-lang.org/ref/stdlib-ownable/".to_string(),
                category: "Tact Stdlib".to_string(),
                code_examples: vec![
                    TonCodeExample {
                        language: "tact".to_string(),
                        code: "import \"@stdlib/ownable\";\n\ncontract MyContract with Ownable {\n    owner: Address;\n\n    init(owner: Address) {\n        self.owner = owner;\n    }\n\n    receive(\"protected\") {\n        self.requireOwner();  // Only owner can call\n        // ... protected logic\n    }\n}".to_string(),
                        description: Some("Using Ownable trait for access control".to_string()),
                        is_complete: true,
                    }
                ],
                related: vec!["tact-stdlib-stoppable".to_string()],
                tags: vec!["tact".to_string(), "stdlib".to_string(), "ownable".to_string(), "access control".to_string()],
            },

            TonDocArticle {
                id: "tact-stdlib-stoppable".to_string(),
                title: "Tact @stdlib/stoppable".to_string(),
                description: "Emergency stop functionality for Tact contracts".to_string(),
                content: "The Stoppable trait allows pausing contract operations. Requires Ownable. Owner sends 'Stop' message to pause. Provides stopped() getter and requireNotStopped()/requireStopped() helpers.".to_string(),
                source: TonDocSource::TactLang,
                url: "https://docs.tact-lang.org/ref/stdlib-stoppable/".to_string(),
                category: "Tact Stdlib".to_string(),
                code_examples: vec![
                    TonCodeExample {
                        language: "tact".to_string(),
                        code: "import \"@stdlib/ownable\";\nimport \"@stdlib/stoppable\";\n\ncontract PausableToken with Ownable, Stoppable {\n    owner: Address;\n    stopped: Bool;\n\n    init(owner: Address) {\n        self.owner = owner;\n        self.stopped = false;\n    }\n\n    receive(msg: Transfer) {\n        self.requireNotStopped();  // Fail if paused\n        // ... transfer logic\n    }\n}".to_string(),
                        description: Some("Pausable contract using Stoppable trait".to_string()),
                        is_complete: true,
                    }
                ],
                related: vec!["tact-stdlib-ownable".to_string()],
                tags: vec!["tact".to_string(), "stdlib".to_string(), "stoppable".to_string(), "pause".to_string(), "emergency".to_string()],
            },

            TonDocArticle {
                id: "tact-stdlib-deploy".to_string(),
                title: "Tact @stdlib/deploy".to_string(),
                description: "Deployment helpers for Tact contracts".to_string(),
                content: "The Deployable trait provides standardized deployment notification. It handles Deploy message and emits DeployOk event. Useful for deployment verification and tracking.".to_string(),
                source: TonDocSource::TactLang,
                url: "https://docs.tact-lang.org/ref/stdlib-deploy/".to_string(),
                category: "Tact Stdlib".to_string(),
                code_examples: vec![
                    TonCodeExample {
                        language: "tact".to_string(),
                        code: "import \"@stdlib/deploy\";\n\ncontract MyContract with Deployable {\n    init() {}\n\n    // Deployable trait adds:\n    // receive(msg: Deploy) { ... notify(DeployOk{...}) }\n}".to_string(),
                        description: Some("Contract with deployment notification".to_string()),
                        is_complete: true,
                    }
                ],
                related: vec!["tact-intro".to_string()],
                tags: vec!["tact".to_string(), "stdlib".to_string(), "deploy".to_string(), "deployable".to_string()],
            },

            // ============================================================================
            // FunC Standard Library
            // ============================================================================

            TonDocArticle {
                id: "func-stdlib".to_string(),
                title: "FunC Standard Library (stdlib.fc)".to_string(),
                description: "Core functions available in all FunC contracts".to_string(),
                content: "The stdlib.fc library wraps common TVM assembly commands. It provides tuple manipulation, dictionary primitives, cell/slice operations, and cryptographic functions. Always imported automatically.".to_string(),
                source: TonDocSource::FunC,
                url: "https://docs.ton.org/v3/documentation/smart-contracts/func/docs/stdlib".to_string(),
                category: "FunC Stdlib".to_string(),
                code_examples: vec![
                    TonCodeExample {
                        language: "func".to_string(),
                        code: ";; Common stdlib functions\n\n;; Get contract address\nslice my_addr = my_address();\n\n;; Get current time\nint now = now();\n\n;; Cell hash\nint hash = cell_hash(my_cell);\n\n;; Random number (use with caution!)\nrandomize_lt();\nint rand = random();\n\n;; Send raw message\nsend_raw_message(msg, mode);".to_string(),
                        description: Some("Common FunC stdlib functions".to_string()),
                        is_complete: false,
                    }
                ],
                related: vec!["func-intro".to_string(), "func-dict".to_string()],
                tags: vec!["func".to_string(), "stdlib".to_string(), "standard library".to_string()],
            },

            TonDocArticle {
                id: "func-dict".to_string(),
                title: "FunC Dictionary Operations".to_string(),
                description: "Working with dictionaries (hashmaps) in FunC".to_string(),
                content: "FunC dictionaries are cell-based hashmaps. Key functions: udict_set/idict_set (set value), udict_get?/idict_get? (get value), udict_delete?/idict_delete? (delete). 'u' prefix for unsigned keys, 'i' for signed.".to_string(),
                source: TonDocSource::FunC,
                url: "https://docs.ton.org/v3/documentation/smart-contracts/func/docs/stdlib#dictionaries".to_string(),
                category: "FunC Stdlib".to_string(),
                code_examples: vec![
                    TonCodeExample {
                        language: "func".to_string(),
                        code: ";; Dictionary operations\ncell dict = new_dict();\n\n;; Set value (key_bits, key, value, dict)\ndict~udict_set(256, key, value);\n\n;; Get value\n(slice val, int found?) = dict.udict_get?(256, key);\nif (found?) {\n    ;; use val\n}\n\n;; Delete key\n(dict, int deleted?) = dict~udict_delete?(256, key);\n\n;; Iterate dictionary\n(int key, slice val, int found?) = dict.udict_get_min?(256);\nwhile (found?) {\n    ;; process key, val\n    (key, val, found?) = dict.udict_get_next?(256, key);\n}".to_string(),
                        description: Some("Dictionary (hashmap) operations in FunC".to_string()),
                        is_complete: false,
                    }
                ],
                related: vec!["func-stdlib".to_string()],
                tags: vec!["func".to_string(), "dictionary".to_string(), "hashmap".to_string(), "udict".to_string()],
            },

            // ============================================================================
            // TVM Instructions
            // ============================================================================

            TonDocArticle {
                id: "tvm-stack".to_string(),
                title: "TVM Stack Operations".to_string(),
                description: "Stack manipulation instructions in TVM".to_string(),
                content: "TVM is a stack machine with registers s0-s255 (s0 is top). Basic ops: PUSH (add to stack), POP (remove), XCHG (swap), DUP (duplicate), DROP (discard). Stack notation: 'x y - z' means consumes x,y and produces z.".to_string(),
                source: TonDocSource::Tvm,
                url: "https://docs.ton.org/v3/documentation/tvm/instructions".to_string(),
                category: "TVM".to_string(),
                code_examples: vec![
                    TonCodeExample {
                        language: "fift".to_string(),
                        code: "// TVM stack operations in Fift\n5 PUSHINT      // Push 5 onto stack: - 5\n3 PUSHINT      // Push 3: 5 - 5 3  \nADD            // Add top two: 5 3 - 8\nDUP            // Duplicate top: 8 - 8 8\ns0 s2 XCHG     // Swap s0 and s2\nDROP           // Discard top".to_string(),
                        description: Some("Basic TVM stack operations".to_string()),
                        is_complete: false,
                    }
                ],
                related: vec!["tvm-overview".to_string(), "tvm-arithmetic".to_string()],
                tags: vec!["tvm".to_string(), "stack".to_string(), "push".to_string(), "pop".to_string(), "xchg".to_string()],
            },

            TonDocArticle {
                id: "tvm-arithmetic".to_string(),
                title: "TVM Arithmetic Instructions".to_string(),
                description: "Mathematical operations in TVM".to_string(),
                content: "TVM supports 257-bit signed integers. Basic ops: ADD, SUB, MUL, DIV, MOD. Division modes: DIVMOD (quotient+remainder), MULDIV (multiply then divide to avoid overflow). Comparison: LESS, EQUAL, GREATER.".to_string(),
                source: TonDocSource::Tvm,
                url: "https://docs.ton.org/v3/documentation/tvm/instructions".to_string(),
                category: "TVM".to_string(),
                code_examples: vec![
                    TonCodeExample {
                        language: "fift".to_string(),
                        code: "// Arithmetic operations\n10 PUSHINT\n3 PUSHINT\nDIVMOD       // 10 3 - 3 1 (quotient=3, remainder=1)\n\n// Multiply-divide (avoids overflow)\n1000000 PUSHINT\n1000000 PUSHINT  \n1000000 PUSHINT\nMULDIV       // (1000000 * 1000000) / 1000000 = 1000000".to_string(),
                        description: Some("TVM arithmetic instructions".to_string()),
                        is_complete: false,
                    }
                ],
                related: vec!["tvm-stack".to_string()],
                tags: vec!["tvm".to_string(), "arithmetic".to_string(), "math".to_string(), "div".to_string(), "mul".to_string()],
            },

            TonDocArticle {
                id: "tvm-cells".to_string(),
                title: "TVM Cell Instructions".to_string(),
                description: "Cell manipulation instructions in TVM".to_string(),
                content: "Cells store data (up to 1023 bits) and references (up to 4). Builder creates cells, Slice reads them. Key ops: NEWC (new builder), STU/STI (store unsigned/signed), ENDC (finish cell), CTOS (cell to slice), LDU/LDI (load values).".to_string(),
                source: TonDocSource::Tvm,
                url: "https://docs.ton.org/v3/documentation/tvm/instructions".to_string(),
                category: "TVM".to_string(),
                code_examples: vec![
                    TonCodeExample {
                        language: "fift".to_string(),
                        code: "// Building a cell\nNEWC           // New builder\n42 PUSHINT     \n32 STU         // Store 42 as 32-bit unsigned\n-1 PUSHINT\n8 STI          // Store -1 as 8-bit signed  \nENDC           // Finish cell\n\n// Reading a cell\nCTOS           // Cell to slice\n32 LDU         // Load 32-bit unsigned\n8 LDI          // Load 8-bit signed".to_string(),
                        description: Some("Building and reading cells in TVM".to_string()),
                        is_complete: false,
                    }
                ],
                related: vec!["cells-boc".to_string()],
                tags: vec!["tvm".to_string(), "cell".to_string(), "builder".to_string(), "slice".to_string()],
            },

            // ============================================================================
            // DeFi Protocols
            // ============================================================================

            TonDocArticle {
                id: "defi-stonfi".to_string(),
                title: "STON.fi DEX".to_string(),
                description: "Leading decentralized exchange on TON".to_string(),
                content: "STON.fi is the largest DEX on TON with $6.6B+ trading volume. Features AMM swaps, liquidity pools, and farming. Uses Hashed Timelock Contracts for cross-chain swaps. STON token for governance via DAO.".to_string(),
                source: TonDocSource::TonDocs,
                url: "https://ston.fi/".to_string(),
                category: "DeFi".to_string(),
                code_examples: vec![
                    TonCodeExample {
                        language: "typescript".to_string(),
                        code: "import { DEX } from '@ston-fi/sdk';\n\n// Initialize DEX\nconst dex = new DEX.v1({ tonApiKey: 'YOUR_KEY' });\n\n// Get swap quote\nconst quote = await dex.getSwapQuote({\n    offerAddress: USDT_ADDRESS,\n    askAddress: TON_ADDRESS,\n    offerUnits: '1000000000',  // 1000 USDT\n    slippageTolerance: '0.01'\n});".to_string(),
                        description: Some("Using STON.fi SDK for swaps".to_string()),
                        is_complete: false,
                    }
                ],
                related: vec!["defi-dedust".to_string(), "jettons".to_string()],
                tags: vec!["defi".to_string(), "dex".to_string(), "ston.fi".to_string(), "swap".to_string(), "amm".to_string()],
            },

            TonDocArticle {
                id: "defi-dedust".to_string(),
                title: "DeDust DEX".to_string(),
                description: "Decentralized exchange with volatile and stable pools".to_string(),
                content: "DeDust is a DEX on TON featuring DeDust Protocol 2.0. Offers volatile pools (standard AMM) and stable swaps (for stablecoins). Known for gas efficiency and smooth UX. Integrated with TradingView.".to_string(),
                source: TonDocSource::TonDocs,
                url: "https://dedust.io/".to_string(),
                category: "DeFi".to_string(),
                code_examples: vec![
                    TonCodeExample {
                        language: "typescript".to_string(),
                        code: "import { Factory, MAINNET_FACTORY_ADDR } from '@dedust/sdk';\nimport { TonClient4 } from '@ton/ton';\n\nconst client = new TonClient4({ endpoint: 'https://mainnet-v4.tonhubapi.com' });\nconst factory = client.open(Factory.createFromAddress(MAINNET_FACTORY_ADDR));\n\n// Get pool\nconst pool = await factory.getPool(poolType, [assetA, assetB]);".to_string(),
                        description: Some("Using DeDust SDK".to_string()),
                        is_complete: false,
                    }
                ],
                related: vec!["defi-stonfi".to_string()],
                tags: vec!["defi".to_string(), "dex".to_string(), "dedust".to_string(), "swap".to_string()],
            },

            TonDocArticle {
                id: "defi-evaa".to_string(),
                title: "EVAA Lending Protocol".to_string(),
                description: "Lending and borrowing platform on TON".to_string(),
                content: "EVAA is a lending protocol where users can supply assets to earn interest or borrow against collateral. Supports TON and major jettons. Dynamic interest rates based on utilization. Liquidation mechanism for undercollateralized positions.".to_string(),
                source: TonDocSource::TonDocs,
                url: "https://evaa.finance/".to_string(),
                category: "DeFi".to_string(),
                code_examples: vec![],
                related: vec!["defi-stonfi".to_string(), "jettons".to_string()],
                tags: vec!["defi".to_string(), "lending".to_string(), "borrowing".to_string(), "evaa".to_string()],
            },

            TonDocArticle {
                id: "defi-liquid-staking".to_string(),
                title: "Liquid Staking on TON".to_string(),
                description: "Stake TON while maintaining liquidity".to_string(),
                content: "Liquid staking protocols (Bemo, Hipo, Tonstakers) let you stake TON and receive liquid tokens (stTON, hTON, tsTON). These can be used in DeFi while earning staking rewards. Typical APY: 3-5%.".to_string(),
                source: TonDocSource::TonDocs,
                url: "https://docs.ton.org/v3/documentation/dapps/defi/staking".to_string(),
                category: "DeFi".to_string(),
                code_examples: vec![],
                related: vec!["defi-stonfi".to_string()],
                tags: vec!["staking".to_string(), "liquid staking".to_string(), "stton".to_string(), "defi".to_string()],
            },

            // ============================================================================
            // TON Payments / Layer 2
            // ============================================================================

            TonDocArticle {
                id: "ton-payments".to_string(),
                title: "TON Payments (Payment Channels)".to_string(),
                description: "Instant off-chain payments on TON".to_string(),
                content: "TON Payments enables instant, near-zero-fee transactions via payment channels. Similar to Lightning Network. Two parties lock funds in a channel, exchange signed states off-chain, then settle on-chain. Ideal for micropayments.".to_string(),
                source: TonDocSource::TonDocs,
                url: "https://docs.ton.org/v3/documentation/dapps/defi/ton-payments".to_string(),
                category: "Layer 2".to_string(),
                code_examples: vec![
                    TonCodeExample {
                        language: "typescript".to_string(),
                        code: "// Payment channel concept\n// 1. Open channel: both parties deposit funds\n// 2. Off-chain: exchange signed balance updates\n// 3. Close channel: submit final state on-chain\n\n// Example state update (off-chain)\nconst stateUpdate = {\n    channelId: 'abc123',\n    balanceA: toNano('5'),   // Party A has 5 TON\n    balanceB: toNano('15'),  // Party B has 15 TON\n    seqno: 42,\n    signatureA: '...',\n    signatureB: '...'\n};".to_string(),
                        description: Some("Payment channel state update concept".to_string()),
                        is_complete: false,
                    }
                ],
                related: vec!["ton-payments-network".to_string()],
                tags: vec!["payments".to_string(), "layer2".to_string(), "channels".to_string(), "micropayments".to_string()],
            },

            TonDocArticle {
                id: "ton-payments-network".to_string(),
                title: "TON Payment Network (2025)".to_string(),
                description: "Layer-2 payment network in TON's 2025 roadmap".to_string(),
                content: "The TON Payment Network is a Layer-2 solution in TON's 2025 roadmap. Features micro-commissions, near-instant transfers, and seamless asset swaps. Currently in beta. Part of the Accelerator mainnet upgrade.".to_string(),
                source: TonDocSource::TonDocs,
                url: "https://blog.ton.org/".to_string(),
                category: "Layer 2".to_string(),
                code_examples: vec![],
                related: vec!["ton-payments".to_string()],
                tags: vec!["layer2".to_string(), "payment network".to_string(), "2025".to_string(), "roadmap".to_string()],
            },

            // ============================================================================
            // Ecosystem & Integrations
            // ============================================================================

            TonDocArticle {
                id: "telegram-mini-apps".to_string(),
                title: "Telegram Mini Apps on TON".to_string(),
                description: "Building Mini Apps with TON integration".to_string(),
                content: "Telegram Mini Apps (formerly Web Apps) can integrate with TON for payments and authentication. Use TON Connect for wallet connection. Access via Telegram's 950M+ users. Perfect for games, DeFi, and social apps.".to_string(),
                source: TonDocSource::TonDocs,
                url: "https://docs.ton.org/v3/guidelines/ton-connect/integration".to_string(),
                category: "Ecosystem".to_string(),
                code_examples: vec![
                    TonCodeExample {
                        language: "typescript".to_string(),
                        code: "import { TonConnectUI } from '@tonconnect/ui';\n\n// Initialize TON Connect in Mini App\nconst tonConnectUI = new TonConnectUI({\n    manifestUrl: 'https://yourapp.com/tonconnect-manifest.json'\n});\n\n// Connect wallet\nawait tonConnectUI.connectWallet();\n\n// Send transaction\nawait tonConnectUI.sendTransaction({\n    messages: [{\n        address: destinationAddress,\n        amount: toNano('1').toString()\n    }]\n});".to_string(),
                        description: Some("TON Connect in Telegram Mini App".to_string()),
                        is_complete: false,
                    }
                ],
                related: vec!["ton-connect".to_string()],
                tags: vec!["telegram".to_string(), "mini app".to_string(), "web app".to_string(), "integration".to_string()],
            },

            TonDocArticle {
                id: "ton-sdk-js".to_string(),
                title: "TON JavaScript SDK".to_string(),
                description: "Official JavaScript/TypeScript SDK for TON".to_string(),
                content: "@ton/ton is the official SDK for TON development. Provides wallet management, contract deployment, message encoding, and blockchain queries. Works with Node.js and browsers. Successor to TonWeb.".to_string(),
                source: TonDocSource::TonDocs,
                url: "https://docs.ton.org/v3/documentation/smart-contracts/sdk/javascript".to_string(),
                category: "SDKs".to_string(),
                code_examples: vec![
                    TonCodeExample {
                        language: "typescript".to_string(),
                        code: "import { TonClient, WalletContractV4, internal } from '@ton/ton';\nimport { mnemonicToPrivateKey } from '@ton/crypto';\n\n// Initialize client\nconst client = new TonClient({\n    endpoint: 'https://toncenter.com/api/v2/jsonRPC'\n});\n\n// Create wallet from mnemonic\nconst mnemonics = 'word1 word2 ... word24'.split(' ');\nconst keyPair = await mnemonicToPrivateKey(mnemonics);\nconst wallet = WalletContractV4.create({\n    workchain: 0,\n    publicKey: keyPair.publicKey\n});\n\n// Send transaction\nconst contract = client.open(wallet);\nawait contract.sendTransfer({\n    secretKey: keyPair.secretKey,\n    messages: [internal({\n        to: 'EQ...',\n        value: '1',\n        body: 'Hello TON!'\n    })]\n});".to_string(),
                        description: Some("Send transaction using @ton/ton SDK".to_string()),
                        is_complete: true,
                    }
                ],
                related: vec!["ton-connect".to_string()],
                tags: vec!["sdk".to_string(), "javascript".to_string(), "typescript".to_string(), "@ton/ton".to_string()],
            },
        ]
    }

    /// Get a security pattern by ID
    pub fn get_security_pattern(&self, id: &str) -> Option<TonSecurityPattern> {
        self.get_security_patterns()
            .into_iter()
            .find(|p| p.id == id)
    }

    /// Get a documentation article by ID
    pub fn get_documentation_article(&self, id: &str) -> Option<TonDocArticle> {
        self.get_documentation_articles()
            .into_iter()
            .find(|a| a.id == id)
    }

    /// Get the API version
    pub async fn get_version(&self) -> Result<String> {
        let spec = self.get_spec().await?;
        Ok(spec.info.version)
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
        let _client = TonClient::new();
    }

    #[test]
    fn test_security_patterns_exist() {
        let client = TonClient::new();
        let patterns = client.get_security_patterns();
        assert!(!patterns.is_empty(), "Security patterns should exist");
        assert!(
            patterns.len() >= 10,
            "Should have at least 10 security patterns"
        );
    }

    #[test]
    fn test_documentation_articles_exist() {
        let client = TonClient::new();
        let articles = client.get_documentation_articles();
        assert!(!articles.is_empty(), "Documentation articles should exist");
    }

    #[test]
    fn test_additional_technologies() {
        let client = TonClient::new();
        let techs = client.get_additional_technologies();
        assert!(techs.len() >= 5, "Should have additional technologies");

        // Check for key technologies
        let has_tact = techs.iter().any(|t| t.identifier == "ton:tact");
        let has_security = techs.iter().any(|t| t.identifier == "ton:security");
        assert!(has_tact, "Should have Tact technology");
        assert!(has_security, "Should have Security technology");
    }
}
