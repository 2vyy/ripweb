//! Execution Orchestration
//!
//! The `run` module contains the top-level dispatch loop for processing
//! search queries and platform URLs. It coordinates fetching, retries,
//! caching, and final output formatting based on output mode.

use std::fmt::Write;
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
    research::{
        batch::run_batch,
        find::{MatchMode, filter_markdown_blocks, parse_terms},
        wayback::{WaybackError, resolve_snapshot},
        wikidata::{WikidataError, execute as execute_wikidata},
    },
    router::{GitHubRouteType, PlatformRoute, Route, route},
    search::{
        platforms::{
            arxiv::{arxiv_api_url, format_arxiv_content, parse_arxiv_atom},
            github,
            hackernews::{hn_api_url, parse_hn_json},
            reddit::{parse_reddit_json, reddit_json_url},
            stackoverflow::{
                SoContent, format_so_content, parse_so_answers, parse_so_question, so_answers_url,
                so_question_url,
            },
            wikipedia::{parse_wiki_summary, wiki_summary_url},
            youtube::{
                extract_caption_url, format_youtube_content, parse_caption_xml,
                parse_youtube_oembed, youtube_oembed_url,
            },
        },
        search_query,
    },
    verbosity::OutputFormat,
};

pub async fn dispatch(
    cli: &Cli,
    input: &str,
    client: &Arc<rquest::Client>,
    retry: RetryConfig,
    sems: DomainSemaphores,
    cache: Option<Arc<Cache>>,
) -> Result<(String, usize), RipwebError> {
    if let Some(query) = &cli.wikidata {
        let markdown = execute_wikidata(query, client).await.map_err(|e| match e {
            WikidataError::Query(msg) => RipwebError::Config(msg),
            WikidataError::Network(msg) | WikidataError::Parse(msg) => RipwebError::Network(msg),
        })?;
        let rendered = with_source_format(
            markdown,
            cli,
            cli.format,
            "https://query.wikidata.org/",
            "wikidata",
            Some("Wikidata SPARQL Result"),
        )?;
        return Ok((rendered, 1));
    }

    if cli.batch {
        return run_batch(cli, client, retry, sems, cache).await;
    }

    dispatch_single(cli, input, client, retry, sems, cache).await
}

pub(crate) async fn dispatch_single(
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
        Route::Query(effective.clone())
    } else {
        route(&effective)
    };

    if cli.as_of.is_some() {
        let archive_target = match &route {
            Route::Query(_) => {
                return Err(RipwebError::Config("--as-of requires URL input".into()));
            }
            Route::Url(PlatformRoute::Generic(url)) => url.clone(),
            Route::Url(_) => url::Url::parse(&effective)
                .map_err(|e| RipwebError::Config(format!("invalid URL for --as-of: {e}")))?,
        };
        return handle_generic_url(client, archive_target, cli, retry, sems, cache).await;
    }

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
            let (source_url, source_type) = match &route_type {
                GitHubRouteType::Readme => (
                    format!("https://github.com/{owner}/{repo}"),
                    "github_readme",
                ),
                GitHubRouteType::Issues => (
                    format!("https://github.com/{owner}/{repo}/issues"),
                    "github_issues",
                ),
                GitHubRouteType::Issue(id) => (
                    format!("https://github.com/{owner}/{repo}/issues/{id}"),
                    "github_issue",
                ),
            };
            let text = github::handle_github(client, &owner, &repo, &route_type, cli.verbosity)
                .await
                .map_err(|e| RipwebError::Network(e.to_string()))?;
            Ok((
                with_source_format(text, cli, cli.format, &source_url, source_type, None)?,
                1,
            ))
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
            Ok((
                with_source_format(
                    format_reddit(&content, cli.verbosity),
                    cli,
                    cli.format,
                    &url,
                    "reddit",
                    Some(&content.title),
                )?,
                1,
            ))
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
            let source_url = format!("https://news.ycombinator.com/item?id={item_id}");
            Ok((
                with_source_format(
                    format_hn(&content, cli.verbosity),
                    cli,
                    cli.format,
                    &source_url,
                    "hackernews",
                    Some(&content.title),
                )?,
                1,
            ))
        }
        PlatformRoute::Wikipedia { title } => {
            if cli.verbosity.density_tier() >= 3 {
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
            let source_url = format!("https://en.wikipedia.org/wiki/{title}");
            Ok((
                with_source_format(
                    text,
                    cli,
                    cli.format,
                    &source_url,
                    "wikipedia",
                    Some(&title),
                )?,
                1,
            ))
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
            let source_url = format!("https://stackoverflow.com/questions/{question_id}");
            Ok((
                with_source_format(
                    format_so_content(&content, cli.verbosity),
                    cli,
                    cli.format,
                    &source_url,
                    "stackoverflow",
                    Some(&content.title),
                )?,
                1,
            ))
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
            let source_url = format!("https://arxiv.org/abs/{paper_id}");
            Ok((
                with_source_format(
                    format_arxiv_content(&content, cli.verbosity),
                    cli,
                    cli.format,
                    &source_url,
                    "arxiv",
                    Some(&content.title),
                )?,
                1,
            ))
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
                with_source_format(
                    format_youtube_content(&oembed, transcript.as_deref(), cli.verbosity),
                    cli,
                    cli.format,
                    &original_url,
                    "youtube",
                    Some(&oembed.title),
                )?,
                1,
            ))
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
    if let Some(as_of) = cli.as_of.as_deref() {
        let resolved = resolve_snapshot(url.as_str(), as_of, client)
            .await
            .map_err(|e| match e {
                WaybackError::InvalidDate => {
                    RipwebError::Config("invalid --as-of date, expected YYYY-MM-DD".into())
                }
                WaybackError::NotFound => RipwebError::NoContent,
                WaybackError::Network(msg) | WaybackError::Parse(msg) => RipwebError::Network(msg),
            })?;

        let snapshot_url = url::Url::parse(&resolved.snapshot_url)
            .map_err(|e| RipwebError::Network(format!("invalid Wayback snapshot URL: {e}")))?;

        let mut archived_cli = cli.clone();
        archived_cli.as_of = None;
        let (archived_text, count) =
            handle_generic_url_inner(client, snapshot_url, &archived_cli, retry, sems, cache)
                .await?;

        let prelude = match cli.format {
            OutputFormat::Plain => format!(
                "Archived snapshot requested {requested} -> {resolved} ({snapshot})\n\n",
                requested = resolved.requested_date,
                resolved = resolved.snapshot_date,
                snapshot = resolved.snapshot_url
            ),
            _ => format!(
                "> **Archived snapshot** requested `{requested}` → `{resolved}` ([Wayback URL]({snapshot}))\n\n",
                requested = resolved.requested_date,
                resolved = resolved.snapshot_date,
                snapshot = resolved.snapshot_url
            ),
        };
        return Ok((format!("{prelude}{archived_text}"), count));
    }

    handle_generic_url_inner(client, url, cli, retry, sems, cache).await
}

async fn handle_generic_url_inner(
    client: &Arc<rquest::Client>,
    url: url::Url,
    cli: &Cli,
    retry: RetryConfig,
    sems: DomainSemaphores,
    cache: Option<Arc<Cache>>,
) -> Result<(String, usize), RipwebError> {
    if cli.allow_cloud
        && let Some(jina_text) = fetch_via_jina(client, &url).await
    {
        let rendered = format!(
            "<!-- Processed via Jina.ai Cloud Proxy -->\n\n{}",
            format_generic(&jina_text, &url, cli.verbosity)
        );
        return Ok((apply_generic_format(rendered, cli, &url)?, 1));
    }

    if let Some((markdown, _src)) = probe_markdown(client, &url).await {
        return Ok((
            apply_generic_format(format_generic(&markdown, &url, cli.verbosity), cli, &url)?,
            1,
        ));
    }

    if let Some(llms) = fetch_llms_txt(client, &url).await {
        return Ok((
            apply_generic_format(format_generic(&llms, &url, cli.verbosity), cli, &url)?,
            1,
        ));
    }

    let (text, count) = run_crawler(client, url.clone(), cli, retry, sems, cache).await?;

    if text.trim().len() < 150 && count == 1 {
        if cli.allow_cloud {
            if let Some(jina_text) = fetch_via_jina(client, &url).await {
                let rendered = format!(
                    "<!-- Processed via Jina.ai Cloud Proxy -->\n\n{}",
                    format_generic(&jina_text, &url, cli.verbosity)
                );
                return Ok((apply_generic_format(rendered, cli, &url)?, 1));
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

    Ok((
        apply_generic_format(format_generic(&text, &url, cli.verbosity), cli, &url)?,
        count,
    ))
}

pub fn format_generic(text: &str, url: &url::Url, mode: crate::verbosity::Verbosity) -> String {
    use crate::minify::strip_tracking;
    let clean_url = strip_tracking(url.as_str());
    let delimiter = format!("# --- [Source: {clean_url}] ---\n\n");
    match mode.density_tier() {
        1 => format!("{delimiter}- [Generic Page]({clean_url})"),
        2 => {
            let char_count = text.chars().count();
            let snippet: String = text.chars().take(2000).collect();
            let truncated = if char_count > 2000 {
                "... (truncated)"
            } else {
                ""
            };
            format!("{delimiter}{snippet}{truncated}")
        }
        _ => format!("{delimiter}{text}"),
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
    let scoped_query = if let Some(domain) = cli.site.as_deref() {
        format!("{query} site:{domain}")
    } else {
        query.to_owned()
    };

    let mut items = search_query(client, &scoped_query, &cli.searxng_url, cli.max_pages)
        .await
        .map_err(RipwebError::Network)?;
    let mut ranked = rank_search_results(items, query);

    if let Some(domain) = cli.site.as_deref() {
        let filtered: Vec<_> = ranked
            .into_iter()
            .filter(|item| domain_matches(item.url.as_str(), domain))
            .collect();

        if filtered.is_empty() {
            eprintln!(
                "Warning: no results matched --site {domain}; retrying without domain scope."
            );
            items = search_query(client, query, &cli.searxng_url, cli.max_pages)
                .await
                .map_err(RipwebError::Network)?;
            ranked = rank_search_results(items, query);
        } else {
            ranked = filtered;
        }
    }

    if ranked.is_empty() {
        return Err(RipwebError::NoContent);
    }

    let output = apply_find_terms(
        format_search_results(&ranked, cli.verbosity, cli.format),
        cli.find.as_deref(),
    )?;
    Ok((output, ranked.len()))
}

fn rank_search_results(
    items: Vec<crate::search::SearchResult>,
    query: &str,
) -> Vec<crate::search::SearchResult> {
    let scored = crate::search::pipeline::score_results(items, query);
    scored.into_iter().map(|s| s.result).collect()
}

fn domain_matches(url: &str, domain: &str) -> bool {
    let normalized_domain = domain.trim().trim_start_matches('.').to_ascii_lowercase();
    let normalized_domain = normalized_domain
        .strip_prefix("www.")
        .unwrap_or(&normalized_domain)
        .to_owned();
    if normalized_domain.is_empty() {
        return true;
    }

    url::Url::parse(url)
        .ok()
        .and_then(|parsed| parsed.host_str().map(|host| host.to_ascii_lowercase()))
        .map(|host| {
            let host = host.strip_prefix("www.").unwrap_or(&host).to_owned();
            host == normalized_domain || host.ends_with(&format!(".{normalized_domain}"))
        })
        .unwrap_or(false)
}

pub fn format_search_results(
    items: &[crate::search::SearchResult],
    mode: crate::verbosity::Verbosity,
    format: OutputFormat,
) -> String {
    if items.is_empty() {
        return String::new();
    }

    match format {
        OutputFormat::Md => format_search_results_md(items, mode),
        OutputFormat::Plain => markdown_to_plain(&format_search_results_md(items, mode)),
        OutputFormat::Structured => format_search_results_structured(items, mode),
    }
}

fn format_search_results_md(
    items: &[crate::search::SearchResult],
    mode: crate::verbosity::Verbosity,
) -> String {
    let mut output = String::new();

    for item in items {
        match mode.density_tier() {
            1 => {
                let _ = writeln!(output, "- [{}]({})", item.title, item.url);
            }
            2 => {
                let _ = writeln!(output, "- [{}]({})", item.title, item.url);
                if let Some(snip) = &item.snippet {
                    let cleaned = snip.replace('\n', " ");
                    let _ = writeln!(output, "  > {}", cleaned);
                }
            }
            _ => {
                let _ = writeln!(output, "### [{}]({})", item.title, item.url);
                if let Some(snip) = &item.snippet {
                    let _ = writeln!(output, "{}", snip);
                }
                let _ = writeln!(output, "---");
            }
        }
    }
    output.trim().to_owned()
}

fn format_search_results_structured(
    items: &[crate::search::SearchResult],
    mode: crate::verbosity::Verbosity,
) -> String {
    let mut out = String::new();
    for item in items {
        let body = match mode.density_tier() {
            1 => format!("- [{}]({})", item.title, item.url),
            2 => {
                let mut b = format!("- [{}]({})", item.title, item.url);
                if let Some(snip) = &item.snippet {
                    b.push_str(&format!("\n\n{}", snip.replace('\n', " ")));
                }
                b
            }
            _ => {
                let mut b = format!("### [{}]({})", item.title, item.url);
                if let Some(snip) = &item.snippet {
                    b.push_str(&format!("\n\n{}", snip));
                }
                b
            }
        };
        if !out.is_empty() {
            out.push_str("\n\n");
        }
        out.push_str(&structured_block(
            &item.url,
            "search_result",
            Some(&item.title),
            &body,
        ));
    }
    out
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
            tables_priority: cli.tables,
        },
    );
    let _pages = crawler.crawl(url).await;
    let count = _pages.len();
    Ok((format_output(&_pages), count))
}

pub fn format_reddit(
    c: &crate::search::platforms::reddit::RedditContent,
    mode: crate::verbosity::Verbosity,
) -> String {
    let mut out = String::new();
    match mode.density_tier() {
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

pub fn format_hn(
    c: &crate::search::platforms::hackernews::HnContent,
    mode: crate::verbosity::Verbosity,
) -> String {
    let mut out = String::new();
    match mode.density_tier() {
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

fn apply_generic_format(
    markdown: String,
    cli: &Cli,
    url: &url::Url,
) -> Result<String, RipwebError> {
    with_source_format(
        markdown,
        cli,
        cli.format,
        url.as_str(),
        "generic",
        Some("Generic Page"),
    )
}

fn with_source_format(
    markdown: String,
    cli: &Cli,
    format: OutputFormat,
    source_url: &str,
    source_type: &str,
    fallback_title: Option<&str>,
) -> Result<String, RipwebError> {
    let markdown = apply_find_terms(markdown, cli.find.as_deref())?;
    match format {
        OutputFormat::Md => Ok(markdown),
        OutputFormat::Plain => Ok(markdown_to_plain(&markdown)),
        OutputFormat::Structured => {
            let title = fallback_title
                .map(std::borrow::ToOwned::to_owned)
                .or_else(|| infer_title_from_markdown(&markdown))
                .unwrap_or_else(|| source_url.to_owned());
            Ok(structured_block(
                source_url,
                source_type,
                Some(&title),
                &markdown,
            ))
        }
    }
}

fn apply_find_terms(markdown: String, find_terms_raw: Option<&str>) -> Result<String, RipwebError> {
    let Some(raw) = find_terms_raw else {
        return Ok(markdown);
    };
    let terms = parse_terms(raw);
    if terms.is_empty() {
        return Ok(markdown);
    }

    let result = filter_markdown_blocks(&markdown, &terms);
    match result.match_mode {
        MatchMode::AllTerms => Ok(result.filtered_text),
        MatchMode::Partial => {
            eprintln!(
                "Warning: no block matched all --find terms ({}) ; returning strongest partial matches.",
                terms.join(", ")
            );
            Ok(result.filtered_text)
        }
        MatchMode::NoMatch => {
            eprintln!("No content matched --find terms: {}", terms.join(", "));
            Err(RipwebError::NoContent)
        }
    }
}

fn structured_block(
    source_url: &str,
    source_type: &str,
    title: Option<&str>,
    body: &str,
) -> String {
    let title = title.unwrap_or(source_url).replace('\n', " ");
    format!(
        "---\nsource: {source_url}\ntitle: {title}\ntype: {source_type}\nfetched: unknown\n---\n\n{}",
        body.trim()
    )
}

fn infer_title_from_markdown(markdown: &str) -> Option<String> {
    for line in markdown.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty()
            || trimmed.starts_with("<!--")
            || trimmed == "---"
            || trimmed.starts_with("# --- [Source:")
        {
            continue;
        }
        let without_heading = trimmed.trim_start_matches('#').trim();
        let without_quote = without_heading.trim_start_matches('>').trim();
        let without_list = without_quote
            .trim_start_matches("- ")
            .trim_start_matches("* ")
            .trim();
        if !without_list.is_empty() {
            return Some(without_list.to_owned());
        }
    }
    None
}

pub fn markdown_to_plain(markdown: &str) -> String {
    let link_re =
        regex::Regex::new(r"\[([^\]]+)\]\(([^)]+)\)").expect("markdown link regex should compile");
    let ordered_re = regex::Regex::new(r"^\s*\d+\.\s+").expect("ordered-list regex should compile");

    let mut lines: Vec<String> = Vec::new();
    let mut in_code_fence = false;

    for raw in markdown.lines() {
        let trimmed = raw.trim();

        if trimmed.starts_with("```") {
            in_code_fence = !in_code_fence;
            if !in_code_fence {
                lines.push(String::new());
            }
            continue;
        }

        if in_code_fence {
            lines.push(raw.to_owned());
            continue;
        }

        if trimmed.starts_with("<!--") || trimmed == "---" {
            continue;
        }

        if trimmed.starts_with("# --- [Source: ") && trimmed.ends_with("] ---") {
            let source = trimmed
                .trim_start_matches("# --- [Source: ")
                .trim_end_matches("] ---");
            lines.push(format!("Source: {source}"));
            lines.push(String::new());
            continue;
        }

        let mut line = raw.to_owned();
        if line.trim_start().starts_with('#') {
            line = line.trim_start_matches('#').trim_start().to_owned();
        }
        if line.trim_start().starts_with("> ") {
            line = line.trim_start().trim_start_matches("> ").to_owned();
        }
        if line.trim_start().starts_with("- ") || line.trim_start().starts_with("* ") {
            line = line.trim_start()[2..].to_owned();
        }
        line = ordered_re.replace(&line, "").into_owned();
        line = link_re.replace_all(&line, "$1 ($2)").into_owned();
        line = line.replace("**", "").replace("__", "").replace('`', "");
        lines.push(line.trim_end().to_owned());
    }

    let mut compacted: Vec<String> = Vec::new();
    let mut prev_blank = false;
    for line in lines {
        let is_blank = line.trim().is_empty();
        if is_blank && prev_blank {
            continue;
        }
        prev_blank = is_blank;
        compacted.push(line);
    }

    compacted.join("\n").trim().to_owned()
}

#[cfg(test)]
mod tests {
    use super::{apply_find_terms, domain_matches};
    use crate::error::RipwebError;
    use crate::research::batch::normalize_batch_url;

    #[test]
    fn domain_matching_accepts_subdomains() {
        assert!(domain_matches(
            "https://docs.rust-lang.org/book",
            "rust-lang.org"
        ));
        assert!(!domain_matches("https://example.com", "rust-lang.org"));
    }

    #[test]
    fn normalize_batch_url_adds_https() {
        let normalized = normalize_batch_url("example.com/path");
        assert_eq!(normalized.as_deref(), Some("https://example.com/path"));
    }

    #[test]
    fn find_filter_errors_when_no_terms_match() {
        let result = apply_find_terms("alpha beta".to_owned(), Some("gamma"));
        assert!(matches!(result, Err(RipwebError::NoContent)));
    }
}
