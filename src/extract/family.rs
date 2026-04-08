use crate::config::family_hint_for_host;
use super::boilerplate::tag_attribute;
use url::Url;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageFamily {
    Docs,
    Article,
    Product,
    Listing,
    Search,
    Forum,
    Generic,
}

#[derive(Default)]
pub struct TextStats {
    pub word_count: usize,
    pub total_text_len: usize,
    pub headings: usize,
    pub paragraphs: usize,
    pub code_fences: usize,
    pub list_items: usize,
    pub link_count: usize,
    pub short_lines: usize,
}

const DOC_HINTS: &[&str] = &[
    "doc", "docs", "reference", "api", "manual", "guide", "guides", "tutorial", "learn",
    "developer", "developers", "readthedocs", "gitbook", "docusaurus",
];
const ARTICLE_HINTS: &[&str] = &[
    "article", "post", "story", "blog", "news", "entry", "content", "prose",
];
const PRODUCT_HINTS: &[&str] = &[
    "product", "pdp", "buybox", "price", "pricing", "spec", "specs", "sku", "item",
    "details", "purchase", "cart", "merchant", "offer",
];

pub fn url_family_hint(source_url: &str) -> Option<PageFamily> {
    let url = Url::parse(source_url).ok()?;
    let host = url.host_str()?.to_ascii_lowercase();
    let path = url.path().to_ascii_lowercase();

    // 1. Host-based hints from config
    if let Some(hint) = family_hint_for_host(&host) {
        match hint {
            "docs" => return Some(PageFamily::Docs),
            "article" => return Some(PageFamily::Article),
            "product" => return Some(PageFamily::Product),
            _ => {}
        }
    }

    // 2. Path-based common patterns
    if path.contains("/docs/") || path.contains("/wiki/") || path.contains("/guide/") || path.contains("/tutorial/") {
        return Some(PageFamily::Docs);
    }
    if path.contains("/p/") || path.contains("/product/") || path.contains("/item/") || path.contains("/shop/") {
        return Some(PageFamily::Product);
    }
    if path.contains("/blog/") || path.contains("/article/") || path.contains("/news/") || path.contains("/story/") {
        return Some(PageFamily::Article);
    }
    if path.contains("/search") || path.contains("/q/") {
        return Some(PageFamily::Search);
    }
    if path.contains("/forum/") || path.contains("/thread/") || path.contains("/discuss/") {
        return Some(PageFamily::Forum);
    }

    None
}

pub fn detect_family(dom: &tl::VDom, url_hint: PageFamily) -> PageFamily {
    if url_hint != PageFamily::Generic {
        return url_hint;
    }

    let parser = dom.parser();

    // 1. Meta tag hints (OpenGraph, Schema.org)
    let og_type = extract_meta_property(dom, "og:type");
    let schema_type = extract_meta_itemtype(dom);

    if let Some(t) = og_type.as_deref() {
        match t {
            "article" => return PageFamily::Article,
            "product" => return PageFamily::Product,
            "website" | "object" => {}
            _ => {}
        }
    }

    if let Some(t) = schema_type.as_deref() {
        if t.contains("Article") || t.contains("BlogPosting") || t.contains("NewsArticle") {
            return PageFamily::Article;
        }
        if t.contains("Product") {
            return PageFamily::Product;
        }
        if t.contains("SearchResultsPage") {
            return PageFamily::Search;
        }
        if t.contains("ItemList") {
            return PageFamily::Listing;
        }
    }

    // 2. Global DOM hints
    if dom.query_selector(r#"input[type="search"]"#).map(|mut i| i.next()).is_some()
        && dom.query_selector("form").map(|mut i| i.next()).is_some()
    {
        // Simple search page heuristic: presence of search input and results cards
        let result_hints = ["result", "hit", "item", "product", "listing"];
        let mut has_results = false;
        for hint in result_hints {
            let selector = format!(r#"div[class*="{}"]"#, hint);
            if dom.query_selector(&selector).map(|mut i| i.next()).is_some() {
                has_results = true;
                break;
            }
        }
        if has_results {
            return PageFamily::Search;
        }
    }

    PageFamily::Generic
}

fn extract_meta_property(dom: &tl::VDom, property: &str) -> Option<String> {
    let selector = format!(r#"meta[property="{}"]"#, property);
    let mut iter = dom.query_selector(&selector)?;
    let handle = iter.next()?;
    let tag = handle.get(dom.parser())?.as_tag()?;
    tag_attribute(tag, "content")
}

fn extract_meta_itemtype(dom: &tl::VDom) -> Option<String> {
    let mut iter = dom.query_selector("[itemtype]")?;
    let handle = iter.next()?;
    let tag = handle.get(dom.parser())?.as_tag()?;
    tag_attribute(tag, "itemtype")
}

pub fn classify_candidate_family(
    tag: &tl::HTMLTag,
    rendered: &str,
    stats: &TextStats,
    url_family: PageFamily,
) -> PageFamily {
    if url_family != PageFamily::Generic {
        return url_family;
    }

    let hint_text = [tag_attribute(tag, "id"), tag_attribute(tag, "class")]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();

    let code_heavy = stats.code_fences > 0
        || (stats.headings >= 3 && stats.list_items >= 3 && stats.link_count >= 8);
    let prose_heavy = stats.paragraphs >= 3 && stats.word_count >= 180;
    let price_markers = count_price_markers(rendered);
    let spec_markers = count_spec_markers(rendered);
    let productish =
        price_markers > 0 && (spec_markers > 0 || stats.list_items >= 2 || stats.headings >= 1);

    if DOC_HINTS.iter().any(|hint| hint_text.contains(hint)) || code_heavy {
        return PageFamily::Docs;
    }
    if PRODUCT_HINTS.iter().any(|hint| hint_text.contains(hint)) || productish {
        return PageFamily::Product;
    }
    if ARTICLE_HINTS.iter().any(|hint| hint_text.contains(hint)) || prose_heavy {
        return PageFamily::Article;
    }

    PageFamily::Generic
}

pub fn family_score_adjustment(
    family: PageFamily,
    stats: &TextStats,
    price_markers: usize,
    spec_markers: usize,
) -> i64 {
    match family {
        PageFamily::Docs => {
            (stats.headings as i64) * 12
                + (stats.code_fences as i64) * 18
                + (stats.list_items as i64) * 4
                - (stats.short_lines as i64)
        }
        PageFamily::Article => {
            (stats.paragraphs as i64) * 14
                + (stats.word_count as i64 / 20)
                - (stats.link_count as i64) * 2
        }
        PageFamily::Product => {
            (stats.headings as i64) * 16
                + (stats.list_items as i64) * 12
                + (price_markers as i64) * 40
                + (spec_markers as i64) * 22
                - (stats.link_count as i64) * 4
        }
        PageFamily::Listing | PageFamily::Search | PageFamily::Forum | PageFamily::Generic => 0,
    }
}

pub fn count_price_markers(text: &str) -> usize {
    text.lines()
        .filter(|line| {
            let trimmed = line.trim();
            trimmed.contains('$')
                || trimmed.contains("Current price")
                || trimmed.contains("Sale price")
                || trimmed.contains("Price when purchased")
        })
        .count()
}

pub fn count_spec_markers(text: &str) -> usize {
    const SPEC_HINTS: &[&str] = &[
        "specifications",
        "specs",
        "product details",
        "about this item",
        "key features",
        "warranty",
        "dimensions",
        "brand",
        "model",
        "isbn",
        "sku",
    ];
    text.lines()
        .filter(|line| {
            let lower = line.trim().to_ascii_lowercase();
            SPEC_HINTS.iter().any(|hint| lower.contains(hint))
        })
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extract::render::{render_tag, cleanup_markdown};
    use crate::extract::candidate::score_text;

    #[test]
    fn classifies_docs_candidates() {
        let html = r#"<html><body>
          <div class="docs reference-content">
            <h1>API Reference</h1><h2>Example</h2>
            <pre><code>fn main() { println!("hi"); }</code></pre>
            <ul><li>Item</li><li>Other</li></ul>
          </div></body></html>"#;
        let dom = tl::parse(html, tl::ParserOptions::default()).unwrap();
        let parser = dom.parser();
        let tag = dom.query_selector("div")
            .and_then(|mut it| it.next())
            .and_then(|h| h.get(parser))
            .and_then(|n| n.as_tag())
            .unwrap();
        let rendered = cleanup_markdown(&render_tag(tag, parser));
        let stats = score_text(&rendered);
        assert_eq!(classify_candidate_family(tag, &rendered, &stats, PageFamily::Generic), PageFamily::Docs);
    }

    #[test]
    fn classifies_article_candidates() {
        let html = r#"<html><body>
          <article class="story-body">
            <h1>Story Title</h1>
            <p>One two three four five six seven eight nine ten eleven twelve thirteen fourteen fifteen sixteen seventeen eighteen nineteen twenty twenty-one twenty-two twenty-three twenty-four.</p>
            <p>Another paragraph with enough prose to look like a real article rather than a sparse card or utility block.</p>
            <p>A third paragraph keeps the article heuristic clearly on the prose-heavy side for classification.</p>
          </article></body></html>"#;
        let dom = tl::parse(html, tl::ParserOptions::default()).unwrap();
        let parser = dom.parser();
        let tag = dom.query_selector("article")
            .and_then(|mut it| it.next())
            .and_then(|h| h.get(parser))
            .and_then(|n| n.as_tag())
            .unwrap();
        let rendered = cleanup_markdown(&render_tag(tag, parser));
        let stats = score_text(&rendered);
        assert_eq!(classify_candidate_family(tag, &rendered, &stats, PageFamily::Generic), PageFamily::Article);
    }

    #[test]
    fn classifies_product_candidates() {
        let html = r#"<html><body>
          <section class="product-details buybox">
            <h1>Ip Man 1-4 (Box Set) (Blu-ray)</h1>
            <p>Current price is USD$22.99</p>
            <h2>Key item features</h2>
            <ul><li>Action, Biography, Drama</li><li>Movie &amp; tv media format: Blu-ray</li></ul>
            <h2>Specifications</h2>
            <table><tr><th>Director</th><td>Wilson Yip</td></tr><tr><th>Resolution</th><td>1080p</td></tr></table>
          </section></body></html>"#;
        let dom = tl::parse(html, tl::ParserOptions::default()).unwrap();
        let parser = dom.parser();
        let tag = dom.query_selector("section")
            .and_then(|mut it| it.next())
            .and_then(|h| h.get(parser))
            .and_then(|n| n.as_tag())
            .unwrap();
        let rendered = cleanup_markdown(&render_tag(tag, parser));
        let stats = score_text(&rendered);
        assert_eq!(classify_candidate_family(tag, &rendered, &stats, PageFamily::Generic), PageFamily::Product);
    }
}
