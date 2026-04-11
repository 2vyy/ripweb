//! Search scoring pipeline.
//!
//! `score_results` is the single entry point: it takes raw engine results,
//! applies all six metadata scorers, and returns results sorted by composite
//! score descending. No network calls; pure CPU work.

use std::collections::HashMap;

use crate::config::get_config;
use crate::search::SearchResult;
use crate::search::scoring::{
    ScoredResult, ScorerInput, ScoringWeights, blocklist_penalty, domain_diversity, domain_trust,
    extract_host, project_match, snippet_relevance, url_pattern,
};
use crate::search::trace::ScorerContribution;

/// Score and rank a list of raw search results.
///
/// Results are returned sorted by composite score (highest first).
/// Each `ScoredResult` carries an audit trail of all scorer contributions.
#[must_use]
pub fn score_results(results: Vec<SearchResult>, query: &str) -> Vec<ScoredResult> {
    if results.is_empty() {
        return Vec::new();
    }

    let cfg = get_config();
    score_results_with_weights(
        results,
        query,
        &cfg.search.trust,
        &cfg.search.blocklist,
        &cfg.search.scoring,
    )
}

/// Score and rank results using explicitly provided scorer weights.
#[must_use]
pub fn score_results_with_weights(
    results: Vec<SearchResult>,
    query: &str,
    trust: &crate::config::TrustConfig,
    blocklist: &crate::config::BlocklistConfig,
    weights: &ScoringWeights,
) -> Vec<ScoredResult> {
    if results.is_empty() {
        return Vec::new();
    }

    // Track how many times each domain has appeared (engine rank order).
    let mut domain_counts: HashMap<String, usize> = HashMap::new();

    let mut scored: Vec<ScoredResult> = results
        .into_iter()
        .enumerate()
        .map(|(rank, result)| {
            // Compute host before borrowing result in ScorerInput.
            let host = extract_host(&result.url);

            let input = ScorerInput {
                result: &result,
                query,
                engine_rank: rank,
            };

            // Stateless per-result scorers.
            let trust_c = apply_weight(domain_trust::score(&input, trust), weights.domain_trust);
            let pattern_c = apply_weight(url_pattern::score(&input), weights.url_pattern);
            let project_c = apply_weight(project_match::score(&input), weights.project_match);
            let blocklist_c = apply_weight(
                blocklist_penalty::score(&input, blocklist),
                weights.blocklist_penalty,
            );
            let snippet_c =
                apply_weight(snippet_relevance::score(&input), weights.snippet_relevance);

            // Stateful diversity scorer — depends on engine-rank position.
            // input borrow ends here; result is moved below.
            let prev_count = *domain_counts.get(&host).unwrap_or(&0);
            domain_counts.insert(host, prev_count + 1);
            let diversity_c = apply_weight(
                domain_diversity::score_for_occurrence(prev_count),
                weights.domain_diversity,
            );

            let contributions = vec![
                trust_c,
                pattern_c,
                project_c,
                blocklist_c,
                snippet_c,
                diversity_c,
            ];

            let score: f64 = contributions.iter().map(|c| c.delta).sum();

            ScoredResult {
                result,
                score,
                contributions,
            }
        })
        .collect();

    // Sort by composite score descending; stable sort preserves engine order
    // for ties (the engine's ranking is the tiebreaker).
    scored.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    scored
}

/// Scale a scorer's delta by a weight, preserving the reason string.
fn apply_weight(mut c: ScorerContribution, weight: f64) -> ScorerContribution {
    c.delta *= weight;
    c
}
