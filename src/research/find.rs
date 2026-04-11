use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchMode {
    AllTerms,
    Partial,
    NoMatch,
}

#[derive(Debug, Clone)]
pub struct FindResult {
    pub filtered_text: String,
    pub matched_terms: Vec<String>,
    pub match_mode: MatchMode,
}

pub fn parse_terms(raw: &str) -> Vec<String> {
    let mut seen = HashSet::new();
    raw.split(',')
        .map(str::trim)
        .filter(|term| !term.is_empty())
        .map(|term| term.to_ascii_lowercase())
        .filter(|term| seen.insert(term.clone()))
        .collect()
}

pub fn matched_terms_in_text(text: &str, terms: &[String]) -> Vec<String> {
    let lower = text.to_ascii_lowercase();
    terms
        .iter()
        .filter(|term| lower.contains(term.as_str()))
        .cloned()
        .collect()
}

pub fn filter_markdown_blocks(text: &str, terms: &[String]) -> FindResult {
    if terms.is_empty() {
        return FindResult {
            filtered_text: text.to_owned(),
            matched_terms: Vec::new(),
            match_mode: MatchMode::AllTerms,
        };
    }

    let blocks: Vec<(usize, String)> = text
        .split("\n\n")
        .enumerate()
        .map(|(idx, block)| (idx, block.trim().to_owned()))
        .filter(|(_, block)| !block.is_empty())
        .collect();

    let all_term_hits: Vec<String> = blocks
        .iter()
        .filter_map(|(_, block)| {
            let lower = block.to_ascii_lowercase();
            terms
                .iter()
                .all(|term| lower.contains(term))
                .then(|| block.clone())
        })
        .collect();

    if !all_term_hits.is_empty() {
        let joined = all_term_hits.join("\n\n");
        return FindResult {
            filtered_text: joined.clone(),
            matched_terms: matched_terms_in_text(&joined, terms),
            match_mode: MatchMode::AllTerms,
        };
    }

    let mut partial_hits: Vec<(usize, usize, usize, String)> = blocks
        .iter()
        .filter_map(|(idx, block)| {
            let lower = block.to_ascii_lowercase();
            let matched_count = terms
                .iter()
                .filter(|term| lower.contains(term.as_str()))
                .count();
            (matched_count > 0).then(|| {
                let mention_count: usize =
                    terms.iter().map(|term| lower.matches(term).count()).sum();
                (*idx, matched_count, mention_count, block.clone())
            })
        })
        .collect();

    if partial_hits.is_empty() {
        return FindResult {
            filtered_text: String::new(),
            matched_terms: Vec::new(),
            match_mode: MatchMode::NoMatch,
        };
    }

    partial_hits.sort_by(|a, b| {
        b.1.cmp(&a.1)
            .then_with(|| b.2.cmp(&a.2))
            .then_with(|| a.0.cmp(&b.0))
    });
    partial_hits.truncate(5);
    partial_hits.sort_by_key(|item| item.0);

    let selected = partial_hits
        .into_iter()
        .map(|(_, _, _, block)| block)
        .collect::<Vec<_>>()
        .join("\n\n");

    FindResult {
        filtered_text: selected.clone(),
        matched_terms: matched_terms_in_text(&selected, terms),
        match_mode: MatchMode::Partial,
    }
}

#[cfg(test)]
mod tests {
    use super::{MatchMode, filter_markdown_blocks, parse_terms};

    #[test]
    fn parse_terms_normalizes_and_deduplicates() {
        let terms = parse_terms("Rust, async, rust,  ");
        assert_eq!(terms, vec!["rust", "async"]);
    }

    #[test]
    fn prefers_all_terms_blocks() {
        let text = "rust async intro\n\nonly rust here";
        let result = filter_markdown_blocks(text, &parse_terms("rust,async"));
        assert_eq!(result.match_mode, MatchMode::AllTerms);
        assert_eq!(result.filtered_text, "rust async intro");
    }

    #[test]
    fn falls_back_to_partial_when_needed() {
        let text = "rust only\n\nasync only\n\nmisc";
        let result = filter_markdown_blocks(text, &parse_terms("rust,async"));
        assert_eq!(result.match_mode, MatchMode::Partial);
        assert!(result.filtered_text.contains("rust only"));
        assert!(result.filtered_text.contains("async only"));
    }
}
