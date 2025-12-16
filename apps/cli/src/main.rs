use std::io::Read;

use anyhow::Result;
use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    fmt()
        .with_env_filter(filter)
        .with_target(true)
        .without_time()
        .compact()
        .with_writer(std::io::stderr)
        .init();

    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("query") | Some("--oneshot") => {
            let mut max_results: Option<usize> = None;
            let mut json_output = false;

            let mut positionals = Vec::new();
            let mut pending = args.collect::<Vec<_>>().into_iter();
            while let Some(arg) = pending.next() {
                match arg.as_str() {
                    "--json" => json_output = true,
                    "--max-results" | "--maxResults" | "-n" => {
                        let value = pending
                            .next()
                            .ok_or_else(|| anyhow::anyhow!("{arg} requires a value"))?;
                        max_results = Some(value.parse()?);
                    }
                    _ => positionals.push(arg),
                }
            }

            let query = if !positionals.is_empty() {
                positionals.join(" ")
            } else {
                let mut buf = String::new();
                std::io::stdin().read_to_string(&mut buf)?;
                let trimmed = buf.trim();
                if trimmed.is_empty() {
                    anyhow::bail!(
                        "missing query string (usage: docs-mcp-cli query [--json] [--max-results N] \"...\")"
                    );
                }
                trimmed.to_string()
            };

            let response = docs_mcp::oneshot_query(&query, max_results).await?;
            if json_output {
                println!("{}", serde_json::to_string_pretty(&response)?);
            } else {
                for item in response.content {
                    println!("{}", item.text);
                }
            }
            Ok(())
        }
        _ => docs_mcp::run_server().await,
    }
}
