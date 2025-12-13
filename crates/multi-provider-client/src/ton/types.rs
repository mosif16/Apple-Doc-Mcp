use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

// ============================================================================
// OpenAPI Types (for tonapi.io REST API)
// ============================================================================

/// OpenAPI specification structure (simplified)
/// Uses flatten to capture any extra fields we don't explicitly handle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiSpec {
    pub openapi: String,
    pub info: OpenApiInfo,
    #[serde(default)]
    pub servers: Vec<OpenApiServer>,
    #[serde(default)]
    pub paths: HashMap<String, PathItem>,
    #[serde(default)]
    pub tags: Vec<OpenApiTag>,
    /// Capture all other fields we don't explicitly handle
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

/// Path item can contain HTTP methods plus extra fields like $ref, parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathItem {
    #[serde(default)]
    pub get: Option<OpenApiOperation>,
    #[serde(default)]
    pub post: Option<OpenApiOperation>,
    #[serde(default)]
    pub put: Option<OpenApiOperation>,
    #[serde(default)]
    pub delete: Option<OpenApiOperation>,
    #[serde(default)]
    pub patch: Option<OpenApiOperation>,
    #[serde(default)]
    pub parameters: Vec<OpenApiParameter>,
    /// Capture all other fields
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiInfo {
    pub title: String,
    pub version: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiServer {
    pub url: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiTag {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default, rename = "externalDocs")]
    pub external_docs: Option<OpenApiExternalDocs>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiExternalDocs {
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiOperation {
    #[serde(rename = "operationId")]
    pub operation_id: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub parameters: Vec<OpenApiParameter>,
    #[serde(default)]
    pub responses: HashMap<String, Value>,
    /// Capture extra fields
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

impl PathItem {
    /// Get all operations with their HTTP method
    pub fn operations(&self) -> Vec<(&str, &OpenApiOperation)> {
        let mut ops = Vec::new();
        if let Some(op) = &self.get {
            ops.push(("get", op));
        }
        if let Some(op) = &self.post {
            ops.push(("post", op));
        }
        if let Some(op) = &self.put {
            ops.push(("put", op));
        }
        if let Some(op) = &self.delete {
            ops.push(("delete", op));
        }
        if let Some(op) = &self.patch {
            ops.push(("patch", op));
        }
        ops
    }
}

/// OpenAPI parameter - can be inline or a $ref reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiParameter {
    /// Name (optional if this is a $ref)
    #[serde(default)]
    pub name: Option<String>,
    /// Location: "path", "query", "header", "cookie" (optional if $ref)
    #[serde(rename = "in", default)]
    pub location: Option<String>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub schema: Option<OpenApiSchema>,
    /// Reference to a parameter defined in components/parameters
    #[serde(rename = "$ref", default)]
    pub ref_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiSchema {
    #[serde(rename = "type")]
    pub schema_type: Option<String>,
    #[serde(default)]
    pub format: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiResponse {
    pub description: String,
}

// ============================================================================
// TON Documentation Source Types
// ============================================================================

/// Represents the source of TON documentation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TonDocSource {
    /// tonapi.io REST API (OpenAPI spec)
    TonApi,
    /// docs.ton.org official documentation
    TonDocs,
    /// Tact language documentation (docs.tact-lang.org)
    TactLang,
    /// Security best practices
    Security,
    /// TVM (TON Virtual Machine) documentation
    Tvm,
    /// FunC language documentation
    FunC,
    /// Tolk language documentation (new)
    Tolk,
}

impl TonDocSource {
    pub fn name(&self) -> &'static str {
        match self {
            TonDocSource::TonApi => "TON API",
            TonDocSource::TonDocs => "TON Docs",
            TonDocSource::TactLang => "Tact Language",
            TonDocSource::Security => "TON Security",
            TonDocSource::Tvm => "TVM",
            TonDocSource::FunC => "FunC",
            TonDocSource::Tolk => "Tolk",
        }
    }

    pub fn base_url(&self) -> &'static str {
        match self {
            TonDocSource::TonApi => "https://tonapi.io",
            TonDocSource::TonDocs => "https://docs.ton.org",
            TonDocSource::TactLang => "https://docs.tact-lang.org",
            TonDocSource::Security => "https://docs.ton.org/v3/guidelines/smart-contracts/security",
            TonDocSource::Tvm => "https://docs.ton.org/v3/documentation/tvm",
            TonDocSource::FunC => "https://docs.ton.org/v3/documentation/smart-contracts/func",
            TonDocSource::Tolk => "https://docs.ton.org/v3/documentation/smart-contracts/tolk",
        }
    }
}

// ============================================================================
// Unified TON Technology Types
// ============================================================================

/// Normalized technology representation for TON
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TonTechnology {
    pub identifier: String,
    pub title: String,
    pub description: String,
    pub url: String,
    pub endpoint_count: usize,
    /// The documentation source for this technology
    #[serde(default = "default_source")]
    pub source: TonDocSource,
}

fn default_source() -> TonDocSource {
    TonDocSource::TonApi
}

/// Category of TON endpoints (grouped by tag)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TonCategory {
    pub tag: String,
    pub description: String,
    pub endpoints: Vec<TonEndpointSummary>,
    #[serde(default = "default_source")]
    pub source: TonDocSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TonEndpointSummary {
    pub operation_id: String,
    pub method: String,
    pub path: String,
    pub summary: Option<String>,
    pub description: Option<String>,
}

/// Detailed endpoint information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TonEndpoint {
    pub operation_id: String,
    pub method: String,
    pub path: String,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub parameters: Vec<TonParameterSpec>,
    pub responses: HashMap<String, String>,
    #[serde(default = "default_source")]
    pub source: TonDocSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TonParameterSpec {
    pub name: String,
    pub location: String,
    pub required: bool,
    pub description: Option<String>,
    pub schema_type: Option<String>,
}

impl TonEndpoint {
    pub fn from_openapi(path: &str, method: &str, op: &OpenApiOperation) -> Self {
        Self {
            operation_id: op
                .operation_id
                .clone()
                .unwrap_or_else(|| format!("{}_{}", method, path.replace('/', "_"))),
            method: method.to_string(),
            path: path.to_string(),
            summary: op.summary.clone(),
            description: op.description.clone(),
            tags: op.tags.clone(),
            parameters: op
                .parameters
                .iter()
                .filter_map(|p| {
                    // Only include parameters with a name (skip $ref for now)
                    p.name.as_ref().map(|name| TonParameterSpec {
                        name: name.clone(),
                        location: p.location.clone().unwrap_or_else(|| "query".to_string()),
                        required: p.required,
                        description: p.description.clone(),
                        schema_type: p.schema.as_ref().and_then(|s| s.schema_type.clone()),
                    })
                })
                .collect(),
            responses: op
                .responses
                .iter()
                .map(|(k, v)| {
                    // Try to extract description from response value
                    let desc = v
                        .get("description")
                        .and_then(|d| d.as_str())
                        .unwrap_or("Response")
                        .to_string();
                    (k.clone(), desc)
                })
                .collect(),
            source: TonDocSource::TonApi,
        }
    }
}

impl TonEndpointSummary {
    pub fn from_openapi(path: &str, method: &str, op: &OpenApiOperation) -> Self {
        Self {
            operation_id: op
                .operation_id
                .clone()
                .unwrap_or_else(|| format!("{}_{}", method, path.replace('/', "_"))),
            method: method.to_string(),
            path: path.to_string(),
            summary: op.summary.clone(),
            description: op.description.clone(),
        }
    }
}

// ============================================================================
// Smart Contract Language Types
// ============================================================================

/// Smart contract programming language on TON
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TonContractLanguage {
    /// FunC - The original low-level language
    FunC,
    /// Tact - High-level TypeScript-like language
    Tact,
    /// Tolk - Next-generation language replacing FunC
    Tolk,
    /// Fift - Stack-based assembler language
    Fift,
}

impl TonContractLanguage {
    pub fn name(&self) -> &'static str {
        match self {
            TonContractLanguage::FunC => "FunC",
            TonContractLanguage::Tact => "Tact",
            TonContractLanguage::Tolk => "Tolk",
            TonContractLanguage::Fift => "Fift",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            TonContractLanguage::FunC => {
                "Domain-specific, C-like, statically typed language for low-level TON smart contracts"
            }
            TonContractLanguage::Tact => {
                "High-level, TypeScript-like language focused on efficiency and simplicity"
            }
            TonContractLanguage::Tolk => {
                "Next-generation language replacing FunC with expressive syntax and robust type system"
            }
            TonContractLanguage::Fift => {
                "Stack-based programming language for managing TON smart contracts and TVM assembly"
            }
        }
    }
}

// ============================================================================
// TON Documentation Article Types
// ============================================================================

/// A documentation article from docs.ton.org or docs.tact-lang.org
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TonDocArticle {
    /// Unique identifier (slug or path)
    pub id: String,
    /// Article title
    pub title: String,
    /// Short description/abstract
    pub description: String,
    /// Full markdown content
    pub content: String,
    /// Documentation source
    pub source: TonDocSource,
    /// URL to the article
    pub url: String,
    /// Category/section the article belongs to
    pub category: String,
    /// Code examples in the article
    #[serde(default)]
    pub code_examples: Vec<TonCodeExample>,
    /// Related article IDs
    #[serde(default)]
    pub related: Vec<String>,
    /// Tags for searchability
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Code example from TON documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TonCodeExample {
    /// Programming language (func, tact, tolk, fift, typescript, etc.)
    pub language: String,
    /// The code snippet
    pub code: String,
    /// Optional description
    pub description: Option<String>,
    /// Whether this is a complete runnable example
    #[serde(default)]
    pub is_complete: bool,
}

// ============================================================================
// Security Best Practices Types
// ============================================================================

/// TON security vulnerability category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TonSecurityCategory {
    /// Integer handling issues (overflow, underflow, signed/unsigned)
    IntegerHandling,
    /// Message handling vulnerabilities
    MessageHandling,
    /// Replay attack vulnerabilities
    ReplayProtection,
    /// Gas-related vulnerabilities
    GasManagement,
    /// Access control issues
    AccessControl,
    /// Data storage vulnerabilities
    DataStorage,
    /// Randomness issues
    Randomness,
    /// Race condition vulnerabilities
    RaceConditions,
    /// Code upgrade vulnerabilities
    CodeUpgrade,
    /// External call vulnerabilities
    ExternalCalls,
}

impl TonSecurityCategory {
    pub fn name(&self) -> &'static str {
        match self {
            TonSecurityCategory::IntegerHandling => "Integer Handling",
            TonSecurityCategory::MessageHandling => "Message Handling",
            TonSecurityCategory::ReplayProtection => "Replay Protection",
            TonSecurityCategory::GasManagement => "Gas Management",
            TonSecurityCategory::AccessControl => "Access Control",
            TonSecurityCategory::DataStorage => "Data Storage",
            TonSecurityCategory::Randomness => "Randomness",
            TonSecurityCategory::RaceConditions => "Race Conditions",
            TonSecurityCategory::CodeUpgrade => "Code Upgrade",
            TonSecurityCategory::ExternalCalls => "External Calls",
        }
    }
}

/// A security best practice or vulnerability pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TonSecurityPattern {
    /// Unique identifier
    pub id: String,
    /// Title of the security pattern
    pub title: String,
    /// Category
    pub category: TonSecurityCategory,
    /// Severity level (critical, high, medium, low)
    pub severity: String,
    /// Description of the vulnerability/pattern
    pub description: String,
    /// What NOT to do (vulnerable code)
    pub vulnerable_pattern: Option<TonCodeExample>,
    /// What TO do (secure code)
    pub secure_pattern: Option<TonCodeExample>,
    /// Mitigation steps
    pub mitigations: Vec<String>,
    /// Related patterns
    #[serde(default)]
    pub related: Vec<String>,
}

// ============================================================================
// TVM (TON Virtual Machine) Types
// ============================================================================

/// TVM instruction category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TvmInstructionCategory {
    /// Stack manipulation (PUSH, POP, XCHG, etc.)
    Stack,
    /// Arithmetic operations
    Arithmetic,
    /// Comparison operations
    Comparison,
    /// Cell operations
    Cell,
    /// Control flow (IFELSE, WHILE, etc.)
    ControlFlow,
    /// Dictionary operations
    Dictionary,
    /// Cryptographic operations
    Crypto,
    /// Debug operations
    Debug,
    /// Other/Miscellaneous
    Other,
}

/// A TVM instruction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvmInstruction {
    /// Instruction mnemonic (e.g., "PUSHINT", "ADD")
    pub mnemonic: String,
    /// Opcode (hex)
    pub opcode: String,
    /// Stack notation (e.g., "x y - x+y")
    pub stack_notation: Option<String>,
    /// Description
    pub description: String,
    /// Gas cost
    pub gas_cost: Option<u32>,
    /// Category
    pub category: TvmInstructionCategory,
    /// Fift equivalent
    pub fift_equivalent: Option<String>,
}

// ============================================================================
// Jetton/NFT Token Types
// ============================================================================

/// TON token standard type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TonTokenStandard {
    /// Jetton (fungible token, like ERC-20)
    Jetton,
    /// NFT (non-fungible token)
    Nft,
    /// SBT (Soul Bound Token)
    Sbt,
}

impl TonTokenStandard {
    pub fn name(&self) -> &'static str {
        match self {
            TonTokenStandard::Jetton => "Jetton (TEP-74)",
            TonTokenStandard::Nft => "NFT (TEP-62)",
            TonTokenStandard::Sbt => "SBT (TEP-85)",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            TonTokenStandard::Jetton => {
                "Fungible token standard for TON, similar to ERC-20 on Ethereum"
            }
            TonTokenStandard::Nft => {
                "Non-fungible token standard for TON, similar to ERC-721 on Ethereum"
            }
            TonTokenStandard::Sbt => "Soul Bound Token standard - non-transferable NFTs for identity",
        }
    }
}

/// Token standard documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TonTokenDoc {
    /// Token standard
    pub standard: TonTokenStandard,
    /// TEP number (TON Enhancement Proposal)
    pub tep: String,
    /// Description
    pub description: String,
    /// Contract interfaces
    pub interfaces: Vec<TonContractInterface>,
    /// Example implementations
    pub examples: Vec<TonCodeExample>,
}

/// Smart contract interface definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TonContractInterface {
    /// Interface name (e.g., "jetton_wallet", "nft_item")
    pub name: String,
    /// Get methods
    pub get_methods: Vec<TonGetMethod>,
    /// Internal message handlers
    pub internal_handlers: Vec<TonMessageHandler>,
    /// External message handlers (if any)
    #[serde(default)]
    pub external_handlers: Vec<TonMessageHandler>,
}

/// A get method on a TON contract
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TonGetMethod {
    /// Method name
    pub name: String,
    /// Parameters
    pub parameters: Vec<TonMethodParam>,
    /// Return type description
    pub returns: String,
    /// Description
    pub description: String,
}

/// A message handler in a TON contract
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TonMessageHandler {
    /// Handler name or op code
    pub name: String,
    /// Op code (hex)
    pub op_code: Option<String>,
    /// Message body structure
    pub body: Vec<TonMethodParam>,
    /// Description
    pub description: String,
}

/// A parameter for methods or messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TonMethodParam {
    /// Parameter name
    pub name: String,
    /// Type (e.g., "int", "slice", "cell", "address")
    pub param_type: String,
    /// Description
    pub description: Option<String>,
}

// ============================================================================
// Wallet Types
// ============================================================================

/// TON wallet version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TonWalletVersion {
    /// Version name (e.g., "v3r2", "v4r2", "v5")
    pub version: String,
    /// Description
    pub description: String,
    /// Features supported
    pub features: Vec<String>,
    /// Whether this is the recommended version
    pub recommended: bool,
    /// Code hash
    pub code_hash: Option<String>,
}

// ============================================================================
// Search Result Types
// ============================================================================

/// Unified search result across all TON documentation sources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TonSearchResult {
    /// Result ID
    pub id: String,
    /// Result title
    pub title: String,
    /// Short description
    pub description: String,
    /// Documentation source
    pub source: TonDocSource,
    /// URL to the documentation
    pub url: String,
    /// Result type (api, article, security, tvm, etc.)
    pub result_type: TonResultType,
    /// Relevance score
    pub score: f32,
    /// Code examples (if any)
    #[serde(default)]
    pub code_examples: Vec<TonCodeExample>,
}

/// Type of search result
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TonResultType {
    /// API endpoint from tonapi.io
    ApiEndpoint,
    /// Documentation article
    Article,
    /// Security pattern/best practice
    Security,
    /// TVM instruction
    TvmInstruction,
    /// Smart contract example
    ContractExample,
    /// Token standard documentation
    TokenStandard,
    /// Wallet documentation
    Wallet,
}

impl TonResultType {
    pub fn name(&self) -> &'static str {
        match self {
            TonResultType::ApiEndpoint => "API Endpoint",
            TonResultType::Article => "Documentation",
            TonResultType::Security => "Security",
            TonResultType::TvmInstruction => "TVM Instruction",
            TonResultType::ContractExample => "Contract Example",
            TonResultType::TokenStandard => "Token Standard",
            TonResultType::Wallet => "Wallet",
        }
    }
}
