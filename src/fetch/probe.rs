//! URL Probing
//!
//! Implements speculative HEAD/GET requests to find hidden `.md`
//! or `llms.txt` files before resorting to full HTML scraping.

use url::Url;

/// Probe a URL for the highest-quality content representation,
/// trying each source in priority order before falling back to HTML.
///
/// Priority:
/// 1. `<url>.md` — native Markdown (nbdev, Mintlify)
/// 2. `<origin>/llms.txt` — site-level LLM index
/// 3. `<origin>/llms-full.txt` — fully expanded LLM index
/// 4. `<url>/index.html.md` — alternative Mintlify pattern
///
/// Returns `Some((content, source_hint))` on first hit, `None` if all miss.
pub async fn probe_markdown(client: &rquest::Client, url: &Url) -> Option<(String, ProbeSource)> {
    // 1. Try `.md` suffix on the exact page URL
    if let Some(md_url) = with_md_suffix(url)
        && let Some(text) = try_get_text(client, &md_url).await
    {
        return Some((text, ProbeSource::MdSuffix));
    }

    // 2. Try `index.html.md` for directory-style URLs
    if url.path().ends_with('/') || !url.path().contains('.') {
        let candidate = url.join("index.html.md").ok()?;
        if let Some(text) = try_get_text(client, &candidate).await {
            return Some((text, ProbeSource::MdSuffix));
        }
    }

    None
}

/// Probe the site origin for an `llms.txt` or `llms-full.txt` index.
///
/// This is separate from `probe_markdown` because it applies site-wide,
/// not to specific pages. Returns the first hit.
pub async fn probe_llms_index(
    client: &rquest::Client,
    origin: &Url,
) -> Option<(String, ProbeSource)> {
    let candidates = ["/llms-full.txt", "/llms.txt", "/.well-known/llms.txt"];
    for path in candidates {
        let candidate = origin.join(path).ok()?;
        if let Some(text) = try_get_text(client, &candidate).await {
            let source = if path.contains("full") {
                ProbeSource::LlmsFullTxt
            } else {
                ProbeSource::LlmsTxt
            };
            return Some((text, source));
        }
    }
    None
}

/// Which probe path produced the content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeSource {
    MdSuffix,
    LlmsTxt,
    LlmsFullTxt,
}

impl std::fmt::Display for ProbeSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MdSuffix => write!(f, ".md"),
            Self::LlmsTxt => write!(f, "llms.txt"),
            Self::LlmsFullTxt => write!(f, "llms-full.txt"),
        }
    }
}

fn with_md_suffix(url: &Url) -> Option<Url> {
    let path = url.path();
    if path.ends_with('/') || path.ends_with(".md") || path.ends_with(".html") {
        return None; // Already a directory or doc file — skip
    }
    let new_path = format!("{path}.md");
    let mut new_url = url.clone();
    new_url.set_path(&new_path);
    Some(new_url)
}

async fn try_get_text(client: &rquest::Client, url: &Url) -> Option<String> {
    let resp = client.get(url.as_str()).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let ct = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_ascii_lowercase();
    // Only accept text responses — not HTML pages masquerading as .md
    if ct.contains("html") && !ct.contains("markdown") {
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
    use crate::fetch::client::build_client;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn with_md_suffix_appends_to_plain_path() {
        let url = Url::parse("https://docs.example.com/getting-started").unwrap();
        let result = with_md_suffix(&url).unwrap();
        assert_eq!(result.path(), "/getting-started.md");
    }

    #[test]
    fn with_md_suffix_skips_trailing_slash() {
        let url = Url::parse("https://docs.example.com/section/").unwrap();
        assert!(with_md_suffix(&url).is_none());
    }

    #[test]
    fn with_md_suffix_skips_existing_md() {
        let url = Url::parse("https://docs.example.com/page.md").unwrap();
        assert!(with_md_suffix(&url).is_none());
    }

    #[test]
    fn with_md_suffix_skips_html_files() {
        let url = Url::parse("https://docs.example.com/page.html").unwrap();
        assert!(with_md_suffix(&url).is_none());
    }

    #[tokio::test]
    async fn probe_markdown_prefers_md_suffix_when_available() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/docs/getting-started.md"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/markdown")
                    .set_body_string("# Getting started"),
            )
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/docs/getting-started/index.html.md"))
            .respond_with(ResponseTemplate::new(200).set_body_string("fallback"))
            .expect(0)
            .mount(&server)
            .await;

        let client = build_client().unwrap();
        let url = Url::parse(&format!("{}/docs/getting-started", server.uri())).unwrap();
        let result = probe_markdown(&client, &url).await;

        assert_eq!(
            result,
            Some(("# Getting started".to_string(), ProbeSource::MdSuffix))
        );
    }

    #[tokio::test]
    async fn probe_markdown_uses_index_html_md_for_directory_urls() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/docs/index.html.md"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/plain")
                    .set_body_string("Directory index markdown"),
            )
            .mount(&server)
            .await;

        let client = build_client().unwrap();
        let url = Url::parse(&format!("{}/docs/", server.uri())).unwrap();
        let result = probe_markdown(&client, &url).await;

        assert_eq!(
            result,
            Some((
                "Directory index markdown".to_string(),
                ProbeSource::MdSuffix
            ))
        );
    }

    #[tokio::test]
    async fn probe_markdown_rejects_html_and_empty_markdown_responses() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/docs/page.md"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_raw("<html>not markdown</html>", "text/html; charset=utf-8"),
            )
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/docs/index.html.md"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/docs/empty.md"))
            .respond_with(ResponseTemplate::new(200).set_body_raw("   \n\t", "text/markdown"))
            .mount(&server)
            .await;

        let client = build_client().unwrap();
        let html_url = Url::parse(&format!("{}/docs/page", server.uri())).unwrap();
        let empty_url = Url::parse(&format!("{}/docs/empty", server.uri())).unwrap();

        assert!(probe_markdown(&client, &html_url).await.is_none());
        assert!(probe_markdown(&client, &empty_url).await.is_none());
    }

    #[tokio::test]
    async fn probe_llms_index_uses_full_then_root_then_well_known() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/llms-full.txt"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/plain")
                    .set_body_string("full index"),
            )
            .mount(&server)
            .await;

        let client = build_client().unwrap();
        let origin = Url::parse(&server.uri()).unwrap();
        let first = probe_llms_index(&client, &origin).await;
        assert_eq!(
            first,
            Some(("full index".to_string(), ProbeSource::LlmsFullTxt))
        );

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/llms-full.txt"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/llms.txt"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/plain")
                    .set_body_string("root index"),
            )
            .mount(&server)
            .await;

        let origin = Url::parse(&server.uri()).unwrap();
        let second = probe_llms_index(&client, &origin).await;
        assert_eq!(
            second,
            Some(("root index".to_string(), ProbeSource::LlmsTxt))
        );

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/llms-full.txt"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/llms.txt"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/.well-known/llms.txt"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/plain")
                    .set_body_string("well-known index"),
            )
            .mount(&server)
            .await;

        let origin = Url::parse(&server.uri()).unwrap();
        let third = probe_llms_index(&client, &origin).await;
        assert_eq!(
            third,
            Some(("well-known index".to_string(), ProbeSource::LlmsTxt))
        );
    }

    #[test]
    fn probe_source_display_names_are_stable() {
        assert_eq!(ProbeSource::MdSuffix.to_string(), ".md");
        assert_eq!(ProbeSource::LlmsTxt.to_string(), "llms.txt");
        assert_eq!(ProbeSource::LlmsFullTxt.to_string(), "llms-full.txt");
    }
}
