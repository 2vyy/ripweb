use url::Url;

/// Normalise a URL for deduplication and cache-key purposes:
/// * strip `#fragment` anchors
/// * strip trailing slashes from the path
///
/// Returns `None` if `raw` cannot be parsed as an absolute URL.
pub fn normalize(raw: &str) -> Option<String> {
    let mut url = Url::parse(raw).ok()?;

    // Only accept absolute URLs (must have a host).
    url.host()?;

    // Drop the fragment — #anchors must not affect dedup.
    url.set_fragment(None);

    // Strip trailing slash from any path segment that isn't the bare root.
    let path = url.path().to_owned();
    if path.len() > 1 && path.ends_with('/') {
        url.set_path(path.trim_end_matches('/'));
    }

    Some(url.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_fragment() {
        let n = normalize("https://docs.rs/tokio#structs").unwrap();
        assert_eq!(n, "https://docs.rs/tokio");
    }

    #[test]
    fn strips_trailing_slash() {
        let n = normalize("https://docs.rs/tokio/").unwrap();
        assert_eq!(n, "https://docs.rs/tokio");
    }

    #[test]
    fn anchor_and_slash_variants_are_identical() {
        let a = normalize("https://docs.rs/tokio#structs").unwrap();
        let b = normalize("https://docs.rs/tokio/").unwrap();
        let c = normalize("https://docs.rs/tokio").unwrap();
        assert_eq!(a, b);
        assert_eq!(b, c);
    }

    #[test]
    fn preserves_query_string() {
        let n = normalize("https://example.com/search?q=rust&page=2").unwrap();
        assert_eq!(n, "https://example.com/search?q=rust&page=2");
    }

    #[test]
    fn strips_fragment_but_keeps_query() {
        let n = normalize("https://example.com/search?q=rust#results").unwrap();
        assert_eq!(n, "https://example.com/search?q=rust");
    }

    #[test]
    fn root_slash_is_kept() {
        // The root path "/" is meaningful and must not be stripped to ""
        let n = normalize("https://example.com/").unwrap();
        assert_eq!(n, "https://example.com/");
    }

    #[test]
    fn returns_none_for_relative_url() {
        assert!(normalize("/relative/path").is_none());
    }

    #[test]
    fn returns_none_for_garbage() {
        assert!(normalize("not a url at all!!!").is_none());
    }
}
