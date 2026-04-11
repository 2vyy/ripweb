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

use ripweb::{
    cli::Cli,
    cli_utils::{finish_spinner, maybe_track, setup_tracing, write_stdout},
    error::RipwebError,
    fetch::{RetryConfig, cache::Cache, client::build_client, politeness::DomainSemaphores},
    research::wayback::validate_date,
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
