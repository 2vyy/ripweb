//! Reciprocal Rank Fusion for multi-engine result merging.
//!
//! `rrf_fuse` takes a slice of `(engine_name, results)` pairs and returns a
//! single deduplicated, RRF-scored list sorted by score descending.

use std::collections::HashMap;

use crate::search::SearchResult;

pub const DEFAULT_RRF_K: f64 = 60.0;

/// Normalise a URL string to a canonical dedup key.
/// Lowercases scheme+host (url::Url does this), strips trailing slash from path.
fn normalise(url: &str) -> String {
    match url::Url::parse(url) {
        Ok(mut u) => {
            let path = u.path().trim_end_matches('/').to_owned();
            u.set_path(&path);
            u.to_string()
        }
        Err(_) => url.to_ascii_lowercase(),
    }
}

/// Merge multiple ranked result lists using Reciprocal Rank Fusion.
///
/// `engine_lists`: slice of `(engine_name, Vec<SearchResult>)` pairs.
/// Returns a deduplicated list sorted by RRF score descending.
/// The `SearchResult` carried forward for each URL is taken from the first
/// engine that returned it.
pub fn rrf_fuse(engine_lists: &[(&str, Vec<SearchResult>)]) -> Vec<SearchResult> {
    rrf_fuse_with_k(engine_lists, DEFAULT_RRF_K)
}

/// Merge multiple ranked result lists with a caller-specified RRF `k`.
pub fn rrf_fuse_with_k(engine_lists: &[(&str, Vec<SearchResult>)], k: f64) -> Vec<SearchResult> {
    let k = if k <= 0.0 { DEFAULT_RRF_K } else { k };
    let mut scores: HashMap<String, (f64, SearchResult)> = HashMap::new();

    for (_engine, results) in engine_lists {
        for (rank, result) in results.iter().enumerate() {
            let key = normalise(&result.url);
            let contribution = 1.0 / (k + rank as f64 + 1.0);
            scores
                .entry(key)
                .and_modify(|(score, _)| *score += contribution)
                .or_insert_with(|| (contribution, result.clone()));
        }
    }

    let mut fused: Vec<(f64, SearchResult)> = scores.into_values().collect();
    fused.sort_by(|(sa, ra), (sb, rb)| {
        sb.partial_cmp(sa)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| ra.url.cmp(&rb.url))
    });

    fused.into_iter().map(|(_, r)| r).collect()
}
