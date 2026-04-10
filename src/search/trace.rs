//! Query execution trace for deterministic audit and debugging.
//!
//! `QueryTrace` captures the full audit trail for a single query execution.
//! `scorer_contributions` is empty in Phase 0; Phase 1+ scoring populates it.

use crate::search::eval_types::SearchResultRecord;
use serde::{Deserialize, Serialize};

/// One scorer's contribution to a single result's final score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScorerContribution {
    /// Name of the scorer (e.g. "domain_trust", "url_pattern").
    pub scorer: String,
    /// Score delta: positive = boost, negative = penalty.
    pub delta: f64,
    /// Human-readable explanation for this delta.
    pub reason: String,
}

/// Per-result scorer contributions bundled with the result's URL.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultContributions {
    pub url: String,
    pub contributions: Vec<ScorerContribution>,
}

/// Full audit trace for a single query execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryTrace {
    pub query: String,
    /// Raw results from the search engine, in engine order.
    pub engine_results: Vec<SearchResultRecord>,
    /// Final ranked URLs (after scoring/fusion).
    pub final_rank: Vec<String>,
    /// Per-result scorer contributions, in final rank order.
    pub scorer_contributions: Vec<ResultContributions>,
}

impl QueryTrace {
    /// Construct a baseline trace from raw engine results (no scoring).
    #[must_use]
    pub fn from_engine_results(query: &str, results: &[SearchResultRecord]) -> Self {
        Self {
            query: query.to_owned(),
            engine_results: results.to_vec(),
            final_rank: results.iter().map(|r| r.url.clone()).collect(),
            scorer_contributions: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_result(url: &str) -> SearchResultRecord {
        SearchResultRecord {
            url: url.to_owned(),
            title: "Title".to_owned(),
            snippet: None,
        }
    }

    #[test]
    fn from_engine_results_sets_final_rank_to_engine_order() {
        let results = vec![
            make_result("https://a.example.com"),
            make_result("https://b.example.com"),
        ];
        let trace = QueryTrace::from_engine_results("test query", &results);
        assert_eq!(
            trace.final_rank,
            vec!["https://a.example.com", "https://b.example.com",]
        );
    }

    #[test]
    fn from_engine_results_has_no_scorer_contributions() {
        let results = vec![make_result("https://a.example.com")];
        let trace = QueryTrace::from_engine_results("test query", &results);
        assert!(trace.scorer_contributions.is_empty());
    }

    #[test]
    fn result_contributions_roundtrips_through_json() {
        let rc = ResultContributions {
            url: "https://docs.rs/tokio".to_owned(),
            contributions: vec![ScorerContribution {
                scorer: "domain_trust".to_owned(),
                delta: 2.0,
                reason: "high-trust domain".to_owned(),
            }],
        };
        let json = serde_json::to_string(&rc).unwrap();
        let back: ResultContributions = serde_json::from_str(&json).unwrap();
        assert_eq!(back.url, rc.url);
        assert_eq!(back.contributions[0].scorer, "domain_trust");
        assert!((back.contributions[0].delta - 2.0).abs() < 1e-9);
    }

    #[test]
    fn query_trace_roundtrips_through_json() {
        let results = vec![make_result("https://tokio.rs/tokio/tutorial")];
        let trace = QueryTrace::from_engine_results("tokio async", &results);
        let json = serde_json::to_string(&trace).unwrap();
        let back: QueryTrace = serde_json::from_str(&json).unwrap();
        assert_eq!(back.query, "tokio async");
        assert_eq!(back.final_rank, vec!["https://tokio.rs/tokio/tutorial"]);
        assert_eq!(
            back.engine_results[0].url,
            "https://tokio.rs/tokio/tutorial"
        );
        assert_eq!(back.engine_results[0].title, "Title");
        assert_eq!(back.engine_results[0].snippet, None);
        assert!(back.scorer_contributions.is_empty());
    }
}
