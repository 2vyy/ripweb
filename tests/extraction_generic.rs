use ripweb::extract::{Extractor, web::WebExtractor};

fn assert_snapshot_named(name: &str, content: &str) {
    let mut settings = insta::Settings::clone_current();
    settings.set_prepend_module_to_snapshot(false);
    settings.bind(|| insta::assert_snapshot!(name, content));
}

// ── Nuke List ────────────────────────────────────────────────────────────────

#[test]
fn nuke_list_strips_nav_footer_header_aside_svg_form_iframe() {
    let html = br#"
    <html><head><meta charset="utf-8"></head><body>
      <header>HEADER_SENTINEL</header>
      <nav>NAV_SENTINEL</nav>
      <aside>ASIDE_SENTINEL</aside>
      <svg><text>SVG_SENTINEL</text></svg>
      <form><input value="FORM_SENTINEL"></form>
      <iframe src="about:blank">IFRAME_SENTINEL</iframe>
      <main>
        <p>This is the real article content that should survive extraction.
        It needs enough words so the SPA fallback is not triggered by the
        word-count heuristic. Adding more text here to be safe.</p>
      </main>
      <footer>FOOTER_SENTINEL</footer>
    </body></html>
    "#;

    let result = WebExtractor::extract(html, Some("text/html; charset=utf-8")).unwrap();

    for sentinel in &[
        "HEADER_SENTINEL",
        "NAV_SENTINEL",
        "ASIDE_SENTINEL",
        "SVG_SENTINEL",
        "FORM_SENTINEL",
        "IFRAME_SENTINEL",
        "FOOTER_SENTINEL",
    ] {
        assert!(
            !result.contains(sentinel),
            "Nuke list failed: '{}' found in output:\n{}",
            sentinel,
            result
        );
    }
    assert!(
        result.contains("real article content"),
        "Main content missing from output"
    );
}

// ── Main / Article heuristics ────────────────────────────────────────────────

#[test]
fn extracts_content_from_main_tag() {
    let html = br#"
    <html><body>
      <nav>Navigation noise</nav>
      <main><h1>Main Heading</h1><p>Content inside main tag which is the target.</p></main>
      <footer>Footer noise</footer>
    </body></html>
    "#;

    let result = WebExtractor::extract(html, None).unwrap();
    assert!(
        result.contains("# Main Heading"),
        "heading was not rendered as Markdown: {result}"
    );
    assert!(result.contains("Content inside main tag"));
    assert!(!result.contains("Navigation noise"));
    assert!(!result.contains("Footer noise"));
}

#[test]
fn extracts_content_from_article_tag_when_no_main() {
    let html = br#"
    <html><body>
      <header>Header noise</header>
      <article><h2>Article Heading</h2><p>Content inside article tag.</p></article>
      <footer>Footer noise</footer>
    </body></html>
    "#;

    let result = WebExtractor::extract(html, None).unwrap();
    assert!(
        result.contains("## Article Heading"),
        "heading was not rendered as Markdown: {result}"
    );
    assert!(result.contains("Content inside article tag"));
    assert!(!result.contains("Header noise"));
}

#[test]
fn renders_links_lists_and_code_as_markdown() {
    let html = br#"
    <html><body>
      <main>
        <p>See the <a href="https://example.com/docs?utm_source=test&id=42">documentation</a> and use <code>ripweb fetch</code>.</p>
        <ul>
          <li>First item</li>
          <li>Second item</li>
        </ul>
        <pre><code>fn main() {
    println!("hello");
}</code></pre>
      </main>
    </body></html>
    "#;

    let result = WebExtractor::extract(html, None).unwrap();
    assert!(
        result.contains("[documentation](https://example.com/docs?id=42)"),
        "link was not preserved as Markdown: {result}"
    );
    assert!(
        result.contains("`ripweb fetch`"),
        "inline code missing: {result}"
    );
    assert!(
        result.contains("- First item"),
        "unordered list missing: {result}"
    );
    assert!(result.contains("```"), "code fence missing: {result}");
    assert!(
        result.contains("println!(\"hello\");"),
        "code block content missing: {result}"
    );
}

// ── SPA fallback ─────────────────────────────────────────────────────────────

#[test]
fn spa_fallback_extracts_next_data_when_body_is_sparse() {
    let html = include_bytes!("extraction/generic/spa_next_data.html");
    let result = WebExtractor::extract(html, None).unwrap();

    // Should pull the content string out of __NEXT_DATA__ JSON
    assert!(
        result.contains("Zero-cost abstractions"),
        "SPA fallback did not extract __NEXT_DATA__ content:\n{}",
        result
    );
    // The loading spinner text should not dominate
    assert!(
        !result.trim().starts_with("Loading"),
        "Output starts with loading spinner text — SPA fallback not triggered"
    );
}

#[test]
fn spa_fallback_not_triggered_when_content_sufficient() {
    let html = br#"
    <html><body>
      <main>
        <p>One two three four five six seven eight nine ten eleven twelve thirteen
        fourteen fifteen sixteen seventeen eighteen nineteen twenty twenty-one
        twenty-two twenty-three twenty-four twenty-five twenty-six twenty-seven
        twenty-eight twenty-nine thirty thirty-one thirty-two thirty-three
        thirty-four thirty-five thirty-six thirty-seven thirty-eight thirty-nine
        forty forty-one forty-two forty-three forty-four forty-five forty-six
        forty-seven forty-eight forty-nine fifty fifty-one fifty-two fifty-three
        fifty-four fifty-five fifty-six fifty-seven fifty-eight fifty-nine sixty
        sixty-one sixty-two sixty-three sixty-four sixty-five sixty-six sixty-seven
        sixty-eight sixty-nine seventy seventy-one seventy-two seventy-three
        seventy-four seventy-five seventy-six seventy-seven seventy-eight seventy-nine
        eighty eighty-one eighty-two eighty-three eighty-four eighty-five eighty-six
        eighty-seven eighty-eight eighty-nine ninety ninety-one ninety-two ninety-three
        ninety-four ninety-five ninety-six ninety-seven ninety-eight ninety-nine hundred</p>
      </main>
      <script id="__NEXT_DATA__" type="application/json">{"props":{"pageProps":{"content":"SHOULD_NOT_APPEAR"}}}</script>
    </body></html>
    "#;

    let result = WebExtractor::extract(html, None).unwrap();
    assert!(
        !result.contains("SHOULD_NOT_APPEAR"),
        "SPA fallback triggered despite sufficient content"
    );
    assert!(result.contains("One two three"), "Main content missing");
}

// ── Charset decoding ─────────────────────────────────────────────────────────

#[test]
fn charset_from_content_type_header_overrides_utf8_default() {
    let html = "<!DOCTYPE html><html><head><meta charset=\"utf-8\"></head><body>\
                <main><p>Café au lait costs €3.50 — naïve résumé</p></main></body></html>";
    let bytes = html.as_bytes();

    let result = WebExtractor::extract(bytes, Some("text/html; charset=utf-8")).unwrap();
    assert!(result.contains("Café"), "UTF-8 not decoded: {}", result);
    assert!(result.contains("€"), "Euro sign not decoded: {}", result);
    assert!(
        result.contains("naïve"),
        "Diacritic not decoded: {}",
        result
    );
}

#[test]
fn charset_falls_back_to_meta_tag_when_no_content_type() {
    let html = b"<!DOCTYPE html><html><head><meta charset=\"utf-8\"></head><body>\
                 <main><p>Resume with accents: \xc3\xa9l\xc3\xa8ve</p></main></body></html>";

    let result = WebExtractor::extract(html, None).unwrap();
    assert!(
        result.contains("lève") || result.contains("l\u{e8}ve"),
        "Meta charset failed"
    );
}

// ── Snapshot tests (insta) ───────────────────────────────────────────────────

#[test]
fn extraction_bloated_generic() {
    let html = include_bytes!("extraction/generic/bloated_generic.html");
    let result = WebExtractor::extract(html, Some("text/html; charset=utf-8")).unwrap();
    assert_snapshot_named("extraction__bloated_generic", &result);
}

#[test]
fn extraction_article_clean() {
    let html = include_bytes!("extraction/generic/article_clean.html");
    let result = WebExtractor::extract(html, Some("text/html; charset=utf-8")).unwrap();
    assert_snapshot_named("extraction__article_clean", &result);
}

#[test]
fn extraction_spa_next_data() {
    let html = include_bytes!("extraction/generic/spa_next_data.html");
    let result = WebExtractor::extract(html, None).unwrap();
    assert_snapshot_named("extraction__spa_next_data", &result);
}

// ── Page-Family Snapshot Tests ────────────────────────────────────────────────

#[test]
fn extraction_docs_sidebar() {
    let html = include_bytes!("extraction/generic/docs_sidebar.html");
    let result = WebExtractor::extract_with_url(
        html,
        Some("text/html; charset=utf-8"),
        Some("https://docs.example.com/api"),
    )
    .unwrap();
    assert!(
        !result.contains("On this page"),
        "TOC clone leaked: {result}"
    );
    assert!(result.contains("fetch"), "API content missing: {result}");
    assert!(result.contains("```rust"), "rust fence missing: {result}");
    assert_snapshot_named("extraction__docs_sidebar", &result);
}

#[test]
fn extraction_listing_results() {
    let html = include_bytes!("extraction/generic/listing_results.html");
    let result = WebExtractor::extract(html, Some("text/html; charset=utf-8")).unwrap();
    assert!(
        result.contains("The Async Book"),
        "first result missing: {result}"
    );
    assert!(
        result.contains("Tokio Tutorial"),
        "second result missing: {result}"
    );
    assert!(
        !result.contains("Next page"),
        "pagination chrome leaked: {result}"
    );
    assert_snapshot_named("extraction__listing_results", &result);
}

#[test]
fn extraction_product_detail() {
    let html = include_bytes!("extraction/generic/product_detail.html");
    let result = WebExtractor::extract_with_url(
        html,
        Some("text/html; charset=utf-8"),
        Some("https://www.keychron.com/products/q1-pro"),
    )
    .unwrap();
    assert!(
        result.contains("Keychron Q1 Pro"),
        "product title missing: {result}"
    );
    assert!(result.contains("199.00"), "price missing: {result}");
    assert!(result.contains("Hot-swap"), "spec table missing: {result}");
    assert!(
        !result.contains("Customers also viewed"),
        "recommendation rail leaked: {result}"
    );
    assert_snapshot_named("extraction__product_detail", &result);
}

#[test]
fn extraction_forum_thread() {
    let html = include_bytes!("extraction/generic/forum_thread.html");
    let result = WebExtractor::extract_with_url(
        html,
        Some("text/html; charset=utf-8"),
        Some("https://stackoverflow.com/questions/12345"),
    )
    .unwrap();
    assert!(
        result.contains("borrow checker"),
        "question content missing: {result}"
    );
    assert!(
        result.contains("use-after-free"),
        "accepted answer missing: {result}"
    );
    assert!(
        !result.contains("Hot Network Questions"),
        "sidebar leaked: {result}"
    );
    assert!(result.contains("```rust"), "rust fence missing: {result}");
    assert_snapshot_named("extraction__forum_thread", &result);
}

#[test]
fn test_forum_ranking_postprocess() {
    let html = r#"
        <html>
        <head><meta property="og:type" content="forum"></head>
        <body>
            <main class="forum-thread">
                <div class="post op">
                    <h1>How to use Ripweb?</h1>
                    <div class="content">I am new to this.</div>
                </div>
                <div class="post answer" id="a2">
                    <div class="vote-count">10</div>
                    <div class="content">Second answer here.</div>
                </div>
                <div class="post answer accepted" id="a1">
                    <div class="vote-count">5</div>
                    <div class="content">First answer (accepted).</div>
                </div>
                <div class="post answer" id="a3">
                    <div class="vote-count">50</div>
                    <div class="content">Third answer (high score).</div>
                </div>
            </main>
        </body>
        </html>
    "#;

    let result = ripweb::extract::web::WebExtractor::extract_with_url(
        html.as_bytes(),
        Some("text/html"),
        Some("https://example.com/forum/thread/123"),
    )
    .unwrap();

    // Check for "Answers" header added by post-processor
    assert!(result.contains("## Answers"));

    // Check order: a1 (accepted) -> a3 (score 50) -> a2 (score 10)
    let pos_a1 = result.find("First answer (accepted)").unwrap();
    let pos_a3 = result.find("Third answer (high score)").unwrap();
    let pos_a2 = result.find("Second answer here").unwrap();

    assert!(
        pos_a1 < pos_a3,
        "Accepted answer should come before high-score answer"
    );
    assert!(
        pos_a3 < pos_a2,
        "High-score answer should come before lower-score answer"
    );

    // Verify meta headers
    assert!(result.contains("[Score: 5] (Accepted Answer)"));
    assert!(result.contains("[Score: 50]"));
    assert!(result.contains("[Score: 10]"));
}
