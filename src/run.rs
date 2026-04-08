use std::sync::Arc;
use std::time::Duration;

use crate::{
    cli::{Cli, OutputMode},
    error::RipwebError,
    fetch::{
        cache::Cache,
        crawler::{format_output, Crawler, CrawledPage, CrawlerConfig},
        llms_txt::fetch_llms_txt,
        politeness::DomainSemaphores,
        RetryConfig,
    },
    minify::collapse,
    router::{route, PlatformRoute, Route},
    search::{
        duckduckgo,
        github,
        hackernews::{hn_api_url, parse_hn_json},
        reddit::{parse_reddit_json, reddit_json_url},
    },
};

pub async fn dispatch(
    cli: &Cli,
    input: &str,
    client: &Arc<rquest::Client>,
    retry: RetryConfig,
    sems: DomainSemaphores,
    cache: Option<Arc<Cache>>,
) -> Result<(String, usize), RipwebError> {
    let effective = if cli.force_url && !input.starts_with("http://") && !input.starts_with("https://") {
        format!("https://{input}")
    } else {
        input.to_owned()
    };

    let route = if cli.force_query {
        Route::Query(effective)
    } else {
        route(&effective)
    };

    match route {
        Route::Query(q) => handle_query(client, &q, cli, retry, sems, cache).await,
        Route::Url(platform) => handle_platform(client, platform, cli, retry, sems, cache).await,
    }
}

pub fn apply_output_mode(text: String, mode: OutputMode) -> String {
    match mode {
        OutputMode::Markdown => text.trim().to_owned(),
        OutputMode::Aggressive => collapse(text.trim()),
    }
}

async fn handle_platform(
    client: &Arc<rquest::Client>,
    platform: PlatformRoute,
    cli: &Cli,
    retry: RetryConfig,
    sems: DomainSemaphores,
    cache: Option<Arc<Cache>>,
) -> Result<(String, usize), RipwebError> {
    match platform {
        PlatformRoute::GitHub { owner, repo } => {
            let text = github::fetch_readme(client, &owner, &repo)
                .await
                .map_err(|e| RipwebError::Network(e.to_string()))?;
            Ok((text, 1))
        }
        PlatformRoute::Reddit { url } => {
            let json_url = reddit_json_url(&url)
                .ok_or_else(|| RipwebError::Config(format!("invalid Reddit URL: {url}")))?;
            let body = client
                .get(&json_url)
                .send()
                .await
                .map_err(|e| RipwebError::Network(e.to_string()))?
                .text()
                .await
                .map_err(|e| RipwebError::Network(e.to_string()))?;
            let content = parse_reddit_json(&body)
                .map_err(|e| RipwebError::Network(format!("Reddit JSON parse: {e}")))?;
            Ok((format_reddit(&content), 1))
        }
        PlatformRoute::HackerNews { item_id } => {
            let api = hn_api_url(&item_id);
            let body = client
                .get(api.as_str())
                .send()
                .await
                .map_err(|e| RipwebError::Network(e.to_string()))?
                .text()
                .await
                .map_err(|e| RipwebError::Network(e.to_string()))?;
            let content = parse_hn_json(&body)
                .map_err(|e| RipwebError::Network(format!("HN JSON parse: {e}")))?;
            Ok((format_hn(&content), 1))
        }
        PlatformRoute::Generic(url) => {
            if let Some(llms) = fetch_llms_txt(client, &url).await {
                return Ok((llms, 1));
            }
            run_crawler(client, url, cli, retry, sems, cache).await
        }
        _ => Err(RipwebError::Config("unhandled platform route".into())),
    }
}

async fn handle_query(
    client: &Arc<rquest::Client>,
    query: &str,
    cli: &Cli,
    retry: RetryConfig,
    sems: DomainSemaphores,
    cache: Option<Arc<Cache>>,
) -> Result<(String, usize), RipwebError> {
    let urls = duckduckgo::search(client, query, 3)
        .await
        .map_err(|e| RipwebError::Network(e.to_string()))?;

    let mut all_pages: Vec<CrawledPage> = Vec::new();

    for url_str in urls {
        if all_pages.len() >= cli.max_pages { break; }

        let url = url::Url::parse(&url_str)
            .map_err(|e| RipwebError::Config(format!("DDG returned invalid URL: {e}")))?;

        let pages = if let Some(llms) = fetch_llms_txt(client, &url).await {
            vec![CrawledPage { url: url_str, content: llms }]
        } else {
            let remaining = cli.max_pages.saturating_sub(all_pages.len());
            Crawler::new(
                Arc::clone(client),
                sems.clone(),
                cache.clone(),
                RetryConfig { max_retries: 2, base_delay: retry.base_delay },
                CrawlerConfig { max_depth: cli.max_depth, max_pages: remaining },
            )
            .crawl(url)
            .await
        };

        all_pages.extend(pages);
    }

    let count = all_pages.len();
    Ok((format_output(&all_pages), count))
}

async fn run_crawler(
    client: &Arc<rquest::Client>,
    url: url::Url,
    cli: &Cli,
    retry: RetryConfig,
    sems: DomainSemaphores,
    cache: Option<Arc<Cache>>,
) -> Result<(String, usize), RipwebError> {
    let crawler = Crawler::new(
        Arc::clone(client),
        sems,
        cache,
        RetryConfig { max_retries: 2, base_delay: retry.base_delay },
        CrawlerConfig { max_depth: cli.max_depth, max_pages: cli.max_pages },
    );
    let pages = crawler.crawl(url).await;
    let count = pages.len();
    Ok((format_output(&pages), count))
}

fn format_reddit(c: &crate::search::reddit::RedditContent) -> String {
    let mut out = format!("# {}\n\n{}", c.title, c.selftext);
    if !c.comments.is_empty() {
        out.push_str("\n\n## Comments\n\n");
        out.push_str(&c.comments.join("\n\n---\n\n"));
    }
    out
}

fn format_hn(c: &crate::search::hackernews::HnContent) -> String {
    let mut out = format!("# {}", c.title);
    if let Some(text) = &c.text {
        out.push_str(&format!("\n\n{text}"));
    }
    if !c.comments.is_empty() {
        out.push_str("\n\n## Comments\n\n");
        out.push_str(&c.comments.join("\n\n---\n\n"));
    }
    out
}
