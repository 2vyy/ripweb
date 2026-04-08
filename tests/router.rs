use ripweb::router::{route, PlatformRoute, Route};

// ── Input mode detection ──────────────────────────────────────────────────────

#[test]
fn bare_phrase_routes_to_query() {
    assert!(matches!(route("rust async traits"), Route::Query(_)));
}

#[test]
fn bare_domain_routes_to_query() {
    // No scheme → treated as a search query, not a URL
    assert!(matches!(route("docs.rs/tokio"), Route::Query(_)));
}

#[test]
fn http_url_routes_to_url_mode() {
    assert!(matches!(route("http://example.com/page"), Route::Url(_)));
}

#[test]
fn https_url_routes_to_url_mode() {
    assert!(matches!(route("https://example.com/page"), Route::Url(_)));
}

// ── Platform classification ───────────────────────────────────────────────────

#[test]
fn github_repo_url_classifies_correctly() {
    let Route::Url(PlatformRoute::GitHub { owner, repo }) =
        route("https://github.com/tokio-rs/tokio")
    else {
        panic!("expected GitHub route");
    };
    assert_eq!(owner, "tokio-rs");
    assert_eq!(repo, "tokio");
}

#[test]
fn github_url_with_subpath_still_extracts_owner_repo() {
    let Route::Url(PlatformRoute::GitHub { owner, repo }) =
        route("https://github.com/rust-lang/rust")
    else {
        panic!("expected GitHub route");
    };
    assert_eq!(owner, "rust-lang");
    assert_eq!(repo, "rust");
}

#[test]
fn github_org_only_url_falls_through_to_generic() {
    let Route::Url(route) = route("https://github.com/tokio-rs") else {
        panic!("expected URL route");
    };
    assert!(
        matches!(route, PlatformRoute::Generic(_)),
        "github.com/owner with no repo should be Generic, got {route:?}"
    );
}

#[test]
fn reddit_url_classified_as_reddit() {
    assert!(matches!(
        route("https://www.reddit.com/r/rust/comments/abc/title/"),
        Route::Url(PlatformRoute::Reddit { .. })
    ));
}

#[test]
fn reddit_without_www_classified_as_reddit() {
    assert!(matches!(
        route("https://reddit.com/r/programming/comments/xyz/foo/"),
        Route::Url(PlatformRoute::Reddit { .. })
    ));
}

#[test]
fn hackernews_item_url_extracts_id() {
    let Route::Url(PlatformRoute::HackerNews { item_id }) =
        route("https://news.ycombinator.com/item?id=12345")
    else {
        panic!("expected HackerNews route");
    };
    assert_eq!(item_id, "12345");
}

#[test]
fn hackernews_without_id_falls_through_to_generic() {
    let Route::Url(route) = route("https://news.ycombinator.com/") else {
        panic!("expected URL route");
    };
    assert!(
        matches!(route, PlatformRoute::Generic(_)),
        "HN URL without ?id= should be Generic, got {route:?}"
    );
}

#[test]
fn generic_url_is_passthrough() {
    assert!(matches!(
        route("https://docs.rs/tokio/latest"),
        Route::Url(PlatformRoute::Generic(_))
    ));
}

// ── New platform routes ───────────────────────────────────────────────────────

#[test]
fn wikipedia_wiki_url_extracts_title() {
    let Route::Url(PlatformRoute::Wikipedia { title }) =
        route("https://en.wikipedia.org/wiki/Rust_(programming_language)")
    else {
        panic!("expected Wikipedia route");
    };
    assert_eq!(title, "Rust_(programming_language)");
}

#[test]
fn wikipedia_non_wiki_url_falls_through_to_generic() {
    assert!(matches!(
        route("https://en.wikipedia.org/about"),
        Route::Url(PlatformRoute::Generic(_))
    ));
}

#[test]
fn stackoverflow_question_url_extracts_id() {
    let Route::Url(PlatformRoute::StackOverflow { question_id }) =
        route("https://stackoverflow.com/questions/57430839/why-does-rust-borrow-checker")
    else {
        panic!("expected StackOverflow route");
    };
    assert_eq!(question_id, 57430839);
}

#[test]
fn stackoverflow_non_question_url_falls_through_to_generic() {
    assert!(matches!(
        route("https://stackoverflow.com/tags/rust"),
        Route::Url(PlatformRoute::Generic(_))
    ));
}

#[test]
fn arxiv_abs_url_extracts_paper_id() {
    let Route::Url(PlatformRoute::ArXiv { paper_id }) =
        route("https://arxiv.org/abs/1706.03762")
    else {
        panic!("expected ArXiv route");
    };
    assert_eq!(paper_id, "1706.03762");
}

#[test]
fn arxiv_pdf_url_extracts_paper_id() {
    let Route::Url(PlatformRoute::ArXiv { paper_id }) =
        route("https://arxiv.org/pdf/1706.03762.pdf")
    else {
        panic!("expected ArXiv route");
    };
    assert_eq!(paper_id, "1706.03762");
}
