//! OpenAlex API placeholder module.
//!
//! This module provides basic URL construction utilities for future
//! structured-work retrieval integrations.

use url::Url;

pub fn work_url(work_id: &str) -> Result<Url, url::ParseError> {
    Url::parse(&format!("https://api.openalex.org/works/{}", work_id))
}

#[cfg(test)]
mod tests {
    use super::work_url;

    #[test]
    fn work_url_points_to_openalex_api() {
        let url = work_url("W2741809807").expect("valid openalex URL");
        assert_eq!(url.host_str(), Some("api.openalex.org"));
        assert!(url.path().contains("/works/"));
    }
}
