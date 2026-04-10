//! Project entity match scorer.
//!
//! Detects likely project/crate names in the query and checks whether the
//! result's title or hostname contains the same token.
//!
//! Project token heuristic:
//!   - Contains a hyphen or underscore (e.g. `serde-json`, `serde_json`)
//!   - Is ≥ 4 characters, all ASCII lowercase, and not a common English word

use crate::search::scoring::{ScorerInput, extract_host};
use crate::search::trace::ScorerContribution;

/// Score a result based on whether the query's project token appears in the
/// result's title or host.
#[must_use]
pub fn score(input: &ScorerInput) -> ScorerContribution {
    let tokens = extract_project_tokens(input.query);
    if tokens.is_empty() {
        return ScorerContribution {
            scorer: "project_match".to_owned(),
            delta: 0.0,
            reason: "no project token detected in query".to_owned(),
        };
    }

    let title_lower = input.result.title.to_ascii_lowercase();
    let host = extract_host(input.result.url.as_str());

    for token in &tokens {
        let tok_lower = token.to_ascii_lowercase();
        if title_lower.contains(tok_lower.as_str()) || host.contains(tok_lower.as_str()) {
            return ScorerContribution {
                scorer: "project_match".to_owned(),
                delta: 1.5,
                reason: format!("project token '{token}' found in title or host"),
            };
        }
    }

    ScorerContribution {
        scorer: "project_match".to_owned(),
        delta: 0.0,
        reason: format!("project token(s) {:?} not found in title or host", tokens),
    }
}

/// Extract candidate project tokens from the query.
fn extract_project_tokens(query: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    for word in query.split_whitespace() {
        // crate-name with hyphen or underscore (e.g. serde-json, tokio_tracing)
        if word.contains('-') || word.contains('_') {
            tokens.push(word.to_owned());
            continue;
        }
        // Lowercase word ≥ 4 chars that is not a common English word
        let all_lower_alpha = word.chars().all(|c| c.is_ascii_lowercase());
        if all_lower_alpha && word.len() >= 4 && !is_common_word(word) {
            tokens.push(word.to_owned());
        }
    }
    tokens.dedup();
    tokens
}

/// Words that are very common in technical queries but are not project names.
fn is_common_word(w: &str) -> bool {
    matches!(
        w,
        "rust"
            | "with"
            | "from"
            | "into"
            | "over"
            | "that"
            | "this"
            | "when"
            | "what"
            | "have"
            | "then"
            | "they"
            | "will"
            | "about"
            | "async"
            | "await"
            | "impl"
            | "struct"
            | "trait"
            | "enum"
            | "macro"
            | "derive"
            | "error"
            | "using"
            | "guide"
            | "tutorial"
            | "example"
            | "code"
            | "type"
            | "docs"
            | "file"
            | "build"
            | "test"
            | "data"
            | "http"
            | "https"
            | "json"
            | "toml"
            | "yaml"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hyphenated_word_is_always_a_project_token() {
        let tokens = extract_project_tokens("serde-json parsing");
        assert!(tokens.contains(&"serde-json".to_owned()));
    }

    #[test]
    fn underscore_word_is_a_project_token() {
        let tokens = extract_project_tokens("serde_json parsing");
        assert!(tokens.contains(&"serde_json".to_owned()));
    }

    #[test]
    fn short_words_are_excluded() {
        let tokens = extract_project_tokens("use get run");
        assert!(
            tokens.is_empty(),
            "words shorter than 4 chars must be excluded"
        );
    }

    #[test]
    fn common_words_are_excluded() {
        let tokens = extract_project_tokens("rust async await trait");
        assert!(
            tokens.is_empty(),
            "common words must be excluded, got {:?}",
            tokens
        );
    }

    #[test]
    fn crate_name_is_extracted() {
        let tokens = extract_project_tokens("tokio async runtime");
        assert!(tokens.contains(&"tokio".to_owned()));
    }
}
