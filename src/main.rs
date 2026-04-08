use std::io::{self, Write};
use std::sync::Arc;
use std::time::Duration;

use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use is_terminal::IsTerminal;
use tiktoken_rs::cl100k_base;
use tracing_subscriber::EnvFilter;

use ripweb::{
    cli::{Cli, OutputMode},
    error::RipwebError,
    fetch::{
        cache::Cache,
        client::build_client,
        crawler::{format_output, Crawler, CrawledPage, CrawlerConfig},
        llms_txt::fetch_llms_txt,
        politeness::DomainSemaphores,
        RetryConfig,
    },
    router::{route, PlatformRoute, Route},
    search::{
        duckduckgo,
        github,
        hackernews::{hn_api_url, parse_hn_json},
        reddit::{parse_reddit_json, reddit_json_url},
    },
    minify::collapse,
};

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    setup_tracing(cli.verbose);

    // ── --clean-cache shortcircuit ──────────────────────────────────────────
    if cli.clean_cache {
        match Cache::xdg() {
            Some(cache) => {
                // We expose the cache dir via xdg(); clean it.
                if let Some(dirs) = directories::ProjectDirs::from("", "", "ripweb") {
                    let dir = dirs.cache_dir();
                    match std::fs::remove_dir_all(dir) {
                        Ok(()) => eprintln!("Cache cleared: {}", dir.display()),
                        Err(e) if e.kind() == io::ErrorKind::NotFound => {
                            eprintln!("Cache already empty.");
                        }
                        Err(e) => {
                            eprintln!("Error clearing cache: {e}");
                            std::process::exit(1);
                        }
                    }
                }
                let _ = cache; // suppress unused warning
            }
            None => eprintln!("Could not determine XDG cache directory."),
        }
        return;
    }

    // ── Require positional arg when not --clean-cache ───────────────────────
    let input = match &cli.query_or_url {
        Some(s) => s.as_str(),
        None => {
            eprintln!("Error: a URL or query is required.");
            std::process::exit(1);
        }
    };

    // ── TTY detection ────────────────────────────────────────────────────────
    let is_tty = io::stdout().is_terminal();

    // ── Progress spinner (stderr only, disabled when piped) ─────────────────
    let spinner: Option<ProgressBar> = if is_tty {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::with_template("{spinner:.cyan} {msg}")
                .unwrap()
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
        );
        pb.enable_steady_tick(Duration::from_millis(80));
        pb.set_message("Fetching…");
        Some(pb)
    } else {
        None
    };

    // ── Build shared components ───────────────────────────────────────────────
    let client = Arc::new(match build_client() {
        Ok(c) => c,
        Err(e) => {
            finish_spinner(&spinner);
            eprintln!("Error building HTTP client: {e}");
            std::process::exit(1);
        }
    });

    let retry = RetryConfig::default();
    let sems = DomainSemaphores::new(3);
    let cache = Cache::xdg().map(Arc::new);

    // ── Ctrl+C handler races against the fetch ───────────────────────────────
    let result = tokio::select! {
        r = dispatch(&cli, input, &client, retry, sems, cache) => r,
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
            eprintln!("Error: {e}");
            std::process::exit(e.exit_code());
        }
    };

    let text = apply_output_mode(text, cli.mode);

    if text.trim().is_empty() {
        eprintln!("Error: {}", RipwebError::NoContent);
        std::process::exit(4);
    }

    // ── --stat mode ──────────────────────────────────────────────────────────
    if cli.stat {
        let tokens = count_tokens(&text);
        let size_mb = text.len() as f64 / 1_048_576.0;
        // Stats are the output data in --stat mode → stdout
        write_stdout(&format!(
            "Pages: {page_count} | Raw Size: {size_mb:.2} MB | Tokens: {tokens}\n"
        ));
        return;
    }

    // ── --copy mode ──────────────────────────────────────────────────────────
    if cli.copy {
        match copy_to_clipboard(&text) {
            Ok(()) => eprintln!("Copied to clipboard."),
            Err(e) => {
                eprintln!("Clipboard error: {e}");
                std::process::exit(1);
            }
        }
        return;
    }

    // ── Normal output — stdout with SIGPIPE / BrokenPipe handling ────────────
    write_stdout(&text);
    write_stdout("\n");
}

// ── Core dispatch ─────────────────────────────────────────────────────────────

async fn dispatch(
    cli: &Cli,
    input: &str,
    client: &Arc<rquest::Client>,
    retry: RetryConfig,
    sems: DomainSemaphores,
    cache: Option<Arc<Cache>>,
) -> Result<(String, usize), RipwebError> {
    // Determine effective URL when -u forces URL mode without a scheme.
    let effective = if cli.force_url && !input.starts_with("http://") && !input.starts_with("https://") {
        format!("https://{input}")
    } else {
        input.to_owned()
    };

    let route = if cli.force_query {
        Route::Query(effective)
    } else {
        route(&effective)
    };

    match route {
        Route::Query(q) => handle_query(client, &q, cli, retry, sems, cache).await,
        Route::Url(platform) => handle_platform(client, platform, cli, retry, sems, cache).await,
    }
}

// ── Platform handlers ─────────────────────────────────────────────────────────

async fn handle_platform(
    client: &Arc<rquest::Client>,
    platform: PlatformRoute,
    cli: &Cli,
    retry: RetryConfig,
    sems: DomainSemaphores,
    cache: Option<Arc<Cache>>,
) -> Result<(String, usize), RipwebError> {
    match platform {
        PlatformRoute::GitHub { owner, repo } => {
            let text = github::fetch_readme(client, &owner, &repo)
                .await
                .map_err(|e| RipwebError::Network(e.to_string()))?;
            Ok((text, 1))
        }

        PlatformRoute::Reddit { url } => {
            let json_url = reddit_json_url(&url)
                .ok_or_else(|| RipwebError::Config(format!("invalid Reddit URL: {url}")))?;
            let body = client
                .get(&json_url)
                .send()
                .await
                .map_err(|e| RipwebError::Network(e.to_string()))?
                .text()
                .await
                .map_err(|e| RipwebError::Network(e.to_string()))?;
            let content = parse_reddit_json(&body)
                .map_err(|e| RipwebError::Network(format!("Reddit JSON parse: {e}")))?;
            let text = format_reddit(&content);
            Ok((text, 1))
        }

        PlatformRoute::HackerNews { item_id } => {
            let api = hn_api_url(&item_id);
            let body = client
                .get(api.as_str())
                .send()
                .await
                .map_err(|e| RipwebError::Network(e.to_string()))?
                .text()
                .await
                .map_err(|e| RipwebError::Network(e.to_string()))?;
            let content = parse_hn_json(&body)
                .map_err(|e| RipwebError::Network(format!("HN JSON parse: {e}")))?;
            let text = format_hn(&content);
            Ok((text, 1))
        }

        PlatformRoute::Generic(url) => {
            // Auto-discovery: try llms.txt first.
            if let Some(llms) = fetch_llms_txt(client, &url).await {
                return Ok((llms, 1));
            }
            // Fall back to HTML crawler.
            run_crawler(client, url, cli, retry, sems, cache).await
        }
        // Safety net for any future PlatformRoute variants (#[non_exhaustive]).
        _ => Err(RipwebError::Config("unhandled platform route".into())),
    }
}

async fn handle_query(
    client: &Arc<rquest::Client>,
    query: &str,
    cli: &Cli,
    retry: RetryConfig,
    sems: DomainSemaphores,
    cache: Option<Arc<Cache>>,
) -> Result<(String, usize), RipwebError> {
    let urls = duckduckgo::search(client, query, 3)
        .await
        .map_err(|e| RipwebError::Network(e.to_string()))?;

    let mut all_pages: Vec<CrawledPage> = Vec::new();

    for url_str in urls {
        if all_pages.len() >= cli.max_pages {
            break;
        }

        let url = url::Url::parse(&url_str)
            .map_err(|e| RipwebError::Config(format!("DDG returned invalid URL: {e}")))?;

        // For each result URL try llms.txt, then crawl.
        let pages = if let Some(llms) = fetch_llms_txt(client, &url).await {
            vec![CrawledPage { url: url_str, content: llms }]
        } else {
            let remaining = cli.max_pages.saturating_sub(all_pages.len());
            Crawler::new(
                Arc::clone(client),
                sems.clone(),
                cache.clone(),
                RetryConfig { max_retries: 2, base_delay: retry.base_delay },
                CrawlerConfig { max_depth: cli.max_depth, max_pages: remaining },
            )
            .crawl(url)
            .await
        };

        all_pages.extend(pages);
    }

    let count = all_pages.len();
    Ok((format_output(&all_pages), count))
}

async fn run_crawler(
    client: &Arc<rquest::Client>,
    url: url::Url,
    cli: &Cli,
    retry: RetryConfig,
    sems: DomainSemaphores,
    cache: Option<Arc<Cache>>,
) -> Result<(String, usize), RipwebError> {
    let crawler = build_crawler(client, cli, retry.base_delay, sems, cache);
    let pages = crawler.crawl(url).await;
    let count = pages.len();
    Ok((format_output(&pages), count))
}

fn build_crawler(
    client: &Arc<rquest::Client>,
    cli: &Cli,
    base_delay: Duration,
    sems: DomainSemaphores,
    cache: Option<Arc<Cache>>,
) -> Crawler {
    Crawler::new(
        Arc::clone(client),
        sems,
        cache,
        RetryConfig { max_retries: 2, base_delay },
        CrawlerConfig { max_depth: cli.max_depth, max_pages: cli.max_pages },
    )
}

fn apply_output_mode(text: String, mode: OutputMode) -> String {
    match mode {
        OutputMode::Markdown => text.trim().to_owned(),
        OutputMode::Aggressive => collapse(text.trim()),
    }
}

// ── Text formatters ───────────────────────────────────────────────────────────

fn format_reddit(c: &ripweb::search::reddit::RedditContent) -> String {
    let mut out = format!("# {}\n\n{}", c.title, c.selftext);
    if !c.comments.is_empty() {
        out.push_str("\n\n## Comments\n\n");
        out.push_str(&c.comments.join("\n\n---\n\n"));
    }
    out
}

fn format_hn(c: &ripweb::search::hackernews::HnContent) -> String {
    let mut out = format!("# {}", c.title);
    if let Some(text) = &c.text {
        out.push_str(&format!("\n\n{text}"));
    }
    if !c.comments.is_empty() {
        out.push_str("\n\n## Comments\n\n");
        out.push_str(&c.comments.join("\n\n---\n\n"));
    }
    out
}

// ── Output helpers ────────────────────────────────────────────────────────────

/// Write to stdout, catching BrokenPipe and exiting 0 rather than panicking.
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

fn count_tokens(text: &str) -> usize {
    cl100k_base()
        .map(|bpe| bpe.encode_with_special_tokens(text).len())
        .unwrap_or(0)
}

fn copy_to_clipboard(text: &str) -> Result<(), String> {
    let mut board = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    board.set_text(text).map_err(|e| e.to_string())
}

fn finish_spinner(spinner: &Option<ProgressBar>) {
    if let Some(pb) = spinner {
        pb.finish_and_clear();
    }
}

// ── Logging setup ─────────────────────────────────────────────────────────────

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
        .with_writer(io::stderr) // metadata to stderr ONLY
        .init();
}
