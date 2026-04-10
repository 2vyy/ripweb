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
}
