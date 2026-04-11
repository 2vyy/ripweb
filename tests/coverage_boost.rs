use clap::Parser;
use ripweb::cli::Cli;
use ripweb::cli_utils::{classify_source, unix_timestamp_seconds};

#[test]
fn classify_wikidata_returns_wikidata_type() {
    let cli = Cli::parse_from(&["ripweb", "--wikidata", "Q"]);
    let (url, q, stype, domain) = classify_source(&cli, "ignored");
    assert_eq!(stype, "wikidata");
    assert_eq!(domain.as_deref(), Some("wikidata.org"));
    assert_eq!(q, Some("Q".to_string()));
    assert!(url.is_some());
}

#[test]
fn classify_batch_returns_batch_type() {
    let cli = Cli::parse_from(&["ripweb", "--batch"]);
    let (url, q, stype, domain) = classify_source(&cli, "anything");
    assert_eq!(stype, "batch");
    assert!(url.is_none());
    assert!(q.is_none());
    assert!(domain.is_none());
}

#[test]
fn classify_force_url_prepends_https() {
    let cli = Cli::parse_from(&["ripweb", "example.com", "--url"]);
    let (url, q, stype, domain) = classify_source(&cli, "example.com");
    assert_eq!(stype, "generic");
    assert_eq!(q, None);
    assert_eq!(domain.as_deref(), Some("example.com"));
    assert!(url.unwrap().starts_with("https://"));
}

#[test]
fn classify_github_issue_parses_issue_url() {
    let cli = Cli::parse_from(&["ripweb", "https://github.com/owner/repo/issues/42"]);
    let (url, q, stype, domain) = classify_source(&cli, "https://github.com/owner/repo/issues/42");
    assert_eq!(stype, "github");
    assert_eq!(domain.as_deref(), Some("github.com"));
    assert!(url.unwrap().contains("issues/42"));
}

#[test]
fn unix_timestamp_seconds_is_numeric() {
    let s = unix_timestamp_seconds();
    let n: u64 = s.parse().expect("timestamp should parse as u64");
    assert!(n > 0);
}
