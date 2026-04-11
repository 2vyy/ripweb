//! Semantic Scholar API placeholder module.
//!
//! This module provides basic URL construction utilities for future
//! structured-paper integrations.

use url::Url;

pub fn paper_url(paper_id: &str) -> Result<Url, url::ParseError> {
    Url::parse(&format!(
        "https://api.semanticscholar.org/graph/v1/paper/{}",
        paper_id
    ))
}

#[cfg(test)]
mod tests {
    use super::paper_url;

    #[test]
    fn paper_url_points_to_graph_api() {
        let url = paper_url("CorpusID:12345").expect("valid semantic scholar URL");
        assert_eq!(url.host_str(), Some("api.semanticscholar.org"));
        assert!(url.path().contains("/graph/v1/paper/"));
    }
}
