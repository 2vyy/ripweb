use crate::config::family_hint_for_host;
use crate::error::RipwebError;
use crate::minify::strip_tracking;
use super::Extractor;
use encoding_rs::Encoding;
use url::Url;

pub struct WebExtractor;

const MAX_INPUT_BYTES: usize = 5 * 1024 * 1024;

/// Tags whose entire subtrees are stripped before content extraction.
const NUKE_TAGS: &[&str] = &[
    "nav",
    "footer",
    "header",
    "aside",
    "style",
    "svg",
    "iframe",
    "form",
    "script",
    "noscript",
];

/// CSS-like priority order for content root selection.
const CONTENT_ROOTS: &[&str] = &["main", "article"];
const FALLBACK_CANDIDATE_TAGS: &[&str] = &["section", "table"];
const POSITIVE_HINTS: &[&str] = &[
    "article", "content", "main", "post", "entry", "body", "text", "doc", "docs", "markdown",
    "prose", "story",
];
const NEGATIVE_HINTS: &[&str] = &[
    "nav", "menu", "sidebar", "footer", "header", "cookie", "modal", "popup", "banner", "share",
    "social", "breadcrumb", "comment", "related", "recommend", "promo", "advert", "ad-",
    "utility", "toolbar", "newsletter", "subscribe", "sitemap", "carousel", "slider",
];
const HINTED_DIV_SELECTORS: &[&str] = &[
    r#"div[id*="content"]"#,
    r#"div[class*="content"]"#,
    r#"div[id*="article"]"#,
    r#"div[class*="article"]"#,
    r#"div[id*="post"]"#,
    r#"div[class*="post"]"#,
    r#"div[id*="entry"]"#,
    r#"div[class*="entry"]"#,
    r#"div[id*="doc"]"#,
    r#"div[class*="doc"]"#,
    r#"div[id*="markdown"]"#,
    r#"div[class*="markdown"]"#,
    r#"div[id*="prose"]"#,
    r#"div[class*="prose"]"#,
    r#"div[id*="story"]"#,
    r#"div[class*="story"]"#,
];
const MAX_FALLBACK_CANDIDATES_PER_SELECTOR: usize = 32;
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

impl Extractor for WebExtractor {
    fn extract(bytes: &[u8], content_type: Option<&str>) -> Result<String, RipwebError> {
        if bytes.len() > MAX_INPUT_BYTES {
            return Err(RipwebError::InputTooLarge(bytes.len()));
        }
        let html = decode_charset(bytes, content_type);
        Ok(extract_from_str(&html, None))
    }
}

impl WebExtractor {
    pub fn extract_with_url(
        bytes: &[u8],
        content_type: Option<&str>,
        source_url: Option<&str>,
    ) -> Result<String, RipwebError> {
        if bytes.len() > MAX_INPUT_BYTES {
            return Err(RipwebError::InputTooLarge(bytes.len()));
        }
        let html = decode_charset(bytes, content_type);
        Ok(extract_from_str(&html, source_url))
    }
}

fn extract_from_str(html: &str, source_url: Option<&str>) -> String {
    let dom = match tl::parse(html, tl::ParserOptions::default()) {
        Ok(d) => d,
        Err(_) => return String::new(),
    };
    let text = extract_best_candidate(&dom, source_url);

    if word_count(&text) < 100
        && let Some(spa) = extract_next_data(&dom).filter(|s| word_count(s) > word_count(&text))
    {
        return cleanup_markdown(&spa);
    }

    text
}

fn extract_best_candidate(dom: &tl::VDom, source_url: Option<&str>) -> String {
    let parser = dom.parser();
    let mut best: Option<ScoredCandidate> = None;
    let url_family = source_url
        .and_then(host_family_hint)
        .unwrap_or(PageFamily::Generic);

    for selector in CONTENT_ROOTS {
        if let Some(hits) = dom.query_selector(selector) {
            for handle in hits {
                let Some(node) = handle.get(parser) else {
                    continue;
                };
                if let Some(tag) = node.as_tag() {
                    consider_candidate(&mut best, score_candidate(tag, parser, url_family));
                }
            }
        }
    }

    for selector in FALLBACK_CANDIDATE_TAGS {
        if let Some(hits) = dom.query_selector(selector) {
            for handle in hits.take(MAX_FALLBACK_CANDIDATES_PER_SELECTOR) {
                let Some(node) = handle.get(parser) else {
                    continue;
                };
                if let Some(tag) = node.as_tag() {
                    consider_candidate(&mut best, score_candidate(tag, parser, url_family));
                }
            }
        }
    }

    for selector in HINTED_DIV_SELECTORS {
        if let Some(hits) = dom.query_selector(selector) {
            for handle in hits.take(MAX_FALLBACK_CANDIDATES_PER_SELECTOR) {
                let Some(node) = handle.get(parser) else {
                    continue;
                };
                if let Some(tag) = node.as_tag() {
                    consider_candidate(&mut best, score_candidate(tag, parser, url_family));
                }
            }
        }
    }

    if let Some(body) = dom
        .query_selector("body")
        .and_then(|mut hits| hits.next())
        .and_then(|handle| handle.get(parser))
        .and_then(|node| node.as_tag())
    {
        consider_candidate(&mut best, score_candidate(body, parser, url_family));
    }

    best.map(|candidate| candidate.text)
        .unwrap_or_else(|| cleanup_markdown(&extract_body_markdown(dom)))
}

#[derive(Debug)]
struct ScoredCandidate {
    score: i64,
    text: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PageFamily {
    Docs,
    Article,
    Product,
    Generic,
}

fn consider_candidate(best: &mut Option<ScoredCandidate>, candidate: Option<ScoredCandidate>) {
    let Some(candidate) = candidate else { return };
    if best.as_ref().is_none_or(|current| candidate.score > current.score) {
        *best = Some(candidate);
    }
}

fn score_candidate(
    tag: &tl::HTMLTag,
    parser: &tl::Parser,
    url_family: PageFamily,
) -> Option<ScoredCandidate> {
    let name = tag.name().as_utf8_str().to_ascii_lowercase();
    if should_strip_subtree(tag) || NUKE_TAGS.contains(&name.as_str()) {
        return None;
    }

    let text = cleanup_markdown(&render_tag(tag, parser));
    if text.is_empty() {
        return None;
    }

    let stats = score_text(&text);
    if stats.word_count == 0 {
        return None;
    }

    let family = classify_candidate_family(tag, &text, &stats, url_family);
    let price_markers = count_price_markers(&text);
    let spec_markers = count_spec_markers(&text);

    let mut score = stats.word_count as i64;
    score += (stats.paragraphs as i64) * 24;
    score += (stats.headings as i64) * 18;
    score += (stats.code_fences as i64) * 20;
    score += (stats.list_items as i64) * 10;
    score -= (stats.link_count as i64) * 6;
    score -= (stats.short_lines as i64) * 2;

    score += match name.as_str() {
        "article" => 80,
        "main" => 60,
        "section" => 20,
        "div" => 10,
        "table" => 12,
        "body" => -40,
        _ => 0,
    };

    let hint_text = [tag_attribute(tag, "id"), tag_attribute(tag, "class")]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();

    for hint in POSITIVE_HINTS {
        if hint_text.contains(hint) {
            score += 24;
        }
    }
    for hint in PRODUCT_HINTS {
        if hint_text.contains(hint) {
            score += 20;
        }
    }
    for hint in NEGATIVE_HINTS {
        if hint_text.contains(hint) {
            score -= 60;
        }
    }

    score += family_score_adjustment(family, &stats, price_markers, spec_markers);

    Some(ScoredCandidate { score, text })
}

fn classify_candidate_family(
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
    let productish = price_markers > 0 && (spec_markers > 0 || stats.list_items >= 2 || stats.headings >= 1);

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

fn host_family_hint(source_url: &str) -> Option<PageFamily> {
    let host = Url::parse(source_url).ok()?.host_str()?.to_ascii_lowercase();
    match family_hint_for_host(&host)? {
        "docs" => Some(PageFamily::Docs),
        "article" => Some(PageFamily::Article),
        "product" => Some(PageFamily::Product),
        _ => Some(PageFamily::Generic),
    }
}

fn family_score_adjustment(
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

fn count_price_markers(text: &str) -> usize {
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

fn count_spec_markers(text: &str) -> usize {
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

fn tag_attribute(tag: &tl::HTMLTag, name: &str) -> Option<String> {
    tag.attributes()
        .get(name)
        .flatten()
        .map(|value| value.as_utf8_str().to_string())
}

#[derive(Default)]
struct TextStats {
    word_count: usize,
    headings: usize,
    paragraphs: usize,
    code_fences: usize,
    list_items: usize,
    link_count: usize,
    short_lines: usize,
}

fn score_text(text: &str) -> TextStats {
    let mut stats = TextStats {
        word_count: word_count(text),
        ..TextStats::default()
    };

    let mut in_code_fence = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            stats.code_fences += 1;
            in_code_fence = !in_code_fence;
            continue;
        }
        if in_code_fence {
            continue;
        }
        if trimmed.starts_with('#') {
            stats.headings += 1;
        }
        if trimmed.starts_with("- ") || ordered_list_prefix(trimmed) {
            stats.list_items += 1;
        }
        if !trimmed.is_empty() && trimmed.len() < 35 {
            stats.short_lines += 1;
        }
        stats.link_count += trimmed.matches("](").count();
    }

    stats.paragraphs = text
        .split("\n\n")
        .filter(|chunk| !chunk.trim().is_empty())
        .count();

    stats
}

fn ordered_list_prefix(line: &str) -> bool {
    let digits = line.bytes().take_while(|b| b.is_ascii_digit()).count();
    digits > 0 && line[digits..].starts_with(". ")
}

fn render_markdown(node: &tl::Node, parser: &tl::Parser) -> String {
    match node {
        tl::Node::Tag(tag) => render_tag(tag, parser),
        tl::Node::Raw(bytes) => normalize_text(&bytes.as_utf8_str()),
        tl::Node::Comment(_) => String::new(),
    }
}

fn render_tag(tag: &tl::HTMLTag, parser: &tl::Parser) -> String {
    let name = tag.name().as_utf8_str().to_ascii_lowercase();

    if should_strip_subtree(tag) || NUKE_TAGS.contains(&name.as_str()) {
        return String::new();
    }

    match name.as_str() {
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => render_heading(tag, parser, &name),
        "p" => wrap_block(render_children_inline(tag, parser)),
        "br" => "\n".to_owned(),
        "hr" => "\n\n---\n\n".to_owned(),
        "pre" => render_pre(tag, parser),
        "code" => render_inline_code(tag, parser),
        "a" => render_link(tag, parser),
        "em" | "i" => wrap_inline_marker("*", render_children_inline(tag, parser)),
        "strong" | "b" => wrap_inline_marker("**", render_children_inline(tag, parser)),
        "blockquote" => render_blockquote(tag, parser),
        "ul" => render_list(tag, parser, false),
        "ol" => render_list(tag, parser, true),
        "li" => render_children_inline(tag, parser),
        "img" => render_image(tag),
        "table" => render_table(tag, parser),
        "thead" | "tbody" | "tfoot" | "tr" => render_children_blocks(tag, parser),
        "th" | "td" => render_children_inline(tag, parser),
        "main" | "article" | "section" | "div" | "body" => render_children_blocks(tag, parser),
        "span" | "small" | "time" | "label" | "summary" | "details" => {
            render_children_inline(tag, parser)
        }
        _ => render_children_blocks(tag, parser),
    }
}

fn render_heading(tag: &tl::HTMLTag, parser: &tl::Parser, name: &str) -> String {
    let level = name
        .strip_prefix('h')
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(1)
        .clamp(1, 6);
    let text = render_children_inline(tag, parser);
    if text.is_empty() {
        String::new()
    } else {
        format!("\n\n{} {}\n\n", "#".repeat(level), text)
    }
}

fn render_pre(tag: &tl::HTMLTag, parser: &tl::Parser) -> String {
    let code = tag.inner_text(parser).replace('\r', "");
    let trimmed = code.trim_matches('\n');
    if trimmed.trim().is_empty() {
        String::new()
    } else {
        format!("\n\n```\n{}\n```\n\n", trimmed)
    }
}

fn render_inline_code(tag: &tl::HTMLTag, parser: &tl::Parser) -> String {
    let code = normalize_text(&tag.inner_text(parser));
    if code.is_empty() {
        String::new()
    } else {
        format!("`{code}`")
    }
}

fn render_link(tag: &tl::HTMLTag, parser: &tl::Parser) -> String {
    let text = render_children_inline(tag, parser);
    let href = tag
        .attributes()
        .get("href")
        .flatten()
        .map(|v| v.as_utf8_str().to_string())
        .unwrap_or_default();

    if href.is_empty() {
        return text;
    }

    let href = strip_tracking(&href);
    let label = if text.is_empty() { href.clone() } else { text };
    format!("[{label}]({href})")
}

fn render_blockquote(tag: &tl::HTMLTag, parser: &tl::Parser) -> String {
    let inner = cleanup_markdown(&render_children_blocks(tag, parser));
    if inner.is_empty() {
        return String::new();
    }

    let quoted = inner
        .lines()
        .map(|line| {
            if line.is_empty() {
                ">".to_owned()
            } else {
                format!("> {line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!("\n\n{quoted}\n\n")
}

fn render_list(tag: &tl::HTMLTag, parser: &tl::Parser, ordered: bool) -> String {
    let mut items = Vec::new();

    for handle in tag.children().top().iter() {
        let Some(child) = handle.get(parser) else {
            continue;
        };
        let Some(li) = child.as_tag() else {
            continue;
        };
        if li.name().as_utf8_str().to_ascii_lowercase() != "li" {
            continue;
        }

        let content = cleanup_markdown(&render_children_blocks(li, parser));
        if content.is_empty() {
            continue;
        }

        let marker = if ordered {
            format!("{}. ", items.len() + 1)
        } else {
            "- ".to_owned()
        };
        items.push(indent_block(&content, &marker, "  "));
    }

    if items.is_empty() {
        String::new()
    } else {
        format!("\n\n{}\n\n", items.join("\n"))
    }
}

fn render_image(tag: &tl::HTMLTag) -> String {
    tag.attributes()
        .get("alt")
        .flatten()
        .map(|alt| normalize_text(&alt.as_utf8_str()))
        .filter(|alt| !alt.is_empty())
        .map(|alt| format!("[Image: {alt}]"))
        .unwrap_or_default()
}

fn render_table(tag: &tl::HTMLTag, parser: &tl::Parser) -> String {
    let rows = collect_table_rows(tag, parser);
    if rows.is_empty() {
        return String::new();
    }

    let mut out = String::from("\n\n");
    for row in rows {
        out.push_str("| ");
        out.push_str(&row.join(" | "));
        out.push_str(" |\n");
    }
    out.push('\n');
    out
}

fn collect_table_rows(tag: &tl::HTMLTag, parser: &tl::Parser) -> Vec<Vec<String>> {
    let mut rows = Vec::new();

    for handle in tag.children().top().iter() {
        let Some(child) = handle.get(parser) else {
            continue;
        };
        let Some(child_tag) = child.as_tag() else {
            continue;
        };
        let name = child_tag.name().as_utf8_str().to_ascii_lowercase();
        match name.as_str() {
            "thead" | "tbody" | "tfoot" => rows.extend(collect_table_rows(child_tag, parser)),
            "tr" => {
                let mut cells = Vec::new();
                for cell_handle in child_tag.children().top().iter() {
                    let Some(cell_node) = cell_handle.get(parser) else {
                        continue;
                    };
                    let Some(cell_tag) = cell_node.as_tag() else {
                        continue;
                    };
                    let cell_name = cell_tag.name().as_utf8_str().to_ascii_lowercase();
                    if !matches!(cell_name.as_str(), "th" | "td") {
                        continue;
                    }
                    let cell = render_children_inline(cell_tag, parser);
                    if !cell.is_empty() {
                        cells.push(cell);
                    }
                }
                if !cells.is_empty() {
                    rows.push(cells);
                }
            }
            _ => {}
        }
    }

    rows
}

fn render_children_blocks(tag: &tl::HTMLTag, parser: &tl::Parser) -> String {
    let mut out = String::new();

    for handle in tag.children().top().iter() {
        let Some(child) = handle.get(parser) else {
            continue;
        };
        let rendered = render_markdown(child, parser);
        if rendered.is_empty() {
            continue;
        }

        if !out.is_empty() && !out.ends_with('\n') && !rendered.starts_with('\n') {
            out.push(' ');
        }
        out.push_str(&rendered);
    }

    out
}

fn should_strip_subtree(tag: &tl::HTMLTag) -> bool {
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

fn render_children_inline(tag: &tl::HTMLTag, parser: &tl::Parser) -> String {
    let parts = tag
        .children()
        .top()
        .iter()
        .filter_map(|handle| handle.get(parser))
        .map(|child| render_markdown(child, parser))
        .filter(|part| !part.trim().is_empty())
        .collect::<Vec<_>>();

    normalize_inline_spacing(&parts.join(" "))
}

fn normalize_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn normalize_inline_spacing(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == ' ' {
            let next = chars.peek().copied();
            if out.ends_with(' ')
                || matches!(next, Some(',' | '.' | ';' | ':' | '!' | '?' | ')' | ']' | '}'))
                || matches!(out.chars().last(), Some('(' | '[' | '{' | '\n'))
            {
                continue;
            }
        }
        out.push(ch);
    }

    out.trim().to_owned()
}

fn wrap_inline_marker(marker: &str, text: String) -> String {
    if text.is_empty() {
        String::new()
    } else {
        format!("{marker}{text}{marker}")
    }
}

fn wrap_block(text: String) -> String {
    if text.is_empty() {
        String::new()
    } else {
        format!("\n\n{text}\n\n")
    }
}

fn indent_block(text: &str, first_prefix: &str, rest_prefix: &str) -> String {
    text.lines()
        .enumerate()
        .map(|(idx, line)| {
            if idx == 0 {
                format!("{first_prefix}{line}")
            } else if line.is_empty() {
                rest_prefix.to_owned()
            } else {
                format!("{rest_prefix}{line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn cleanup_markdown(text: &str) -> String {
    let mut out = String::new();
    let mut blank_run = 0usize;

    for line in text.lines() {
        let line = line.trim_end();
        if line.is_empty() {
            blank_run += 1;
            if blank_run <= 2 && !out.is_empty() {
                out.push('\n');
            }
            continue;
        }

        blank_run = 0;
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str(line);
    }

    out.trim().to_owned()
}

/// Extract visible text from the `<body>`, applying the nuke list globally.
fn extract_body_markdown(dom: &tl::VDom) -> String {
    let parser = dom.parser();
    if let Some(text) = dom
        .query_selector("body")
        .and_then(|mut hits| hits.next())
        .and_then(|handle| handle.get(parser))
        .map(|node| render_markdown(node, parser))
    {
        return text;
    }

    dom.nodes()
        .iter()
        .map(|n| render_markdown(n, parser))
        .filter(|s| !s.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Extract content from `<script id="__NEXT_DATA__">` JSON payload.
/// Walks every string value in the JSON tree and concatenates them.
fn extract_next_data(dom: &tl::VDom) -> Option<String> {
    let parser = dom.parser();
    let mut iter = dom.query_selector(r#"script[id="__NEXT_DATA__"]"#)?;
    let handle = iter.next()?;
    let node = handle.get(parser)?;
    let tag = node.as_tag()?;
    let raw_json = tag.inner_text(parser);

    let value: serde_json::Value = serde_json::from_str(&raw_json).ok()?;
    let mut parts: Vec<String> = Vec::new();
    collect_json_strings(&value, &mut parts);
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n\n"))
    }
}

/// Returns true when a JSON string leaf looks like prose rather than metadata.
#[inline]
fn is_content_leaf(s: &str) -> bool {
    s.len() > 20 && s.bytes().filter(|&b| b == b' ').count() >= 2
}

/// Depth-first walk of a JSON value, collecting prose string leaves.
fn collect_json_strings(value: &serde_json::Value, out: &mut Vec<String>) {
    match value {
        serde_json::Value::String(s) => {
            let trimmed = s.trim().to_owned();
            if is_content_leaf(&trimmed) {
                out.push(trimmed);
            }
        }
        serde_json::Value::Array(arr) => {
            for v in arr {
                collect_json_strings(v, out);
            }
        }
        serde_json::Value::Object(map) => {
            for v in map.values() {
                collect_json_strings(v, out);
            }
        }
        _ => {}
    }
}

/// Count whitespace-delimited words.
fn word_count(s: &str) -> usize {
    s.split_whitespace().count()
}

// ── Charset decoding ─────────────────────────────────────────────────────────

/// Decode raw bytes to UTF-8, consulting the Content-Type header first,
/// then the in-document `<meta charset>` tag, then assuming UTF-8.
fn decode_charset(bytes: &[u8], content_type: Option<&str>) -> String {
    let encoding = content_type
        .and_then(charset_from_content_type)
        .or_else(|| charset_from_meta(bytes))
        .unwrap_or(encoding_rs::UTF_8);

    let (cow, _, _) = encoding.decode(bytes);
    cow.into_owned()
}

/// Parse `charset=` out of a `Content-Type` header value.
fn charset_from_content_type(ct: &str) -> Option<&'static Encoding> {
    ct.split(';').skip(1).find_map(|param| {
        let param = param.trim();
        let (key, val) = param.split_once('=')?;
        if key.trim().eq_ignore_ascii_case("charset") {
            Encoding::for_label(val.trim().trim_matches('"').as_bytes())
        } else {
            None
        }
    })
}

/// Scan the first ~1 KiB of raw bytes for a `<meta charset="...">` or
/// `<meta http-equiv="Content-Type" content="...; charset=...">` tag.
fn charset_from_meta(bytes: &[u8]) -> Option<&'static Encoding> {
    let head = &bytes[..bytes.len().min(4096)];
    let head_str = String::from_utf8_lossy(head);
    let lower = head_str.to_ascii_lowercase();
    let idx = lower.find("charset")?;
    let after = lower[idx + 7..].trim_start();
    let after = after.strip_prefix('=')?;
    let after = after.trim_start().trim_start_matches('"');
    let end = after
        .find(|c: char| c == '"' || c == '\'' || c == ';' || c.is_whitespace())
        .unwrap_or(after.len().min(32));
    let label = &after[..end];
    Encoding::for_label(label.as_bytes())
}

// ── Unit tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_count_empty() {
        assert_eq!(word_count(""), 0);
        assert_eq!(word_count("   \n\t  "), 0);
    }

    #[test]
    fn word_count_basic() {
        assert_eq!(word_count("hello world foo"), 3);
    }

    #[test]
    fn charset_from_content_type_parses_label() {
        let enc = charset_from_content_type("text/html; charset=Shift_JIS");
        assert!(enc.is_some());
        assert_eq!(enc.unwrap().name(), "Shift_JIS");
    }

    #[test]
    fn charset_from_content_type_handles_quoted_value() {
        let enc = charset_from_content_type("text/html; charset=\"utf-8\"");
        assert!(enc.is_some());
        assert_eq!(enc.unwrap().name(), "UTF-8");
    }

    #[test]
    fn charset_from_meta_detects_utf8() {
        let html = b"<head><meta charset=\"utf-8\"></head>";
        let enc = charset_from_meta(html);
        assert!(enc.is_some());
        assert_eq!(enc.unwrap().name(), "UTF-8");
    }

    #[test]
    fn classifies_docs_candidates() {
        let html = r#"
        <html><body>
          <div class="docs reference-content">
            <h1>API Reference</h1>
            <h2>Example</h2>
            <pre><code>fn main() { println!("hi"); }</code></pre>
            <ul><li>Item</li><li>Other</li></ul>
          </div>
        </body></html>
        "#;
        let dom = tl::parse(html, tl::ParserOptions::default()).unwrap();
        let parser = dom.parser();
        let tag = dom
            .query_selector("div")
            .and_then(|mut it| it.next())
            .and_then(|h| h.get(parser))
            .and_then(|n| n.as_tag())
            .unwrap();
        let rendered = cleanup_markdown(&render_tag(tag, parser));
        let stats = score_text(&rendered);
        assert_eq!(
            classify_candidate_family(tag, &rendered, &stats, PageFamily::Generic),
            PageFamily::Docs
        );
    }

    #[test]
    fn classifies_article_candidates() {
        let html = r#"
        <html><body>
          <article class="story-body">
            <h1>Story Title</h1>
            <p>One two three four five six seven eight nine ten eleven twelve thirteen fourteen fifteen sixteen seventeen eighteen nineteen twenty twenty-one twenty-two twenty-three twenty-four.</p>
            <p>Another paragraph with enough prose to look like a real article rather than a sparse card or utility block.</p>
            <p>A third paragraph keeps the article heuristic clearly on the prose-heavy side for classification.</p>
          </article>
        </body></html>
        "#;
        let dom = tl::parse(html, tl::ParserOptions::default()).unwrap();
        let parser = dom.parser();
        let tag = dom
            .query_selector("article")
            .and_then(|mut it| it.next())
            .and_then(|h| h.get(parser))
            .and_then(|n| n.as_tag())
            .unwrap();
        let rendered = cleanup_markdown(&render_tag(tag, parser));
        let stats = score_text(&rendered);
        assert_eq!(
            classify_candidate_family(tag, &rendered, &stats, PageFamily::Generic),
            PageFamily::Article
        );
    }

    #[test]
    fn classifies_product_candidates() {
        let html = r#"
        <html><body>
          <section class="product-details buybox">
            <h1>Ip Man 1-4 (Box Set) (Blu-ray)</h1>
            <p>Current price is USD$22.99</p>
            <h2>Key item features</h2>
            <ul>
              <li>Action, Biography, Drama</li>
              <li>Movie &amp; tv media format: Blu-ray</li>
            </ul>
            <h2>Specifications</h2>
            <table>
              <tr><th>Director</th><td>Wilson Yip</td></tr>
              <tr><th>Resolution</th><td>1080p</td></tr>
            </table>
          </section>
        </body></html>
        "#;
        let dom = tl::parse(html, tl::ParserOptions::default()).unwrap();
        let parser = dom.parser();
        let tag = dom
            .query_selector("section")
            .and_then(|mut it| it.next())
            .and_then(|h| h.get(parser))
            .and_then(|n| n.as_tag())
            .unwrap();
        let rendered = cleanup_markdown(&render_tag(tag, parser));
        let stats = score_text(&rendered);
        assert_eq!(
            classify_candidate_family(tag, &rendered, &stats, PageFamily::Generic),
            PageFamily::Product
        );
    }
}
