//! CLI Entry Point
//!
//! The main binary orchestrates input parsing, environment configuration,
//! and high-level execution via the `run` module. It handles graceful
//! shutdown (Ctrl-C) and clipboard integration.

use std::io::{self, Write};
use std::sync::Arc;
use std::time::Duration;

use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use is_terminal::IsTerminal;
use tiktoken_rs::cl100k_base;
use tracing_subscriber::EnvFilter;

use ripweb::{
    cli::Cli,
    error::RipwebError,
    fetch::{RetryConfig, cache::Cache, client::build_client, politeness::DomainSemaphores},
    research::{
        find::{matched_terms_in_text, parse_terms},
        track::{SessionEntry, append_jsonl},
        wayback::validate_date,
    },
    router::{GitHubRouteType, PlatformRoute, Route, route},
    run::dispatch,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    setup_tracing(cli.verbose);

    if cli.clean_cache {
        if let Some(dirs) = directories::ProjectDirs::from("", "", "ripweb") {
            let dir = dirs.cache_dir();
            match std::fs::remove_dir_all(dir) {
                Ok(()) => eprintln!("Cache cleared: {}", dir.display()),
                Err(e) if e.kind() == io::ErrorKind::NotFound => eprintln!("Cache already empty."),
                Err(e) => {
                    anyhow::bail!("Error clearing cache: {e}");
                }
            }
        } else {
            eprintln!("Could not determine XDG cache directory.");
        }
        return Ok(());
    }

    if let Some(date) = cli.as_of.as_deref()
        && validate_date(date).is_err()
    {
        anyhow::bail!("Error: invalid --as-of date, expected YYYY-MM-DD");
    }

    let input = cli.query_or_url.as_deref().unwrap_or("");
    if input.is_empty() && !cli.batch && cli.wikidata.is_none() {
        anyhow::bail!("Error: a URL or query is required.");
    }

    let is_tty = io::stdout().is_terminal();
    let spinner: Option<ProgressBar> = if is_tty {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::with_template("{spinner:.cyan} {msg}")
                .unwrap_or_else(|_| ProgressStyle::default_spinner())
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
        );
        pb.enable_steady_tick(Duration::from_millis(80));
        pb.set_message("Fetching…");
        Some(pb)
    } else {
        None
    };

    let client = Arc::new(match build_client() {
        Ok(c) => c,
        Err(e) => {
            finish_spinner(&spinner);
            anyhow::bail!("Error building HTTP client: {e}");
        }
    });

    let result = tokio::select! {
        r = dispatch(&cli, input, &client, RetryConfig::default(), DomainSemaphores::new(3), Cache::xdg().map(Arc::new)) => r,
        _ = tokio::signal::ctrl_c() => {
            finish_spinner(&spinner);
            let _ = io::stdout().flush();
            let _ = writeln!(io::stdout());
            std::process::exit(0);
        }
    };

    finish_spinner(&spinner);

    let (text, page_count) = match result {
        Ok(pair) => pair,
        Err(e) => {
            maybe_track(&cli, input, None, 0, Some(e.to_string()), e.exit_code());
            eprintln!("Error: {e}");
            std::process::exit(e.exit_code());
        }
    };

    if text.trim().is_empty() {
        maybe_track(
            &cli,
            input,
            Some(&text),
            0,
            Some(RipwebError::NoContent.to_string()),
            4,
        );
        eprintln!("Error: {}", RipwebError::NoContent);
        std::process::exit(4);
    }

    // Always compute token count (used by --stat).
    let token_count = cl100k_base()
        .map(|bpe| bpe.encode_with_special_tokens(&text).len())
        .unwrap_or(0);

    if cli.stat {
        maybe_track(&cli, input, Some(&text), token_count, None, 0);
        let size_mb = text.len() as f64 / 1_048_576.0;
        write_stdout(&format!(
            "Pages: {page_count} | Raw Size: {size_mb:.2} MB | Tokens: {token_count}\n"
        ));
        return Ok(());
    }

    if cli.copy {
        maybe_track(&cli, input, Some(&text), token_count, None, 0);
        match arboard::Clipboard::new().and_then(|mut b| b.set_text(&text).map(|_| ())) {
            Ok(()) => eprintln!("Copied to clipboard."),
            Err(e) => {
                anyhow::bail!("Clipboard error: {e}");
            }
        }
        return Ok(());
    }

    maybe_track(&cli, input, Some(&text), token_count, None, 0);
    write_stdout(&text);
    write_stdout("\n");
    Ok(())
}

fn write_stdout(text: &str) {
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

fn finish_spinner(spinner: &Option<ProgressBar>) {
    if let Some(pb) = spinner {
        pb.finish_and_clear();
    }
}

fn setup_tracing(verbose: u8) {
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

fn maybe_track(
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

    let entry = SessionEntry {
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

fn classify_source(
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
                    GitHubRouteType::Issues => {
                        format!("https://github.com/{owner}/{repo}/issues")
                    }
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

fn unix_timestamp_seconds() -> String {
    let now = std::time::SystemTime::now();
    let secs = now
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    secs.to_string()
}
