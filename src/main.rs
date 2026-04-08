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
    run::dispatch,
};

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    setup_tracing(cli.verbose);

    if cli.clean_cache {
        if let Some(dirs) = directories::ProjectDirs::from("", "", "ripweb") {
            let dir = dirs.cache_dir();
            match std::fs::remove_dir_all(dir) {
                Ok(()) => eprintln!("Cache cleared: {}", dir.display()),
                Err(e) if e.kind() == io::ErrorKind::NotFound => eprintln!("Cache already empty."),
                Err(e) => {
                    eprintln!("Error clearing cache: {e}");
                    std::process::exit(1);
                }
            }
        } else {
            eprintln!("Could not determine XDG cache directory.");
        }
        return;
    }

    let input = match &cli.query_or_url {
        Some(s) => s.as_str(),
        None => {
            eprintln!("Error: a URL or query is required.");
            std::process::exit(1);
        }
    };

    let is_tty = io::stdout().is_terminal();
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

    let client = Arc::new(match build_client() {
        Ok(c) => c,
        Err(e) => {
            finish_spinner(&spinner);
            eprintln!("Error building HTTP client: {e}");
            std::process::exit(1);
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
            eprintln!("Error: {e}");
            std::process::exit(e.exit_code());
        }
    };

    if text.trim().is_empty() {
        eprintln!("Error: {}", RipwebError::NoContent);
        std::process::exit(4);
    }

    if cli.stat {
        let tokens = cl100k_base()
            .map(|bpe| bpe.encode_with_special_tokens(&text).len())
            .unwrap_or(0);
        let size_mb = text.len() as f64 / 1_048_576.0;
        write_stdout(&format!(
            "Pages: {page_count} | Raw Size: {size_mb:.2} MB | Tokens: {tokens}\n"
        ));
        return;
    }

    if cli.copy {
        match arboard::Clipboard::new().and_then(|mut b| b.set_text(&text).map(|_| ())) {
            Ok(()) => eprintln!("Copied to clipboard."),
            Err(e) => {
                eprintln!("Clipboard error: {e}");
                std::process::exit(1);
            }
        }
        return;
    }

    write_stdout(&text);
    write_stdout("\n");
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
