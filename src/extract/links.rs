use std::collections::HashSet;

use url::Url;

/// Tags whose subtrees are not crawled for outbound links — mirrors the
/// nuke list in `web.rs` for content extraction.
const SKIP_TAGS: &[&str] = &[
    "nav", "footer", "header", "aside", "style", "script", "noscript",
    "svg", "iframe", "form",
];

/// Content root tags — only links found inside these are followed.
const CONTENT_ROOTS: &[&str] = &["main", "article"];

/// Extract all `<a href>` links found inside `<main>` or `<article>` elements,
/// resolved against `base`, filtered to the same host, and normalised (fragments
/// stripped, trailing slashes removed).
///
/// Links outside content areas (nav, footer, etc.) are ignored to avoid crawl
/// traps and irrelevant navigation pages.
pub fn extract_content_links(html: &str, base: &Url) -> Vec<Url> {
    let dom = match tl::parse(html, tl::ParserOptions::default()) {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };
    let parser = dom.parser();
    let base_host = base.host_str().unwrap_or("");

    let mut seen: HashSet<String> = HashSet::new();
    let mut links: Vec<Url> = Vec::new();

    for root_tag in CONTENT_ROOTS {
        let Some(handle) = dom
            .query_selector(root_tag)
            .and_then(|mut it| it.next())
        else {
            continue;
        };

        let Some(node) = handle.get(parser) else {
            continue;
        };

        collect_links(node, parser, base, base_host, &mut seen, &mut links);
    }

    links
}

/// Recursively walk `node`, skipping non-content subtrees, collecting `<a href>` links.
fn collect_links(
    node: &tl::Node,
    parser: &tl::Parser,
    base: &Url,
    base_host: &str,
    seen: &mut HashSet<String>,
    out: &mut Vec<Url>,
) {
    let tl::Node::Tag(tag) = node else { return };

    let name = tag.name().as_utf8_str().to_ascii_lowercase();

    if SKIP_TAGS.contains(&name.as_str()) {
        return;
    }

    // Collect href from <a> tags.
    if name == "a"
        && let Some(href_bytes) = tag.attributes().get("href").flatten()
    {
        let href = href_bytes.as_utf8_str();
        if let Some(url) = resolve_and_normalise(href.as_ref(), base, base_host) {
            let key = url.as_str().to_owned();
            if seen.insert(key) {
                out.push(url);
            }
        }
    }

    // Recurse into children.
    for handle in tag.children().top().iter() {
        if let Some(child) = handle.get(parser) {
            collect_links(child, parser, base, base_host, seen, out);
        }
    }
}

/// Parse `href` relative to `base`, strip its fragment, and return it only if
/// it belongs to `base_host`.  Returns `None` for external hosts, javascript:
/// links, mailto:, etc.
fn resolve_and_normalise(href: &str, base: &Url, base_host: &str) -> Option<Url> {
    let mut url = base.join(href).ok()?;

    // Reject non-http(s) schemes (mailto, javascript, data, …)
    if !matches!(url.scheme(), "http" | "https") {
        return None;
    }

    // Same-domain check.
    if url.host_str() != Some(base_host) {
        return None;
    }

    // Strip fragment.
    url.set_fragment(None);

    // Strip trailing slash from non-root paths.
    let path = url.path().to_owned();
    if path.len() > 1 && path.ends_with('/') {
        url.set_path(path.trim_end_matches('/'));
    }

    Some(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Url {
        Url::parse("https://example.com").unwrap()
    }

    #[test]
    fn extracts_links_from_main() {
        let html = r#"
        <html><body>
          <nav><a href="/nav-link">Should be ignored</a></nav>
          <main>
            <a href="/page1">Page 1</a>
            <a href="/page2">Page 2</a>
          </main>
          <footer><a href="/footer-link">Should be ignored</a></footer>
        </body></html>
        "#;

        let links = extract_content_links(html, &base());
        let paths: Vec<_> = links.iter().map(|u| u.path()).collect();
        assert!(paths.contains(&"/page1"), "page1 missing: {:?}", paths);
        assert!(paths.contains(&"/page2"), "page2 missing: {:?}", paths);
        assert!(!paths.contains(&"/nav-link"), "nav link must be excluded");
        assert!(!paths.contains(&"/footer-link"), "footer link must be excluded");
    }

    #[test]
    fn extracts_links_from_article() {
        let html = r#"
        <html><body>
          <aside><a href="/sidebar">Ignored</a></aside>
          <article>
            <a href="/referenced-post">See this post</a>
          </article>
        </body></html>
        "#;

        let links = extract_content_links(html, &base());
        let paths: Vec<_> = links.iter().map(|u| u.path()).collect();
        assert!(paths.contains(&"/referenced-post"));
        assert!(!paths.contains(&"/sidebar"));
    }

    #[test]
    fn strips_fragment_from_extracted_links() {
        let html = r#"<html><body><main>
          <a href="/page#section">Anchored</a>
        </main></body></html>"#;

        let links = extract_content_links(html, &base());
        assert_eq!(links.len(), 1);
        assert!(links[0].fragment().is_none(), "fragment must be stripped");
        assert_eq!(links[0].path(), "/page");
    }

    #[test]
    fn ignores_external_links() {
        let html = r#"<html><body><main>
          <a href="/internal">Internal</a>
          <a href="https://other.com/page">External</a>
        </main></body></html>"#;

        let links = extract_content_links(html, &base());
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].path(), "/internal");
    }

    #[test]
    fn resolves_relative_links_against_base() {
        let base = Url::parse("https://example.com/docs/intro").unwrap();
        let html = r#"<html><body><main>
          <a href="advanced">Advanced</a>
        </main></body></html>"#;

        let links = extract_content_links(html, &base);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].as_str(), "https://example.com/docs/advanced");
    }

    #[test]
    fn deduplicates_same_url() {
        let html = r#"<html><body><main>
          <a href="/page">Link A</a>
          <a href="/page">Link B (dupe)</a>
          <a href="/page#anchor">Link C (anchor dupe)</a>
        </main></body></html>"#;

        let links = extract_content_links(html, &base());
        let count = links.iter().filter(|u| u.path() == "/page").count();
        assert_eq!(count, 1, "duplicates must be deduplicated");
    }
}
