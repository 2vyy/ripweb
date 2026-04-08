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
pub async fn probe_markdown(
    client: &rquest::Client,
    url: &Url,
) -> Option<(String, ProbeSource)> {
    // 1. Try `.md` suffix on the exact page URL
    if let Some(md_url) = with_md_suffix(url) {
        if let Some(text) = try_get_text(client, &md_url).await {
            return Some((text, ProbeSource::MdSuffix));
        }
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
    if text.trim().is_empty() { None } else { Some(text) }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
