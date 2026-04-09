// tests/search_scoring.rs

mod common;

#[test]
fn search_config_has_docs_rs_as_high_trust_by_default() {
    let cfg = ripweb::config::get_config();
    assert!(
        cfg.search.trust.high.iter().any(|d| d == "docs.rs"),
        "docs.rs must be in high-trust tier by default"
    );
}

#[test]
fn search_config_has_nonzero_domain_trust_weight() {
    let cfg = ripweb::config::get_config();
    assert!(
        cfg.search.weights.domain_trust > 0.0,
        "domain_trust weight must be positive"
    );
}

// ── domain_trust ─────────────────────────────────────────────────────────────

mod domain_trust_tests {
    use ripweb::search::scoring::{ScorerInput, domain_trust};
    use ripweb::config::TrustConfig;
    use ripweb::search::SearchResult;

    fn make_result(url: &str) -> SearchResult {
        SearchResult {
            url: url.to_owned(),
            title: "Title".to_owned(),
            snippet: None,
        }
    }

    fn trust_cfg() -> TrustConfig {
        TrustConfig {
            high: vec!["docs.rs".to_owned(), "tokio.rs".to_owned()],
            medium: vec!["stackoverflow.com".to_owned()],
            low: vec!["quora.com".to_owned()],
        }
    }

    #[test]
    fn high_trust_domain_gets_positive_delta() {
        let result = make_result("https://docs.rs/tokio/latest/tokio/");
        let input = ScorerInput { result: &result, query: "tokio", engine_rank: 0 };
        let c = domain_trust::score(&input, &trust_cfg());
        assert!(c.delta > 0.0, "high trust must give positive delta, got {}", c.delta);
        assert_eq!(c.scorer, "domain_trust");
    }

    #[test]
    fn subdomain_of_high_trust_domain_gets_boost() {
        let result = make_result("https://blog.tokio.rs/post");
        let input = ScorerInput { result: &result, query: "tokio", engine_rank: 0 };
        let c = domain_trust::score(&input, &trust_cfg());
        assert!(c.delta > 0.0);
    }

    #[test]
    fn medium_trust_domain_gets_smaller_positive_delta_than_high() {
        let high_result = make_result("https://docs.rs/tokio/");
        let med_result = make_result("https://stackoverflow.com/questions/1");
        let input_high = ScorerInput { result: &high_result, query: "tokio", engine_rank: 0 };
        let input_med = ScorerInput { result: &med_result, query: "tokio", engine_rank: 0 };
        let high_c = domain_trust::score(&input_high, &trust_cfg());
        let med_c = domain_trust::score(&input_med, &trust_cfg());
        assert!(
            high_c.delta > med_c.delta,
            "high trust delta ({}) must exceed medium ({})",
            high_c.delta, med_c.delta
        );
        assert!(med_c.delta >= 0.0);
    }

    #[test]
    fn low_trust_domain_gets_negative_delta() {
        let result = make_result("https://quora.com/What-is-Rust");
        let input = ScorerInput { result: &result, query: "rust", engine_rank: 0 };
        let c = domain_trust::score(&input, &trust_cfg());
        assert!(c.delta < 0.0, "low trust must give negative delta, got {}", c.delta);
    }

    #[test]
    fn unknown_domain_gets_zero_delta() {
        let result = make_result("https://example.com/post");
        let input = ScorerInput { result: &result, query: "rust", engine_rank: 0 };
        let c = domain_trust::score(&input, &trust_cfg());
        assert_eq!(c.delta, 0.0);
    }
}

// ── blocklist_penalty ─────────────────────────────────────────────────────────

mod blocklist_tests {
    use ripweb::search::scoring::{ScorerInput, blocklist_penalty};
    use ripweb::config::BlocklistConfig;
    use ripweb::search::SearchResult;

    fn make_result(url: &str) -> SearchResult {
        SearchResult { url: url.to_owned(), title: "Title".to_owned(), snippet: None }
    }

    fn blocklist() -> BlocklistConfig {
        BlocklistConfig { domains: vec!["w3schools.com".to_owned(), "geeksforgeeks.org".to_owned()] }
    }

    #[test]
    fn blocklisted_domain_gets_heavy_penalty() {
        let result = make_result("https://w3schools.com/rust/rust_intro.asp");
        let input = ScorerInput { result: &result, query: "rust", engine_rank: 0 };
        let c = blocklist_penalty::score(&input, &blocklist());
        assert!(c.delta <= -3.0, "blocklist penalty must be at most -3.0, got {}", c.delta);
        assert_eq!(c.scorer, "blocklist_penalty");
    }

    #[test]
    fn subdomain_of_blocklisted_domain_also_penalised() {
        let result = make_result("https://practice.geeksforgeeks.org/problems/rust");
        let input = ScorerInput { result: &result, query: "rust", engine_rank: 0 };
        let c = blocklist_penalty::score(&input, &blocklist());
        assert!(c.delta <= -3.0);
    }

    #[test]
    fn non_blocklisted_domain_gets_zero_penalty() {
        let result = make_result("https://docs.rs/tokio/");
        let input = ScorerInput { result: &result, query: "tokio", engine_rank: 0 };
        let c = blocklist_penalty::score(&input, &blocklist());
        assert_eq!(c.delta, 0.0);
    }
}

// ── url_pattern ───────────────────────────────────────────────────────────────

mod url_pattern_tests {
    use ripweb::search::scoring::{ScorerInput, url_pattern};
    use ripweb::search::SearchResult;

    fn make_result(url: &str) -> SearchResult {
        SearchResult { url: url.to_owned(), title: "Title".to_owned(), snippet: None }
    }

    #[test]
    fn docs_rs_url_gets_boost() {
        let r = make_result("https://docs.rs/tokio/latest/tokio/");
        let inp = ScorerInput { result: &r, query: "tokio", engine_rank: 0 };
        let c = url_pattern::score(&inp);
        assert!(c.delta > 0.0, "docs.rs URLs must get a boost, got {}", c.delta);
        assert_eq!(c.scorer, "url_pattern");
    }

    #[test]
    fn reference_path_gets_boost() {
        let r = make_result("https://doc.rust-lang.org/reference/types.html");
        let inp = ScorerInput { result: &r, query: "rust types", engine_rank: 0 };
        let c = url_pattern::score(&inp);
        assert!(c.delta > 0.0, "/reference/ path must get a boost, got {}", c.delta);
    }

    #[test]
    fn medium_com_host_gets_penalty() {
        let r = make_result("https://medium.com/@someone/rust-tutorial");
        let inp = ScorerInput { result: &r, query: "rust", engine_rank: 0 };
        let c = url_pattern::score(&inp);
        assert!(c.delta < 0.0, "medium.com must get a penalty, got {}", c.delta);
    }

    #[test]
    fn dev_to_host_gets_penalty() {
        let r = make_result("https://dev.to/article/rust-for-beginners");
        let inp = ScorerInput { result: &r, query: "rust", engine_rank: 0 };
        let c = url_pattern::score(&inp);
        assert!(c.delta < 0.0, "dev.to must get a penalty, got {}", c.delta);
    }

    #[test]
    fn neutral_url_gets_zero_delta() {
        let r = make_result("https://example.com/some-page");
        let inp = ScorerInput { result: &r, query: "rust", engine_rank: 0 };
        let c = url_pattern::score(&inp);
        assert_eq!(c.delta, 0.0, "neutral URL must get 0.0, got {}", c.delta);
    }

    #[test]
    fn github_io_docs_host_gets_boost() {
        let r = make_result("https://rust-lang.github.io/rfcs/");
        let inp = ScorerInput { result: &r, query: "rfcs", engine_rank: 0 };
        let c = url_pattern::score(&inp);
        assert!(c.delta > 0.0, "github.io docs hosts must get a boost, got {}", c.delta);
    }
}

// ── project_match ─────────────────────────────────────────────────────────────

mod project_match_tests {
    use ripweb::search::scoring::{ScorerInput, project_match};
    use ripweb::search::SearchResult;

    fn make_result(url: &str, title: &str) -> SearchResult {
        SearchResult { url: url.to_owned(), title: title.to_owned(), snippet: None }
    }

    #[test]
    fn project_name_in_title_gives_boost() {
        let r = make_result("https://docs.rs/tokio/latest/tokio/", "tokio - Rust");
        let inp = ScorerInput { result: &r, query: "tokio async runtime rust", engine_rank: 0 };
        let c = project_match::score(&inp);
        assert!(c.delta > 0.0, "project name in title must boost, got {}", c.delta);
        assert_eq!(c.scorer, "project_match");
    }

    #[test]
    fn project_name_in_host_gives_boost() {
        let r = make_result("https://tokio.rs/tokio/tutorial", "Tutorial | Tokio");
        let inp = ScorerInput { result: &r, query: "tokio async runtime", engine_rank: 0 };
        let c = project_match::score(&inp);
        assert!(c.delta > 0.0, "project name in host must boost, got {}", c.delta);
    }

    #[test]
    fn generic_query_with_no_project_token_gets_zero() {
        let r = make_result("https://example.com/page", "Some Page");
        // "the" "and" "for" are all common words, no project token
        let inp = ScorerInput { result: &r, query: "the and for", engine_rank: 0 };
        let c = project_match::score(&inp);
        assert_eq!(c.delta, 0.0, "no project token must give 0.0, got {}", c.delta);
    }

    #[test]
    fn hyphenated_crate_name_is_detected_as_project_token() {
        let r = make_result("https://docs.rs/serde-json/", "serde-json - Rust");
        let inp = ScorerInput { result: &r, query: "serde-json serialization", engine_rank: 0 };
        let c = project_match::score(&inp);
        assert!(c.delta > 0.0, "hyphenated crate name must be detected, got {}", c.delta);
    }
}

// ── snippet_relevance ─────────────────────────────────────────────────────────


mod snippet_relevance_tests {
    use ripweb::search::scoring::{ScorerInput, snippet_relevance};
    use ripweb::search::SearchResult;

    fn make_result(url: &str, snippet: Option<&str>) -> SearchResult {
        SearchResult {
            url: url.to_owned(),
            title: "Title".to_owned(),
            snippet: snippet.map(str::to_owned),
        }
    }

    #[test]
    fn all_query_terms_in_snippet_gives_max_coverage() {
        let r = make_result("https://tokio.rs/", Some("Tokio is an async runtime for Rust"));
        let inp = ScorerInput { result: &r, query: "tokio async runtime", engine_rank: 0 };
        let c = snippet_relevance::score(&inp);
        assert!(c.delta > 0.0, "full coverage must give positive delta, got {}", c.delta);
        assert_eq!(c.scorer, "snippet_relevance");
    }

    #[test]
    fn no_query_terms_in_snippet_gives_zero() {
        let r = make_result("https://example.com/", Some("Completely unrelated content here"));
        let inp = ScorerInput { result: &r, query: "tokio async runtime", engine_rank: 0 };
        let c = snippet_relevance::score(&inp);
        assert_eq!(c.delta, 0.0, "no coverage must give 0.0, got {}", c.delta);
    }

    #[test]
    fn missing_snippet_gives_zero() {
        let r = make_result("https://example.com/", None);
        let inp = ScorerInput { result: &r, query: "tokio async runtime", engine_rank: 0 };
        let c = snippet_relevance::score(&inp);
        assert_eq!(c.delta, 0.0, "no snippet must give 0.0, got {}", c.delta);
    }

    #[test]
    fn partial_coverage_gives_intermediate_delta() {
        // "tokio" matches, "async" matches, "runtime" does not
        let r = make_result("https://tokio.rs/", Some("Tokio and async programming in Rust"));
        let inp = ScorerInput { result: &r, query: "tokio async runtime", engine_rank: 0 };
        let full_r = make_result("https://a.com/", Some("Tokio async runtime in Rust"));
        let full_inp = ScorerInput { result: &full_r, query: "tokio async runtime", engine_rank: 0 };
        let partial = snippet_relevance::score(&inp);
        let full = snippet_relevance::score(&full_inp);
        assert!(
            partial.delta < full.delta,
            "partial coverage ({}) must be less than full ({})",
            partial.delta,
            full.delta
        );
        assert!(partial.delta > 0.0);
    }
}

// ── domain_diversity ──────────────────────────────────────────────────────────

mod domain_diversity_tests {
    use ripweb::search::scoring::domain_diversity;

    #[test]
    fn first_occurrence_gets_zero_penalty() {
        let c = domain_diversity::score_for_occurrence(0);
        assert_eq!(c.delta, 0.0, "first occurrence must get 0.0, got {}", c.delta);
        assert_eq!(c.scorer, "domain_diversity");
    }

    #[test]
    fn second_occurrence_gets_penalty() {
        let c = domain_diversity::score_for_occurrence(1);
        assert!(c.delta < 0.0, "second occurrence must get negative delta, got {}", c.delta);
    }

    #[test]
    fn third_occurrence_gets_larger_penalty_than_second() {
        let second = domain_diversity::score_for_occurrence(1);
        let third = domain_diversity::score_for_occurrence(2);
        assert!(
            third.delta < second.delta,
            "third occurrence penalty ({}) must exceed second ({})",
            third.delta,
            second.delta
        );
    }

    #[test]
    fn penalty_is_deterministic() {
        let a = domain_diversity::score_for_occurrence(3);
        let b = domain_diversity::score_for_occurrence(3);
        assert_eq!(a.delta, b.delta);
    }
}
