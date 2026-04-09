//! Benchmark schema types for search quality evaluation.
//!
//! These types are intentionally kept in `src/` so ranking code added in
//! later phases can emit `SearchResultRecord` values directly.

use serde::{Deserialize, Serialize};

/// A single labeled query entry in the benchmark fixture.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkQuery {
    /// The search query string.
    pub query: String,
    /// Intent label: "official_docs" | "emerging_project_docs" |
    ///   "code_error_lookup" | "general_technical".
    pub intent: String,
    /// Gold URLs — any of these in results counts as a hit.
    pub gold_urls: Vec<String>,
    /// Subset of gold_urls that must rank highest for full credit (relevance=2).
    pub gold_priority: Vec<String>,
    /// URLs that must NOT appear in results (known spam/noise).
    #[serde(default)]
    pub negative_urls: Vec<String>,
    /// Pre-recorded baseline search results for deterministic evaluation.
    /// Embedded in the fixture so no live network is needed.
    pub baseline_results: Vec<SearchResultRecord>,
}

/// A single search result, used both in fixtures and in `QueryTrace`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultRecord {
    pub url: String,
    pub title: String,
    pub snippet: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn benchmark_query_roundtrips_through_json() {
        let q = BenchmarkQuery {
            query: "tokio async runtime".to_owned(),
            intent: "official_docs".to_owned(),
            gold_urls: vec!["https://tokio.rs/tokio/tutorial".to_owned()],
            gold_priority: vec!["https://tokio.rs/tokio/tutorial".to_owned()],
            negative_urls: vec!["https://medium.com".to_owned()],
            baseline_results: vec![SearchResultRecord {
                url: "https://tokio.rs/tokio/tutorial".to_owned(),
                title: "Tutorial | Tokio".to_owned(),
                snippet: Some("Welcome to Tokio.".to_owned()),
            }],
        };
        let json = serde_json::to_string(&q).unwrap();
        let back: BenchmarkQuery = serde_json::from_str(&json).unwrap();
        assert_eq!(back.query, q.query);
        assert_eq!(back.gold_urls, q.gold_urls);
        assert_eq!(back.baseline_results.len(), 1);
        assert_eq!(back.intent, q.intent);
        assert_eq!(back.gold_priority, q.gold_priority);
        assert_eq!(back.negative_urls, q.negative_urls);
        assert_eq!(back.baseline_results[0].url, q.baseline_results[0].url);
        assert_eq!(
            back.baseline_results[0].snippet,
            q.baseline_results[0].snippet
        );
    }

    #[test]
    fn search_result_record_negative_urls_default_to_empty() {
        let json = r#"{
            "query": "foo",
            "intent": "general_technical",
            "gold_urls": [],
            "gold_priority": [],
            "baseline_results": []
        }"#;
        let q: BenchmarkQuery = serde_json::from_str(json).unwrap();
        assert!(
            q.negative_urls.is_empty(),
            "negative_urls must default to []"
        );
    }
}
