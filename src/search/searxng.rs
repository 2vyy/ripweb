//! SearXNG Metasearch Engine
//!
//! Integrates with self-hosted or public SearXNG instances using
//! their JSON API to retrieve aggregated search results.

use serde::Deserialize;

/// Result from a SearXNG search instance.
#[derive(Debug, Deserialize)]
pub struct SearxResult {
    pub url: String,
    pub title: String,
    #[serde(default)]
    pub content: Option<String>,
}

/// Top-level response from `GET /search?format=json`.
#[derive(Debug, Deserialize)]
struct SearxResponse {
    results: Vec<SearxResult>,
}

/// Parse a SearXNG JSON response body into a list of result URLs.
pub fn parse_searxng_json(json: &str) -> Result<Vec<super::SearchResult>, serde_json::Error> {
    let response: SearxResponse = serde_json::from_str(json)?;
    Ok(response
        .results
        .into_iter()
        .map(|r| super::SearchResult {
            url: r.url,
            title: r.title,
            snippet: r.content,
        })
        .collect())
}

/// Build a SearXNG search URL for a given instance base URL and query.
///
/// The instance URL should not include a trailing slash.
/// Example: `build_searxng_url("https://searx.be", "rust async", 10)`
pub fn build_searxng_url(instance: &str, query: &str, limit: usize) -> String {
    let encoded: String = url::form_urlencoded::byte_serialize(query.as_bytes()).collect();
    // SearXNG doesn't have a native `limit` param — we request page 1 and truncate.
    // Also request English language results and general category only.
    format!(
        "{instance}/search?q={encoded}&format=json&language=en&categories=general&pageno=1&limit={limit}"
    )
}

/// Fetch the top `limit` result URLs for `query` from a SearXNG instance.
pub async fn search(
    client: &rquest::Client,
    instance_url: &str,
    query: &str,
    limit: usize,
) -> Result<Vec<super::SearchResult>, String> {
    let url = build_searxng_url(instance_url, query, limit);
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("SearXNG network error: {e}"))?;

    let body = resp
        .text()
        .await
        .map_err(|e| format!("SearXNG read error: {e}"))?;

    let mut urls =
        parse_searxng_json(&body).map_err(|e| format!("SearXNG JSON parse error: {e}"))?;

    urls.truncate(limit);

    if urls.is_empty() {
        Err("SearXNG returned no results".into())
    } else {
        Ok(urls)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_searxng_json_extracts_urls() {
        let json = r#"{
            "results": [
                {"url": "https://example.com/a", "title": "A", "content": "Snippet A"},
                {"url": "https://example.com/b", "title": "B"},
                {"url": "https://example.com/c", "title": "C", "content": null}
            ],
            "number_of_results": 3
        }"#;
        let res = parse_searxng_json(json).unwrap();
        let urls: Vec<String> = res.into_iter().map(|s| s.url).collect();
        assert_eq!(
            urls,
            vec![
                "https://example.com/a",
                "https://example.com/b",
                "https://example.com/c"
            ]
        );
    }

    #[test]
    fn parse_searxng_json_handles_empty_results() {
        let json = r#"{"results": [], "number_of_results": 0}"#;
        let res = parse_searxng_json(json).unwrap();
        assert!(res.is_empty());
    }

    #[test]
    fn build_searxng_url_encodes_query() {
        let url = build_searxng_url("https://searx.be", "rust async await", 5);
        assert!(url.starts_with("https://searx.be/search?q=rust"));
        assert!(url.contains("format=json"));
    }

    #[test]
    fn build_searxng_url_special_chars() {
        let url = build_searxng_url("https://searx.be", "c++ templates", 3);
        // The + should be encoded
        assert!(!url.contains("c++ templates"));
    }
}
