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
