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
pub fn arxiv_api_url(arxiv_id: &str) -> Url {
    Url::parse(&format!(
        "https://export.arxiv.org/api/query?id_list={arxiv_id}&max_results=1"
    ))
    .expect("statically-constructed URL is always valid")
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
pub fn format_arxiv_content(content: &ArxivContent) -> String {
    let authors = if content.authors.is_empty() {
        "Unknown".to_string()
    } else if content.authors.len() <= 3 {
        content.authors.join(", ")
    } else {
        format!("{} et al.", content.authors[0])
    };

    format!(
        "# {}\n\n**Authors:** {}\n**Published:** {}\n**ArXiv:** {}\n\n## Abstract\n\n{}",
        content.title,
        authors,
        content.published,
        content.arxiv_id,
        content.abstract_text,
    )
}

fn extract_between<'a>(haystack: &'a str, open: &str, close: &str, skip: usize) -> Option<&'a str> {
    extract_between_nth(haystack, open, close, skip)
}

fn extract_between_nth<'a>(haystack: &'a str, open: &str, close: &str, skip: usize) -> Option<&'a str> {
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
            authors: vec!["Vaswani".into(), "Shazeer".into(), "Parmar".into(), "Uszkoreit".into()],
            published: "2017-06-12".into(),
            abstract_text: "The dominant sequence transduction models...".into(),
            arxiv_id: "1706.03762".into(),
        };
        let md = format_arxiv_content(&content);
        assert!(md.contains("Vaswani et al."));
        assert!(md.contains("## Abstract"));
    }
}
