/// Tracking / noise query parameters that add no semantic value.
static STRIP_PARAMS: &[&str] = &[
    // UTM campaign tracking
    "utm_source",
    "utm_medium",
    "utm_campaign",
    "utm_term",
    "utm_content",
    "utm_id",
    // Social / ad platform click IDs
    "fbclid",
    "gclid",
    "gclsrc",
    "dclid",
    "gbraid",
    "wbraid",
    "msclkid",
    "twclid",
    "ttclid",
    // Miscellaneous analytics
    "mc_cid",
    "mc_eid",
    "_hsenc",
    "_hsmi",
    "hsCtaTracking",
    "ref",
    "referrer",
    "source",
    "campaign",
];

/// Strip known tracking / noise parameters from a URL string.
///
/// Returns the URL with those parameters removed.  If the input is not a
/// parseable URL it is returned unchanged.  Fragment identifiers are also
/// removed (they carry no server-side meaning).
pub fn strip_tracking(url: &str) -> String {
    let Ok(mut parsed) = url::Url::parse(url) else {
        return url.to_owned();
    };

    // Collect the pairs we want to keep.
    let kept: Vec<(String, String)> = parsed
        .query_pairs()
        .filter(|(k, _)| !STRIP_PARAMS.contains(&k.as_ref()))
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();

    // Rebuild the query string (or remove it entirely).
    if kept.is_empty() {
        parsed.set_query(None);
    } else {
        parsed.query_pairs_mut().clear().extend_pairs(&kept);
    }

    // Always drop fragments.
    parsed.set_fragment(None);

    parsed.into()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_utm_params() {
        let url = "https://example.com/page?utm_source=newsletter&utm_medium=email&id=42";
        let result = strip_tracking(url);
        assert!(!result.contains("utm_source"), "utm_source leaked: {result}");
        assert!(!result.contains("utm_medium"), "utm_medium leaked: {result}");
        assert!(result.contains("id=42"), "real param lost: {result}");
    }

    #[test]
    fn strips_fbclid() {
        let url = "https://example.com/?fbclid=IwAR3abc123";
        let result = strip_tracking(url);
        assert!(!result.contains("fbclid"), "fbclid leaked: {result}");
    }

    #[test]
    fn removes_fragment() {
        let url = "https://example.com/page#section-3";
        let result = strip_tracking(url);
        assert!(!result.contains('#'), "fragment leaked: {result}");
    }

    #[test]
    fn keeps_non_tracking_params() {
        let url = "https://example.com/search?q=rust+async&page=2";
        let result = strip_tracking(url);
        assert!(result.contains("q=rust"), "q param lost: {result}");
        assert!(result.contains("page=2"), "page param lost: {result}");
    }

    #[test]
    fn invalid_url_returned_unchanged() {
        let input = "not a url at all";
        assert_eq!(strip_tracking(input), input);
    }

    #[test]
    fn all_params_stripped_removes_query_string() {
        let url = "https://example.com/?utm_source=x&fbclid=y";
        let result = strip_tracking(url);
        assert!(!result.contains('?'), "empty query string leaked: {result}");
    }
}
