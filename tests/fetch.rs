/// Tests for the fetch layer: HTTP client retry logic, XDG cache, and
/// llms.txt auto-discovery. Network-simulation tests (politeness, preflight)
/// live in network.rs.
use std::time::Duration;

use ripweb::fetch::{
    cache::Cache,
    client::{FetchError, RetryConfig, build_client, fetch_with_retry},
    llms_txt::fetch_llms_txt,
};
use url::Url;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use tempfile::TempDir;

// ── Helpers ───────────────────────────────────────────────────────────────────

fn fast_retry() -> RetryConfig {
    RetryConfig {
        max_retries: 2,
        base_delay: Duration::from_millis(1),
    }
}

fn temp_cache() -> (TempDir, Cache) {
    let dir = TempDir::new().unwrap();
    let cache = Cache::new(dir.path().to_path_buf(), Duration::from_secs(86400));
    (dir, cache)
}

// ── HTTP client — happy path ──────────────────────────────────────────────────

#[tokio::test]
async fn client_succeeds_on_200() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string("hello"))
        .mount(&server)
        .await;

    let client = build_client().unwrap();
    let resp = fetch_with_retry(&client, &server.uri(), &fast_retry())
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
}

// ── HTTP client — retry on 429 ────────────────────────────────────────────────

#[tokio::test]
async fn client_retries_twice_on_429_then_succeeds() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(429))
        .up_to_n_times(2)
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
        .mount(&server)
        .await;

    let client = build_client().unwrap();
    let resp = fetch_with_retry(&client, &server.uri(), &fast_retry())
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn client_fails_after_max_retries_on_429() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(429))
        .mount(&server)
        .await;

    let client = build_client().unwrap();
    let err = fetch_with_retry(&client, &server.uri(), &fast_retry())
        .await
        .unwrap_err();
    assert!(
        matches!(err, FetchError::RateLimited),
        "expected RateLimited, got: {err}"
    );
}

// ── HTTP client — retry on 503 / 504 ─────────────────────────────────────────

#[tokio::test]
async fn client_retries_on_503_then_succeeds() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(503))
        .up_to_n_times(1)
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string("recovered"))
        .mount(&server)
        .await;

    let client = build_client().unwrap();
    let resp = fetch_with_retry(&client, &server.uri(), &fast_retry())
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn client_fails_after_max_retries_on_503() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;

    let client = build_client().unwrap();
    let err = fetch_with_retry(&client, &server.uri(), &fast_retry())
        .await
        .unwrap_err();
    assert!(
        matches!(err, FetchError::ServerError(503)),
        "expected ServerError(503), got: {err}"
    );
}

// ── HTTP client — non-retryable errors ───────────────────────────────────────

#[tokio::test]
async fn client_does_not_retry_on_404() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let client = build_client().unwrap();
    let err = fetch_with_retry(&client, &server.uri(), &fast_retry())
        .await
        .unwrap_err();
    assert!(
        matches!(err, FetchError::ServerError(404)),
        "expected ServerError(404), got: {err}"
    );
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

// ── Cache — round-trip ────────────────────────────────────────────────────────

#[tokio::test]
async fn cache_put_then_get_returns_same_bytes() {
    let (_dir, cache) = temp_cache();
    let url = "https://example.com/page";
    let data = b"hello cached world";

    cache.put(url, data).await.unwrap();
    let got = cache.get(url).await.unwrap();
    assert_eq!(got, data);
}

#[tokio::test]
async fn cache_get_returns_none_for_missing_entry() {
    let (_dir, cache) = temp_cache();
    assert!(cache.get("https://example.com/missing").await.is_none());
}

#[tokio::test]
async fn cache_fragment_variants_share_cache_slot() {
    let (_dir, cache) = temp_cache();
    cache
        .put("https://docs.rs/tokio", b"tokio docs content")
        .await
        .unwrap();

    let got = cache.get("https://docs.rs/tokio#structs").await;
    assert!(got.is_some(), "fragment variant did not hit cache");
    assert_eq!(got.unwrap(), b"tokio docs content");
}

// ── Cache — staleness ─────────────────────────────────────────────────────────

#[tokio::test]
async fn cache_stale_entry_returns_none() {
    let (dir, cache) = temp_cache();
    let url = "https://example.com/stale";

    cache.put(url, b"old content").await.unwrap();

    let path = cache.cache_path(url);
    let old_time = std::time::SystemTime::now() - Duration::from_secs(86400 * 2); // 2 days ago
    let ft = filetime::FileTime::from_system_time(old_time);
    filetime::set_file_mtime(&path, ft).unwrap();

    assert!(
        cache.get(url).await.is_none(),
        "stale entry was returned instead of None"
    );

    drop(dir);
}

#[tokio::test]
async fn cache_fresh_entry_is_returned() {
    let (_dir, cache) = temp_cache();
    let url = "https://example.com/fresh";
    cache.put(url, b"fresh content").await.unwrap();
    assert!(cache.get(url).await.is_some());
}

// ── llms.txt discovery ────────────────────────────────────────────────────────

const LLMS_BODY: &str = "# API Reference\n\nThis site's LLM-optimised context file.\nAll endpoints are documented here.";

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

    Mock::given(method("GET"))
        .and(path("/llms.txt"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

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

#[tokio::test]
async fn llms_txt_success_means_html_page_is_never_fetched() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/llms.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_string(LLMS_BODY))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string("<html>should not be fetched</html>"),
        )
        .expect(0)
        .mount(&server)
        .await;

    let client = build_client().unwrap();
    let origin = Url::parse(&server.uri()).unwrap();
    let result = fetch_llms_txt(&client, &origin).await;

    assert!(result.is_some());
}

// ── Domain Politeness ─────────────────────────────────────────────────────────

use ripweb::fetch::politeness::DomainSemaphores;
use std::time::Instant;

/// Three tasks hitting the same host concurrently must be serialised to at
/// most 3-at-a-time. A 4th attempt must wait for one of the first three.
#[tokio::test]
async fn politeness_limits_concurrent_requests_per_host() {
    let sems = DomainSemaphores::new(3);

    let start = Instant::now();

    let p1 = sems.acquire("example.com").await;
    let p2 = sems.acquire("example.com").await;
    let p3 = sems.acquire("example.com").await;

    let sems_clone = sems.clone();
    let waiter = tokio::spawn(async move {
        let _p4 = sems_clone.acquire("example.com").await;
        Instant::now()
    });

    tokio::time::sleep(Duration::from_millis(50)).await;
    drop(p1);

    let unblocked_at = waiter.await.unwrap();
    let waited = unblocked_at.duration_since(start);

    assert!(
        waited >= Duration::from_millis(40),
        "4th request was not blocked by the semaphore (waited {:?})",
        waited
    );

    drop(p2);
    drop(p3);
}

/// Different hosts must get independent semaphores.
#[tokio::test]
async fn politeness_different_hosts_are_independent() {
    let sems = DomainSemaphores::new(3);

    let _a1 = sems.acquire("alpha.example.com").await;
    let _a2 = sems.acquire("alpha.example.com").await;
    let _a3 = sems.acquire("alpha.example.com").await;

    let got_b =
        tokio::time::timeout(Duration::from_millis(50), sems.acquire("beta.example.com")).await;

    assert!(
        got_b.is_ok(),
        "request to different host was blocked by unrelated semaphore"
    );
}

/// Host key must be normalised to lowercase so `example.com` and `EXAMPLE.COM`
/// share the same semaphore slot.
#[tokio::test]
async fn politeness_host_key_is_case_insensitive() {
    let sems = DomainSemaphores::new(1);

    let p1 = sems.acquire("Example.Com").await;

    let blocked =
        tokio::time::timeout(Duration::from_millis(30), sems.acquire("EXAMPLE.COM")).await;

    assert!(blocked.is_err(), "uppercase variant bypassed the semaphore");
    drop(p1);
}

// ── Pre-flight Checks ─────────────────────────────────────────────────────────

use ripweb::fetch::preflight::{PreflightCheck, PreflightError};

#[test]
fn preflight_accepts_html_response() {
    let result = PreflightCheck::validate(Some("text/html; charset=utf-8"), Some(1024 * 100));
    assert!(result.is_ok());
}

#[test]
fn preflight_accepts_missing_content_length() {
    let result = PreflightCheck::validate(Some("text/html"), None);
    assert!(result.is_ok());
}

#[test]
fn preflight_rejects_pdf() {
    let err = PreflightCheck::validate(Some("application/pdf"), Some(512)).unwrap_err();
    assert!(matches!(err, PreflightError::NonTextMime(_)));
}

#[test]
fn preflight_rejects_zip() {
    let err = PreflightCheck::validate(Some("application/zip"), Some(512)).unwrap_err();
    assert!(matches!(err, PreflightError::NonTextMime(_)));
}

#[test]
fn preflight_rejects_video() {
    let err = PreflightCheck::validate(Some("video/mp4"), Some(512)).unwrap_err();
    assert!(matches!(err, PreflightError::NonTextMime(_)));
}

#[test]
fn preflight_rejects_response_over_5mb() {
    const FIVE_MB: u64 = 5 * 1024 * 1024;
    let err = PreflightCheck::validate(Some("text/html"), Some(FIVE_MB + 1)).unwrap_err();
    assert!(matches!(err, PreflightError::TooLarge(_)));
}

#[test]
fn preflight_accepts_response_exactly_at_5mb() {
    const FIVE_MB: u64 = 5 * 1024 * 1024;
    let result = PreflightCheck::validate(Some("text/html"), Some(FIVE_MB));
    assert!(result.is_ok());
}

#[test]
fn preflight_accepts_plain_text() {
    let result = PreflightCheck::validate(Some("text/plain"), Some(1024));
    assert!(result.is_ok());
}

#[test]
fn preflight_rejects_missing_content_type() {
    let err = PreflightCheck::validate(None, None).unwrap_err();
    assert!(matches!(err, PreflightError::MissingContentType));
}
