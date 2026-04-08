use ripweb::fetch::{client::build_client, llms_txt::fetch_llms_txt};
use url::Url;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const LLMS_BODY: &str = "# API Reference\n\nThis site's LLM-optimised context file.\nAll endpoints are documented here.";

// ── Happy paths ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn llms_txt_found_at_root_path() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/llms.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_string(LLMS_BODY))
        .mount(&server)
        .await;

    let client = build_client().unwrap();
    let origin = Url::parse(&server.uri()).unwrap();
    let result = fetch_llms_txt(&client, &origin).await;

    assert!(result.is_some(), "expected Some, got None");
    assert!(result.unwrap().contains("LLM-optimised"));
}

#[tokio::test]
async fn llms_txt_found_at_well_known_fallback() {
    let server = MockServer::start().await;

    // Root path → 404
    Mock::given(method("GET"))
        .and(path("/llms.txt"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    // Well-known path → 200
    Mock::given(method("GET"))
        .and(path("/.well-known/llms.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_string(LLMS_BODY))
        .mount(&server)
        .await;

    let client = build_client().unwrap();
    let origin = Url::parse(&server.uri()).unwrap();
    let result = fetch_llms_txt(&client, &origin).await;

    assert!(result.is_some(), "well-known fallback should return Some");
    assert!(result.unwrap().contains("LLM-optimised"));
}

// ── Failure paths ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn llms_txt_returns_none_when_both_paths_404() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let client = build_client().unwrap();
    let origin = Url::parse(&server.uri()).unwrap();
    let result = fetch_llms_txt(&client, &origin).await;

    assert!(result.is_none(), "expected None when both paths 404");
}

// ── Short-circuit proof ───────────────────────────────────────────────────────

/// When llms.txt is found, the root HTML page must NOT be fetched.
/// This proves that callers can skip the HTML crawler entirely based on
/// the return value of `fetch_llms_txt`.
#[tokio::test]
async fn llms_txt_success_means_html_page_is_never_needed() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/llms.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_string(LLMS_BODY))
        .mount(&server)
        .await;

    // Root path MUST NOT be fetched — zero expected hits.
    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_string("<html>should not be fetched</html>"))
        .expect(0)
        .mount(&server)
        .await;

    let client = build_client().unwrap();
    let origin = Url::parse(&server.uri()).unwrap();
    let result = fetch_llms_txt(&client, &origin).await;

    assert!(result.is_some());
    // wiremock enforces expect(0) when the MockServer drops
}
