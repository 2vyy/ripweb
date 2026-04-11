use std::sync::Arc;
use std::time::Duration;

use ripweb::fetch::{
    client::{RetryConfig, build_client},
    crawler::{Crawler, CrawlerConfig, format_output},
    politeness::DomainSemaphores,
};
use url::Url;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn fast_retry() -> RetryConfig {
    RetryConfig {
        max_retries: 0,
        base_delay: Duration::from_millis(1),
    }
}

fn page_html(content: &str, links: &[(&str, &str)]) -> String {
    let link_html: String = links
        .iter()
        .map(|(href, text)| format!(r#"<a href="{href}">{text}</a>"#))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        r#"<html><body>
        <nav><a href="/ignored">Nav link — must not be followed</a></nav>
        <main>
          <p>{content}</p>
          {link_html}
        </main>
        </body></html>"#
    )
}

// ── max-pages ─────────────────────────────────────────────────────────────────

/// Crawl with max_pages=2.  The seed links to 3 more pages.  The crawler must
/// stop after fetching 2 total (seed + 1 follower).
#[tokio::test]
async fn crawl_respects_max_pages() {
    let server = MockServer::start().await;

    // Seed page links to /a, /b, /c
    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_string(page_html(
            "Seed page with many links",
            &[("/a", "Page A"), ("/b", "Page B"), ("/c", "Page C")],
        )))
        .mount(&server)
        .await;

    for (p, label) in [
        ("/a", "Content of A"),
        ("/b", "Content of B"),
        ("/c", "Content of C"),
    ] {
        Mock::given(method("GET"))
            .and(path(p))
            .respond_with(ResponseTemplate::new(200).set_body_string(page_html(label, &[])))
            .mount(&server)
            .await;
    }

    let seed = Url::parse(&format!("{}/", server.uri())).unwrap();
    let client = Arc::new(build_client().unwrap());
    let crawler = Crawler::new(
        client,
        DomainSemaphores::new(3),
        None,
        fast_retry(),
        CrawlerConfig {
            max_depth: 1,
            max_pages: 2,
            tables_priority: false,
        },
    );

    let pages = crawler.crawl(seed).await;

    assert_eq!(
        pages.len(),
        2,
        "expected exactly 2 pages, got {}: {:?}",
        pages.len(),
        pages.iter().map(|p| p.url.as_str()).collect::<Vec<_>>()
    );
}

// ── Anchor deduplication ──────────────────────────────────────────────────────

/// The seed page links to /page, /page#section-1, and /page#section-2.
/// All three must be treated as the same URL — only one fetch should occur.
#[tokio::test]
async fn anchor_variants_are_deduplicated() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_string(page_html(
            "Seed page",
            &[
                ("/page", "Base link"),
                ("/page#section-1", "Anchor 1"),
                ("/page#section-2", "Anchor 2"),
            ],
        )))
        .mount(&server)
        .await;

    // /page must only be fetched once.
    Mock::given(method("GET"))
        .and(path("/page"))
        .respond_with(ResponseTemplate::new(200).set_body_string(page_html(
            "The actual page content unique text here present",
            &[],
        )))
        .expect(1) // wiremock assertion: exactly 1 hit
        .mount(&server)
        .await;

    let seed = Url::parse(&format!("{}/", server.uri())).unwrap();
    let client = Arc::new(build_client().unwrap());
    let crawler = Crawler::new(
        client,
        DomainSemaphores::new(3),
        None,
        fast_retry(),
        CrawlerConfig {
            max_depth: 1,
            max_pages: 10,
            tables_priority: false,
        },
    );

    let pages = crawler.crawl(seed).await;

    // wiremock verifies the `expect(1)` when the server drops.
    let page_urls: Vec<_> = pages.iter().map(|p| p.url.as_str()).collect();
    let page_count = page_urls.iter().filter(|u| u.contains("/page")).count();
    assert_eq!(
        page_count, 1,
        "/page fetched more than once: {:?}",
        page_urls
    );
}

// ── LLM delimiter ─────────────────────────────────────────────────────────────

#[test]
fn format_output_injects_source_delimiters() {
    use ripweb::fetch::crawler::CrawledPage;

    let pages = vec![
        CrawledPage {
            url: "https://example.com/".to_owned(),
            content: "First page.".to_owned(),
        },
        CrawledPage {
            url: "https://example.com/two".to_owned(),
            content: "Second page.".to_owned(),
        },
    ];
    let out = format_output(&pages);
    assert!(out.contains("# --- [Source: https://example.com/] ---"));
    assert!(out.contains("# --- [Source: https://example.com/two] ---"));
    assert!(out.contains("First page."));
    assert!(out.contains("Second page."));
    // Delimiter must appear before the content.
    let delim_pos = out
        .find("# --- [Source: https://example.com/] ---")
        .unwrap();
    let content_pos = out.find("First page.").unwrap();
    assert!(delim_pos < content_pos, "delimiter must precede content");
}
