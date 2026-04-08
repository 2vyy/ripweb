use url::Url;

#[derive(Debug)]
#[non_exhaustive]
pub enum DdgError {
    Network(rquest::Error),
    NoResults,
}

impl std::fmt::Display for DdgError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Network(e) => write!(f, "network error: {e}"),
            Self::NoResults => write!(f, "DuckDuckGo returned no results"),
        }
    }
}

/// Build the DDG HTML search endpoint URL for `query`.
pub fn ddg_search_url(query: &str) -> Url {
    let mut url = Url::parse("https://html.duckduckgo.com/html/")
        .expect("base URL is always valid");
    url.query_pairs_mut().append_pair("q", query);
    url
}

/// Extract result URLs from a DDG HTML response body.
///
/// DDG wraps real URLs in `/l/?uddg=<percent-encoded-url>` redirect hrefs.
/// This function decodes those and returns the actual target URLs.
/// Returns at most `limit` URLs.
pub fn parse_ddg_html(html: &str, limit: usize) -> Vec<String> {
    let Ok(dom) = tl::parse(html, tl::ParserOptions::default()) else {
        return Vec::new();
    };
    let parser = dom.parser();

    let Some(anchors) = dom.query_selector("a.result__a") else {
        return Vec::new();
    };

    let mut results = Vec::new();

    for handle in anchors {
        if results.len() >= limit {
            break;
        }

        let Some(node) = handle.get(parser) else { continue };
        let Some(tag) = node.as_tag() else { continue };
        let Some(href_val) = tag.attributes().get("href").flatten() else { continue };

        let href = href_val.as_utf8_str();

        if let Some(decoded) = decode_ddg_href(href.as_ref()) {
            results.push(decoded);
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
) -> Result<Vec<String>, DdgError> {
    let url = ddg_search_url(query);
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
