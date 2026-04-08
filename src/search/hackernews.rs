//! HackerNews (Algolia) API
//!
//! Integrates with the Algolia-backed HN search API to retrieve 
//! story metadata and the full comment tree in a structured format.

use serde::Deserialize;
use url::Url;

pub struct HnContent {
    pub title: String,
    pub text: Option<String>,
    pub comments: Vec<String>,
}

/// Build the Algolia HN API URL for the given item ID.
pub fn hn_api_url(item_id: &str) -> Url {
    Url::parse(&format!("https://hn.algolia.com/api/v1/items/{item_id}"))
        .expect("statically-constructed URL is always valid")
}

/// Parse the Algolia HN item JSON, extracting title, OP text, and
/// direct child comment texts (null texts skipped).
pub fn parse_hn_json(json: &str) -> Result<HnContent, serde_json::Error> {
    let item: HnItem = serde_json::from_str(json)?;

    let text = item.text.as_deref().map(strip_html).filter(|s| !s.is_empty());

    let comments = item
        .children
        .iter()
        .filter_map(|c| c.text.as_deref())
        .map(strip_html)
        .filter(|s| !s.is_empty())
        .collect();

    Ok(HnContent { title: item.title, text, comments })
}

/// Strip HTML tags from HN's text field, returning plain text.
///
/// HN's Algolia API wraps all text in `<p>` and other tags.
/// We reuse `tl` (already a dependency) to extract clean text.
fn strip_html(html: &str) -> String {
    let Ok(dom) = tl::parse(html, tl::ParserOptions::default()) else {
        return html.to_owned();
    };
    let parser = dom.parser();
    dom.nodes()
        .iter()
        .filter_map(|n| match n {
            tl::Node::Raw(b) => Some(b.as_utf8_str().into_owned()),
            tl::Node::Tag(tag) => {
                // Collect text from leaf tags (p, em, etc.)
                let mut parts = Vec::new();
                collect_raw_text(tag, parser, &mut parts);
                if parts.is_empty() { None } else { Some(parts.join(" ")) }
            }
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn collect_raw_text(tag: &tl::HTMLTag, parser: &tl::Parser, out: &mut Vec<String>) {
    for handle in tag.children().top().iter() {
        if let Some(node) = handle.get(parser) {
            match node {
                tl::Node::Raw(b) => {
                    let s = b.as_utf8_str().into_owned();
                    let trimmed = s.trim().to_owned();
                    if !trimmed.is_empty() {
                        out.push(trimmed);
                    }
                }
                tl::Node::Tag(child_tag) => collect_raw_text(child_tag, parser, out),
                _ => {}
            }
        }
    }
}

// ── Serde types ───────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct HnItem {
    #[serde(default)]
    title: String,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    children: Vec<HnChild>,
}

#[derive(Deserialize)]
struct HnChild {
    #[serde(default)]
    text: Option<String>,
}
