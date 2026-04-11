use ripweb::research::find::{MatchMode, filter_markdown_blocks, parse_terms};

#[test]
fn find_prefers_blocks_with_all_terms() {
    let page = include_str!("research/find_fixtures/multi_term_page.html");
    let terms = parse_terms("rust,tokio");
    let filtered = filter_markdown_blocks(page, &terms);

    assert_eq!(filtered.match_mode, MatchMode::AllTerms);
    assert!(filtered.filtered_text.to_ascii_lowercase().contains("rust"));
    assert!(
        filtered
            .filtered_text
            .to_ascii_lowercase()
            .contains("tokio")
    );
}
