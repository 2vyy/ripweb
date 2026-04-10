mod common;

use ripweb::search::{SearchResult, pipeline::score_results};

fn make_result(url: &str, title: &str, snippet: Option<&str>) -> SearchResult {
    SearchResult {
        url: url.to_owned(),
        title: title.to_owned(),
        snippet: snippet.map(str::to_owned),
    }
}

#[test]
fn docs_rs_ranks_above_medium_com_for_same_query() {
    let results = vec![
        make_result(
            "https://medium.com/rustlang/tokio-intro",
            "Introduction to Tokio - Medium",
            Some("A beginner's guide to Tokio async runtime."),
        ),
        make_result(
            "https://docs.rs/tokio/latest/tokio/",
            "tokio - Rust",
            Some("An asynchronous runtime for writing reliable applications."),
        ),
    ];
    let scored = score_results(results, "tokio async runtime");
    // docs.rs must rank first regardless of its position in input
    assert_eq!(
        scored[0].result.url, "https://docs.rs/tokio/latest/tokio/",
        "docs.rs must rank above medium.com"
    );
}

#[test]
fn blocklisted_domain_ranks_last() {
    let results = vec![
        make_result(
            "https://w3schools.com/rust/intro.asp",
            "Rust Introduction - W3Schools",
            Some("Learn Rust programming at W3Schools."),
        ),
        make_result(
            "https://docs.rs/tokio/latest/tokio/",
            "tokio - Rust",
            Some("An async runtime for Rust."),
        ),
        make_result(
            "https://stackoverflow.com/questions/123",
            "How to use Tokio?",
            Some("I am trying to understand async in Rust with Tokio."),
        ),
    ];
    let scored = score_results(results, "tokio async rust");
    assert_ne!(
        scored.last().unwrap().result.url,
        "https://docs.rs/tokio/latest/tokio/",
        "docs.rs must not be last"
    );
    assert_eq!(
        scored.last().unwrap().result.url,
        "https://w3schools.com/rust/intro.asp",
        "w3schools.com must rank last"
    );
}

#[test]
fn duplicate_domain_results_are_penalised() {
    let results = vec![
        make_result(
            "https://medium.com/@a/rust-intro",
            "Rust for Beginners - Medium",
            Some("Introduction to Rust programming"),
        ),
        make_result(
            "https://medium.com/@b/rust-advanced",
            "Advanced Rust - Medium",
            Some("Advanced topics in Rust"),
        ),
        make_result(
            "https://docs.rs/tokio/latest/tokio/",
            "tokio - Rust",
            Some("Async runtime for Rust."),
        ),
    ];
    let scored = score_results(results, "rust");
    let medium_scores: Vec<f64> = scored
        .iter()
        .filter(|s| s.result.url.contains("medium.com"))
        .map(|s| s.score)
        .collect();
    assert_eq!(
        medium_scores.len(),
        2,
        "both medium.com results must be present"
    );
    // The second medium.com result must score lower than the first
    // (domain diversity penalty applies)
    assert!(
        medium_scores[0] > medium_scores[1]
            || scored
                .iter()
                .position(|s| s.result.url.contains("medium.com/@a"))
                > scored
                    .iter()
                    .position(|s| s.result.url.contains("medium.com/@b")),
        "second medium.com result must be penalised relative to first"
    );
}

#[test]
fn score_results_returns_same_count_as_input() {
    let results = vec![
        make_result("https://a.example.com", "A", None),
        make_result("https://b.example.com", "B", None),
        make_result("https://c.example.com", "C", None),
    ];
    let scored = score_results(results, "query");
    assert_eq!(scored.len(), 3);
}

#[test]
fn score_results_with_empty_input_returns_empty() {
    let scored = score_results(vec![], "query");
    assert!(scored.is_empty());
}

#[test]
fn each_scored_result_has_six_contributions() {
    let results = vec![make_result(
        "https://docs.rs/tokio/",
        "tokio - Rust",
        Some("Async runtime"),
    )];
    let scored = score_results(results, "tokio");
    assert_eq!(
        scored[0].contributions.len(),
        6,
        "must have one contribution per scorer (domain_trust, url_pattern, project_match, blocklist_penalty, snippet_relevance, domain_diversity)"
    );
}
