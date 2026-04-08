fn main() {
    println!("Generating torture fixtures...");

    // === Original 4 fixtures ===
    generate_deeply_nested_divs();
    generate_million_links();
    generate_giant_inline_svg();
    generate_binary_disguised_as_html();

    // === Track A: DOM Structure Attacks ===
    println!("\n--- DOM Structure Attacks ---");
    generate_dom_attacks();

    // === Track A: SPA & JavaScript Attacks ===
    println!("\n--- SPA & JavaScript Attacks ---");
    generate_spa_attacks();

    // === Track A: Encoding & Charset Attacks ===
    println!("\n--- Encoding & Charset Attacks ---");
    generate_encoding_attacks();

    // === Track A: Content Density Attacks ===
    println!("\n--- Content Density Attacks ---");
    generate_density_attacks();

    println!("\nDone! All fixtures created.");
}

// ============================================================================
// ORIGINAL 4 FIXTURES
// ============================================================================

fn generate_deeply_nested_divs() {
    let mut html = String::new();
    html.push_str("<!DOCTYPE html><html><head><meta charset=\"utf-8\"></head><body>");

    let mut current = String::new();
    for i in 0..10000 {
        current.push_str("<div>");
        if i == 9999 {
            current.push_str("The real content is here at the innermost level. This sentence proves the extractor can reach depth 10000.");
        }
    }
    for _ in 0..10000 {
        current.push_str("</div>");
    }

    html.push_str(&current);
    html.push_str("</body></html>");

    std::fs::write(
        "corpus/torture/deeply_nested_divs.html",
        &html,
    )
    .unwrap();
    println!("  torture/deeply_nested_divs.html - {} bytes", html.len());
}

fn generate_million_links() {
    let mut html = String::new();
    html.push_str("<!DOCTYPE html><html><head><meta charset=\"utf-8\"></head><body>");
    html.push_str("<nav>");
    for i in 0..10000 {
        html.push_str(&format!("<a href=\"/link{}\">Link {}</a>", i, i));
    }
    html.push_str("</nav>");
    html.push_str("<main><p>The real content is the only paragraph in this document. All those 10,000 links above should be stripped by the nav nuke tag.</p></main>");
    html.push_str("</body></html>");

    std::fs::write("corpus/torture/million_links.html", &html).unwrap();
    println!("  torture/million_links.html - {} bytes", html.len());
}

fn generate_giant_inline_svg() {
    let target_size = 6 * 1024 * 1024;
    let mut html = String::new();
    html.push_str("<!DOCTYPE html><html><head><meta charset=\"utf-8\"></head><body>");
    html.push_str("<main><p>Real text content before the SVG bomb.</p></main>");
    html.push_str("<svg xmlns=\"http://www.w3.org/2000/svg\">");
    let base_svg_end = "</svg></body></html>";
    let padding_needed = target_size - html.len() - base_svg_end.len();
    html.push_str(&" ".repeat(padding_needed.max(0)));
    html.push_str(base_svg_end);

    std::fs::write("corpus/torture/giant_inline_svg.html", &html).unwrap();
    println!("  torture/giant_inline_svg.html - {} bytes", html.len());
}

fn generate_binary_disguised_as_html() {
    let mut data = Vec::new();
    data.extend_from_slice(b"<!DOCTYPE html><html><head><title>Test</title></head><body>");

    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;
    let mut state = seed;
    for _ in 0..10000 {
        state = state.wrapping_mul(1103515245).wrapping_add(12345);
        let byte = 0x80 | ((state >> 16) as u8 & 0x7F);
        data.push(byte);
    }
    data.extend_from_slice(b"</body></html>");

    std::fs::write(
        "corpus/torture/binary_disguised_as_html.html",
        &data,
    )
    .unwrap();
    println!(
        "  torture/binary_disguised_as_html.html - {} bytes",
        data.len()
    );
}

// ============================================================================
// TRACK A: DOM STRUCTURE ATTACKS
// ============================================================================

fn generate_dom_attacks() {
    // Create directory
    let _ = std::fs::create_dir("corpus/torture/dom");

    // 1. no_main_no_article.html - Content in div.content, no main/article
    let html1 = r#"<!DOCTYPE html>
<html><head><meta charset="utf-8"></head><body>
<header><h1>Site Header</h1></header>
<nav><a href="/">Home</a><a href="/about">About</a></nav>
<div class="content">
    <h2>Important Article Title</h2>
    <p>This is the real content inside a div with class "content". There is no <main> or <article> tag in this document. The extractor should fall back to extracting from the body and include this paragraph.</p>
    <p>Another paragraph with more details about the topic.</p>
</div>
<footer><p>Copyright info</p></footer>
</body></html>"#;
    std::fs::write(
        "corpus/torture/dom/no_main_no_article.html",
        html1,
    )
    .unwrap();
    println!(
        "  torture/dom/no_main_no_article.html - {} bytes",
        html1.len()
    );

    // 2. fake_main_is_nav.html - main contains only nav links, real content elsewhere
    let html2 = r#"<!DOCTYPE html>
<html><head><meta charset="utf-8"></head><body>
<main>
    <nav><a href="/p1">Post 1</a><a href="/p2">Post 2</a><a href="/p3">Post 3</a><a href="/p4">Post 4</a><a href="/p5">Post 5</a></nav>
</main>
<div class="post-body">
    <h1>The Real Article Title</h1>
    <p>This is the actual article content that the user came to read. It resides outside the <main> tag in a div with class post-body. The extractor should detect that main contains less than 100 words and fall back to body extraction.</p>
    <p>More content here to ensure we have enough words.</p>
</div>
</body></html>"#;
    std::fs::write(
        "corpus/torture/dom/fake_main_is_nav.html",
        html2,
    )
    .unwrap();
    println!(
        "  torture/dom/fake_main_is_nav.html - {} bytes",
        html2.len()
    );

    // 3. triple_nested_main.html - Three nested main tags (malformed)
    let html3 = r#"<!DOCTYPE html>
<html><head><meta charset="utf-8"></head><body>
<main><main><main>
    <p>Innermost content - this is the real text inside three nested main tags.</p>
</main></main></main>
</body></html>"#;
    std::fs::write(
        "corpus/torture/dom/triple_nested_main.html",
        html3,
    )
    .unwrap();
    println!(
        "  torture/dom/triple_nested_main.html - {} bytes",
        html3.len()
    );

    // 4. content_in_table.html - Content in table (old government/academic sites)
    let html4 = r#"<!DOCTYPE html>
<html><head><meta charset="utf-8"></head><body>
<header><h1>Government Portal</h1></header>
<table border="0" cellpadding="5">
<tbody>
<tr><td colspan="2"><h2>Policy Document</h2></td></tr>
<tr>
<td><strong>Section 1:</strong></td>
<td>This is the actual policy content that appears inside a table cell. Old government and academic sites commonly use table-based layouts. This content should be extracted via body fallback since there is no main or article tag.</td>
</tr>
<tr>
<td><strong>Section 2:</strong></td>
<td>Additional policy text in another table cell with more details about the regulations.</td>
</tr>
</tbody>
</table>
<footer><p>Department of Example</p></footer>
</body></html>"#;
    std::fs::write(
        "corpus/torture/dom/content_in_table.html",
        html4,
    )
    .unwrap();
    println!(
        "  torture/dom/content_in_table.html - {} bytes",
        html4.len()
    );
}

// ============================================================================
// TRACK A: SPA & JAVASCRIPT ATTACKS
// ============================================================================

fn generate_spa_attacks() {
    let _ = std::fs::create_dir("corpus/torture/spa");

    // 1. nuxt_state.html - Empty main + __NUXT_DATA__ (Nuxt format)
    let html1 = r#"<!DOCTYPE html>
<html><head><meta charset="utf-8"></head><body>
<main></main>
<script type="application/json" id="__NUXT_DATA__">[
  {"title":"Nuxt.js Guide","description":"A comprehensive guide to Nuxt"},
  {"content":"This is the actual article content embedded in Nuxt state. The content appears in the array format used by Nuxt 2.x. This text should be extracted by the SPA fallback similar to __NEXT_DATA__ but handling Nuxt's array structure."},
  {"items":[{"text":"First item in the list"},{"text":"Second item"}]}
]</script>
</body></html>"#;
    std::fs::write("corpus/torture/spa/nuxt_state.html", html1).unwrap();
    println!("  torture/spa/nuxt_state.html - {} bytes", html1.len());

    // 2. spa_empty_body.html - Pure SPA shell, no content anywhere
    let html2 = r#"<!DOCTYPE html>
<html><head><meta charset="utf-8"></head><body>
<div id="root"></div>
<div id="app"></div>
<script src="/app.js"></script>
</body></html>"#;
    std::fs::write(
        "corpus/torture/spa/spa_empty_body.html",
        html2,
    )
    .unwrap();
    println!("  torture/spa/spa_empty_body.html - {} bytes", html2.len());

    // 3. json_ld_rich.html - No visible text, only JSON-LD schema
    let html3 = r#"<!DOCTYPE html>
<html><head><meta charset="utf-8"></head><body>
<main>
    <p></p>
</main>
<script type="application/ld+json">
{
  "@context": "https://schema.org",
  "@type": "Article",
  "headline": "Hidden Article Title From JSON-LD",
  "articleBody": "This is the real article content inside the schema.org JSON-LD articleBody field. It is inside a script tag which should be nuked. The extractor should NOT extract this content - it should return NoContent since there is no visible text after nuking.",
  "author": {
    "@type": "Person",
    "name": "Jane Author"
  },
  "datePublished": "2024-03-15"
}
</script>
</body></html>"#;
    std::fs::write("corpus/torture/spa/json_ld_rich.html", html3).unwrap();
    println!("  torture/spa/json_ld_rich.html - {} bytes", html3.len());
}

// ============================================================================
// TRACK A: ENCODING & CHARSET ATTACKS
// ============================================================================

fn generate_encoding_attacks() {
    let _ = std::fs::create_dir("corpus/torture/encoding");

    // 1. conflicting_charset.html - UTF-8 bytes but meta says ISO-8859-1
    // We'll generate actual UTF-8 bytes that contain multi-byte characters
    let utf8_content = "<main><p>Café résumé naïve</p></main>";
    let html1 = format!(
        "<!DOCTYPE html><html><head><meta charset=\"ISO-8859-1\"></head>{}</body></html>",
        utf8_content
    );
    std::fs::write(
        "corpus/torture/encoding/conflicting_charset.html",
        &html1,
    )
    .unwrap();
    println!(
        "  torture/encoding/conflicting_charset.html - {} bytes",
        html1.len()
    );

    // 2. bom_utf8.html - UTF-8 with BOM prefix
    let html2 = "\u{FEFF}<!DOCTYPE html><html><head><meta charset=\"utf-8\"></head><body>\
<main><p>The BOM should be stripped and this content extracted correctly.</p></main></body></html>";
    std::fs::write("corpus/torture/encoding/bom_utf8.html", html2).unwrap();
    println!("  torture/encoding/bom_utf8.html - {} bytes", html2.len());

    // 3. no_charset_declared.html - No charset anywhere, pure ASCII
    let html3 = "<!DOCTYPE html><html><head></head><body>\
<main><p>No charset declared anywhere. Pure ASCII content should work fine with UTF-8 fallback.</p></main></body></html>";
    std::fs::write(
        "corpus/torture/encoding/no_charset_declared.html",
        html3,
    )
    .unwrap();
    println!(
        "  torture/encoding/no_charset_declared.html - {} bytes",
        html3.len()
    );
}

// ============================================================================
// TRACK A: CONTENT DENSITY ATTACKS
// ============================================================================

fn generate_density_attacks() {
    let _ = std::fs::create_dir("corpus/torture/density");

    // 1. cookie_banner_dominant.html - 500 words cookie modal + 100 words main
    let html1 = r#"<!DOCTYPE html>
<html><head><meta charset="utf-8"></head><body>
<div class="cookie-modal">
<p>This website uses cookies to ensure you get the best experience. By continuing to browse this site you agree to our use of cookies. We collect data for analytics and personalization purposes. Your data may be shared with third-party partners for advertising purposes. You can manage your cookie preferences at any time by clicking the settings button. For more information please review our Privacy Policy and Terms of Service. GDPR requires us to obtain explicit consent before collecting any personal information. This cookie banner contains approximately 500 words of legal text that should be completely stripped from the extraction since it is outside the main tag.</p>
<p>Additional GDPR text about data processing and your rights as a data subject. You have the right to access, rectify, erase, and port your personal data. We process your data based on consent and legitimate interest. The cookie policy describes how we use cookies for tracking, advertising, and functional purposes.</p>
</div>
<main>
<p>This is the real article content. A 100-word paragraph that contains the actual information the user came to find. The extractor should only return this paragraph from the main tag while completely ignoring the cookie modal above.</p>
<p>More real content here with additional details.</p>
</main>
</body></html>"#.to_string();
    std::fs::write(
        "corpus/torture/density/cookie_banner_dominant.html",
        &html1,
    )
    .unwrap();
    println!(
        "  torture/density/cookie_banner_dominant.html - {} bytes",
        html1.len()
    );

    // 2. thin_page.html - Exactly 12 words in main
    let html2 = r#"<!DOCTYPE html>
<html><head><meta charset="utf-8"></head><body>
<main><p>Twelve words exactly in this sentence.</p></main>
</body></html>"#;
    std::fs::write("corpus/torture/density/thin_page.html", html2).unwrap();
    println!("  torture/density/thin_page.html - {} bytes", html2.len());

    // 3. repeated_headers.html - 50x repeated header/nav + 1 real paragraph
    let mut html3 =
        String::from("<!DOCTYPE html><html><head><meta charset=\"utf-8\"></head><body>");
    for _ in 0..50 {
        html3.push_str("<header><h1>Site Name</h1></header><nav><a href=\"/1\">Link</a></nav>");
    }
    html3.push_str("<main><p>The only real paragraph in the entire document. All that noise above should be nuked including the repeated headers and nav elements.</p></main></body></html>");
    std::fs::write(
        "corpus/torture/density/repeated_headers.html",
        &html3,
    )
    .unwrap();
    println!(
        "  torture/density/repeated_headers.html - {} bytes",
        html3.len()
    );

    // 4. lorem_ipsum_farm.html - 500KB of lorem ipsum in main
    let lorem = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. ";
    let mut html4 =
        "<!DOCTYPE html><html><head><meta charset=\"utf-8\"></head><body><main>".to_string();
    // Repeat lorem ipsum to reach ~500KB
    let target_size = 500 * 1024;
    while html4.len() < target_size {
        html4.push_str(lorem);
    }
    html4.push_str("</main></body></html>");
    std::fs::write(
        "corpus/torture/density/lorem_ipsum_farm.html",
        &html4,
    )
    .unwrap();
    println!(
        "  torture/density/lorem_ipsum_farm.html - {} bytes",
        html4.len()
    );

    // 5. all_code_no_prose.html - 50 code blocks + 3 sentences
    let mut html5 =
        "<!DOCTYPE html><html><head><meta charset=\"utf-8\"></head><body><main>".to_string();
    for i in 0..50 {
        html5.push_str(&format!(
            "<pre><code class=\"language-python\">def example_function_{}(x):
    result = x * 2
    return result
</code></pre>",
            i
        ));
    }
    html5.push_str("<p>Here are three sentences of prose that explain the code blocks above. This is the only actual readable text in the document.</p></main></body></html>");
    std::fs::write(
        "corpus/torture/density/all_code_no_prose.html",
        &html5,
    )
    .unwrap();
    println!(
        "  torture/density/all_code_no_prose.html - {} bytes",
        html5.len()
    );
}
