//! Marginalia Search (Small Web)
//!
//! Interacts with the Marginalia search engine API which targets the
//! "small web" (independent logs, non-commercial sites).

use serde::Deserialize;

/// A single result from the Marginalia Search API.
#[derive(Debug, Deserialize)]
pub struct MarginaliaResult {
    pub url: String,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    /// Marginalia's spam/quality score. Higher = less SEO-polluted.
    #[serde(default)]
    pub quality: Option<f64>,
}

/// Top-level response from the Marginalia API.
#[derive(Debug, Deserialize)]
struct MarginaliaResponse {
    results: Vec<MarginaliaResult>,
}

/// Build the Marginalia public API endpoint URL.
///
/// Uses the public demo key (`public`) which is functional for low-volume use.
/// Full URL: `https://api.marginalia.nu/public/search/<query>?page=0`
pub fn build_marginalia_url(query: &str) -> String {
    let encoded: String = url::form_urlencoded::byte_serialize(query.as_bytes()).collect();
    format!("https://api.marginalia.nu/public/search/{encoded}?page=0")
}

pub fn parse_marginalia_json(json: &str) -> Result<Vec<super::SearchResult>, serde_json::Error> {
    let response: MarginaliaResponse = serde_json::from_str(json)?;
    Ok(response
        .results
        .into_iter()
        .map(|r| super::SearchResult {
            url: r.url,
            title: r.title,
            snippet: r.description,
        })
        .collect())
}

/// Fetch the top `limit` result URLs for `query` from Marginalia.
///
/// Marginalia focuses on the non-commercial, non-SEO web: personal blogs,
/// old documentation, and small projects. Excellent complement to DDG for
/// finding authentic developer content buried by SEO churn.
pub async fn search(
    client: &rquest::Client,
    query: &str,
    limit: usize,
) -> Result<Vec<super::SearchResult>, String> {
    let url = build_marginalia_url(query);
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Marginalia network error: {e}"))?;

    let body = resp
        .text()
        .await
        .map_err(|e| format!("Marginalia read error: {e}"))?;

    let mut urls =
        parse_marginalia_json(&body).map_err(|e| format!("Marginalia JSON parse error: {e}"))?;

    urls.truncate(limit);

    if urls.is_empty() {
        Err("Marginalia returned no results".into())
    } else {
        Ok(urls)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_marginalia_json_extracts_urls() {
        let json = r#"{
            "results": [
                {"url": "https://blog.example.com/rust", "title": "Rust blog post", "description": "A blog post about Rust", "quality": 85.0},
                {"url": "https://old-docs.example.com/", "title": "Old docs"},
                {"url": "https://personal-site.example.com/post", "title": "Personal post", "quality": 72.5}
            ]
        }"#;
        let res = parse_marginalia_json(json).unwrap();
        let urls: Vec<String> = res.into_iter().map(|s| s.url).collect();
        assert_eq!(
            urls,
            vec![
                "https://blog.example.com/rust",
                "https://old-docs.example.com/",
                "https://personal-site.example.com/post",
            ]
        );
    }

    #[test]
    fn parse_marginalia_json_handles_empty() {
        let json = r#"{"results": []}"#;
        let res = parse_marginalia_json(json).unwrap();
        assert!(res.is_empty());
    }

    #[test]
    fn build_marginalia_url_encodes_query() {
        let url = build_marginalia_url("rust async await");
        assert!(url.contains("api.marginalia.nu/public/search/"));
        // spaces should be encoded
        assert!(!url.contains(" "));
        assert!(url.ends_with("?page=0"));
    }
}
