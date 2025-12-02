use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

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
    pub responses: HashMap<String, Value>,  // Use Value for flexible response parsing
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

/// Normalized technology representation for TON
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TonTechnology {
    pub identifier: String,
    pub title: String,
    pub description: String,
    pub url: String,
    pub endpoint_count: usize,
}

/// Category of TON endpoints (grouped by tag)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TonCategory {
    pub tag: String,
    pub description: String,
    pub endpoints: Vec<TonEndpointSummary>,
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
                    let desc = v.get("description")
                        .and_then(|d| d.as_str())
                        .unwrap_or("Response")
                        .to_string();
                    (k.clone(), desc)
                })
                .collect(),
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
