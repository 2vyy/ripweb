//! Web Crawler & Discovery
//!
//! Implements a BFS crawler that respects global page budgets and
//! max-depth constraints. Extracts links and schedules future fetches.

use std::collections::{HashSet, VecDeque};
use std::sync::Arc;

use url::Url;

use super::{
    cache::Cache,
    client::{RetryConfig, fetch_with_retry},
    normalize::normalize,
    politeness::DomainSemaphores,
    preflight::PreflightCheck,
};
use crate::extract::{links::extract_content_links, web::WebExtractor};

/// Configuration for a single crawl run.
pub struct CrawlerConfig {
    /// How many hops away from the seed URL to follow links.
    pub max_depth: u32,
    /// Hard cap on total pages fetched (including the seed).
    pub max_pages: usize,
    /// Boost table-heavy candidates in extractor scoring.
    pub tables_priority: bool,
}

impl Default for CrawlerConfig {
    fn default() -> Self {
        Self {
            max_depth: 1,
            max_pages: 10,
            tables_priority: false,
        }
    }
}

/// A single successfully crawled page.
pub struct CrawledPage {
    pub url: String,
    pub content: String,
}

struct FetchedPage {
    bytes: Vec<u8>,
    content_type: Option<String>,
}

/// Orchestrates fetching, caching, link extraction, and politeness.
pub struct Crawler {
    client: Arc<rquest::Client>,
    sems: DomainSemaphores,
    cache: Option<Arc<Cache>>,
    retry: RetryConfig,
    config: CrawlerConfig,
}

impl Crawler {
    pub fn new(
        client: Arc<rquest::Client>,
        sems: DomainSemaphores,
        cache: Option<Arc<Cache>>,
        retry: RetryConfig,
        config: CrawlerConfig,
    ) -> Self {
        Self {
            client,
            sems,
            cache,
            retry,
            config,
        }
    }

    /// Crawl starting from `seed`, returning pages in discovery order.
    pub async fn crawl(&self, seed: Url) -> Vec<CrawledPage> {
        let mut visited: HashSet<String> = HashSet::new();
        let mut results: Vec<CrawledPage> = Vec::new();

        // Queue entries are (url, depth).
        let mut queue: VecDeque<(Url, u32)> = VecDeque::new();

        let seed_key = normalize(seed.as_str()).unwrap_or_else(|| seed.to_string());
        visited.insert(seed_key);
        queue.push_back((seed, 0));

        while let Some((url, depth)) = queue.pop_front() {
            if results.len() >= self.config.max_pages {
                break;
            }

            let url_str = url.as_str().to_owned();
            let host = url.host_str().unwrap_or("").to_owned();

            // Try cache first.
            let page = if let Some(cache) = &self.cache {
                if let Some(cached) = cache.get(&url_str).await {
                    Some(FetchedPage {
                        bytes: cached,
                        content_type: None,
                    })
                } else {
                    self.fetch_page(&url_str, &host).await
                }
            } else {
                self.fetch_page(&url_str, &host).await
            };

            let Some(page) = page else { continue };

            // Cache the freshly fetched bytes.
            if let Some(cache) = &self.cache {
                let _ = cache.put(&url_str, &page.bytes).await;
            }

            let html = String::from_utf8_lossy(&page.bytes);
            let content = if self.config.tables_priority {
                WebExtractor::extract_with_url_options(
                    &page.bytes,
                    page.content_type.as_deref(),
                    Some(&url_str),
                    true,
                )
            } else {
                WebExtractor::extract_with_url(
                    &page.bytes,
                    page.content_type.as_deref(),
                    Some(&url_str),
                )
            }
            .unwrap_or_else(|e| {
                tracing::warn!("extraction failed for {}: {}", url_str, e);
                String::new()
            });

            results.push(CrawledPage {
                url: url_str.clone(),
                content,
            });

            // Follow links only if we haven't hit max_depth.
            if depth < self.config.max_depth {
                let links = extract_content_links(&html, &url);
                for link in links {
                    let key = normalize(link.as_str()).unwrap_or_else(|| link.to_string());
                    if visited.insert(key) {
                        queue.push_back((link, depth + 1));
                    }
                }
            }
        }

        results
    }

    async fn fetch_page(&self, url: &str, host: &str) -> Option<FetchedPage> {
        let _permit = self.sems.acquire(host).await.ok()?;

        let resp = fetch_with_retry(&self.client, url, &self.retry)
            .await
            .ok()?;

        // Pre-flight: check Content-Type and Content-Length before consuming body.
        let content_type = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_owned());
        let content_length = resp
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok());

        PreflightCheck::validate(content_type.as_deref(), content_length).ok()?;

        resp.bytes().await.ok().map(|b| FetchedPage {
            bytes: b.to_vec(),
            content_type,
        })
    }
}

/// Format the LLM source delimiter for a given URL.
pub fn llm_delimiter(url: &str) -> String {
    let url = crate::minify::strip_tracking(url);
    format!("\n\n# --- [Source: {url}] ---\n\n")
}

/// Stitch crawled pages into a single string with source delimiters.
pub fn format_output(pages: &[CrawledPage]) -> String {
    pages
        .iter()
        .map(|p| format!("{}{}", llm_delimiter(&p.url), p.content))
        .collect::<Vec<_>>()
        .join("")
}
