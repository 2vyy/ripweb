//! Jina Reader Proxy
//!
//! Formats URLs for the `r.jina.ai` markdown-rehydration service.
//! Used as a high-fidelity fallback for complex or JS-heavy pages.

use url::Url;

/// Build the `r.jina.ai` proxy URL for any target URL.
///
/// Jina Reader converts any page to LLM-friendly Markdown, handles JS rendering,
/// and strips nav/ad noise. No API key required for basic usage.
pub fn jina_url(target: &Url) -> Url {
    Url::parse(&format!("https://r.jina.ai/{target}"))
        .expect("jina URL construction is always valid")
}

/// Fetch a URL via the Jina.ai Reader proxy, returning clean Markdown.
///
/// Returns `None` on any HTTP error or empty response. Used as a high-quality
/// fallback when local extraction produces insufficient content.
pub async fn fetch_via_jina(client: &rquest::Client, target: &Url) -> Option<String> {
    let proxy_url = jina_url(target);
    let resp = client
        .get(proxy_url.as_str())
        .header("Accept", "text/markdown,text/plain")
        .send()
        .await
        .ok()?;

    if !resp.status().is_success() {
        return None;
    }
    let text = resp.text().await.ok()?;
    if text.trim().is_empty() {
        None
    } else {
        Some(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jina_url_prepends_proxy() {
        let target =
            Url::parse("https://en.wikipedia.org/wiki/Rust_(programming_language)").unwrap();
        let jina = jina_url(&target);
        assert_eq!(
            jina.as_str(),
            "https://r.jina.ai/https://en.wikipedia.org/wiki/Rust_(programming_language)"
        );
    }

    #[test]
    fn jina_url_works_with_query_params() {
        let target = Url::parse("https://docs.rs/tokio/latest/tokio/struct.Runtime.html").unwrap();
        let jina = jina_url(&target);
        assert!(
            jina.as_str()
                .starts_with("https://r.jina.ai/https://docs.rs/")
        );
    }
}
