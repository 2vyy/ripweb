use ripweb::fetch::normalize::normalize;

#[test]
fn normalize_strips_fragments_and_tracking_params() {
    let normalized = normalize("https://example.com/path/?utm_source=test#section");
    assert_eq!(normalized.as_deref(), Some("https://example.com/path"));
}

#[test]
fn normalize_preserves_meaningful_query_params() {
    let normalized = normalize("https://example.com/search?q=rust&page=2");
    assert_eq!(
        normalized.as_deref(),
        Some("https://example.com/search?q=rust&page=2")
    );
}

#[test]
fn normalize_rejects_non_absolute_urls() {
    assert!(normalize("/relative/path").is_none());
}
