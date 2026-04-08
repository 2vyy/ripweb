//! DuckDuckGo Instant Answer API
//!
//! Interacts with the DDG Zero-Click Info API to retrieve "Instant Answer"
//! facts, definitions, and high-level summaries for a given query.

use serde::Deserialize;

/// A structured answer from DuckDuckGo's Zero-Click Info (Instant Answer) API.
///
/// This is separate from the HTML SERP scrape. It returns knowledge panel
/// data: abstracts, definitions, and related topics. Excellent for entity
/// lookups as a complement to Wikipedia.
#[derive(Debug, Deserialize)]
pub struct InstantAnswer {
    /// The main text summary (from Wikipedia or a curated source).
    #[serde(rename = "AbstractText")]
    pub abstract_text: String,

    /// A one-sentence definition (for terms/concepts).
    #[serde(rename = "Definition")]
    pub definition: String,

    /// The source name (e.g. "Wikipedia").
    #[serde(rename = "AbstractSource")]
    pub abstract_source: String,

    /// The canonical URL for the abstract source.
    #[serde(rename = "AbstractURL")]
    pub abstract_url: String,
}

impl InstantAnswer {
    /// Returns true if this answer contains meaningful content.
    pub fn has_content(&self) -> bool {
        !self.abstract_text.is_empty() || !self.definition.is_empty()
    }

    /// Format the instant answer as a concise Markdown preamble.
    pub fn to_markdown(&self) -> String {
        let mut out = String::new();

        if !self.abstract_text.is_empty() {
            out.push_str(&self.abstract_text);
            if !self.abstract_source.is_empty() && !self.abstract_url.is_empty() {
                out.push_str(&format!(
                    "\n\n*Source: [{}]({})*",
                    self.abstract_source, self.abstract_url
                ));
            }
        } else if !self.definition.is_empty() {
            out.push_str(&self.definition);
        }

        out
    }
}

/// Build the DuckDuckGo Zero-Click Info API URL.
///
/// `no_html=1` strips HTML from the response fields.
/// `skip_disambig=1` skips disambiguation pages and jumps straight to results.
pub fn ddg_instant_url(query: &str) -> String {
    let encoded: String = url::form_urlencoded::byte_serialize(query.as_bytes()).collect();
    format!("https://api.duckduckgo.com/?q={encoded}&format=json&no_html=1&skip_disambig=1")
}

/// Parse the DDG Instant Answer JSON response.
pub fn parse_ddg_instant(json: &str) -> Result<InstantAnswer, serde_json::Error> {
    serde_json::from_str(json)
}

/// Fetch and parse a DDG instant answer. Returns `None` if nothing useful found.
pub async fn fetch_instant(client: &rquest::Client, query: &str) -> Option<String> {
    let url = ddg_instant_url(query);
    let body = client.get(&url).send().await.ok()?.text().await.ok()?;

    let answer = parse_ddg_instant(&body).ok()?;
    if answer.has_content() {
        Some(answer.to_markdown())
    } else {
        None
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ddg_instant_extracts_abstract() {
        let json = r#"{
            "AbstractText": "Rust is a multi-paradigm, general-purpose programming language.",
            "AbstractSource": "Wikipedia",
            "AbstractURL": "https://en.wikipedia.org/wiki/Rust_(programming_language)",
            "Definition": "",
            "DefinitionSource": "",
            "DefinitionURL": ""
        }"#;
        let answer = parse_ddg_instant(json).unwrap();
        assert!(answer.has_content());
        assert!(answer.abstract_text.contains("Rust"));
        assert_eq!(answer.abstract_source, "Wikipedia");
    }

    #[test]
    fn parse_ddg_instant_extracts_definition_when_no_abstract() {
        let json = r#"{
            "AbstractText": "",
            "AbstractSource": "",
            "AbstractURL": "",
            "Definition": "A statically typed language focused on safety.",
            "DefinitionSource": "Wiktionary",
            "DefinitionURL": "https://en.wiktionary.org/wiki/rust"
        }"#;
        let answer = parse_ddg_instant(json).unwrap();
        assert!(answer.has_content());
        let md = answer.to_markdown();
        assert!(md.contains("statically typed"));
    }

    #[test]
    fn has_content_returns_false_for_empty_answer() {
        let json = r#"{
            "AbstractText": "",
            "AbstractSource": "",
            "AbstractURL": "",
            "Definition": "",
            "DefinitionSource": "",
            "DefinitionURL": ""
        }"#;
        let answer = parse_ddg_instant(json).unwrap();
        assert!(!answer.has_content());
    }

    #[test]
    fn to_markdown_formats_with_source_link() {
        let answer = InstantAnswer {
            abstract_text: "Rust is fast and safe.".into(),
            abstract_source: "Wikipedia".into(),
            abstract_url: "https://en.wikipedia.org/wiki/Rust".into(),
            definition: "".into(),
        };
        let md = answer.to_markdown();
        assert!(md.contains("Rust is fast and safe."));
        assert!(md.contains("[Wikipedia]"));
    }

    #[test]
    fn ddg_instant_url_encodes_query() {
        let url = ddg_instant_url("what is rust");
        assert!(url.contains("api.duckduckgo.com"));
        assert!(url.contains("format=json"));
        assert!(url.contains("no_html=1"));
        assert!(!url.contains(" "));
    }
}
