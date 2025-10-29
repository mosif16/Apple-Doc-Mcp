use std::{fs, path::PathBuf, sync::Arc};

use anyhow::{anyhow, Context, Result};
use apple_docs_core::{
    bootstrap, state::AppContext, ServerConfig, ServerMode, ToolExecutor, ToolExecutorError,
};
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::Shell;
use indicatif::ProgressBar;
use output::{OutputFormat, Renderer};
use progress::spinner;
use serde::Serialize;
use serde_json::Value;
use tracing::info;
use tracing_subscriber::{fmt, EnvFilter};

#[derive(Debug, Parser, Clone)]
#[command(
    name = "apple-docs",
    version,
    about = "Interact with the Apple Developer Documentation MCP tooling from the shell."
)]
struct Cli {
    /// Preferred renderer for command output.
    #[arg(long, global = true, value_enum, default_value = "markdown")]
    format: OutputFormat,
    /// Override the cache directory used by the Apple Docs client.
    #[arg(long, global = true)]
    cache_dir: Option<PathBuf>,
    /// Disable ANSI colors in CLI output.
    #[arg(long, global = true)]
    no_color: bool,
    /// Suppress non-critical CLI output.
    #[arg(long, global = true)]
    quiet: bool,
    /// Disable progress indicators for long-running tasks.
    #[arg(long, global = true)]
    no_progress: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand, Clone)]
enum Command {
    /// Run the MCP server over STDIO (JSON-RPC transport).
    Serve,
    /// Inspect and invoke available tools.
    Tools {
        #[command(subcommand)]
        command: ToolCommand,
    },
    /// Manage cache state for offline workflows.
    Cache {
        #[command(subcommand)]
        command: CacheCommand,
    },
    /// View recent tool telemetry captured by the server.
    Telemetry {
        /// Maximum number of telemetry entries to display (0 = all).
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    /// Generate shell completion scripts.
    Completions {
        #[arg(value_enum)]
        shell: Shell,
    },
}

#[derive(Debug, Subcommand, Clone)]
enum ToolCommand {
    /// List registered tools and their descriptions.
    List,
    /// Execute a tool by name with optional JSON arguments.
    Call {
        name: String,
        /// Tool arguments expressed as JSON (`{"key": "value"}`) or @path to a JSON file.
        #[arg(short, long)]
        arguments: Option<String>,
        /// Stream intermediate output when supported.
        #[arg(long)]
        stream: bool,
    },
}

#[derive(Debug, Subcommand, Clone)]
enum CacheCommand {
    /// Report cache directory status and entry counts.
    Status,
    /// Warm caches by fetching remote metadata and frameworks.
    Warmup {
        /// Framework identifiers to prefetch (may be repeated).
        #[arg(long, alias = "framework")]
        frameworks: Vec<String>,
        /// Skip the global technologies index refresh.
        #[arg(long)]
        skip_technologies: bool,
        /// Force a fresh download instead of serving cached entries.
        #[arg(long)]
        refresh: bool,
    },
    /// Clear the in-memory caches while keeping disk artifacts.
    ClearMemory,
}

#[derive(Clone, Debug, Serialize)]
struct CacheStatusReport {
    path: String,
    exists: bool,
    readable: bool,
    file_count: usize,
}

#[derive(Clone, Debug, Serialize)]
struct WarmupSummary {
    refreshed: bool,
    technologies_cached: bool,
    technology_count: Option<usize>,
    frameworks_cached: Vec<String>,
}

impl Cli {
    fn progress_enabled(&self) -> bool {
        !self.quiet && !self.no_progress
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    init_tracing(&cli)?;

    if cli.no_color {
        std::env::set_var("NO_COLOR", "1");
    }

    let mut config = ServerConfig::default();
    config.cache_dir = cli.cache_dir.clone();
    config.mode = match cli.command {
        Command::Serve => ServerMode::Stdio,
        _ => ServerMode::Headless,
    };

    let runtime = bootstrap(config).await?;
    let executor = runtime.executor();
    let session = session::SessionManager::new(executor.context());
    session.restore().await;

    let result: Result<()> = match &cli.command {
        Command::Serve => runtime.clone().serve().await,
        Command::Completions { shell } => {
            let mut command = Cli::command();
            clap_complete::generate(*shell, &mut command, "apple-docs", &mut std::io::stdout());
            Ok(())
        }
        Command::Tools { command } => {
            let renderer = Renderer::new(cli.format);
            handle_tool_command(command.clone(), &cli, &renderer, executor.clone()).await
        }
        Command::Cache { command } => {
            let renderer = Renderer::new(cli.format);
            handle_cache_command(command.clone(), &cli, &renderer, executor.clone()).await
        }
        Command::Telemetry { limit } => {
            let renderer = Renderer::new(cli.format);
            handle_telemetry_command(*limit, &cli, &renderer, executor.clone()).await
        }
    };

    session.persist().await;
    result
}

async fn handle_tool_command(
    command: ToolCommand,
    cli: &Cli,
    renderer: &Renderer,
    executor: ToolExecutor,
) -> Result<()> {
    match command {
        ToolCommand::List => {
            let definitions = executor.list_tools().await;
            if cli.quiet {
                return Ok(());
            }
            renderer.tool_definitions(&definitions)?;
        }
        ToolCommand::Call {
            name,
            arguments,
            stream,
        } => {
            let payload = parse_arguments(arguments)?;
            let label = if stream {
                format!("Streaming `{name}`...")
            } else {
                format!("Calling `{name}`...")
            };
            let spinner = spinner(cli.progress_enabled(), label);
            let result = executor.call_tool(&name, payload).await;
            match result {
                Ok(response) => {
                    finish_spinner(spinner, Some(format!("Tool `{name}` completed")));
                    if !cli.quiet {
                        renderer.tool_response(&response)?;
                    }
                }
                Err(ToolExecutorError::UnknownTool(_)) => {
                    finish_spinner(spinner, None);
                    anyhow::bail!("unknown tool: {name}");
                }
                Err(ToolExecutorError::Execution { source, .. }) => {
                    finish_spinner(spinner, None);
                    return Err(source.context(format!("tool `{name}` failed")));
                }
            }
        }
    }

    Ok(())
}

async fn handle_cache_command(
    command: CacheCommand,
    cli: &Cli,
    renderer: &Renderer,
    executor: ToolExecutor,
) -> Result<()> {
    let context = executor.context();
    match command {
        CacheCommand::Status => {
            let path = context.client.cache_dir().clone();
            let (exists, readable, file_count) = match fs::read_dir(&path) {
                Ok(entries) => {
                    let count = entries.filter_map(std::result::Result::ok).count();
                    (true, true, count)
                }
                Err(error) => {
                    let exists = path.exists();
                    let readable = false;
                    info!(
                        target: "apple_docs_cli",
                        error = %error,
                        path = %path.display(),
                        "unable to inspect cache directory"
                    );
                    (exists, readable, 0)
                }
            };

            if cli.quiet {
                return Ok(());
            }

            let report = CacheStatusReport {
                path: path.display().to_string(),
                exists,
                readable,
                file_count,
            };
            renderer.cache_status(&report)?;
        }
        CacheCommand::Warmup {
            frameworks,
            skip_technologies,
            refresh,
        } => {
            let client = context.client.clone();
            let mut technology_count = None;
            if !skip_technologies {
                let label = if refresh {
                    "Refreshing technologies index..."
                } else {
                    "Loading technologies index..."
                };
                let spinner = spinner(cli.progress_enabled(), label);
                let result = if refresh {
                    client.refresh_technologies().await
                } else {
                    client.get_technologies().await
                };
                match result {
                    Ok(map) => {
                        technology_count = Some(map.len());
                        finish_spinner(spinner, Some(format!("Cached {} technologies", map.len())));
                    }
                    Err(error) => {
                        finish_spinner(spinner, None);
                        return Err(error.context("failed to warm technology index"));
                    }
                }
            }

            let mut cached_frameworks = Vec::new();
            for framework in frameworks {
                let label = if refresh {
                    format!("Refreshing framework `{framework}`...")
                } else {
                    format!("Loading framework `{framework}`...")
                };
                let spinner = spinner(cli.progress_enabled(), label);
                let result = if refresh {
                    client.refresh_framework(&framework).await
                } else {
                    client.get_framework(&framework).await
                };
                match result {
                    Ok(_) => {
                        cached_frameworks.push(framework.clone());
                        finish_spinner(spinner, Some(format!("Cached framework `{framework}`")));
                    }
                    Err(error) => {
                        finish_spinner(spinner, None);
                        return Err(
                            error.context(format!("failed to warm framework `{framework}`"))
                        );
                    }
                }
            }

            if cli.quiet {
                return Ok(());
            }

            let summary = WarmupSummary {
                refreshed: refresh,
                technologies_cached: !skip_technologies,
                technology_count,
                frameworks_cached: cached_frameworks,
            };
            renderer.cache_warmup(&summary)?;
        }
        CacheCommand::ClearMemory => {
            context.client.clear_memory_cache();
            if cli.quiet {
                return Ok(());
            }
            renderer.cache_cleared()?;
        }
    }
    Ok(())
}

async fn handle_telemetry_command(
    limit: usize,
    cli: &Cli,
    renderer: &Renderer,
    executor: ToolExecutor,
) -> Result<()> {
    if cli.quiet {
        return Ok(());
    }

    let context = executor.context();
    let entries = context.telemetry_snapshot().await;
    if entries.is_empty() {
        renderer.no_telemetry()?;
        return Ok(());
    }

    let total = entries.len();
    let start = if limit == 0 {
        0
    } else {
        total.saturating_sub(limit)
    };
    let sliced: Vec<_> = entries.into_iter().skip(start).collect();
    renderer.telemetry(&sliced)?;
    Ok(())
}

fn init_tracing(cli: &Cli) -> Result<()> {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,apple_docs_cli=info"));
    fmt()
        .with_env_filter(filter)
        .with_target(true)
        .without_time()
        .with_ansi(!cli.no_color)
        .compact()
        .with_writer(std::io::stderr)
        .try_init()
        .map_err(|error| anyhow!("failed to initialize logging: {error}"))
}

fn parse_arguments(arguments: Option<String>) -> Result<Value> {
    match arguments {
        Some(raw) if raw.starts_with('@') => {
            let path = raw.trim_start_matches('@');
            let contents =
                fs::read_to_string(path).with_context(|| format!("failed to read {path}"))?;
            serde_json::from_str(&contents)
                .with_context(|| format!("invalid JSON arguments in {path}"))
        }
        Some(raw) => serde_json::from_str(&raw).context("invalid JSON arguments"),
        None => Ok(Value::Object(Default::default())),
    }
}

fn finish_spinner(spinner: Option<ProgressBar>, message: Option<String>) {
    if let Some(progress) = spinner {
        if let Some(msg) = message {
            progress.finish_with_message(msg);
        } else {
            progress.finish_and_clear();
        }
    }
}

mod output {
    use std::fmt::Write;

    use anyhow::Result;
    use apple_docs_core::state::{TelemetryEntry, ToolDefinition, ToolResponse};
    use clap::ValueEnum;
    use serde_json::{self, json};

    #[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
    pub enum OutputFormat {
        Json,
        Markdown,
        Table,
        Text,
    }

    #[derive(Copy, Clone, Debug)]
    pub struct Renderer {
        format: OutputFormat,
    }

    impl Renderer {
        pub fn new(format: OutputFormat) -> Self {
            Self { format }
        }

        pub fn tool_definitions(&self, definitions: &[ToolDefinition]) -> Result<()> {
            match self.format {
                OutputFormat::Json => {
                    let payload = json!({ "tools": definitions });
                    println!("{}", serde_json::to_string_pretty(&payload)?);
                }
                OutputFormat::Markdown => {
                    println!("| Tool | Description |");
                    println!("| --- | --- |");
                    for entry in definitions {
                        println!("| `{}` | {} |", entry.name, sanitize(&entry.description));
                    }
                }
                OutputFormat::Table => {
                    let rows: Vec<Vec<String>> = definitions
                        .iter()
                        .map(|entry| {
                            vec![
                                entry.name.clone(),
                                truncate(&sanitize(&entry.description), 80),
                            ]
                        })
                        .collect();
                    render_table(&["Tool", "Description"], &rows);
                }
                OutputFormat::Text => {
                    for entry in definitions {
                        println!("• {} — {}", entry.name, entry.description);
                    }
                }
            }
            Ok(())
        }

        pub fn tool_response(&self, response: &ToolResponse) -> Result<()> {
            match self.format {
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(response)?);
                }
                OutputFormat::Markdown | OutputFormat::Text => {
                    for content in &response.content {
                        println!("## {}", content.r#type);
                        println!();
                        println!("{}", content.text.trim());
                        println!();
                    }
                    if let Some(metadata) = &response.metadata {
                        println!("```json");
                        println!("{}", serde_json::to_string_pretty(metadata)?);
                        println!("```");
                    }
                }
                OutputFormat::Table => {
                    let rows: Vec<Vec<String>> = response
                        .content
                        .iter()
                        .map(|content| {
                            vec![
                                content.r#type.clone(),
                                truncate(&sanitize(&content.text), 120),
                            ]
                        })
                        .collect();
                    render_table(&["Type", "Content"], &rows);
                    if let Some(metadata) = &response.metadata {
                        println!();
                        println!("Metadata: {}", serde_json::to_string_pretty(metadata)?);
                    }
                }
            }
            Ok(())
        }

        pub fn telemetry(&self, entries: &[TelemetryEntry]) -> Result<()> {
            match self.format {
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(entries)?);
                }
                OutputFormat::Markdown => {
                    println!("| Timestamp | Tool | Latency (ms) | Success |");
                    println!("| --- | --- | ---: | --- |");
                    for entry in entries {
                        println!(
                            "| {} | `{}` | {} | {} |",
                            entry.timestamp, entry.tool, entry.latency_ms, entry.success
                        );
                    }
                }
                OutputFormat::Table => {
                    let rows: Vec<Vec<String>> = entries
                        .iter()
                        .map(|entry| {
                            vec![
                                entry.timestamp.to_string(),
                                entry.tool.clone(),
                                entry.latency_ms.to_string(),
                                entry.success.to_string(),
                            ]
                        })
                        .collect();
                    render_table(&["Timestamp", "Tool", "Latency (ms)", "Success"], &rows);
                }
                OutputFormat::Text => {
                    for entry in entries {
                        println!(
                            "[{}] {} — {} ms ({})",
                            entry.timestamp,
                            entry.tool,
                            entry.latency_ms,
                            if entry.success { "success" } else { "error" }
                        );
                        if let Some(metadata) = &entry.metadata {
                            println!("  metadata: {}", serde_json::to_string_pretty(metadata)?);
                        }
                        if let Some(error) = &entry.error {
                            println!("  error: {error}");
                        }
                    }
                }
            }
            Ok(())
        }

        pub fn cache_status(&self, report: &crate::CacheStatusReport) -> Result<()> {
            match self.format {
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(report)?);
                }
                OutputFormat::Markdown => {
                    println!("| Property | Value |");
                    println!("| --- | --- |");
                    println!("| Path | `{}` |", report.path);
                    println!("| Exists | {} |", report.exists);
                    println!("| Readable | {} |", report.readable);
                    println!("| File Count | {} |", report.file_count);
                }
                OutputFormat::Table => {
                    let rows = vec![
                        vec!["Path".to_string(), report.path.clone()],
                        vec!["Exists".to_string(), report.exists.to_string()],
                        vec!["Readable".to_string(), report.readable.to_string()],
                        vec!["File Count".to_string(), report.file_count.to_string()],
                    ];
                    render_table(&["Property", "Value"], &rows);
                }
                OutputFormat::Text => {
                    println!("Cache directory: {}", report.path);
                    println!("Exists: {}", report.exists);
                    println!("Readable: {}", report.readable);
                    println!("File count: {}", report.file_count);
                }
            }
            Ok(())
        }

        pub fn cache_warmup(&self, summary: &crate::WarmupSummary) -> Result<()> {
            match self.format {
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(summary)?);
                }
                OutputFormat::Markdown => {
                    println!("| Property | Value |");
                    println!("| --- | --- |");
                    println!("| Refreshed | {} |", summary.refreshed);
                    println!("| Technologies Cached | {} |", summary.technologies_cached);
                    println!(
                        "| Technology Count | {} |",
                        summary
                            .technology_count
                            .map_or_else(|| "n/a".to_string(), |count| count.to_string())
                    );
                    println!(
                        "| Frameworks Cached | {} |",
                        if summary.frameworks_cached.is_empty() {
                            "none".to_string()
                        } else {
                            summary.frameworks_cached.join(", ")
                        }
                    );
                }
                OutputFormat::Table => {
                    let rows = vec![
                        vec!["Refreshed".to_string(), summary.refreshed.to_string()],
                        vec![
                            "Technologies Cached".to_string(),
                            summary.technologies_cached.to_string(),
                        ],
                        vec![
                            "Technology Count".to_string(),
                            summary
                                .technology_count
                                .map_or_else(|| "n/a".to_string(), |count| count.to_string()),
                        ],
                        vec![
                            "Frameworks Cached".to_string(),
                            if summary.frameworks_cached.is_empty() {
                                "none".to_string()
                            } else {
                                summary.frameworks_cached.join(", ")
                            },
                        ],
                    ];
                    render_table(&["Property", "Value"], &rows);
                }
                OutputFormat::Text => {
                    println!("Cache warmup complete:");
                    println!("  Refreshed: {}", summary.refreshed);
                    println!("  Technologies cached: {}", summary.technologies_cached);
                    println!(
                        "  Technology count: {}",
                        summary
                            .technology_count
                            .map_or_else(|| "n/a".to_string(), |count| count.to_string())
                    );
                    if summary.frameworks_cached.is_empty() {
                        println!("  Frameworks cached: none");
                    } else {
                        println!(
                            "  Frameworks cached: {}",
                            summary.frameworks_cached.join(", ")
                        );
                    }
                }
            }
            Ok(())
        }

        pub fn cache_cleared(&self) -> Result<()> {
            match self.format {
                OutputFormat::Json => {
                    let payload = json!({ "event": "clear_memory_cache", "status": "success" });
                    println!("{}", serde_json::to_string_pretty(&payload)?);
                }
                OutputFormat::Markdown | OutputFormat::Text => {
                    println!("In-memory cache cleared.");
                }
                OutputFormat::Table => {
                    let rows = vec![vec!["Status".to_string(), "Cleared".to_string()]];
                    render_table(&["Field", "Value"], &rows);
                }
            }
            Ok(())
        }

        pub fn no_telemetry(&self) -> Result<()> {
            match self.format {
                OutputFormat::Json => {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&Vec::<TelemetryEntry>::new())?
                    );
                }
                OutputFormat::Markdown | OutputFormat::Text => {
                    println!("No telemetry entries recorded yet.");
                }
                OutputFormat::Table => {
                    println!("No telemetry entries recorded yet.");
                }
            }
            Ok(())
        }
    }

    fn render_table(headers: &[&str], rows: &[Vec<String>]) {
        let mut widths: Vec<usize> = headers.iter().map(|header| header.len()).collect();
        for row in rows {
            for (idx, cell) in row.iter().enumerate() {
                widths[idx] = widths[idx].max(cell.len());
            }
        }

        fn render_line(columns: &[&str], widths: &[usize]) -> String {
            let mut line = String::new();
            for (idx, value) in columns.iter().enumerate() {
                let width = widths[idx];
                let _ = write!(line, "| {:width$} ", value, width = width);
            }
            line.push('|');
            line
        }

        let header_line = render_line(headers, &widths);
        println!("{header_line}");
        let separator: String = widths
            .iter()
            .map(|width| format!("|{:-^1$}", "", width + 2))
            .collect::<Vec<_>>()
            .join("");
        println!("{separator}|");

        for row in rows {
            let cols: Vec<&str> = row.iter().map(String::as_str).collect();
            println!("{}", render_line(&cols, &widths));
        }
    }

    fn sanitize(value: &str) -> String {
        value
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn truncate(value: &str, max: usize) -> String {
        if value.len() <= max {
            value.to_string()
        } else {
            let mut truncated = value
                .chars()
                .take(max.saturating_sub(1))
                .collect::<String>();
            truncated.push('…');
            truncated
        }
    }
}

mod progress {
    use std::time::Duration;

    use indicatif::{ProgressBar, ProgressStyle};

    pub fn spinner(message_enabled: bool, message: impl Into<String>) -> Option<ProgressBar> {
        if !message_enabled {
            return None;
        }
        let progress = ProgressBar::new_spinner();
        let style = ProgressStyle::with_template("{spinner} {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_spinner());
        progress.set_style(style);
        progress.set_message(message.into());
        progress.enable_steady_tick(Duration::from_millis(80));
        Some(progress)
    }
}

mod session {
    use super::*;
    use anyhow::{Context, Result};
    use serde::{Deserialize, Serialize};
    use tokio::fs;
    use tracing::{debug, warn};

    const STATE_FILE_NAME: &str = "cli-session.json";

    #[derive(Debug)]
    pub struct SessionManager {
        context: Arc<AppContext>,
        path: PathBuf,
    }

    #[derive(Debug, Default, Serialize, Deserialize)]
    struct PersistedState {
        #[serde(rename = "activeTechnology")]
        active_technology: Option<PersistedTechnology>,
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct PersistedTechnology {
        identifier: String,
        title: String,
    }

    impl SessionManager {
        pub fn new(context: Arc<AppContext>) -> Self {
            let path = context.client.cache_dir().join(STATE_FILE_NAME);
            Self { context, path }
        }

        pub async fn restore(&self) {
            if let Err(error) = self.restore_inner().await {
                warn!(
                    target: "apple_docs_cli",
                    error = %error,
                    "failed to restore CLI session state"
                );
            }
        }

        pub async fn persist(&self) {
            if let Err(error) = self.persist_inner().await {
                warn!(
                    target: "apple_docs_cli",
                    error = %error,
                    "failed to persist CLI session state"
                );
            }
        }

        async fn restore_inner(&self) -> Result<()> {
            if !fs::try_exists(&self.path).await? {
                return Ok(());
            }
            let bytes = fs::read(&self.path).await?;
            if bytes.is_empty() {
                return Ok(());
            }
            let state: PersistedState =
                serde_json::from_slice(&bytes).context("invalid CLI session state")?;
            if let Some(saved) = state.active_technology {
                self.apply(saved).await?;
            }
            Ok(())
        }

        async fn apply(&self, saved: PersistedTechnology) -> Result<()> {
            let technologies = self.context.client.get_technologies().await?;
            let normalized = saved.identifier.to_lowercase();
            let matched = technologies
                .values()
                .find(|tech| {
                    tech.identifier.to_lowercase() == normalized
                        || tech
                            .identifier
                            .rsplit('/')
                            .next()
                            .map(|slug| slug.eq_ignore_ascii_case(&saved.identifier))
                            .unwrap_or(false)
                })
                .cloned();

            match matched {
                Some(technology) => {
                    *self.context.state.active_technology.write().await = Some(technology.clone());
                    self.context.state.framework_cache.write().await.take();
                    self.context.state.framework_index.write().await.take();
                    debug!(
                        target: "apple_docs_cli",
                        identifier = %technology.identifier,
                        "restored active technology from session state"
                    );
                }
                None => {
                    warn!(
                        target: "apple_docs_cli",
                        identifier = %saved.identifier,
                        "stored technology identifier no longer available"
                    );
                    self.context.state.active_technology.write().await.take();
                }
            }
            Ok(())
        }

        async fn persist_inner(&self) -> Result<()> {
            if let Some(parent) = self.path.parent() {
                fs::create_dir_all(parent).await.ok();
            }
            let snapshot = self.context.state.active_technology.read().await.clone();
            let state = PersistedState {
                active_technology: snapshot.map(|tech| PersistedTechnology {
                    identifier: tech.identifier,
                    title: tech.title,
                }),
            };
            let payload = serde_json::to_vec_pretty(&state)?;
            fs::write(&self.path, payload).await?;
            Ok(())
        }
    }
}
