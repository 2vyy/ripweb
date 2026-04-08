use clap::Parser;
use ripweb::cli::{Cli, OutputMode};

// ── Positional argument ───────────────────────────────────────────────────────

#[test]
fn parses_positional_query() {
    let cli = Cli::try_parse_from(["ripweb", "rust async traits"]).unwrap();
    assert_eq!(cli.query_or_url.as_deref(), Some("rust async traits"));
}

#[test]
fn parses_positional_url() {
    let cli = Cli::try_parse_from(["ripweb", "https://docs.rs/tokio"]).unwrap();
    assert_eq!(cli.query_or_url.as_deref(), Some("https://docs.rs/tokio"));
}

// ── Default values ────────────────────────────────────────────────────────────

#[test]
fn max_depth_defaults_to_1() {
    let cli = Cli::try_parse_from(["ripweb", "example"]).unwrap();
    assert_eq!(cli.max_depth, 1);
}

#[test]
fn max_pages_defaults_to_10() {
    let cli = Cli::try_parse_from(["ripweb", "example"]).unwrap();
    assert_eq!(cli.max_pages, 10);
}

#[test]
fn stat_defaults_to_false() {
    let cli = Cli::try_parse_from(["ripweb", "example"]).unwrap();
    assert!(!cli.stat);
}

#[test]
fn copy_defaults_to_false() {
    let cli = Cli::try_parse_from(["ripweb", "example"]).unwrap();
    assert!(!cli.copy);
}

#[test]
fn mode_defaults_to_markdown() {
    let cli = Cli::try_parse_from(["ripweb", "example"]).unwrap();
    assert_eq!(cli.mode, OutputMode::Markdown);
}

// ── Flag parsing ──────────────────────────────────────────────────────────────

#[test]
fn force_url_flag_short() {
    let cli = Cli::try_parse_from(["ripweb", "-u", "docs.rs/tokio"]).unwrap();
    assert!(cli.force_url);
}

#[test]
fn force_url_flag_long() {
    let cli = Cli::try_parse_from(["ripweb", "--url", "docs.rs/tokio"]).unwrap();
    assert!(cli.force_url);
}

#[test]
fn force_query_flag_short() {
    let cli = Cli::try_parse_from(["ripweb", "-q", "https://example.com"]).unwrap();
    assert!(cli.force_query);
}

#[test]
fn force_query_flag_long() {
    let cli = Cli::try_parse_from(["ripweb", "--query", "https://example.com"]).unwrap();
    assert!(cli.force_query);
}

#[test]
fn stat_flag() {
    let cli = Cli::try_parse_from(["ripweb", "--stat", "example"]).unwrap();
    assert!(cli.stat);
}

#[test]
fn copy_flag_short() {
    let cli = Cli::try_parse_from(["ripweb", "-c", "example"]).unwrap();
    assert!(cli.copy);
}

#[test]
fn copy_flag_long() {
    let cli = Cli::try_parse_from(["ripweb", "--copy", "example"]).unwrap();
    assert!(cli.copy);
}

#[test]
fn clean_cache_flag() {
    let cli = Cli::try_parse_from(["ripweb", "--clean-cache"]).unwrap();
    assert!(cli.clean_cache);
    // query_or_url is not required when --clean-cache is set
    assert!(cli.query_or_url.is_none());
}

#[test]
fn max_depth_long_flag() {
    let cli = Cli::try_parse_from(["ripweb", "--max-depth", "3", "example"]).unwrap();
    assert_eq!(cli.max_depth, 3);
}

#[test]
fn max_pages_long_flag() {
    let cli = Cli::try_parse_from(["ripweb", "--max-pages", "5", "example"]).unwrap();
    assert_eq!(cli.max_pages, 5);
}

#[test]
fn mode_long_flag_accepts_aggressive() {
    let cli = Cli::try_parse_from(["ripweb", "--mode", "aggressive", "example"]).unwrap();
    assert_eq!(cli.mode, OutputMode::Aggressive);
}

#[test]
fn mode_short_flag_accepts_markdown_alias() {
    let cli = Cli::try_parse_from(["ripweb", "-m", "md", "example"]).unwrap();
    assert_eq!(cli.mode, OutputMode::Markdown);
}

// ── Verbosity ─────────────────────────────────────────────────────────────────

#[test]
fn verbosity_zero_by_default() {
    let cli = Cli::try_parse_from(["ripweb", "example"]).unwrap();
    assert_eq!(cli.verbose, 0);
}

#[test]
fn verbosity_single_v() {
    let cli = Cli::try_parse_from(["ripweb", "-v", "example"]).unwrap();
    assert_eq!(cli.verbose, 1);
}

#[test]
fn verbosity_triple_v() {
    let cli = Cli::try_parse_from(["ripweb", "-vvv", "example"]).unwrap();
    assert_eq!(cli.verbose, 3);
}

// ── Conflict detection ────────────────────────────────────────────────────────

#[test]
fn url_and_query_flags_conflict() {
    let result = Cli::try_parse_from(["ripweb", "-u", "-q", "example"]);
    assert!(result.is_err(), "combining -u and -q must be rejected by clap");
}

// ── Missing required argument ─────────────────────────────────────────────────

#[test]
fn missing_positional_without_clean_cache_is_error() {
    let result = Cli::try_parse_from(["ripweb"]);
    assert!(result.is_err(), "no positional arg and no --clean-cache must fail");
}

// ── Help does not panic ───────────────────────────────────────────────────────

#[test]
fn help_exits_cleanly() {
    // try_parse_from returns Err on --help (DisplayHelp variant) but must not panic
    let result = Cli::try_parse_from(["ripweb", "--help"]);
    assert!(result.is_err());
    // The error kind should be DisplayHelp, not anything structural
    assert_eq!(
        result.unwrap_err().kind(),
        clap::error::ErrorKind::DisplayHelp
    );
}
