use ripweb::extract::{Extractor, web::WebExtractor};

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
    let html = include_bytes!("fixtures/extract/spa_next_data.html");
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
fn snapshot_bloated_generic_page() {
    let html = include_bytes!("fixtures/extract/bloated_generic.html");
    let result = WebExtractor::extract(html, Some("text/html; charset=utf-8")).unwrap();
    insta::assert_snapshot!(result);
}

#[test]
fn snapshot_article_clean_page() {
    let html = include_bytes!("fixtures/extract/article_clean.html");
    let result = WebExtractor::extract(html, Some("text/html; charset=utf-8")).unwrap();
    insta::assert_snapshot!(result);
}

#[test]
fn snapshot_spa_next_data_page() {
    let html = include_bytes!("fixtures/extract/spa_next_data.html");
    let result = WebExtractor::extract(html, None).unwrap();
    insta::assert_snapshot!(result);
}

// ── Torture tests ─────────────────────────────────────────────────────────────

#[test]
fn torture_spa_empty_body_returns_empty() {
    let html = include_bytes!("fixtures/torture/spa/spa_empty_body.html");
    let result = WebExtractor::extract(html, None);
    let text = result.unwrap_or_default();
    assert!(
        text.trim().is_empty(),
        "Expected empty output for SPA shell"
    );
}

#[test]
fn torture_giant_inline_svg_returns_input_too_large() {
    let html = include_bytes!("fixtures/torture/giant_inline_svg.html");
    let result = WebExtractor::extract(html, Some("text/html"));
    assert!(result.is_err(), "Expected error for oversized input");
    assert!(matches!(
        result,
        Err(ripweb::error::RipwebError::InputTooLarge(_))
    ));
}

#[test]
fn torture_json_ld_returns_minimal_content() {
    let html = include_bytes!("fixtures/torture/spa/json_ld_rich.html");
    let result = WebExtractor::extract(html, Some("text/html; charset=utf-8"));
    let text = result.unwrap_or_default();
    // Script is nuked, so only empty p remains - should be minimal
    assert!(
        text.len() < 100,
        "Expected minimal content for JSON-LD page"
    );
}

#[test]
fn torture_fake_main_prefers_real_content_container() {
    let html = include_bytes!("fixtures/torture/dom/fake_main_is_nav.html");
    let result = WebExtractor::extract(html, Some("text/html; charset=utf-8")).unwrap();

    assert!(
        result.contains("# The Real Article Title"),
        "real article missing: {result}"
    );
    assert!(
        result.contains("actual article content"),
        "real prose missing: {result}"
    );
    assert!(
        !result.contains("Post 1"),
        "nav-only main should not win: {result}"
    );
}

#[test]
fn strips_class_based_boilerplate_subtrees() {
    let html = br#"
    <html><body>
      <div class="header utility-nav">
        <a href="/meetings">Meetings &amp; Events</a>
        <a href="/safe-travel">Safe Travel Information</a>
      </div>
      <div class="content story-body">
        <h1>Spring in Japan: Cherry Blossom Forecast</h1>
        <p>Each spring is marked by the loveliness of sakura blooms bursting to life throughout Japan.</p>
        <h2>Where to see the blooms</h2>
        <p>The warmer climates of Kyushu and Shikoku in the south see the first action in early spring.</p>
      </div>
      <div class="related slider">
        <a href="/spot/1">Follow the cherry blossom trail</a>
        <a href="/spot/2">More flowers to see in spring</a>
      </div>
    </body></html>
    "#;

    let result = WebExtractor::extract(html, Some("text/html; charset=utf-8")).unwrap();

    assert!(
        result.contains("# Spring in Japan: Cherry Blossom Forecast"),
        "main content heading missing: {result}"
    );
    assert!(
        result.contains("Where to see the blooms"),
        "main content section missing: {result}"
    );
    assert!(
        !result.contains("Meetings & Events"),
        "utility boilerplate leaked into output: {result}"
    );
    assert!(
        !result.contains("More flowers to see in spring"),
        "related slider content leaked into output: {result}"
    );
}

#[test]
fn torture_no_main_no_article_prefers_content_div() {
    let html = include_bytes!("fixtures/torture/dom/no_main_no_article.html");
    let result = WebExtractor::extract(html, Some("text/html; charset=utf-8")).unwrap();

    assert!(
        result.contains("## Important Article Title"),
        "content div heading missing: {result}"
    );
    assert!(
        result.contains("real content inside a div"),
        "content div prose missing: {result}"
    );
    assert!(
        !result.contains("Site Header"),
        "header noise leaked: {result}"
    );
    assert!(!result.contains("[Home]"), "nav links leaked: {result}");
}

#[test]
fn torture_content_in_table_is_extracted() {
    let html = include_bytes!("fixtures/torture/dom/content_in_table.html");
    let result = WebExtractor::extract(html, Some("text/html; charset=utf-8")).unwrap();

    assert!(
        result.contains("## Policy Document"),
        "table heading missing: {result}"
    );
    assert!(
        result.contains("Section 1"),
        "table content missing: {result}"
    );
    assert!(
        result.contains("Old government and academic sites"),
        "table prose missing: {result}"
    );
    assert!(
        !result.contains("Government Portal"),
        "header noise leaked: {result}"
    );
}

#[test]
fn prefers_real_content_over_link_heavy_main() {
    let html = br#"
    <html><body>
      <main>
        <nav>
          <a href="/1">Link 1</a><a href="/2">Link 2</a><a href="/3">Link 3</a>
          <a href="/4">Link 4</a><a href="/5">Link 5</a><a href="/6">Link 6</a>
          <a href="/7">Link 7</a><a href="/8">Link 8</a><a href="/9">Link 9</a>
          <a href="/10">Link 10</a>
        </nav>
      </main>
      <section class="story">
        <h1>Actual Story</h1>
        <p>This is a long paragraph of text that should be preferred over the link-heavy main container.
        It has many words and very few links, making it a much better candidate for extraction.</p>
      </section>
    </body></html>
    "#;

    let result = WebExtractor::extract(html, Some("text/html; charset=utf-8")).unwrap();

    assert!(
        result.contains("# Actual Story"),
        "story heading missing: {result}"
    );
    assert!(
        result.contains("long paragraph of text"),
        "story content missing: {result}"
    );
    assert!(
        !result.contains("Link 1"),
        "link-heavy main should have been penalized: {result}"
    );
}

#[test]
fn product_family_url_hint_prefers_buybox_over_link_grid() {
    let html = br#"
    <html><body>
      <main class="search-results">
        <a href="/ip/1">Similar product one</a>
        <a href="/ip/2">Similar product two</a>
        <a href="/ip/3">Similar product three</a>
        <a href="/ip/4">Similar product four</a>
        <a href="/ip/5">Similar product five</a>
        <a href="/ip/6">Similar product six</a>
      </main>
      <section class="product-details buybox">
        <h1>Ip Man 1-4 (Box Set) (Blu-ray)</h1>
        <p>Current price is USD$22.99</p>
        <h2>Key item features</h2>
        <ul>
          <li>Action, Biography, Drama</li>
          <li>Movie &amp; TV media format: Blu-ray</li>
        </ul>
        <h2>Specifications</h2>
        <table>
          <tr><th>Director</th><td>Wilson Yip</td></tr>
          <tr><th>Resolution</th><td>1080p</td></tr>
        </table>
      </section>
    </body></html>
    "#;

    let result = WebExtractor::extract_with_url(
        html,
        Some("text/html; charset=utf-8"),
        Some("https://www.walmart.com/ip/160317419"),
    )
    .unwrap();

    assert!(
        result.contains("# Ip Man 1-4 (Box Set) (Blu-ray)"),
        "product title missing: {result}"
    );
    assert!(
        result.contains("USD$22.99"),
        "product price missing: {result}"
    );
    assert!(result.contains("Director"), "spec table missing: {result}");
    assert!(
        !result.contains("Similar product one"),
        "link grid should not beat the product detail container: {result}"
    );
}

// ── Page-Family Snapshot Tests ────────────────────────────────────────────────

#[test]
fn snapshot_docs_sidebar_page() {
    let html = include_bytes!("fixtures/extract/docs_sidebar.html");
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
    insta::assert_snapshot!(result);
}

#[test]
fn snapshot_listing_results_page() {
    let html = include_bytes!("fixtures/extract/listing_results.html");
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
    insta::assert_snapshot!(result);
}

#[test]
fn snapshot_product_detail_page() {
    let html = include_bytes!("fixtures/extract/product_detail.html");
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
    insta::assert_snapshot!(result);
}

#[test]
fn snapshot_forum_thread_page() {
    let html = include_bytes!("fixtures/extract/forum_thread.html");
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
    insta::assert_snapshot!(result);
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
