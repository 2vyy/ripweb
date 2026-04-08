use crate::config::family_hint_for_host;
use super::boilerplate::tag_attribute;
use url::Url;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageFamily {
    Docs,
    Article,
    Product,
    Generic,
}

#[derive(Default)]
pub struct TextStats {
    pub word_count: usize,
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

pub fn host_family_hint(source_url: &str) -> Option<PageFamily> {
    let host = Url::parse(source_url).ok()?.host_str()?.to_ascii_lowercase();
    match family_hint_for_host(&host)? {
        "docs" => Some(PageFamily::Docs),
        "article" => Some(PageFamily::Article),
        "product" => Some(PageFamily::Product),
        _ => Some(PageFamily::Generic),
    }
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
        PageFamily::Generic => 0,
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
