/// Tags whose entire subtrees are stripped before content extraction.
pub const NUKE_TAGS: &[&str] = &[
    "nav", "footer", "header", "aside", "style", "svg", "iframe", "form", "script", "noscript",
];

pub const NEGATIVE_HINTS: &[&str] = &[
    "nav", "menu", "sidebar", "footer", "header", "cookie", "modal", "popup", "banner", "share",
    "social", "breadcrumb", "comment", "related", "recommend", "promo", "advert", "ad-",
    "utility", "toolbar", "newsletter", "subscribe", "sitemap", "carousel", "slider",
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
