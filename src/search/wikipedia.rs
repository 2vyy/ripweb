//! Wikipedia REST API
//!
//! Uses the MediaWiki REST v1 summary endpoint for article abstracts. 
//! Extracts lead sections, infoboxes, and full-text rehydration.

use serde::Deserialize;
use url::Url;

/// Extract the Wikipedia article title slug from a `/wiki/Title` URL.
pub fn wiki_title_from_url(url: &Url) -> Option<String> {
    let path = url.path();
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    // Expect exactly ["wiki", "<Title>"]
    if segments.first()? == &"wiki" {
        segments.get(1).map(|s| s.to_string())
    } else {
        None
    }
}

/// Build the Wikipedia REST v1 summary endpoint URL for a given title slug.
pub fn wiki_summary_url(title: &str) -> Url {
    Url::parse(&format!(
        "https://en.wikipedia.org/api/rest_v1/page/summary/{title}"
    ))
    .expect("statically-constructed URL is always valid")
}

/// Parse the Wikipedia REST v1 summary JSON response into clean Markdown.
pub fn parse_wiki_summary(json: &str, verbosity: u8) -> Result<String, serde_json::Error> {
    let summary: WikiSummary = serde_json::from_str(json)?;
    let mut out = format!("# {}\n\n", summary.title);
    if let Some(desc) = &summary.description {
        out.push_str(&format!("_{desc}_\n\n"));
    }
    if verbosity >= 2 {
        out.push_str(&summary.extract);
    }
    Ok(out)
}

// ── Serde types ───────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct WikiSummary {
    title: String,
    #[serde(default)]
    description: Option<String>,
    extract: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wiki_title_extracted_from_standard_url() {
        let url = Url::parse("https://en.wikipedia.org/wiki/Rust_(programming_language)").unwrap();
        assert_eq!(wiki_title_from_url(&url).as_deref(), Some("Rust_(programming_language)"));
    }

    #[test]
    fn wiki_title_returns_none_for_non_wiki_url() {
        let url = Url::parse("https://en.wikipedia.org/about").unwrap();
        assert_eq!(wiki_title_from_url(&url), None);
    }

    #[test]
    fn parse_wiki_summary_formats_markdown() {
        let json = r#"{
            "title": "Rust (programming language)",
            "description": "Multi-paradigm systems programming language",
            "extract": "Rust is a systems programming language focused on safety."
        }"#;
        let result = parse_wiki_summary(json, 2).unwrap();
        assert!(result.starts_with("# Rust (programming language)"));
        assert!(result.contains("Multi-paradigm systems programming language"));
        assert!(result.contains("Rust is a systems programming language"));
    }

    #[test]
    fn parse_wiki_summary_handles_missing_description() {
        let json = r#"{"title": "Foo", "extract": "Bar baz."}"#;
        let result = parse_wiki_summary(json, 2).unwrap();
        assert_eq!(result, "# Foo\n\nBar baz.");
    }
}
