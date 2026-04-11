use ripweb::extract::{Extractor, web::WebExtractor};

#[test]
fn torture_spa_empty_body_returns_empty() {
    let html = include_bytes!("extraction/torture/spa/spa_empty_body.html");
    let result = WebExtractor::extract(html, None);
    let text = result.unwrap_or_default();
    assert!(
        text.trim().is_empty(),
        "Expected empty output for SPA shell"
    );
}

#[test]
fn torture_giant_inline_svg_returns_input_too_large() {
    let html = include_bytes!("extraction/torture/giant_inline_svg.html");
    let result = WebExtractor::extract(html, Some("text/html"));
    assert!(result.is_err(), "Expected error for oversized input");
    assert!(matches!(
        result,
        Err(ripweb::error::RipwebError::InputTooLarge(_))
    ));
}

#[test]
fn torture_json_ld_returns_minimal_content() {
    let html = include_bytes!("extraction/torture/spa/json_ld_rich.html");
    let result = WebExtractor::extract(html, Some("text/html; charset=utf-8"));
    let text = result.unwrap_or_default();
    assert!(
        text.len() < 100,
        "Expected minimal content for JSON-LD page"
    );
}

#[test]
fn torture_fake_main_prefers_real_content_container() {
    let html = include_bytes!("extraction/torture/dom/fake_main_is_nav.html");
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
    let html = include_bytes!("extraction/torture/dom/no_main_no_article.html");
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
    let html = include_bytes!("extraction/torture/dom/content_in_table.html");
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
