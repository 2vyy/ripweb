/// Tags whose entire subtrees are stripped before content extraction.
pub const NUKE_TAGS: &[&str] = &[
    "nav", "footer", "header", "aside", "style", "svg", "iframe", "form", "script", "noscript",
];

pub const NEGATIVE_HINTS: &[&str] = &[
    // Navigation and structure
    "nav", "menu", "sidebar", "footer", "header", "breadcrumb", "sitemap", "toolbar",
    // Overlays and interruptions
    "cookie", "modal", "popup", "banner", "notice", "disclosure", "overlay",
    // Social and sharing
    "share", "social",
    // Supplementary content
    "related", "recommend", "promo", "sponsor", "ad-", "advert",
    // Subscription and utility
    "utility", "newsletter", "subscribe",
    // Dynamic content containers
    "carousel", "slider",
    // Docs-specific noise
    "toc", "table-of-contents", "on-this-page", "in-this-article",
    // Blog/news metadata noise
    "byline", "author-bio", "tags", "tag-list", "pagination", "page-nav",
    // UI widgets and annotations
    "widget", "chip", "pill", "annotation", "tooltip",
    // Comments section
    "comment",
    // Copyright / legal
    "copyright", "legal",
];

/// Extract a named attribute value from an HTML tag.
pub fn tag_attribute(tag: &tl::HTMLTag, name: &str) -> Option<String> {
    tag.attributes()
        .get(name)
        .flatten()
        .map(|value| value.as_utf8_str().to_string())
}

/// Returns true when a tag's id/class hints match a known boilerplate pattern.
pub fn should_strip_subtree(tag: &tl::HTMLTag) -> bool {
    let name = tag.name().as_utf8_str().to_ascii_lowercase();
    if !matches!(
        name.as_str(),
        "div" | "section" | "main" | "article" | "aside" | "ul" | "ol" | "li"
    ) {
        return false;
    }

    let hint_text = [tag_attribute(tag, "id"), tag_attribute(tag, "class")]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();

    if hint_text.is_empty() {
        return false;
    }

    NEGATIVE_HINTS.iter().any(|hint| hint_text.contains(hint))
}
