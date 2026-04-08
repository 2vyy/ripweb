use ripweb::search::duckduckgo::{ddg_search_url, parse_ddg_html};

const FIXTURE: &str = include_str!("fixtures/search/ddg_results.html");

// ── URL construction ──────────────────────────────────────────────────────────

#[test]
fn ddg_url_points_to_html_endpoint() {
    let url = ddg_search_url("rust async");
    assert_eq!(url.host_str(), Some("html.duckduckgo.com"));
    assert_eq!(url.path(), "/html/");
}

#[test]
fn ddg_url_encodes_query_in_q_param() {
    let url = ddg_search_url("rust async traits");
    let q: Vec<_> = url
        .query_pairs()
        .filter(|(k, _)| k == "q")
        .map(|(_, v)| v.into_owned())
        .collect();
    assert_eq!(q, vec!["rust async traits"]);
}

// ── HTML parsing ──────────────────────────────────────────────────────────────

#[test]
fn parse_extracts_decoded_urls_from_fixture() {
    let urls = parse_ddg_html(FIXTURE, 10);
    assert!(
        urls.iter().any(|u| u.contains("doc.rust-lang.org")),
        "async book URL missing: {:?}",
        urls
    );
    assert!(
        urls.iter().any(|u| u.contains("tokio.rs")),
        "tokio URL missing: {:?}",
        urls
    );
}

#[test]
fn parse_respects_limit() {
    let urls = parse_ddg_html(FIXTURE, 2);
    assert_eq!(urls.len(), 2, "expected exactly 2 results, got: {:?}", urls);
}

#[test]
fn parse_returns_decoded_urls_not_ddg_redirects() {
    let urls = parse_ddg_html(FIXTURE, 10);
    for url in &urls {
        assert!(
            !url.contains("duckduckgo.com/l/"),
            "DDG redirect URL leaked into results: {url}"
        );
        assert!(
            url.starts_with("http://") || url.starts_with("https://"),
            "result is not an absolute URL: {url}"
        );
    }
}

#[test]
fn parse_handles_limit_larger_than_results() {
    let urls = parse_ddg_html(FIXTURE, 100);
    assert_eq!(urls.len(), 4, "fixture has 4 results");
}

#[test]
fn parse_returns_empty_on_no_results() {
    let urls = parse_ddg_html("<html><body><p>No results found.</p></body></html>", 10);
    assert!(urls.is_empty());
}
