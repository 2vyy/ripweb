//! Domain trust tier scorer.
//!
//! Looks up the result's hostname in the configured trust tiers and returns
//! a score delta: +2.0 for high-trust, +0.5 for medium-trust, -0.5 for
//! low-trust, 0.0 for unknown domains.

use crate::config::TrustConfig;
use crate::search::scoring::{ScorerInput, extract_host, host_matches};
use crate::search::trace::ScorerContribution;

/// Score a result based on its domain's trust tier.
#[must_use]
pub fn score(input: &ScorerInput, trust: &TrustConfig) -> ScorerContribution {
    let host = extract_host(input.result.url.as_str());

    if trust.high.iter().any(|d| host_matches(&host, d)) {
        ScorerContribution {
            scorer: "domain_trust".to_owned(),
            delta: 2.0,
            reason: format!("high-trust domain: {host}"),
        }
    } else if trust.medium.iter().any(|d| host_matches(&host, d)) {
        ScorerContribution {
            scorer: "domain_trust".to_owned(),
            delta: 0.5,
            reason: format!("medium-trust domain: {host}"),
        }
    } else if trust.low.iter().any(|d| host_matches(&host, d)) {
        ScorerContribution {
            scorer: "domain_trust".to_owned(),
            delta: -0.5,
            reason: format!("low-trust domain: {host}"),
        }
    } else {
        ScorerContribution {
            scorer: "domain_trust".to_owned(),
            delta: 0.0,
            reason: format!("unknown domain: {host}"),
        }
    }
}
