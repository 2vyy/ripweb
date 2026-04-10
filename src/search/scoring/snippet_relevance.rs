//! Snippet relevance scorer.
//!
//! Measures what fraction of the query's meaningful terms appear in the
//! result's snippet. Terms shorter than 3 characters are ignored.
//! Returns a delta in [0.0, 1.0] proportional to coverage.

use crate::search::scoring::ScorerInput;
use crate::search::trace::ScorerContribution;

/// Score a result based on query-term coverage in its snippet.
#[must_use]
pub fn score(input: &ScorerInput) -> ScorerContribution {
    let Some(snippet) = input.result.snippet.as_deref() else {
        return ScorerContribution {
            scorer: "snippet_relevance".to_owned(),
            delta: 0.0,
            reason: "no snippet available".to_owned(),
        };
    };

    let terms: Vec<&str> = input
        .query
        .split_whitespace()
        .filter(|w| w.len() >= 3)
        .collect();

    if terms.is_empty() {
        return ScorerContribution {
            scorer: "snippet_relevance".to_owned(),
            delta: 0.0,
            reason: "no scoreable query terms".to_owned(),
        };
    }

    let snippet_lower = snippet.to_ascii_lowercase();
    let matches = terms
        .iter()
        .filter(|&&term| snippet_lower.contains(&term.to_ascii_lowercase()))
        .count();

    let coverage = matches as f64 / terms.len() as f64;
    let delta = coverage; // max 1.0

    ScorerContribution {
        scorer: "snippet_relevance".to_owned(),
        delta,
        reason: format!("{matches}/{} query terms found in snippet", terms.len()),
    }
}
