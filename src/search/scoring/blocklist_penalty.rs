//! Blocklist penalty scorer.
//!
//! Returns a hard -5.0 penalty for domains known to be SEO farms or
//! low-quality content aggregators. Uses the configurable blocklist from
//! `[search.blocklist]` in ripweb.toml.

use crate::config::BlocklistConfig;
use crate::search::scoring::{ScorerInput, extract_host, host_matches};
use crate::search::trace::ScorerContribution;

/// Score a result with a hard penalty if its domain is in the blocklist.
#[must_use]
pub fn score(input: &ScorerInput, blocklist: &BlocklistConfig) -> ScorerContribution {
    let host = extract_host(input.result.url.as_str());

    if blocklist.domains.iter().any(|d| host_matches(&host, d)) {
        ScorerContribution {
            scorer: "blocklist_penalty".to_owned(),
            delta: -5.0,
            reason: format!("blocklisted domain: {host}"),
        }
    } else {
        ScorerContribution {
            scorer: "blocklist_penalty".to_owned(),
            delta: 0.0,
            reason: "not blocklisted".to_owned(),
        }
    }
}
