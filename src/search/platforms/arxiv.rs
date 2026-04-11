//! ArXiv Research Platform
//!
//! Handles Atom API requests to ArXiv to retrieve paper metadata
//! and abstracts. Maps PDF URLs back to abstract pages for clean
//! metadata extraction.

use url::Url;

/// Content extracted from an ArXiv paper.
pub struct ArxivContent {
    pub title: String,
    pub authors: Vec<String>,
    pub published: String,
    pub abstract_text: String,
    pub arxiv_id: String,
}

/// Extract the ArXiv paper ID from URLs like:
/// - `https://arxiv.org/abs/1706.03762`
/// - `https://arxiv.org/pdf/1706.03762`
/// - `https://arxiv.org/abs/1706.03762v3`
pub fn arxiv_id_from_url(url: &Url) -> Option<String> {
    let segs: Vec<&str> = url.path_segments()?.filter(|s| !s.is_empty()).collect();
    if segs.first()? == &"abs" || segs.first()? == &"pdf" {
        segs.get(1).map(|s| s.trim_end_matches(".pdf").to_string())
    } else {
        None
    }
}

/// Build the ArXiv export API URL for a paper ID.
///
/// The export API returns Atom XML with clean metadata.
pub fn arxiv_api_url(arxiv_id: &str) -> Result<Url, url::ParseError> {
    Url::parse(&format!(
        "https://export.arxiv.org/api/query?id_list={arxiv_id}&max_results=1"
    ))
}

/// Parse the ArXiv Atom XML response into structured content.
///
/// Extracts title, authors, published date, and abstract.
pub fn parse_arxiv_atom(xml: &str) -> Option<ArxivContent> {
    // Use simple substring extraction to avoid pulling in an XML crate.
    // ArXiv's Atom feed is well-structured and stable.
    let title = extract_between(xml, "<title>", "</title>", 1)?
        .trim()
        .replace('\n', " ")
        .to_string();

    // Skip the feed-level <title> (index 0), use the entry title (index 1)
    let entry_title = if title.contains("ArXiv") || title.starts_with("1 ") {
        extract_between_nth(xml, "<title>", "</title>", 1)?
            .trim()
            .replace('\n', " ")
            .to_string()
    } else {
        title
    };

    let abstract_text = extract_between(xml, "<summary>", "</summary>", 0)?
        .trim()
        .replace('\n', " ")
        .to_string();

    let published = extract_between(xml, "<published>", "</published>", 0)?
        .trim()
        .chars()
        .take(10) // Keep YYYY-MM-DD
        .collect();

    let arxiv_id = extract_between(xml, "<id>", "</id>", 1)
        .unwrap_or_default()
        .trim()
        .to_string();

    let authors: Vec<String> = {
        let mut names = Vec::new();
        let mut remaining = xml;
        while let Some(start) = remaining.find("<name>") {
            remaining = &remaining[start + 6..];
            if let Some(end) = remaining.find("</name>") {
                names.push(remaining[..end].trim().to_string());
                remaining = &remaining[end + 7..];
            } else {
                break;
            }
        }
        names
    };

    Some(ArxivContent {
        title: entry_title,
        authors,
        published,
        abstract_text,
        arxiv_id,
    })
}

/// Format extracted ArXiv content as clean Markdown.
pub fn format_arxiv_content(content: &ArxivContent, mode: crate::verbosity::Verbosity) -> String {
    let authors = if content.authors.is_empty() {
        "Unknown".to_string()
    } else if content.authors.len() <= 3 {
        content.authors.join(", ")
    } else {
        format!("{} et al.", content.authors[0])
    };

    match mode.density_tier() {
        1 => {
            format!("- [{}]({})", content.title, content.arxiv_id)
        }
        2 => {
            format!(
                "# {}\n\n## Abstract\n\n{}",
                content.title, content.abstract_text
            )
        }
        _ => {
            format!(
                "# {}\n\n**Authors:** {}\n**Published:** {}\n**ArXiv:** {}\n\n## Abstract\n\n{}",
                content.title, authors, content.published, content.arxiv_id, content.abstract_text,
            )
        }
    }
}

fn extract_between<'a>(haystack: &'a str, open: &str, close: &str, skip: usize) -> Option<&'a str> {
    extract_between_nth(haystack, open, close, skip)
}

fn extract_between_nth<'a>(
    haystack: &'a str,
    open: &str,
    close: &str,
    skip: usize,
) -> Option<&'a str> {
    let mut remaining = haystack;
    for _ in 0..=skip {
        let start = remaining.find(open)? + open.len();
        remaining = &remaining[start..];
    }
    let end = remaining.find(close)?;
    Some(&remaining[..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arxiv_id_from_abs_url() {
        let url = Url::parse("https://arxiv.org/abs/1706.03762").unwrap();
        assert_eq!(arxiv_id_from_url(&url).as_deref(), Some("1706.03762"));
    }

    #[test]
    fn arxiv_id_from_pdf_url() {
        let url = Url::parse("https://arxiv.org/pdf/1706.03762.pdf").unwrap();
        assert_eq!(arxiv_id_from_url(&url).as_deref(), Some("1706.03762"));
    }

    #[test]
    fn arxiv_id_from_versioned_url() {
        let url = Url::parse("https://arxiv.org/abs/1706.03762v3").unwrap();
        assert_eq!(arxiv_id_from_url(&url).as_deref(), Some("1706.03762v3"));
    }

    #[test]
    fn format_arxiv_with_many_authors_uses_et_al() {
        let content = ArxivContent {
            title: "Attention Is All You Need".into(),
            authors: vec![
                "Vaswani".into(),
                "Shazeer".into(),
                "Parmar".into(),
                "Uszkoreit".into(),
            ],
            published: "2017-06-12".into(),
            abstract_text: "The dominant sequence transduction models...".into(),
            arxiv_id: "1706.03762".into(),
        };
        let md = format_arxiv_content(&content, crate::verbosity::Verbosity::Full);
        assert!(md.contains("Vaswani et al."));
        assert!(md.contains("## Abstract"));
    }

    #[test]
    fn arxiv_id_returns_none_for_non_abs_or_pdf_paths() {
        let url = Url::parse("https://arxiv.org/help/api").unwrap();
        assert!(arxiv_id_from_url(&url).is_none());
    }

    #[test]
    fn arxiv_api_url_includes_expected_query_params() {
        let api_url = arxiv_api_url("1706.03762v3").unwrap();
        assert_eq!(
            api_url.as_str(),
            "https://export.arxiv.org/api/query?id_list=1706.03762v3&max_results=1"
        );
    }

    #[test]
    fn parse_arxiv_atom_extracts_metadata_and_normalises_fields() {
        let xml = r#"
            <feed xmlns="http://www.w3.org/2005/Atom">
              <title>ArXiv Query: all:attention</title>
              <id>http://arxiv.org/api/query?search_query=all:attention</id>
              <entry>
                <id>http://arxiv.org/abs/1706.03762v3</id>
                <published>2017-06-12T17:13:52Z</published>
                <title>
                  Attention Is All You Need
                </title>
                <summary>
                  The dominant sequence transduction models...
                </summary>
                <author><name>Ashish Vaswani</name></author>
                <author><name>Noam Shazeer</name></author>
              </entry>
            </feed>
        "#;

        let parsed = parse_arxiv_atom(xml).unwrap();
        assert_eq!(parsed.title, "Attention Is All You Need");
        assert_eq!(parsed.authors, vec!["Ashish Vaswani", "Noam Shazeer"]);
        assert_eq!(parsed.published, "2017-06-12");
        assert_eq!(parsed.arxiv_id, "http://arxiv.org/abs/1706.03762v3");
        assert!(
            parsed
                .abstract_text
                .starts_with("The dominant sequence transduction models")
        );
    }

    #[test]
    fn parse_arxiv_atom_returns_none_when_required_fields_are_missing() {
        let xml = r#"
            <feed>
              <title>Feed title</title>
              <title>Entry title</title>
              <published>2017-06-12T17:13:52Z</published>
            </feed>
        "#;
        assert!(parse_arxiv_atom(xml).is_none());
    }

    #[test]
    fn format_arxiv_content_compact_balanced_and_unknown_authors() {
        let content = ArxivContent {
            title: "Attention Is All You Need".into(),
            authors: vec![],
            published: "2017-06-12".into(),
            abstract_text: "A paper about transformers.".into(),
            arxiv_id: "1706.03762".into(),
        };

        let compact = format_arxiv_content(&content, crate::verbosity::Verbosity::Compact);
        assert_eq!(compact, "- [Attention Is All You Need](1706.03762)");

        let balanced = format_arxiv_content(&content, crate::verbosity::Verbosity::Standard);
        assert!(balanced.contains("# Attention Is All You Need"));
        assert!(balanced.contains("## Abstract"));

        let verbose = format_arxiv_content(&content, crate::verbosity::Verbosity::Full);
        assert!(verbose.contains("**Authors:** Unknown"));
    }

    #[test]
    fn format_arxiv_content_joins_small_author_lists() {
        let content = ArxivContent {
            title: "Paper".into(),
            authors: vec!["Alice".into(), "Bob".into()],
            published: "2025-01-01".into(),
            abstract_text: "Abstract".into(),
            arxiv_id: "1234.5678".into(),
        };
        let verbose = format_arxiv_content(&content, crate::verbosity::Verbosity::Full);
        assert!(verbose.contains("**Authors:** Alice, Bob"));
    }
}
