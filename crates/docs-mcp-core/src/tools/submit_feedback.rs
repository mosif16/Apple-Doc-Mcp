use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
use time::OffsetDateTime;
use tokio::io::AsyncWriteExt;

use crate::state::{AppContext, ToolDefinition, ToolHandler, ToolResponse};
use crate::tools::{parse_args, text_response, wrap_handler};

const FEEDBACK_DIR_ENV: &str = "DOCSMCP_FEEDBACK_DIR";

#[derive(Debug, Deserialize)]
struct Args {
    /// Free-form feedback: what worked, what didn’t, what you wish existed.
    feedback: String,
    /// Optional 1–5 rating for overall usefulness.
    rating: Option<u8>,
    /// Bullet suggestions (short, actionable).
    #[serde(default)]
    improvements: Vec<String>,
    /// Concrete examples of missing coverage (queries, symbols, providers).
    #[serde(default, rename = "missingDocs")]
    missing_docs: Vec<String>,
    /// What slowed you down (latency, irrelevant results, formatting, etc.).
    #[serde(default, rename = "painPoints")]
    pain_points: Vec<String>,
    /// Optional client/agent metadata to help reproduce issues.
    #[serde(default)]
    client: Option<ClientInfo>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct ClientInfo {
    #[serde(default, rename = "agentName")]
    agent_name: Option<String>,
    #[serde(default, rename = "agentVersion")]
    agent_version: Option<String>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    platform: Option<String>,
}

#[derive(Debug, Serialize)]
struct FeedbackRecord {
    schema_version: u32,
    #[serde(with = "time::serde::rfc3339")]
    timestamp: OffsetDateTime,
    server: ServerInfo,
    client: Option<ClientInfo>,
    rating: Option<u8>,
    feedback: String,
    improvements: Vec<String>,
    missing_docs: Vec<String>,
    pain_points: Vec<String>,
    environment: serde_json::Value,
    diagnostics: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct ServerInfo {
    name: &'static str,
    version: &'static str,
}

pub fn definition() -> (ToolDefinition, ToolHandler) {
    let definition = ToolDefinition {
        name: "submit_feedback".to_string(),
        description: "Submit feedback about docs-mcp (what worked, what’s missing, how to improve). Writes a structured JSON record into the `Feedback/` folder.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "feedback": {
                    "type": "string",
                    "description": "Free-form feedback: what worked, what didn’t, what you wish existed."
                },
                "rating": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 5,
                    "description": "Optional 1–5 rating for overall usefulness."
                },
                "improvements": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Bullet suggestions (short, actionable)."
                },
                "missingDocs": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Concrete examples of missing coverage (queries, symbols, providers)."
                },
                "painPoints": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "What slowed you down (latency, irrelevant results, formatting, etc.)."
                },
                "client": {
                    "type": "object",
                    "description": "Optional metadata about the calling agent/client.",
                    "properties": {
                        "agentName": {"type": "string"},
                        "agentVersion": {"type": "string"},
                        "model": {"type": "string"},
                        "platform": {"type": "string"}
                    },
                    "additionalProperties": true
                }
            },
            "required": ["feedback"],
            "additionalProperties": false
        }),
        input_examples: Some(vec![
            json!({
                "feedback": "Search is fast, but results for AppKit are often empty. Would love better 'no results' guidance + suggested alternate queries.",
                "rating": 4,
                "improvements": ["Add fuzzy matching for typos", "Expose provider in output header"],
                "missingDocs": ["UIKit UITableViewDiffableDataSource", "AppKit NSAttributedString paragraphStyle"],
                "painPoints": ["Sometimes top docs are too long; would like a shorter 'key points' section first"],
                "client": {"agentName": "Codex CLI", "model": "gpt-5.2"}
            })
        ]),
        allowed_callers: None,
    };

    let handler = wrap_handler(handle_submit_feedback);
    (definition, handler)
}

async fn handle_submit_feedback(context: Arc<AppContext>, value: serde_json::Value) -> Result<ToolResponse> {
    let args: Args = parse_args(value)?;
    validate_args(&args)?;

    let saved_path = write_feedback(&context, args).await?;
    Ok(text_response([format!(
        "Saved feedback to {}. Thank you — this directly guides what we improve next.",
        saved_path.display()
    )])
    .with_metadata(json!({
        "savedPath": saved_path.display().to_string(),
        "schemaVersion": 1
    })))
}

fn validate_args(args: &Args) -> Result<()> {
    if let Some(rating) = args.rating {
        if !(1..=5).contains(&rating) {
            return Err(anyhow!("rating must be between 1 and 5"));
        }
    }
    if args.feedback.trim().is_empty() {
        return Err(anyhow!("feedback must be a non-empty string"));
    }
    Ok(())
}

async fn write_feedback(context: &Arc<AppContext>, args: Args) -> Result<PathBuf> {
    let dir = resolve_feedback_dir()?;
    tokio::fs::create_dir_all(&dir)
        .await
        .with_context(|| format!("create feedback dir {}", dir.display()))?;

    let now = OffsetDateTime::now_utc();
    let pid = std::process::id();
    let file_name = format!(
        "feedback_{}_{}_pid{}.json",
        now.unix_timestamp(),
        now.nanosecond(),
        pid
    );
    let final_path = dir.join(file_name);
    let tmp_path = dir.join(format!(".{}.tmp", final_path.file_name().unwrap().to_string_lossy()));

    let record = build_record(context, args).await?;
    let bytes = serde_json::to_vec_pretty(&record).context("serialize feedback")?;

    let mut file = tokio::fs::File::create(&tmp_path)
        .await
        .with_context(|| format!("create temp feedback file {}", tmp_path.display()))?;
    file.write_all(&bytes).await.context("write feedback")?;
    file.write_all(b"\n").await.context("write newline")?;
    file.flush().await.context("flush feedback file")?;
    drop(file);

    tokio::fs::rename(&tmp_path, &final_path)
        .await
        .with_context(|| format!("rename {} -> {}", tmp_path.display(), final_path.display()))?;

    Ok(final_path)
}

fn resolve_feedback_dir() -> Result<PathBuf> {
    match std::env::var_os(FEEDBACK_DIR_ENV) {
        Some(value) => Ok(PathBuf::from(value)),
        None => Ok(PathBuf::from("Feedback")),
    }
}

async fn build_record(context: &Arc<AppContext>, args: Args) -> Result<FeedbackRecord> {
    let telemetry = context.telemetry_snapshot().await;

    let recent_queries = context.state.recent_queries.lock().await.clone();
    let active_provider = *context.state.active_provider.read().await;
    let active_unified = context.state.active_unified_technology.read().await.clone();
    let active_apple = context.state.active_technology.read().await.clone();

    let cache_stats = context.cache_stats();

    let environment = json!({
        "user": std::env::var("USER").ok(),
        "host": std::env::var("HOSTNAME").ok(),
        "cwd": std::env::current_dir().ok().map(|p| p.display().to_string()),
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "pid": std::process::id(),
    });

    let diagnostics = json!({
        "activeProvider": format!("{active_provider:?}"),
        "activeUnifiedTechnology": active_unified.as_ref().map(|t| t.title.clone()),
        "activeAppleTechnology": active_apple.as_ref().map(|t| t.title.clone()),
        "cacheStats": cache_stats,
        "telemetry": telemetry,
        "recentQueries": recent_queries,
    });

    Ok(FeedbackRecord {
        schema_version: 1,
        timestamp: OffsetDateTime::now_utc(),
        server: ServerInfo {
            name: "docs-mcp",
            version: env!("CARGO_PKG_VERSION"),
        },
        client: args.client,
        rating: args.rating,
        feedback: args.feedback,
        improvements: args.improvements,
        missing_docs: args.missing_docs,
        pain_points: args.pain_points,
        environment,
        diagnostics,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use docs_mcp_client::AppleDocsClient;
    use tempfile::tempdir;

    #[tokio::test]
    async fn writes_feedback_json_record() {
        let dir = tempdir().expect("tempdir");
        std::env::set_var(FEEDBACK_DIR_ENV, dir.path());

        let context = Arc::new(AppContext::new(AppleDocsClient::new()));
        let args = Args {
            feedback: "Hello".to_string(),
            rating: Some(5),
            improvements: vec!["Improve ranking".to_string()],
            missing_docs: vec![],
            pain_points: vec![],
            client: Some(ClientInfo {
                agent_name: Some("test".to_string()),
                agent_version: None,
                model: None,
                platform: None,
            }),
        };

        let path = write_feedback(&context, args).await.expect("write");
        assert!(path.exists(), "expected feedback file to exist");

        let bytes = tokio::fs::read(&path).await.expect("read");
        let parsed: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
        assert_eq!(parsed.get("schema_version").and_then(|v| v.as_u64()), Some(1));
        assert_eq!(parsed.get("feedback").and_then(|v| v.as_str()), Some("Hello"));
    }

    #[test]
    fn rating_validation_rejects_out_of_range() {
        let args = Args {
            feedback: "Hi".to_string(),
            rating: Some(6),
            improvements: vec![],
            missing_docs: vec![],
            pain_points: vec![],
            client: None,
        };
        let err = validate_args(&args).unwrap_err().to_string();
        assert!(err.contains("between 1 and 5"));
    }

    #[test]
    fn feedback_validation_rejects_empty() {
        let args = Args {
            feedback: "   ".to_string(),
            rating: None,
            improvements: vec![],
            missing_docs: vec![],
            pain_points: vec![],
            client: None,
        };
        let err = validate_args(&args).unwrap_err().to_string();
        assert!(err.contains("non-empty"));
    }
}
