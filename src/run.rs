use std::sync::Arc;

use crate::{
    cli::{Cli, OutputMode},
    error::RipwebError,
    extract::jina::fetch_via_jina,
    fetch::{
        cache::Cache,
        crawler::{format_output, Crawler, CrawledPage, CrawlerConfig},
        llms_txt::fetch_llms_txt,
        politeness::DomainSemaphores,
        probe::probe_markdown,
        RetryConfig,
    },
    minify::collapse,
    router::{route, PlatformRoute, Route},
    search::{
        arxiv::{arxiv_api_url, format_arxiv_content, parse_arxiv_atom},
        duckduckgo,
        github,
        hackernews::{hn_api_url, parse_hn_json},
        reddit::{parse_reddit_json, reddit_json_url},
        stackoverflow::{
            format_so_content, parse_so_answers, parse_so_question,
            so_answers_url, so_question_url, SoContent,
        },
        wikipedia::{parse_wiki_summary, wiki_summary_url},
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
        PlatformRoute::Wikipedia { title } => {
            let api = wiki_summary_url(&title);
            let body = client
                .get(api.as_str())
                .send()
                .await
                .map_err(|e| RipwebError::Network(e.to_string()))?
                .text()
                .await
                .map_err(|e| RipwebError::Network(e.to_string()))?;
            let text = parse_wiki_summary(&body)
                .map_err(|e| RipwebError::Network(format!("Wikipedia JSON parse: {e}")))?;
            Ok((text, 1))
        }
        PlatformRoute::StackOverflow { question_id } => {
            // Fetch question title and answers in parallel.
            let (q_body, a_body) = tokio::try_join!(
                async {
                    client
                        .get(so_question_url(question_id).as_str())
                        .send()
                        .await
                        .map_err(|e| RipwebError::Network(e.to_string()))?
                        .text()
                        .await
                        .map_err(|e| RipwebError::Network(e.to_string()))
                },
                async {
                    client
                        .get(so_answers_url(question_id).as_str())
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
            Ok((format_so_content(&content), 1))
        }
        PlatformRoute::ArXiv { paper_id } => {
            let api = arxiv_api_url(&paper_id);
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
            Ok((format_arxiv_content(&content), 1))
        }
        PlatformRoute::Generic(url) => {
            // 1. Try .md / index.html.md probe first (fastest, highest quality)
            if let Some((markdown, _src)) = probe_markdown(client, &url).await {
                return Ok((markdown, 1));
            }
            // 2. Try llms.txt (site-wide LLM index)
            if let Some(llms) = fetch_llms_txt(client, &url).await {
                return Ok((llms, 1));
            }
            // 3. Crawl and extract HTML
            let (text, count) = run_crawler(client, url.clone(), cli, retry, sems, cache).await?;
            // 4. If content is thin, try Jina as a last resort
            let word_count = text.split_whitespace().count();
            if word_count < 150 {
                if let Some(jina_text) = fetch_via_jina(client, &url).await {
                    return Ok((jina_text, count));
                }
            }
            Ok((text, count))
        }
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
    let _pages = crawler.crawl(url).await;
    let count = _pages.len();
    Ok((format_output(&_pages), count))
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
