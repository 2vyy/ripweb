use ripweb::search::{SearchResult, fusion::rrf_fuse};

fn r(url: &str) -> SearchResult {
    SearchResult {
        url: url.to_owned(),
        title: url.to_owned(),
        snippet: None,
    }
}

#[test]
fn rrf_fuse_empty_lists_returns_empty() {
    let result = rrf_fuse(&[]);
    assert!(result.is_empty());
}

#[test]
fn rrf_fuse_single_engine_preserves_order() {
    let list = vec![r("https://a.com"), r("https://b.com"), r("https://c.com")];
    let fused = rrf_fuse(&[("ddg", list)]);
    assert_eq!(fused[0].url, "https://a.com");
    assert_eq!(fused[1].url, "https://b.com");
    assert_eq!(fused[2].url, "https://c.com");
}

#[test]
fn rrf_fuse_deduplicates_by_url_normalised() {
    let ddg = vec![r("https://docs.rs/tokio"), r("https://tokio.rs")];
    let marginalia = vec![r("https://tokio.rs/"), r("https://crates.io/crates/tokio")];
    let fused = rrf_fuse(&[("ddg", ddg), ("marginalia", marginalia)]);
    let urls: Vec<&str> = fused.iter().map(|r| r.url.as_str()).collect();
    let tokio_count = urls.iter().filter(|&&u| u.contains("tokio.rs")).count();
    assert_eq!(tokio_count, 1, "tokio.rs deduped: {:?}", urls);
    assert_eq!(fused.len(), 3, "unexpected count: {:?}", urls);
}

#[test]
fn rrf_fuse_promotes_result_appearing_in_both_engines() {
    let ddg = vec![r("https://tokio.rs"), r("https://docs.rs/tokio")];
    let marginalia = vec![r("https://tokio.rs"), r("https://ryhl.io/blog/")];
    let fused = rrf_fuse(&[("ddg", ddg), ("marginalia", marginalia)]);
    assert_eq!(
        fused[0].url,
        "https://tokio.rs",
        "consensus URL must rank first: {:?}",
        fused.iter().map(|r| &r.url).collect::<Vec<_>>()
    );
}

#[test]
fn rrf_fuse_trailing_slash_treated_as_same_url() {
    let ddg = vec![r("https://tokio.rs/tokio/tutorial")];
    let marginalia = vec![r("https://tokio.rs/tokio/tutorial/")];
    let fused = rrf_fuse(&[("ddg", ddg), ("marginalia", marginalia)]);
    assert_eq!(
        fused.len(),
        1,
        "trailing slash variant must dedup: {:?}",
        fused.iter().map(|r| &r.url).collect::<Vec<_>>()
    );
}

#[test]
fn rrf_fuse_case_insensitive_scheme_and_host() {
    let ddg = vec![r("https://Docs.RS/tokio/latest/tokio/")];
    let marginalia = vec![r("https://docs.rs/tokio/latest/tokio/")];
    let fused = rrf_fuse(&[("ddg", ddg), ("marginalia", marginalia)]);
    assert_eq!(
        fused.len(),
        1,
        "case-normalised dedup must work: {:?}",
        fused.iter().map(|r| &r.url).collect::<Vec<_>>()
    );
}
