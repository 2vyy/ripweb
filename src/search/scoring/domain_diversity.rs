//! Domain diversity scorer.
//!
//! Penalises results when the same domain has already appeared earlier in
//! the engine's result list. The penalty grows with each repeated occurrence.
//!
//! This scorer is stateless at the type level — the caller (`pipeline.rs`)
//! tracks occurrence counts and passes them here.

use crate::search::trace::ScorerContribution;

/// Score a result given how many previous results share the same domain.
///
/// `previous_occurrences = 0` → first time seeing this domain → no penalty.
/// `previous_occurrences = 1` → second result from domain → -0.5.
/// `previous_occurrences = n` → -(0.5 * n).
#[must_use]
pub fn score_for_occurrence(previous_occurrences: usize) -> ScorerContribution {
    if previous_occurrences == 0 {
        ScorerContribution {
            scorer: "domain_diversity".to_owned(),
            delta: 0.0,
            reason: "first result from this domain".to_owned(),
        }
    } else {
        let penalty = -0.5 * previous_occurrences as f64;
        ScorerContribution {
            scorer: "domain_diversity".to_owned(),
            delta: penalty,
            reason: format!(
                "domain already seen {previous_occurrences} time(s) in engine results"
            ),
        }
    }
}
