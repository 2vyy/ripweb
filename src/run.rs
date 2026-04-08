//! Execution Orchestration
//!
//! The `run` module contains the top-level dispatch loop for processing
//! search queries and platform URLs. It coordinates fetching, retries,
//! caching, and final output formatting based on verbosity.

use std::sync::Arc;

use crate::{
    cli::Cli,
    error::RipwebError,
    extract::jina::fetch_via_jina,
    fetch::{
        RetryConfig,
        cache::Cache,
        crawler::{Crawler, CrawlerConfig, format_output},
        llms_txt::fetch_llms_txt,
        politeness::DomainSemaphores,
        probe::probe_markdown,
    },
    router::{PlatformRoute, Route, route},
    search::{
        arxiv::{arxiv_api_url, format_arxiv_content, parse_arxiv_atom},
        ddg_instant, github,
        hackernews::{hn_api_url, parse_hn_json},
        reddit::{parse_reddit_json, reddit_json_url},
        search_query,
        stackoverflow::{
            SoContent, format_so_content, parse_so_answers, parse_so_question, so_answers_url,
            so_question_url,
        },
        tiktok::{parse_tiktok_oembed, tiktok_oembed_url},
        twitter::{parse_twitter_oembed, twitter_oembed_url},
        wikipedia::{parse_wiki_summary, wiki_summary_url},
        youtube::{
            extract_caption_url, format_youtube_content, parse_caption_xml, parse_youtube_oembed,
            youtube_oembed_url,
        },
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
    let effective =
        if cli.force_url && !input.starts_with("http://") && !input.starts_with("https://") {
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

async fn handle_platform(
    client: &Arc<rquest::Client>,
    platform: PlatformRoute,
    cli: &Cli,
    retry: RetryConfig,
    sems: DomainSemaphores,
    cache: Option<Arc<Cache>>,
) -> Result<(String, usize), RipwebError> {
    match platform {
        PlatformRoute::GitHub {
            owner,
            repo,
            route_type,
        } => {
            let text = github::handle_github(client, &owner, &repo, &route_type, cli.verbosity)
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
            Ok((format_reddit(&content, cli.verbosity), 1))
        }
        PlatformRoute::HackerNews { item_id } => {
            let api = hn_api_url(&item_id).map_err(|e| RipwebError::Network(e.to_string()))?;
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
            Ok((format_hn(&content, cli.verbosity), 1))
        }
        PlatformRoute::Wikipedia { title } => {
            if cli.verbosity >= 3 {
                let full_url = url::Url::parse(&format!("https://en.wikipedia.org/wiki/{}", title))
                    .map_err(|e| RipwebError::Network(format!("Invalid Wikipedia URL: {e}")))?;
                return handle_generic_url(client, full_url, cli, retry, sems, cache).await;
            }
            let api = wiki_summary_url(&title).map_err(|e| RipwebError::Network(e.to_string()))?;
            let body = client
                .get(api.as_str())
                .send()
                .await
                .map_err(|e| RipwebError::Network(e.to_string()))?
                .text()
                .await
                .map_err(|e| RipwebError::Network(e.to_string()))?;
            let text = parse_wiki_summary(&body, cli.verbosity)
                .map_err(|e| RipwebError::Network(format!("Wikipedia JSON parse: {e}")))?;
            Ok((text, 1))
        }
        PlatformRoute::StackOverflow { question_id } => {
            // Fetch question title and answers in parallel.
            let (q_body, a_body) = tokio::try_join!(
                async {
                    client
                        .get(
                            so_question_url(question_id)
                                .map_err(|e| RipwebError::Network(e.to_string()))?
                                .as_str(),
                        )
                        .send()
                        .await
                        .map_err(|e| RipwebError::Network(e.to_string()))?
                        .text()
                        .await
                        .map_err(|e| RipwebError::Network(e.to_string()))
                },
                async {
                    client
                        .get(
                            so_answers_url(question_id)
                                .map_err(|e| RipwebError::Network(e.to_string()))?
                                .as_str(),
                        )
                        .send()
                        .await
                        .map_err(|e| RipwebError::Network(e.to_string()))?
                        .text()
                        .await
                        .map_err(|e| RipwebError::Network(e.to_string()))
                }
            )?;
            let title = parse_so_question(&q_body)
                .map_err(|e| RipwebError::Network(format!("SO question parse: {e}")))?;
            let answers = parse_so_answers(&a_body)
                .map_err(|e| RipwebError::Network(format!("SO answers parse: {e}")))?;
            let content = SoContent { title, answers };
            Ok((format_so_content(&content, cli.verbosity), 1))
        }
        PlatformRoute::ArXiv { paper_id } => {
            let api = arxiv_api_url(&paper_id).map_err(|e| RipwebError::Network(e.to_string()))?;
            let body = client
                .get(api.as_str())
                .send()
                .await
                .map_err(|e| RipwebError::Network(e.to_string()))?
                .text()
                .await
                .map_err(|e| RipwebError::Network(e.to_string()))?;
            let content = parse_arxiv_atom(&body)
                .ok_or_else(|| RipwebError::Network("ArXiv returned no results".into()))?;
            Ok((format_arxiv_content(&content, cli.verbosity), 1))
        }
        PlatformRoute::YouTube {
            video_id: _,
            original_url,
        } => {
            // Stage 1: oEmbed for metadata (always)
            let oembed_url = youtube_oembed_url(&original_url);
            let oembed_body = client
                .get(&oembed_url)
                .send()
                .await
                .map_err(|e| RipwebError::Network(e.to_string()))?
                .text()
                .await
                .map_err(|e| RipwebError::Network(e.to_string()))?;
            let oembed = parse_youtube_oembed(&oembed_body)
                .map_err(|e| RipwebError::Network(format!("YouTube oEmbed parse: {e}")))?;

            // Stage 2: timedtext caption transcript (best-effort)
            let transcript = async {
                let page = client
                    .get(&original_url)
                    .send()
                    .await
                    .ok()?
                    .text()
                    .await
                    .ok()?;
                let caption_url = extract_caption_url(&page)?;
                let xml = client
                    .get(&caption_url)
                    .send()
                    .await
                    .ok()?
                    .text()
                    .await
                    .ok()?;
                Some(parse_caption_xml(&xml))
            }
            .await;

            Ok((
                format_youtube_content(&oembed, transcript.as_deref(), cli.verbosity),
                1,
            ))
        }
        PlatformRoute::Twitter { tweet_url } => {
            let oembed_url = twitter_oembed_url(&tweet_url);
            let body = client
                .get(&oembed_url)
                .send()
                .await
                .map_err(|e| RipwebError::Network(e.to_string()))?
                .text()
                .await
                .map_err(|e| RipwebError::Network(e.to_string()))?;
            let text = parse_twitter_oembed(&body)
                .map_err(|e| RipwebError::Network(format!("Twitter oEmbed parse: {e}")))?;
            Ok((text, 1))
        }
        PlatformRoute::TikTok { video_url } => {
            let oembed_url = tiktok_oembed_url(&video_url);
            let body = client
                .get(&oembed_url)
                .send()
                .await
                .map_err(|e| RipwebError::Network(e.to_string()))?
                .text()
                .await
                .map_err(|e| RipwebError::Network(e.to_string()))?;
            let text = parse_tiktok_oembed(&body, cli.verbosity)
                .map_err(|e| RipwebError::Network(format!("TikTok oEmbed parse: {e}")))?;
            Ok((text, 1))
        }
        PlatformRoute::Generic(url) => {
            handle_generic_url(client, url, cli, retry, sems, cache).await
        }
    }
}

async fn handle_generic_url(
    client: &Arc<rquest::Client>,
    url: url::Url,
    cli: &Cli,
    retry: RetryConfig,
    sems: DomainSemaphores,
    cache: Option<Arc<Cache>>,
) -> Result<(String, usize), RipwebError> {
    if cli.allow_cloud
        && cli.verbosity >= 3
        && let Some(jina_text) = fetch_via_jina(client, &url).await
    {
        return Ok((
            format!(
                "<!-- Processed via Jina.ai Cloud Proxy -->\n\n{}",
                jina_text
            ),
            1,
        ));
    }

    if let Some((markdown, _src)) = probe_markdown(client, &url).await {
        return Ok((format_generic(&markdown, &url, cli.verbosity), 1));
    }

    if let Some(llms) = fetch_llms_txt(client, &url).await {
        return Ok((format_generic(&llms, &url, cli.verbosity), 1));
    }

    let (text, count) = run_crawler(client, url.clone(), cli, retry, sems, cache).await?;

    if text.trim().len() < 150 && count == 1 {
        if cli.allow_cloud {
            if let Some(jina_text) = fetch_via_jina(client, &url).await {
                return Ok((
                    format!(
                        "<!-- Processed via Jina.ai Cloud Proxy -->\n\n{}",
                        format_generic(&jina_text, &url, cli.verbosity)
                    ),
                    1,
                ));
            }
        } else {
            eprintln!(
                "Warning: Extracted content is extremely sparse. This site may require JavaScript."
            );
            eprintln!(
                "Hint: Use the --allow-cloud flag to bypass this using a cloud JS-rendering proxy."
            );
        }
    }

    Ok((format_generic(&text, &url, cli.verbosity), count))
}

pub fn format_generic(text: &str, url: &url::Url, verbosity: u8) -> String {
    match verbosity {
        1 => {
            format!("- [Generic Page]({})", url)
        }
        2 => {
            let mut s = format!("# Page: {}\n\n", url);
            let snippet: String = text.chars().take(2000).collect();
            s.push_str(&snippet);
            if text.len() > 2000 {
                s.push_str("... (truncated)");
            }
            s
        }
        _ => text.to_owned(),
    }
}

async fn handle_query(
    client: &Arc<rquest::Client>,
    query: &str,
    cli: &Cli,
    _retry: RetryConfig,
    _sems: DomainSemaphores,
    _cache: Option<Arc<Cache>>,
) -> Result<(String, usize), RipwebError> {
    // Run DDG Instant Answer and the SERP search concurrently
    let (instant_opt, urls_result) = tokio::join!(
        ddg_instant::fetch_instant(client, query),
        search_query(client, query, cli.engine, &cli.searxng_url, cli.max_pages),
    );

    let items = urls_result.map_err(RipwebError::Network)?;

    if items.is_empty() {
        return Err(RipwebError::NoContent);
    }

    let output = format_search_results(&items, instant_opt.as_deref(), cli.verbosity, cli.engine);
    Ok((output, items.len()))
}

pub fn format_search_results(
    items: &[crate::search::SearchResult],
    instant_opt: Option<&str>,
    verbosity: u8,
    engine: crate::cli::SearchEngine,
) -> String {
    let mut output = String::new();
    let engine_name = match engine {
        crate::cli::SearchEngine::Ddg => "DuckDuckGo",
        crate::cli::SearchEngine::Searxng => "SearXNG",
        crate::cli::SearchEngine::Marginalia => "Marginalia",
    };

    for item in items {
        match verbosity {
            1 => {
                output.push_str(&format!("- [{}]({})\n", item.title, item.url));
            }
            2 => {
                output.push_str(&format!("- [{}]({})\n", item.title, item.url));
                if let Some(snip) = &item.snippet {
                    let cleaned = snip.replace('\n', " ");
                    output.push_str(&format!("  > {}\n", cleaned));
                }
            }
            _ => {
                output.push_str(&format!("### [{}]({})\n", item.title, item.url));
                output.push_str(&format!("**Source:** {}\n", engine_name));
                if let Some(snip) = &item.snippet {
                    output.push_str(&format!("{}\n", snip));
                }
                output.push_str("---\n");
            }
        }
    }

    if let Some(instant) = instant_opt {
        output = format!("> {instant}\n\n---\n\n{output}");
    }
    output.trim().to_owned()
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
        RetryConfig {
            max_retries: 2,
            base_delay: retry.base_delay,
        },
        CrawlerConfig {
            max_depth: cli.max_depth,
            max_pages: cli.max_pages,
        },
    );
    let _pages = crawler.crawl(url).await;
    let count = _pages.len();
    Ok((format_output(&_pages), count))
}

pub fn format_reddit(c: &crate::search::reddit::RedditContent, verbosity: u8) -> String {
    let mut out = String::new();
    match verbosity {
        1 => {
            out.push_str(&format!("- [{}]", c.title));
        }
        2 => {
            out.push_str(&format!("# {}\n\n{}", c.title, c.selftext));
            if !c.comments.is_empty() {
                out.push_str("\n\n## Comments\n\n");
                out.push_str(
                    &c.comments
                        .iter()
                        .take(2)
                        .cloned()
                        .collect::<Vec<_>>()
                        .join("\n\n---\n\n"),
                );
            }
        }
        _ => {
            out.push_str(&format!("# {}\n\n{}", c.title, c.selftext));
            if !c.comments.is_empty() {
                out.push_str("\n\n## Comments\n\n");
                out.push_str(&c.comments.join("\n\n---\n\n"));
            }
        }
    }
    out
}

pub fn format_hn(c: &crate::search::hackernews::HnContent, verbosity: u8) -> String {
    let mut out = String::new();
    match verbosity {
        1 => {
            out.push_str(&format!("- [{}]", c.title));
        }
        2 => {
            out.push_str(&format!("# {}", c.title));
            if let Some(text) = &c.text {
                out.push_str(&format!("\n\n{text}"));
            }
            if !c.comments.is_empty() {
                out.push_str("\n\n## Comments\n\n");
                out.push_str(
                    &c.comments
                        .iter()
                        .take(5)
                        .cloned()
                        .collect::<Vec<_>>()
                        .join("\n\n---\n\n"),
                );
            }
        }
        _ => {
            out.push_str(&format!("# {}", c.title));
            if let Some(text) = &c.text {
                out.push_str(&format!("\n\n{text}"));
            }
            if !c.comments.is_empty() {
                out.push_str("\n\n## Comments\n\n");
                out.push_str(&c.comments.join("\n\n---\n\n"));
            }
        }
    }
    out
}
