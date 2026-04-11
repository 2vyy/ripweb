//! Platform Search Dispatcher
//!
//! Provides the unified `search_query` interface for dispatching search
//! requests to available backends (SearXNG, DuckDuckGo Lite, Marginalia),
//! then fusing them with RRF.

pub mod duckduckgo;
pub mod eval_types;
pub mod fusion;
pub mod marginalia;
pub mod pipeline;
pub mod platforms;
pub mod scoring;
pub mod searxng;
pub mod trace;

use crate::config::get_config;

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub url: String,
    pub title: String,
    pub snippet: Option<String>,
}

/// Run multi-engine search and return fused results.
pub async fn search_query(
    client: &rquest::Client,
    query: &str,
    searxng_url: &str,
    limit: usize,
) -> Result<Vec<SearchResult>, String> {
    if query.trim().is_empty() {
        return Err("query is empty".into());
    }
    let rrf_k = get_config().search.scoring.rrf_k;

    let (searx_res, ddg_res, marginalia_res) = tokio::join!(
        async {
            if searxng_url.is_empty() {
                Err("searxng disabled (empty --searxng-url)".to_string())
            } else {
                searxng::search(client, searxng_url, query, limit).await
            }
        },
        async {
            duckduckgo::search(client, query, limit)
                .await
                .map_err(|e| e.to_string())
        },
        async { marginalia::search(client, query, limit).await },
    );

    let mut engine_results: Vec<(&str, Vec<SearchResult>)> = Vec::new();
    let mut errors: Vec<String> = Vec::new();

    match searx_res {
        Ok(results) if !results.is_empty() => engine_results.push(("searxng", results)),
        Ok(_) => errors.push("searxng: no results".into()),
        Err(e) => errors.push(format!("searxng: {e}")),
    }
    match ddg_res {
        Ok(results) if !results.is_empty() => engine_results.push(("ddg", results)),
        Ok(_) => errors.push("ddg: no results".into()),
        Err(e) => errors.push(format!("ddg: {e}")),
    }
    match marginalia_res {
        Ok(results) if !results.is_empty() => engine_results.push(("marginalia", results)),
        Ok(_) => errors.push("marginalia: no results".into()),
        Err(e) => errors.push(format!("marginalia: {e}")),
    }

    if engine_results.is_empty() {
        return Err(format!(
            "all search engines failed or returned no results ({})",
            errors.join("; ")
        ));
    }

    Ok(fusion::rrf_fuse_with_k(&engine_results, rrf_k))
}

/// Fan-out search: query DuckDuckGo and Marginalia in parallel, then fuse
/// with Reciprocal Rank Fusion. The limit is applied per-engine before fusion.
/// Marginalia errors are non-fatal — degrades gracefully to DDG-only results.
pub async fn fan_out_search(
    client: &rquest::Client,
    query: &str,
    limit: usize,
) -> Result<Vec<SearchResult>, String> {
    let rrf_k = get_config().search.scoring.rrf_k;
    let (ddg_res, marginalia_res) = tokio::join!(
        duckduckgo::search(client, query, limit),
        marginalia::search(client, query, limit),
    );

    let ddg = ddg_res.map_err(|e| e.to_string())?;
    // Marginalia errors are non-fatal — degrade gracefully to DDG only.
    let marginalia = marginalia_res.unwrap_or_default();

    let fused = fusion::rrf_fuse_with_k(&[("ddg", ddg), ("marginalia", marginalia)], rrf_k);
    Ok(fused)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_query_rejects_empty_query() {
        let rt = tokio::runtime::Runtime::new().expect("runtime");
        let client = rquest::Client::builder().build().expect("client");
        let err = rt
            .block_on(search_query(&client, "   ", "http://localhost:8080", 5))
            .expect_err("expected empty-query error");
        assert!(err.contains("query is empty"));
    }
}
