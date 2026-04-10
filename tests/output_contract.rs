use ripweb::{
    extract::{Extractor, web::WebExtractor},
    mode::Mode,
    run::{format_generic, format_hn, format_reddit, format_search_results},
    search::{hackernews::HnContent, reddit::RedditContent},
};
use url::Url;

// ── format_generic ────────────────────────────────────────────────────────────

#[test]
fn compact_mode_generic_emits_delimiter_and_link() {
    let url = Url::parse("https://example.com/item").unwrap();
    let out = format_generic("some text", &url, Mode::Compact);
    assert!(
        out.starts_with("# --- [Source:"),
        "must start with source delimiter, got: {out:?}"
    );
    assert!(out.contains("- [Generic Page](https://example.com/item)"));
    assert!(
        !out.contains("some text"),
        "compact must not include body text"
    );
}

#[test]
fn balanced_mode_generic_emits_delimiter_and_snippet() {
    let url = Url::parse("https://example.com/item").unwrap();
    let text = "Line 1\nLine 2\nLine 3\n".repeat(100);

    let out = format_generic(&text, &url, Mode::Balanced);
    assert!(
        out.starts_with("# --- [Source:"),
        "must start with source delimiter"
    );
    assert!(out.contains("Line 1"));
    assert!(out.contains("... (truncated)"));
}

#[test]
fn verbose_mode_generic_emits_full_content() {
    let url = Url::parse("https://example.com/item").unwrap();
    let text = "Line 1\nLine 2\n".repeat(100);

    let out = format_generic(&text, &url, Mode::Verbose);
    assert!(
        out.starts_with("# --- [Source:"),
        "must start with source delimiter"
    );
    assert!(
        !out.contains("... (truncated)"),
        "verbose must not truncate"
    );
    assert!(out.contains("Line 1"));
}

#[test]
fn source_delimiter_strips_tracking_params() {
    let url = Url::parse("https://example.com/item?utm_source=test&id=1").unwrap();
    let out = format_generic("text", &url, Mode::Compact);
    assert!(
        !out.contains("utm_source"),
        "source delimiter must strip tracking params"
    );
    assert!(
        out.contains("id=1"),
        "non-tracking params must be preserved"
    );
}

// ── format_reddit ─────────────────────────────────────────────────────────────

#[test]
fn compact_mode_reddit_title_only() {
    let content = RedditContent {
        title: "Rust is great".into(),
        selftext: "Body text here".into(),
        comments: vec!["Comment 1".into(), "Comment 2".into(), "Comment 3".into()],
    };
    let out = format_reddit(&content, Mode::Compact);
    assert!(out.contains("- [Rust is great]"));
    assert!(!out.contains("Body text here"));
}

#[test]
fn balanced_mode_reddit_top_two_comments() {
    let content = RedditContent {
        title: "Rust is great".into(),
        selftext: "Body text here".into(),
        comments: vec!["Comment 1".into(), "Comment 2".into(), "Comment 3".into()],
    };
    let out = format_reddit(&content, Mode::Balanced);
    assert!(out.contains("# Rust is great"));
    assert!(out.contains("Comment 2"));
    assert!(
        !out.contains("Comment 3"),
        "balanced must cap at 2 comments"
    );
}

#[test]
fn verbose_mode_reddit_full_comment_tree() {
    let content = RedditContent {
        title: "Rust is great".into(),
        selftext: "Body text here".into(),
        comments: vec!["Comment 1".into(), "Comment 2".into(), "Comment 3".into()],
    };
    let out = format_reddit(&content, Mode::Verbose);
    assert!(out.contains("Comment 3"));
}

// ── format_hn ─────────────────────────────────────────────────────────────────

#[test]
fn compact_mode_hn_title_only() {
    let content = HnContent {
        title: "Show HN: Ripweb".into(),
        text: Some("Check out this tool".into()),
        comments: (1..=10).map(|i| format!("Comment {i}")).collect(),
    };
    let out = format_hn(&content, Mode::Compact);
    assert!(out.contains("- [Show HN: Ripweb]"));
    assert!(!out.contains("Check out this tool"));
}

#[test]
fn balanced_mode_hn_top_five_comments() {
    let content = HnContent {
        title: "Show HN: Ripweb".into(),
        text: Some("Check out this tool".into()),
        comments: (1..=10).map(|i| format!("Comment {i}")).collect(),
    };
    let out = format_hn(&content, Mode::Balanced);
    assert!(out.contains("# Show HN: Ripweb"));
    assert!(out.contains("Comment 5"));
    assert!(
        !out.contains("Comment 6"),
        "balanced must cap at 5 comments"
    );
}

#[test]
fn verbose_mode_hn_full_comments() {
    let content = HnContent {
        title: "Show HN: Ripweb".into(),
        text: Some("Check out this tool".into()),
        comments: (1..=10).map(|i| format!("Comment {i}")).collect(),
    };
    let out = format_hn(&content, Mode::Verbose);
    assert!(out.contains("Comment 10"));
}

// ── format_search_results ─────────────────────────────────────────────────────

#[test]
fn compact_mode_search_link_list() {
    let items = vec![ripweb::search::SearchResult {
        title: "Result 1".into(),
        url: "https://r1.com".into(),
        snippet: Some("Snippet 1".into()),
    }];
    let out = format_search_results(&items, None, Mode::Compact, ripweb::cli::SearchEngine::Ddg);
    assert_eq!(out, "- [Result 1](https://r1.com)");
}

#[test]
fn balanced_mode_search_link_plus_snippet() {
    let items = vec![ripweb::search::SearchResult {
        title: "Result 1".into(),
        url: "https://r1.com".into(),
        snippet: Some("Snippet 1".into()),
    }];
    let out = format_search_results(&items, None, Mode::Balanced, ripweb::cli::SearchEngine::Ddg);
    assert!(out.contains("> Snippet 1"));
}

#[test]
fn verbose_mode_search_detailed_card_with_instant() {
    let items = vec![ripweb::search::SearchResult {
        title: "Result 1".into(),
        url: "https://r1.com".into(),
        snippet: Some("Snippet 1".into()),
    }];
    let out = format_search_results(
        &items,
        Some("Instant!"),
        Mode::Verbose,
        ripweb::cli::SearchEngine::Ddg,
    );
    assert!(out.contains("### [Result 1]"));
    assert!(out.contains("> Instant!"));
}

// ── fan_out engine label ──────────────────────────────────────────────────────

#[test]
fn fan_out_engine_label_in_header() {
    let items = vec![ripweb::search::SearchResult {
        title: "Result 1".into(),
        url: "https://r1.com".into(),
        snippet: Some("Snippet 1".into()),
    }];
    let out = format_search_results(
        &items,
        None,
        ripweb::mode::Mode::Balanced,
        ripweb::cli::SearchEngine::FanOut,
    );
    assert!(
        out.contains("DDG")
            || out.contains("Marginalia")
            || out.contains("Multi-engine")
            || out.contains("RRF"),
        "fan-out header must identify multi-engine source, got: {out:?}"
    );
}

// ── GitHub issue format ───────────────────────────────────────────────────────

#[test]
fn compact_mode_github_issue_list_format() {
    let issue = ripweb::search::github::GithubIssue {
        number: 1,
        title: "Bug".into(),
        body: Some("Description".into()),
        labels: vec![],
        user: ripweb::search::github::GithubUser {
            login: "alice".into(),
        },
        html_url: "https://github.com/a/b/issues/1".into(),
    };
    let comments = vec![ripweb::search::github::GithubComment {
        body: Some("Comment 1".into()),
        user: ripweb::search::github::GithubUser {
            login: "bob".into(),
        },
    }];

    let out = ripweb::search::github::format_issue(&issue, &comments, Mode::Compact);
    assert!(out.contains("- [#1] Bug"));
    assert!(!out.contains("Description"));
}

#[test]
fn balanced_mode_github_issue_op_only() {
    let issue = ripweb::search::github::GithubIssue {
        number: 1,
        title: "Bug".into(),
        body: Some("Description".into()),
        labels: vec![],
        user: ripweb::search::github::GithubUser {
            login: "alice".into(),
        },
        html_url: "https://github.com/a/b/issues/1".into(),
    };
    let comments = vec![ripweb::search::github::GithubComment {
        body: Some("Comment 1".into()),
        user: ripweb::search::github::GithubUser {
            login: "bob".into(),
        },
    }];

    let out = ripweb::search::github::format_issue(&issue, &comments, Mode::Balanced);
    assert!(out.contains("Description"));
    assert!(!out.contains("Comment 1"), "balanced must omit comments");
}

#[test]
fn verbose_mode_github_issue_with_comments() {
    let issue = ripweb::search::github::GithubIssue {
        number: 1,
        title: "Bug".into(),
        body: Some("Description".into()),
        labels: vec![],
        user: ripweb::search::github::GithubUser {
            login: "alice".into(),
        },
        html_url: "https://github.com/a/b/issues/1".into(),
    };
    let comments = vec![ripweb::search::github::GithubComment {
        body: Some("Comment 1".into()),
        user: ripweb::search::github::GithubUser {
            login: "bob".into(),
        },
    }];

    let out = ripweb::search::github::format_issue(&issue, &comments, Mode::Verbose);
    assert!(out.contains("Comment 1"));
}

// ── Extraction invariants ─────────────────────────────────────────────────────

#[test]
fn markdown_extraction_normalization() {
    let html = br#"<html><body><main><h1>Title</h1><p>Text <a href="https://example.com/path?utm_source=test&id=2">Link</a></p></main></body></html>"#;
    let result = WebExtractor::extract(html, Some("text/html")).unwrap();
    assert!(result.contains("# Title"));
    assert!(result.contains("[Link](https://example.com/path?id=2)")); // stripped utm_source
}

#[test]
fn output_never_longer_than_input() {
    let fixtures: &[&[u8]] = &[
        include_bytes!("fixtures/extract/article_clean.html"),
        include_bytes!("fixtures/extract/bloated_generic.html"),
    ];
    for html in fixtures {
        let result = WebExtractor::extract(html, Some("text/html")).unwrap_or_default();
        assert!(result.len() <= html.len());
    }
}
