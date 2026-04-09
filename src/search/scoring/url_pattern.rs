//! URL pattern scorer.
//!
//! Boosts results from known documentation hosts and documentation-style
//! URL paths. Penalises known content-farm and tutorial-aggregator hosts.
//! All decisions are explicit and appear in the contribution reason string.

use crate::search::scoring::{ScorerInput, extract_host, host_matches};
use crate::search::trace::ScorerContribution;

/// Hosts that are themselves documentation platforms — always boost.
const DOCS_HOSTS: &[&str] = &[
    "docs.rs",
    "doc.rust-lang.org",
    "developer.mozilla.org",
    "pkg.go.dev",
    "docs.python.org",
    "cppreference.com",
    "api.dart.dev",
];

/// URL path substrings that indicate official documentation.
const BOOST_PATHS: &[&str] = &[
    "/docs/",
    "/reference/",
    "/api/",
    "/guide/",
    "/manual/",
    "/book/",
    "/stable/",
    "/latest/",
];

/// Hosts known to be content farms, aggregators, or tutorial mills.
const PENALISE_HOSTS: &[&str] = &[
    "medium.com",
    "dev.to",
    "hashnode.dev",
    "hashnode.com",
    "blogspot.com",
    "wordpress.com",
    "substack.com",
    "dzone.com",
    "hackernoon.com",
];

/// URL path substrings common in SEO/tutorial-farm content.
const PENALISE_PATHS: &[&str] = &[
    "/what-is-",
    "/beginners-guide-",
    "/introduction-to-",
    "/getting-started-with-",
];

/// Score a result based on its URL host and path patterns.
#[must_use]
pub fn score(input: &ScorerInput) -> ScorerContribution {
    let url_lower = input.result.url.to_ascii_lowercase();
    let host = extract_host(&url_lower);

    // Documentation host — strongest positive signal.
    if DOCS_HOSTS.iter().any(|&d| host_matches(&host, d)) {
        return ScorerContribution {
            scorer: "url_pattern".to_owned(),
            delta: 1.5,
            reason: format!("documentation platform host: {host}"),
        };
    }

    // github.io is used almost exclusively for project documentation pages.
    if host.ends_with(".github.io") {
        return ScorerContribution {
            scorer: "url_pattern".to_owned(),
            delta: 1.0,
            reason: format!("github.io docs host: {host}"),
        };
    }

    // Content-farm host — negative signal.
    if PENALISE_HOSTS.iter().any(|&h| host_matches(&host, h)) {
        return ScorerContribution {
            scorer: "url_pattern".to_owned(),
            delta: -1.0,
            reason: format!("content-farm host: {host}"),
        };
    }

    // Documentation-style path on a non-penalised host.
    if BOOST_PATHS.iter().any(|&p| url_lower.contains(p)) {
        return ScorerContribution {
            scorer: "url_pattern".to_owned(),
            delta: 0.8,
            reason: "documentation URL path".to_owned(),
        };
    }

    // Tutorial-farm URL pattern.
    if PENALISE_PATHS.iter().any(|&p| url_lower.contains(p)) {
        return ScorerContribution {
            scorer: "url_pattern".to_owned(),
            delta: -0.5,
            reason: "tutorial-farm URL pattern".to_owned(),
        };
    }

    ScorerContribution {
        scorer: "url_pattern".to_owned(),
        delta: 0.0,
        reason: "neutral URL pattern".to_owned(),
    }
}
