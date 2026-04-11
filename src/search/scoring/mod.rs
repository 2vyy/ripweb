//! Search result scoring — metadata-only, synchronous, no network.
//!
//! Each scorer is a pure function that takes a `ScorerInput` and returns a
//! `ScorerContribution`. The pipeline in `super::pipeline` wires them together.

pub mod blocklist_penalty;
pub mod domain_diversity;
pub mod domain_trust;
pub mod project_match;
pub mod snippet_relevance;
pub mod url_pattern;

use crate::search::{SearchResult, trace::ScorerContribution};

/// Runtime-tunable weights for metadata scorers and RRF fusion.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct ScoringWeights {
    pub domain_trust: f64,
    pub domain_diversity: f64,
    pub snippet_relevance: f64,
    pub url_pattern: f64,
    pub blocklist_penalty: f64,
    pub project_match: f64,
    /// Reciprocal Rank Fusion constant `k`.
    pub rrf_k: f64,
}

impl Default for ScoringWeights {
    fn default() -> Self {
        Self {
            domain_trust: 1.0,
            domain_diversity: 0.5,
            snippet_relevance: 0.6,
            url_pattern: 0.8,
            blocklist_penalty: 1.0,
            project_match: 1.2,
            rrf_k: 60.0,
        }
    }
}

impl ScoringWeights {
    pub const TUNABLE_FIELDS: usize = 7;

    #[must_use]
    pub fn get(&self, index: usize) -> f64 {
        match index {
            0 => self.domain_trust,
            1 => self.domain_diversity,
            2 => self.snippet_relevance,
            3 => self.url_pattern,
            4 => self.blocklist_penalty,
            5 => self.project_match,
            6 => self.rrf_k,
            _ => 0.0,
        }
    }

    pub fn set(&mut self, index: usize, value: f64) {
        let value = if index == 6 {
            value.max(1.0)
        } else {
            value.max(0.0)
        };
        match index {
            0 => self.domain_trust = value,
            1 => self.domain_diversity = value,
            2 => self.snippet_relevance = value,
            3 => self.url_pattern = value,
            4 => self.blocklist_penalty = value,
            5 => self.project_match = value,
            6 => self.rrf_k = value,
            _ => {}
        }
    }
}

/// Input to a per-result stateless scorer.
pub struct ScorerInput<'a> {
    /// The candidate search result.
    pub result: &'a SearchResult,
    /// The original user query string.
    pub query: &'a str,
    /// Zero-indexed position in the engine's result list (0 = top).
    pub engine_rank: usize,
}

/// A search result after scoring — carries its composite score and
/// an audit trail of per-scorer contributions.
pub struct ScoredResult {
    pub result: SearchResult,
    /// Sum of weighted scorer deltas.
    pub score: f64,
    /// Ordered list of contributions from each scorer.
    pub contributions: Vec<ScorerContribution>,
}

/// Extract the lowercase hostname from a URL string.
/// Returns an empty string on parse failure.
pub fn extract_host(url: &str) -> String {
    url::Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.to_ascii_lowercase()))
        .unwrap_or_default()
}

/// Return true if `host` equals `domain` or is a subdomain of `domain`.
///
/// `host_matches("docs.rs", "docs.rs")` → `true`
/// `host_matches("api.docs.rs", "docs.rs")` → `true`
/// `host_matches("other.com", "docs.rs")` → `false`
pub fn host_matches(host: &str, domain: &str) -> bool {
    host == domain || host.ends_with(&format!(".{domain}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_host_returns_lowercase_hostname() {
        assert_eq!(
            extract_host("https://Docs.RS/tokio/latest/tokio/"),
            "docs.rs"
        );
    }

    #[test]
    fn extract_host_returns_empty_on_invalid_url() {
        assert_eq!(extract_host("not-a-url"), "");
    }

    #[test]
    fn host_matches_exact() {
        assert!(host_matches("docs.rs", "docs.rs"));
    }

    #[test]
    fn host_matches_subdomain() {
        assert!(host_matches("api.docs.rs", "docs.rs"));
    }

    #[test]
    fn host_does_not_match_partial_suffix() {
        // "notdocs.rs" must not match "docs.rs"
        assert!(!host_matches("notdocs.rs", "docs.rs"));
    }

    #[test]
    fn host_does_not_match_different_domain() {
        assert!(!host_matches("other.com", "docs.rs"));
    }

    #[test]
    fn scoring_weights_get_set_roundtrip() {
        let mut w = ScoringWeights::default();
        w.set(0, 1.5);
        w.set(6, 75.0);
        assert!((w.get(0) - 1.5).abs() < f64::EPSILON);
        assert!((w.get(6) - 75.0).abs() < f64::EPSILON);
    }
}
