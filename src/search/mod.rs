//! Platform Search Dispatcher
//!
//! Provides the unified `search_query` interface for dispatching search
//! requests to various backends (DuckDuckGo, SearXNG, Marginalia).
//! Manages engine selection and structural result normalization.

pub mod arxiv;
pub mod ddg_instant;
pub mod duckduckgo;
pub mod eval_types;
pub mod fusion;
pub mod github;
pub mod hackernews;
pub mod marginalia;
pub mod pipeline;
pub mod reddit;
pub mod scoring;
pub mod searxng;
pub mod stackoverflow;
pub mod tiktok;
pub mod trace;
pub mod twitter;
pub mod wikipedia;
pub mod youtube;

use crate::cli::SearchEngine;

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub url: String,
    pub title: String,
    pub snippet: Option<String>,
}

/// Dispatch a text query to the configured search engine.
/// Returns a list of search results, or an error string.
pub async fn search_query(
    client: &rquest::Client,
    query: &str,
    engine: SearchEngine,
    searxng_url: &str,
    limit: usize,
) -> Result<Vec<SearchResult>, String> {
    match engine {
        SearchEngine::Ddg => duckduckgo::search(client, query, limit)
            .await
            .map_err(|e| e.to_string()),
        SearchEngine::Searxng => {
            if searxng_url.is_empty() {
                return Err("--engine=searxng requires --searxng-url to be set. \
                     Example: --searxng-url https://searx.be"
                    .into());
            }
            searxng::search(client, searxng_url, query, limit).await
        }
        SearchEngine::Marginalia => marginalia::search(client, query, limit).await,
        SearchEngine::FanOut => {
            Err("FanOut engine must be dispatched via fan_out_search, not search_query".into())
        }
    }
}

/// Fan-out search: query DuckDuckGo and Marginalia in parallel, then fuse
/// with Reciprocal Rank Fusion. The limit is applied per-engine before fusion.
/// Marginalia errors are non-fatal — degrades gracefully to DDG-only results.
pub async fn fan_out_search(
    client: &rquest::Client,
    query: &str,
    limit: usize,
) -> Result<Vec<SearchResult>, String> {
    let (ddg_res, marginalia_res) = tokio::join!(
        duckduckgo::search(client, query, limit),
        marginalia::search(client, query, limit),
    );

    let ddg = ddg_res.map_err(|e| e.to_string())?;
    // Marginalia errors are non-fatal — degrade gracefully to DDG only.
    let marginalia = marginalia_res.unwrap_or_default();

    let fused = fusion::rrf_fuse(&[("ddg", ddg), ("marginalia", marginalia)]);
    Ok(fused)
}
