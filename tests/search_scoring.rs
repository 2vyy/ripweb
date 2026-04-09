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
