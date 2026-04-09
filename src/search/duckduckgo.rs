//! DuckDuckGo HTML Search
//!
//! Scrapes the non-JS `html.duckduckgo.com` endpoint to retrieve
//! Search Engine Result Page (SERP) data including titles, links,
//! and snippets.

use url::Url;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum DdgError {
    #[error("network error: {0}")]
    Network(#[from] rquest::Error),
    #[error("DuckDuckGo returned no results")]
    NoResults,
    #[error("URL parse error: {0}")]
    Parse(#[from] url::ParseError),
}

/// Build the DDG HTML search endpoint URL for `query`.
pub fn ddg_search_url(query: &str) -> Result<Url, url::ParseError> {
    let mut url = Url::parse("https://html.duckduckgo.com/html/")?;
    url.query_pairs_mut().append_pair("q", query);
    Ok(url)
}

/// Extract result URLs from a DDG HTML response body.
///
/// DDG wraps real URLs in `/l/?uddg=<percent-encoded-url>` redirect hrefs.
/// This function decodes those and returns the actual target URLs.
/// Returns at most `limit` URLs.
pub fn parse_ddg_html(html: &str, limit: usize) -> Vec<super::SearchResult> {
    let Ok(dom) = tl::parse(html, tl::ParserOptions::default()) else {
        return Vec::new();
    };
    let parser = dom.parser();

    let Some(bodies) = dom.query_selector(".result__body") else {
        return Vec::new();
    };

    let mut results = Vec::new();

    for handle in bodies {
        if results.len() >= limit {
            break;
        }

        let Some(node) = handle.get(parser) else { continue; };
        let Some(tag) = node.as_tag() else { continue; };

        // Parse inner HTML to isolate queries to this specific result
        let inner = tag.inner_html(parser);
        let Ok(sub_dom) = tl::parse(&inner, tl::ParserOptions::default()) else { continue; };
        let sub_parser = sub_dom.parser();

        // Extract Title & URL
        let Some(a_node) = sub_dom
            .query_selector(".result__a")
            .and_then(|mut q| q.next())
            .and_then(|h| h.get(sub_parser))
        else {
            continue;
        };
        let Some(a_tag) = a_node.as_tag() else { continue; };
        let Some(href) = a_tag.attributes().get("href").flatten() else { continue; };
        let href_utf8 = href.as_utf8_str();
        let title = a_tag.inner_text(sub_parser).into_owned();

        // Extract Snippet
        let snippet = sub_dom
            .query_selector(".result__snippet")
            .and_then(|mut q| q.next())
            .and_then(|h| h.get(sub_parser))
            .and_then(|n| n.as_tag())
            .map(|t| t.inner_text(sub_parser).into_owned());

        if let Some(decoded) = decode_ddg_href(href_utf8.as_ref()) {
            results.push(super::SearchResult {
                url: decoded,
                title,
                snippet,
            });
        }
    }

    results
}

/// Decode a DDG href into the actual destination URL.
///
/// DDG hrefs are `/l/?uddg=<percent-encoded-url>&rut=...`.
/// We parse the href, extract the `uddg` query parameter, and return it.
/// If the href is already a direct http/https URL, return it as-is.
fn decode_ddg_href(href: &str) -> Option<String> {
    // Direct absolute URL — already decoded.
    if href.starts_with("http://") || href.starts_with("https://") {
        return Some(href.to_owned());
    }

    // DDG redirect: /l/?uddg=<encoded>&...
    // We need a base to make `join` work.
    let base = Url::parse("https://duckduckgo.com").ok()?;
    let full = base.join(href).ok()?;
    full.query_pairs()
        .find(|(k, _)| k == "uddg")
        .map(|(_, v)| v.into_owned())
}

/// Fetch the top `limit` result URLs for `query` from DuckDuckGo.
pub async fn search(
    client: &rquest::Client,
    query: &str,
    limit: usize,
) -> Result<Vec<super::SearchResult>, DdgError> {
    let url = ddg_search_url(query)?;
    let resp = client
        .get(url.as_str())
        .send()
        .await
        .map_err(DdgError::Network)?;

    let html = resp.text().await.map_err(DdgError::Network)?;
    let urls = parse_ddg_html(&html, limit);

    if urls.is_empty() {
        Err(DdgError::NoResults)
    } else {
        Ok(urls)
    }
}
