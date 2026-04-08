use ripweb::{
    extract::{Extractor, web::WebExtractor},
    run::{format_generic, format_hn, format_reddit},
    search::{hackernews::HnContent, reddit::RedditContent},
};
use url::Url;

#[test]
fn verbosity_tier_generic_enforcement() {
    let url = Url::parse("https://example.com/item").unwrap();
    let text = "Line 1\nLine 2\nLine 3\n".repeat(100); // long text

    // V1: Link only
    let v1 = format_generic(&text, &url, 1);
    assert!(v1.contains("- [Generic Page](https://example.com/item)"));
    assert!(!v1.contains("Line 1"));

    // V2: Snippet (2000 chars)
    let v2 = format_generic(&text, &url, 2);
    assert!(v2.contains("# Page: https://example.com/item"));
    assert!(v2.contains("Line 1"));
    assert!(v2.contains("... (truncated)"));
    assert!(v2.len() > 1000 && v2.len() < 2100);

    // V3: Full Context
    let v3 = format_generic(&text, &url, 3);
    assert!(v3.contains("Line 1"));
    assert!(!v3.contains("... (truncated)"));
}

#[test]
fn verbosity_tier_reddit_enforcement() {
    let content = RedditContent {
        title: "Rust is great".into(),
        selftext: "Body text here".into(),
        comments: vec!["Comment 1".into(), "Comment 2".into(), "Comment 3".into()],
    };

    // V1: Title only
    let v1 = format_reddit(&content, 1);
    assert!(v1.contains("- [Rust is great]"));
    assert!(!v1.contains("Body text here"));

    // V2: Title + 2 Comments
    let v2 = format_reddit(&content, 2);
    assert!(v2.contains("# Rust is great"));
    assert!(v2.contains("Comment 2"));
    assert!(!v2.contains("Comment 3"));

    // V3: Full Context
    let v3 = format_reddit(&content, 3);
    assert!(v3.contains("Comment 3"));
}

#[test]
fn verbosity_tier_hn_enforcement() {
    let content = HnContent {
        title: "Show HN: Ripweb".into(),
        text: Some("Check out this tool".into()),
        comments: (1..=10).map(|i| format!("Comment {i}")).collect(),
    };

    // V1: Title only
    let v1 = format_hn(&content, 1);
    assert!(v1.contains("- [Show HN: Ripweb]"));
    assert!(!v1.contains("Check out this tool"));

    // V2: Title + 5 Comments
    let v2 = format_hn(&content, 2);
    assert!(v2.contains("# Show HN: Ripweb"));
    assert!(v2.contains("Comment 5"));
    assert!(!v2.contains("Comment 6"));

    // V3: Full Context
    let v3 = format_hn(&content, 3);
    assert!(v3.contains("Comment 10"));
}

// ── Extraction Invariants ──────────────────────────────────────────────────

#[test]
fn markdown_extraction_normalization() {
    let html = br#"<html><body><main><h1>Title</h1><p>Text <a href="https://example.com/path?utm_source=test&id=2">Link</a></p></main></body></html>"#;
    let result = WebExtractor::extract(html, Some("text/html")).unwrap();
    assert!(result.contains("# Title"));
    assert!(result.contains("[Link](https://example.com/path?id=2)")); // stripped utm_source
}

#[test]
fn verbosity_tier_search_results_enforcement() {
    let items = vec![ripweb::search::SearchResult {
        title: "Result 1".into(),
        url: "https://r1.com".into(),
        snippet: Some("Snippet 1".into()),
    }];

    // V1: Link list
    let v1 = ripweb::run::format_search_results(&items, None, 1, ripweb::cli::SearchEngine::Ddg);
    assert_eq!(v1, "- [Result 1](https://r1.com)");

    // V2: Link + Snippet
    let v2 = ripweb::run::format_search_results(&items, None, 2, ripweb::cli::SearchEngine::Ddg);
    assert!(v2.contains("> Snippet 1"));

    // V3: Detailed
    let v3 = ripweb::run::format_search_results(
        &items,
        Some("Instant!"),
        3,
        ripweb::cli::SearchEngine::Ddg,
    );
    assert!(v3.contains("### [Result 1]"));
    assert!(v3.contains("> Instant!"));
}

#[test]
fn verbosity_tier_github_issue_enforcement() {
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

    // V1: List format
    let v1 = ripweb::search::github::format_issue(&issue, &comments, 1);
    assert!(v1.contains("- [#1] Bug"));
    assert!(!v1.contains("Description"));

    // V2: OP only
    let v2 = ripweb::search::github::format_issue(&issue, &comments, 2);
    assert!(v2.contains("Description"));
    assert!(!v2.contains("Comment 1"));

    // V3: Full
    let v3 = ripweb::search::github::format_issue(&issue, &comments, 3);
    assert!(v3.contains("Comment 1"));
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
