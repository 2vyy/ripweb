use std::{
    io::{self, BufRead},
    sync::Arc,
};

use crate::{
    cli::Cli,
    error::RipwebError,
    fetch::{RetryConfig, cache::Cache, politeness::DomainSemaphores},
    run::dispatch_single,
};

/// Run `--batch` mode by reading URLs from stdin and fetching each concurrently.
pub async fn run_batch(
    cli: &Cli,
    client: &Arc<rquest::Client>,
    retry: RetryConfig,
    sems: DomainSemaphores,
    cache: Option<Arc<Cache>>,
) -> Result<(String, usize), RipwebError> {
    let stdin = io::stdin();
    let mut urls = Vec::new();
    for line in stdin.lock().lines() {
        let line = line
            .map_err(|e| RipwebError::Config(format!("failed reading stdin for --batch: {e}")))?;
        if let Some(url) = normalize_batch_url(&line) {
            urls.push(url);
        }
    }

    if urls.is_empty() {
        return Err(RipwebError::Config(
            "no valid URLs received on stdin for --batch".into(),
        ));
    }

    if urls.len() > cli.max_pages {
        eprintln!(
            "Warning: --batch received {} URLs, truncating to --max-pages {}.",
            urls.len(),
            cli.max_pages
        );
        urls.truncate(cli.max_pages);
    }

    let mut tasks = tokio::task::JoinSet::new();
    for target in urls {
        let client = Arc::clone(client);
        let sems = sems.clone();
        let cache = cache.clone();
        let mut child_cli = cli.clone();
        child_cli.batch = false;
        child_cli.track = None;
        child_cli.query_or_url = Some(target.clone());
        child_cli.force_url = true;

        tasks.spawn(async move {
            let result = dispatch_single(&child_cli, &target, &client, retry, sems, cache).await;
            (target, result)
        });
    }

    let mut merged = String::new();
    let mut success_count = 0usize;

    while let Some(joined) = tasks.join_next().await {
        match joined {
            Ok((url, Ok((text, _)))) => {
                if text.trim().is_empty() {
                    eprintln!("Warning: batch item produced empty output for {url}");
                    continue;
                }
                success_count += 1;
                if !merged.is_empty() {
                    merged.push_str("\n\n");
                }
                merged.push_str(&text);
            }
            Ok((url, Err(e))) => {
                eprintln!("Warning: batch item failed for {url}: {e}");
            }
            Err(e) => {
                eprintln!("Warning: batch worker failed: {e}");
            }
        }
    }

    if success_count == 0 {
        return Err(RipwebError::NoContent);
    }

    Ok((merged, success_count))
}

pub fn normalize_batch_url(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    let candidate = if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        trimmed.to_owned()
    } else {
        format!("https://{trimmed}")
    };

    if url::Url::parse(&candidate).is_ok() {
        Some(candidate)
    } else {
        eprintln!("Warning: skipping invalid batch URL '{trimmed}'");
        None
    }
}

#[cfg(test)]
mod tests {
    use super::normalize_batch_url;

    #[test]
    fn normalize_batch_url_adds_https() {
        let normalized = normalize_batch_url("example.com/path");
        assert_eq!(normalized.as_deref(), Some("https://example.com/path"));
    }
}
