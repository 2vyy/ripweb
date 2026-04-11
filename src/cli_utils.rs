//! CLI helper utilities extracted from the binary for testability.
//!
//! These functions were moved out of `main.rs` so integration tests can exercise
//! the CLI-level helpers without spawning a child process.

use std::io::{self, Write};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use indicatif::ProgressBar;
use tracing_subscriber::EnvFilter;

use crate::cli::Cli;
use crate::error::RipwebError;
use crate::research::find::{matched_terms_in_text, parse_terms};
use crate::research::track::append_jsonl;
use crate::router::{GitHubRouteType, PlatformRoute, Route, route};

/// Write to stdout, handling broken pipe gracefully by exiting 0.
pub fn write_stdout(text: &str) {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    if let Err(e) = handle.write_all(text.as_bytes()) {
        if e.kind() == io::ErrorKind::BrokenPipe {
            std::process::exit(0);
        }
        eprintln!("stdout write error: {e}");
        std::process::exit(1);
    }
}

/// Finish and clear an optional progress spinner.
pub fn finish_spinner(spinner: &Option<ProgressBar>) {
    if let Some(pb) = spinner {
        pb.finish_and_clear();
    }
}

/// Setup tracing subscriber according to verbosity count.
pub fn setup_tracing(verbose: u8) {
    let level = match verbose {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(format!("ripweb={level}")));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(io::stderr)
        .init();
}

/// Append a session entry to the --track JSONL file if configured.
pub fn maybe_track(
    cli: &Cli,
    input: &str,
    output: Option<&str>,
    token_count: usize,
    error: Option<String>,
    exit_code: i32,
) {
    let Some(path) = cli.track.as_deref() else {
        return;
    };

    let (url, query, source_type, domain) = classify_source(cli, input);
    let keywords_found = if let (Some(text), Some(raw_terms)) = (output, cli.find.as_deref()) {
        let parsed = parse_terms(raw_terms);
        matched_terms_in_text(text, &parsed)
    } else {
        Vec::new()
    };

    let entry = crate::research::track::SessionEntry {
        timestamp: unix_timestamp_seconds(),
        url,
        query,
        source_type,
        domain,
        tokens: token_count,
        bytes: output.map_or(0, |text| text.len()),
        cache_hit: false,
        mode: format!("{:?}", cli.verbosity).to_ascii_lowercase(),
        keywords_found,
        output_chars: output.map_or(0, |text| text.chars().count()),
        success: exit_code == 0 && error.is_none(),
        exit_code,
        error,
    };

    if let Err(e) = append_jsonl(path, &entry) {
        eprintln!("Warning: failed to append --track session log: {e}");
    }
}

/// Classify the input into (url, query, source_type, domain)
pub fn classify_source(
    cli: &Cli,
    input: &str,
) -> (Option<String>, Option<String>, String, Option<String>) {
    if cli.wikidata.is_some() {
        return (
            Some("https://query.wikidata.org/".to_owned()),
            cli.wikidata.clone(),
            "wikidata".to_owned(),
            Some("wikidata.org".to_owned()),
        );
    }

    if cli.batch {
        return (None, None, "batch".to_owned(), None);
    }

    let effective =
        if cli.force_url && !input.starts_with("http://") && !input.starts_with("https://") {
            format!("https://{input}")
        } else {
            input.to_owned()
        };

    let routed = if cli.force_query {
        Route::Query(effective.clone())
    } else {
        route(&effective)
    };

    match routed {
        Route::Query(q) => (None, Some(q), "search".to_owned(), cli.site.clone()),
        Route::Url(platform) => match platform {
            PlatformRoute::GitHub {
                owner,
                repo,
                route_type,
            } => {
                let url = match route_type {
                    GitHubRouteType::Readme => format!("https://github.com/{owner}/{repo}"),
                    GitHubRouteType::Issues => format!("https://github.com/{owner}/{repo}/issues"),
                    GitHubRouteType::Issue(id) => {
                        format!("https://github.com/{owner}/{repo}/issues/{id}")
                    }
                };
                (
                    Some(url),
                    None,
                    "github".to_owned(),
                    Some("github.com".to_owned()),
                )
            }
            PlatformRoute::Reddit { url } => (
                Some(url),
                None,
                "reddit".to_owned(),
                Some("reddit.com".to_owned()),
            ),
            PlatformRoute::HackerNews { item_id } => (
                Some(format!("https://news.ycombinator.com/item?id={item_id}")),
                None,
                "hackernews".to_owned(),
                Some("news.ycombinator.com".to_owned()),
            ),
            PlatformRoute::Wikipedia { title } => (
                Some(format!("https://en.wikipedia.org/wiki/{title}")),
                None,
                "wikipedia".to_owned(),
                Some("wikipedia.org".to_owned()),
            ),
            PlatformRoute::StackOverflow { question_id } => (
                Some(format!("https://stackoverflow.com/questions/{question_id}")),
                None,
                "stackoverflow".to_owned(),
                Some("stackoverflow.com".to_owned()),
            ),
            PlatformRoute::ArXiv { paper_id } => (
                Some(format!("https://arxiv.org/abs/{paper_id}")),
                None,
                "arxiv".to_owned(),
                Some("arxiv.org".to_owned()),
            ),
            PlatformRoute::YouTube { original_url, .. } => (
                Some(original_url),
                None,
                "youtube".to_owned(),
                Some("youtube.com".to_owned()),
            ),
            PlatformRoute::Generic(url) => {
                let domain = url.host_str().map(|host| host.to_owned());
                (Some(url.to_string()), None, "generic".to_owned(), domain)
            }
            _ => (Some(input.to_owned()), None, "url".to_owned(), None),
        },
    }
}

/// Current unix timestamp as seconds string
pub fn unix_timestamp_seconds() -> String {
    let now = std::time::SystemTime::now();
    let secs = now
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    secs.to_string()
}
